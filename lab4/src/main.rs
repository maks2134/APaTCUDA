mod array;
mod cpu;
mod gpu;
mod timing;
mod verify;

use array::{FillMode, MAX_N, MIN_N, generate_a};
use cpu::percentiles;
use gpu::{GpuEngine, GpuTimedResult, GpuVariant};
use timing::{fmt_ms, time_call};
use verify::{percentiles_equal, print_percentiles};


#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    All,
    Cpu,
    Gpu,
}

struct Args {
    n: usize,
    mode: Mode,
    reps: usize,
    variants: Vec<GpuVariant>,
    fill: FillMode,
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args {
        n: 100_000,
        mode: Mode::All,
        reps: 10,
        variants: vec![GpuVariant::Shared],
        fill: FillMode::Random,
    };
    let mut variant_set = false;
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--n" => args.n = parse_usize(&mut it, "--n")?,
            "--reps" => args.reps = parse_usize(&mut it, "--reps")?,
            "--fill" => {
                let v = it
                    .next()
                    .ok_or_else(|| "нет значения после --fill".to_string())?;
                args.fill = match v.as_str() {
                    "pattern" => FillMode::Pattern,
                    "random" => FillMode::Random,
                    other => {
                        return Err(format!("неизвестный --fill {other} (pattern|random)"));
                    }
                };
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
    if !variant_set && (args.mode == Mode::All || args.mode == Mode::Gpu) {
        args.variants = GpuVariant::all().to_vec();
    }
    if args.n < MIN_N || args.n > MAX_N {
        return Err(format!("N должен быть в [{MIN_N}, {MAX_N}]"));
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
            format!("неизвестный --gpu-variant '{part}' (basic|shared|streams|vram|oom|all)")
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
        "lab4 — перцентили 25/50/75 (вариант 28), bitonic sort\n\
         \n\
         Usage:\n\
           cargo run --release -- --n 1000000 --mode all --gpu-variant all --reps 1\n\
         \n\
         Options:\n\
           --n N                    размер массива [{MIN_N}..{MAX_N}]\n\
           --mode all|cpu|gpu\n\
           --gpu-variant basic|shared|streams|vram|oom|all\n\
           --reps N\n\
           --fill random|pattern"
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

    let a = generate_a(args.n, args.fill);

    let mut cpu_p = None;
    let mut cpu_ms: Option<f64> = None;

    if args.mode == Mode::All || args.mode == Mode::Cpu {
        let _ = percentiles(&a);
        let t = time_call(|| {
            let mut last = None;
            for _ in 0..args.reps {
                last = Some(percentiles(&a));
            }
            last.unwrap()
        });
        let per = t.elapsed / args.reps as u32;
        cpu_ms = Some(per.as_secs_f64() * 1000.0);
        cpu_p = Some(t.value);
    }

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
            match gpu.run(variant, &a, args.reps) {
                Ok(r) => {
                    let mut verify_ok = true;
                    if let Some(ref c) = cpu_p {
                        verify_ok = percentiles_equal(c, &r.percentiles);
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
    }

    let shared = results
        .iter()
        .find(|(r, _)| r.variant == GpuVariant::Shared)
        .map(|(r, _)| r);

    println!(
        "N={}  reps={}  {}",
        args.n,
        args.reps,
        if gpu_name.is_empty() {
            String::new()
        } else {
            gpu_name
        }
    );
    println!(
        "{:<10} {:>12} {:>8} {:>8}  {}",
        "вариант", "время", "vsCPU", "vsShared", "ст"
    );

    if let Some(cms) = cpu_ms {
        let vs_shared = shared
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
            vs_shared,
            "—"
        );
    }

    for (r, verify_ok) in &results {
        let vs_cpu = match cpu_ms {
            Some(cms) if r.kernel_ms > 0.0 => format!("{:.2}×", cms as f32 / r.kernel_ms),
            _ => "—".into(),
        };
        let vs_shared = match shared {
            Some(c) if r.kernel_ms > 0.0 => format!("{:.2}×", c.kernel_ms / r.kernel_ms),
            _ => "—".into(),
        };
        let st = if cpu_p.is_none() {
            "—"
        } else if *verify_ok {
            "OK"
        } else {
            "FAIL"
        };
        println!(
            "{:<10} {:>12} {:>8} {:>8}  {st}",
            r.variant.label(),
            fmt_ms(r.kernel_ms),
            vs_cpu,
            vs_shared,
        );
    }

    if let Some(ref p) = cpu_p {
        print_percentiles("CPU", p);
    } else if let Some((r, _)) = results.first() {
        print_percentiles("GPU", &r.percentiles);
    }

    if !ok {
        std::process::exit(1);
    }
}
