#[cfg(not(target_arch = "aarch64"))]
compile_error!("эта лаба собирается только под aarch64 (Apple Silicon / ARM64)");

mod matrix;
mod mul_auto;
mod mul_neon;
mod timing;
mod verify;

use matrix::BlockMatrix;
use mul_auto::matmul_auto;
use mul_neon::matmul_neon;
use timing::time_call;
use verify::matrices_close;

const SEED: u32 = 42;
const TOL: f32 = 1e-4;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    All,
    Auto,
    Neon,
}

struct Args {
    l: usize,
    m: usize,
    n: usize,
    mode: Mode,
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args {
        l: 16,
        m: 16,
        n: 16,
        mode: Mode::All,
    };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--l" => args.l = parse_usize(&mut it, "--l")?,
            "--m" => args.m = parse_usize(&mut it, "--m")?,
            "--n" => args.n = parse_usize(&mut it, "--n")?,
            "--mode" => {
                let v = it
                    .next()
                    .ok_or_else(|| "нет значения после --mode".to_string())?;
                args.mode = match v.as_str() {
                    "all" => Mode::All,
                    "auto" => Mode::Auto,
                    "neon" => Mode::Neon,
                    other => {
                        return Err(format!(
                            "неизвестный --mode {other} (ожидается all|auto|neon)"
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
        Mode::Auto => "auto (только C1)",
        Mode::Neon => "neon (только C2)",
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

    let a = BlockMatrix::from_seed(args.l, args.m, SEED);
    let b = BlockMatrix::from_seed(args.m, args.n, SEED.wrapping_add(1));

    println!(
        "блок {}×{} float | внешние A={}×{}  B={}×{} | режим {}",
        matrix::BLOCK,
        matrix::BLOCK,
        args.l,
        args.m,
        args.m,
        args.n,
        mode_label(args.mode)
    );

    let mut c_auto = None;
    let mut c_neon = None;

    if args.mode == Mode::All || args.mode == Mode::Auto {
        let t_auto = time_call(|| matmul_auto(&a, &b));
        println!(
            "C1 авто:  {:>14} тиков cntvct  {:.3} с",
            t_auto.cycles,
            t_auto.elapsed.as_secs_f64()
        );
        c_auto = Some(t_auto.value);
    }

    if args.mode == Mode::All || args.mode == Mode::Neon {
        let t_neon = time_call(|| matmul_neon(&a, &b));
        println!(
            "C2 NEON:  {:>14} тиков cntvct  {:.3} с",
            t_neon.cycles,
            t_neon.elapsed.as_secs_f64()
        );
        c_neon = Some(t_neon.value);
    }

    if let (Some(auto), Some(neon)) = (c_auto.as_ref(), c_neon.as_ref()) {
        let report = matrices_close(auto, neon, TOL);
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
