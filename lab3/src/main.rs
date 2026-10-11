mod cpu;
mod gpu;
mod image_out;
mod matrix;
mod timing;
mod verify;

use std::path::PathBuf;

use cpu::deinterleave;
use gpu::{GpuEngine, GpuTimedResult, GpuVariant};
use matrix::{FillMode, MAX_DIM, MAX_DIM_LARGE, generate_b};
use timing::{fmt_ms, time_call};
use verify::matrices_equal;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    All,
    Cpu,
    Gpu,
}

struct Args {
    k: usize,
    m: usize,
    mode: Mode,
    reps: usize,
    variants: Vec<GpuVariant>,
    save_image: bool,
    out_dir: PathBuf,
    fill: FillMode,
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args {
        k: 256,
        m: 256,
        mode: Mode::All,
        reps: 10,
        variants: vec![GpuVariant::Coalesced],
        save_image: false,
        out_dir: PathBuf::from("output"),
        fill: FillMode::Pattern,
    };
    let mut variant_set = false;
    let mut fill_set = false;
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--k" => args.k = parse_usize(&mut it, "--k")?,
            "--m" => args.m = parse_usize(&mut it, "--m")?,
            "--reps" => args.reps = parse_usize(&mut it, "--reps")?,
            "--out-dir" => {
                args.out_dir = PathBuf::from(
                    it.next()
                        .ok_or_else(|| "нет значения после --out-dir".to_string())?,
                );
            }
            "--save-image" => args.save_image = true,
            "--fill" => {
                let v = it
                    .next()
                    .ok_or_else(|| "нет значения после --fill".to_string())?;
                args.fill = match v.as_str() {
                    "pattern" | "visual" => FillMode::Pattern,
                    "random" => FillMode::Random,
                    other => {
                        return Err(format!("неизвестный --fill {other} (pattern|random)"));
                    }
                };
                fill_set = true;
            }
            "--gpu-variant" => {
                let v = it
                    .next()
                    .ok_or_else(|| "нет значения после --gpu-variant".to_string())?;
                args.variants = parse_variants(&v)?;
                variant_set = true;
            }
            "--mode" => {
                let v = it
                    .next()
                    .ok_or_else(|| "нет значения после --mode".to_string())?;
                args.mode = match v.as_str() {
                    "all" => Mode::All,
                    "cpu" => Mode::Cpu,
                    "gpu" => Mode::Gpu,
                    other => {
                        return Err(format!(
                            "неизвестный --mode {other} (ожидается all|cpu|gpu)"
                        ));
                    }
                };
            }
            "-h" | "--help" => {
                print_help();
                std::process::exit(0);
            }
            other => return Err(format!("неизвестный аргумент: {other}")),
        }
    }
    // PNG only makes sense with a visible pattern.
    if args.save_image && !fill_set {
        args.fill = FillMode::Pattern;
    }
    if !variant_set && (args.mode == Mode::All || args.mode == Mode::Gpu) {
        args.variants = GpuVariant::all().to_vec();
    }
    if args.k == 0 || args.m == 0 {
        return Err("K и M должны быть > 0".into());
    }
    let allow_large = args.variants.len() == 1 && args.variants[0] == GpuVariant::Oom;
    let max_dim = if allow_large { MAX_DIM_LARGE } else { MAX_DIM };
    if args.k > max_dim || args.m > max_dim {
        return Err(format!(
            "K и M должны быть ≤ {max_dim}{}",
            if !allow_large {
                " (для больших — --gpu-variant oom)"
            } else {
                ""
            }
        ));
    }
    if args.reps == 0 {
        return Err("--reps должен быть > 0".into());
    }
    Ok(args)
}

fn parse_variants(s: &str) -> Result<Vec<GpuVariant>, String> {
    if s == "all" {
        return Ok(GpuVariant::all().to_vec());
    }
    let mut out = Vec::new();
    for part in s.split(',') {
        let part = part.trim();
        let v = GpuVariant::parse(part).ok_or_else(|| {
            format!(
                "неизвестный --gpu-variant '{part}' (basic|naive|coalesced|streams|vram|oom|all)"
            )
        })?;
        out.push(v);
    }
    if out.is_empty() {
        return Err("пустой --gpu-variant".into());
    }
    Ok(out)
}

fn parse_usize(it: &mut impl Iterator<Item = String>, flag: &str) -> Result<usize, String> {
    let v = it
        .next()
        .ok_or_else(|| format!("нет значения после {flag}"))?;
    v.parse()
        .map_err(|_| format!("некорректное число для {flag}: {v}"))
}

fn print_help() {
    eprintln!(
        "lab3 — деинтерливинг строк (вариант 28)\n\
         \n\
         Usage:\n\
           cargo run --release -- --k <K> --m <M> --mode all --gpu-variant all --reps 3 --save-image\n\
         \n\
         Options:\n\
           --mode all|cpu|gpu\n\
           --gpu-variant basic|naive|coalesced|streams|vram|oom|all\n\
           --reps N\n\
           --save-image           PNG в output/ (включает fill=pattern)\n\
           --out-dir DIR\n\
           --fill pattern|random  pattern по умолчанию\n\
         \n\
         Смысл PNG: B = полосатый микс; A1 = круг; A2 = диагонали.\n\
         Смотри в первую очередь output/panel_B_A1_A2_*.png"
    );
}

fn main() {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("ошибка: {e}");
            print_help();
            std::process::exit(2);
        }
    };

    let b = generate_b(args.k, args.m, args.fill);

    let mut cpu_a1 = None;
    let mut cpu_a2 = None;
    let mut cpu_ms: Option<f64> = None;

    if args.mode == Mode::All || args.mode == Mode::Cpu {
        let _ = deinterleave(&b);
        let t = time_call(|| {
            let mut last = None;
            for _ in 0..args.reps {
                last = Some(deinterleave(&b));
            }
            last.unwrap()
        });
        let per = t.elapsed / args.reps as u32;
        cpu_ms = Some(per.as_secs_f64() * 1000.0);
        cpu_a1 = Some(t.value.0);
        cpu_a2 = Some(t.value.1);
    }

    let mut gpu_a1 = None;
    let mut gpu_a2 = None;
    let mut results: Vec<(GpuTimedResult, bool)> = Vec::new();
    let mut ok = true;
    let mut gpu_name = String::new();

    if args.mode == Mode::All || args.mode == Mode::Gpu {
        let gpu = match GpuEngine::new() {
            Ok(g) => g,
            Err(e) => {
                eprintln!("ошибка GPU init: {e}");
                std::process::exit(1);
            }
        };
        gpu_name = gpu.device_name().unwrap_or_else(|_| "?".into());

        for &variant in &args.variants {
            match gpu.run(variant, &b, args.reps) {
                Ok(r) => {
                    let mut verify_ok = true;
                    if let (Some(c1), Some(c2)) = (cpu_a1.as_ref(), cpu_a2.as_ref()) {
                        verify_ok = matrices_equal(c1, &r.a1).ok && matrices_equal(c2, &r.a2).ok;
                        ok &= verify_ok;
                    }
                    results.push((r, verify_ok));
                }
                Err(e) => {
                    eprintln!("{}: {e}", variant.label());
                    ok = false;
                }
            }
        }

        for (r, _) in &results {
            if r.variant == GpuVariant::Coalesced {
                gpu_a1 = Some(r.a1.clone());
                gpu_a2 = Some(r.a2.clone());
                break;
            }
        }
        if gpu_a1.is_none() {
            if let Some((r, _)) = results.last() {
                gpu_a1 = Some(r.a1.clone());
                gpu_a2 = Some(r.a2.clone());
            }
        }
    }

    let coal = results
        .iter()
        .find(|(r, _)| r.variant == GpuVariant::Coalesced)
        .map(|(r, _)| r);

    println!(
        "{}×{} → {}×{}  reps={}  {}",
        b.rows,
        b.cols,
        args.k,
        args.m,
        args.reps,
        if gpu_name.is_empty() {
            String::new()
        } else {
            gpu_name
        }
    );
    // Методичка: единый формат времени + GPU через CUDA events (ядро).
    // H2D/D2H в отчёт не выводим — копирования не входят в обязательный замер.
    println!(
        "{:<10} {:>12} {:>8} {:>8}  {}",
        "вариант", "время", "vsCPU", "vsCoal", "ст"
    );

    if let Some(cms) = cpu_ms {
        let vs_coal = coal
            .map(|c| {
                if c.kernel_ms > 0.0 {
                    format!("{:.2}×", cms as f32 / c.kernel_ms)
                } else {
                    "—".into()
                }
            })
            .unwrap_or_else(|| "—".into());
        println!(
            "{:<10} {:>12} {:>8} {:>8}  {}",
            "cpu",
            fmt_ms(cms as f32),
            "1.00×",
            vs_coal,
            "—"
        );
    }

    for (r, verify_ok) in &results {
        let vs_cpu = match cpu_ms {
            Some(cms) if r.kernel_ms > 0.0 => format!("{:.2}×", cms as f32 / r.kernel_ms),
            _ => "—".into(),
        };
        let vs_coal = match coal {
            Some(c) if r.kernel_ms > 0.0 => format!("{:.2}×", c.kernel_ms / r.kernel_ms),
            _ => "—".into(),
        };
        let st = if cpu_a1.is_none() {
            "—"
        } else if *verify_ok {
            "OK"
        } else {
            "FAIL"
        };
        // GPU «время» = kernel (CUDA events), не включая H2D/D2H.
        println!(
            "{:<10} {:>12} {:>8} {:>8}  {st}",
            r.variant.label(),
            fmt_ms(r.kernel_ms),
            vs_cpu,
            vs_coal,
        );
    }

    let a1_show = cpu_a1.as_ref().or(gpu_a1.as_ref());
    let a2_show = cpu_a2.as_ref().or(gpu_a2.as_ref());

    if args.save_image {
        if let (Some(a1), Some(a2)) = (a1_show, a2_show) {
            match image_out::save_deinterleave_set(&b, a1, a2, &args.out_dir) {
                Ok(paths) => {
                    if let Some(panel) = paths.iter().find(|p| {
                        p.file_name()
                            .and_then(|n| n.to_str())
                            .is_some_and(|n| n.starts_with("panel"))
                    }) {
                        println!("png: {}", panel.display());
                    }
                }
                Err(e) => {
                    eprintln!("png: {e}");
                    ok = false;
                }
            }
        }
    }

    if !ok {
        std::process::exit(1);
    }
}
