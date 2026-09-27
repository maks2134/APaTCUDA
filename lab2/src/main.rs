#[cfg(not(target_arch = "aarch64"))]
compile_error!("эта лаба собирается только под aarch64 (Apple Silicon / ARM64)");

mod cache_params;
mod matrix;
mod mul_l3;
mod mul_neon;
mod timing;
mod verify;

use cache_params::{TileHierarchy, choose_tiles, detect_cache_sizes, microblock_bytes};
use matrix::BlockMatrix;
use mul_l3::matmul_l3;
use mul_neon::matmul_neon;
use timing::time_call;
use verify::matrices_close;

const SEED: u32 = 42;
const TOL: f32 = 1e-4;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    All,
    Neon,
    L3,
}

struct Args {
    l: usize,
    m: usize,
    n: usize,
    mode: Mode,
    l3_bytes: Option<usize>,
    reps: usize,
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args {
        l: 16,
        m: 16,
        n: 16,
        mode: Mode::All,
        l3_bytes: None,
        reps: 1,
    };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--l" => args.l = parse_usize(&mut it, "--l")?,
            "--m" => args.m = parse_usize(&mut it, "--m")?,
            "--n" => args.n = parse_usize(&mut it, "--n")?,
            "--l3-bytes" => args.l3_bytes = Some(parse_usize(&mut it, "--l3-bytes")?),
            "--reps" => args.reps = parse_usize(&mut it, "--reps")?,
            "--mode" => {
                let v = it
                    .next()
                    .ok_or_else(|| "нет значения после --mode".to_string())?;
                args.mode = match v.as_str() {
                    "all" => Mode::All,
                    "neon" => Mode::Neon,
                    "l3" | "cache" => Mode::L3,
                    other => {
                        return Err(format!(
                            "неизвестный --mode {other} (ожидается all|neon|l3)"
                        ));
                    }
                };
            }
            other => return Err(format!("неизвестный аргумент: {other}")),
        }
    }
    if args.l == 0 || args.m == 0 || args.n == 0 {
        return Err("L, M, N должны быть > 0".into());
    }
    if args.reps == 0 {
        return Err("--reps должен быть > 0".into());
    }
    Ok(args)
}

fn parse_usize(it: &mut impl Iterator<Item = String>, flag: &str) -> Result<usize, String> {
    let v = it
        .next()
        .ok_or_else(|| format!("нет значения после {flag}"))?;
    v.parse()
        .map_err(|_| format!("некорректное число для {flag}: {v}"))
}

fn mode_label(mode: Mode) -> &'static str {
    match mode {
        Mode::All => "all (C1+C2)",
        Mode::Neon => "neon (только C1)",
        Mode::L3 => "l3 (только C2)",
    }
}

fn fmt_bytes(n: usize) -> String {
    if n >= 1024 * 1024 {
        format!("{:.2} MiB", n as f64 / (1024.0 * 1024.0))
    } else if n >= 1024 {
        format!("{:.2} KiB", n as f64 / 1024.0)
    } else {
        format!("{n} B")
    }
}

fn print_tiles(tiles: &TileHierarchy) {
    for (level, t) in [("L1", tiles.l1), ("L2", tiles.l2), ("L3", tiles.l3)] {
        println!(
            "  {level}: Mr={} Kc={} Nc={}  | A={} B={} C={} | ширина A={} B={} (mod 64)",
            t.mr,
            t.kc,
            t.nc,
            fmt_bytes(t.a_bytes()),
            fmt_bytes(t.b_bytes()),
            fmt_bytes(t.c_bytes()),
            t.width_a_bytes(),
            t.width_b_bytes(),
        );
    }
}

fn main() {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("ошибка: {e}");
            std::process::exit(2);
        }
    };

    let caches = detect_cache_sizes(args.l3_bytes);
    let tiles = choose_tiles(&caches);

    let a = BlockMatrix::from_seed(args.l, args.m, SEED);
    let b = BlockMatrix::from_seed(args.m, args.n, SEED.wrapping_add(1));

    let a_bytes = args.l * args.m * microblock_bytes();
    let b_bytes = args.m * args.n * microblock_bytes();
    let c_bytes = args.l * args.n * microblock_bytes();
    let working = a_bytes + b_bytes + c_bytes;

    println!(
        "блок {}×{} float | внешние A={}×{}  B={}×{} | режим {} | reps={}",
        matrix::BLOCK,
        matrix::BLOCK,
        args.l,
        args.m,
        args.m,
        args.n,
        mode_label(args.mode),
        args.reps
    );
    println!(
        "кэш: L1D={}  L2={}  L3={}{}",
        fmt_bytes(caches.l1),
        fmt_bytes(caches.l2),
        fmt_bytes(caches.l3),
        if caches.l3_from_sysctl {
            " (sysctl)"
        } else if args.l3_bytes.is_some() {
            " (--l3-bytes)"
        } else {
            " (default SLC)"
        }
    );
    println!(
        "рабочий набор A+B+C={}  (выигрыш кэша заметен, когда набор ≫ L2)",
        fmt_bytes(working)
    );
    println!(
        "бюджет блока (cache/3·0.9): L1={}  L2={}  L3={}",
        fmt_bytes(cache_params::block_bytes(caches.l1)),
        fmt_bytes(cache_params::block_bytes(caches.l2)),
        fmt_bytes(cache_params::block_bytes(caches.l3)),
    );
    println!("тайлы (микроблоки):");
    print_tiles(&tiles);

    let mut c_neon = None;
    let mut c_l3 = None;
    let mut t_neon_s = None;
    let mut t_l3_s = None;
    let reps = args.reps;

    if args.mode == Mode::All || args.mode == Mode::Neon {
        let _ = matmul_neon(&a, &b);
        let t = time_call(|| {
            let mut last = None;
            for _ in 0..reps {
                last = Some(matmul_neon(&a, &b));
            }
            last.unwrap()
        });
        let per = t.elapsed.as_secs_f64() / reps as f64;
        println!(
            "C1 NEON:  {:>14} тиков cntvct  {:.3} с  ({:.3} с/rep)",
            t.cycles,
            t.elapsed.as_secs_f64(),
            per
        );
        t_neon_s = Some(per);
        c_neon = Some(t.value);
    }

    if args.mode == Mode::All || args.mode == Mode::L3 {
        let _ = matmul_l3(&a, &b, &tiles);
        let t = time_call(|| {
            let mut last = None;
            for _ in 0..reps {
                last = Some(matmul_l3(&a, &b, &tiles));
            }
            last.unwrap()
        });
        let per = t.elapsed.as_secs_f64() / reps as f64;
        println!(
            "C2 L3:    {:>14} тиков cntvct  {:.3} с  ({:.3} с/rep)",
            t.cycles,
            t.elapsed.as_secs_f64(),
            per
        );
        t_l3_s = Some(per);
        c_l3 = Some(t.value);
    }

    if let (Some(tn), Some(tl)) = (t_neon_s, t_l3_s) {
        if tl > 0.0 {
            println!("ускорение C2/C1: {:.2}×  (L3-тайлинг vs NEON)", tn / tl);
        }
    }

    if let (Some(neon), Some(l3)) = (c_neon.as_ref(), c_l3.as_ref()) {
        let report = matrices_close(neon, l3, TOL);
        println!(
            "совпадение: {}  (макс. ошибка {:.2e})",
            if report.ok { "да" } else { "нет" },
            report.max_abs_err
        );
        if !report.ok {
            std::process::exit(1);
        }
    }
}
