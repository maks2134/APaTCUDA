//! Lab 1 — blocked matrix multiply: auto-vectorization (C1) vs SSE2 (C2).
//!
//! Variant: float, block size 12. Release only. Timing: RDTSC on x86_64.

mod matrix;
mod mul_auto;
mod mul_sse2;
mod timing;
mod verify;

use matrix::BlockMatrix;
use mul_auto::matmul_auto;
use mul_sse2::matmul_sse2;
use timing::{cycles_label, time_call};
use verify::matrices_close;

fn print_usage(argv0: &str) {
    eprintln!(
        "Usage: {argv0} [--l L] [--m M] [--n N] [--seed S] [--tol T] [--skip-sse2]\n\
         \n\
         Outer dimensions are counts of 12×12 float blocks.\n\
         Example: {argv0} --l 48 --m 48 --n 48"
    );
}

struct Args {
    l: usize,
    m: usize,
    n: usize,
    seed: u32,
    tol: f32,
    skip_sse2: bool,
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args {
        l: 16,
        m: 16,
        n: 16,
        seed: 42,
        tol: 1e-4,
        skip_sse2: false,
    };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                print_usage("lab1");
                std::process::exit(0);
            }
            "--l" => {
                args.l = parse_usize(&mut it, "--l")?;
            }
            "--m" => {
                args.m = parse_usize(&mut it, "--m")?;
            }
            "--n" => {
                args.n = parse_usize(&mut it, "--n")?;
            }
            "--seed" => {
                args.seed = parse_u32(&mut it, "--seed")?;
            }
            "--tol" => {
                args.tol = parse_f32(&mut it, "--tol")?;
            }
            "--skip-sse2" => {
                args.skip_sse2 = true;
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    if args.l == 0 || args.m == 0 || args.n == 0 {
        return Err("L, M, N must be > 0".into());
    }
    Ok(args)
}

fn parse_usize(it: &mut impl Iterator<Item = String>, flag: &str) -> Result<usize, String> {
    let v = it
        .next()
        .ok_or_else(|| format!("missing value after {flag}"))?;
    v.parse()
        .map_err(|_| format!("invalid usize for {flag}: {v}"))
}

fn parse_u32(it: &mut impl Iterator<Item = String>, flag: &str) -> Result<u32, String> {
    let v = it
        .next()
        .ok_or_else(|| format!("missing value after {flag}"))?;
    v.parse()
        .map_err(|_| format!("invalid u32 for {flag}: {v}"))
}

fn parse_f32(it: &mut impl Iterator<Item = String>, flag: &str) -> Result<f32, String> {
    let v = it
        .next()
        .ok_or_else(|| format!("missing value after {flag}"))?;
    v.parse()
        .map_err(|_| format!("invalid f32 for {flag}: {v}"))
}

fn main() {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("error: {e}");
            print_usage("lab1");
            std::process::exit(2);
        }
    };

    let a = BlockMatrix::from_seed(args.l, args.m, args.seed);
    let b = BlockMatrix::from_seed(args.m, args.n, args.seed.wrapping_add(1));

    println!(
        "lab1 vectorization — float blocks {block}×{block}",
        block = matrix::BLOCK
    );
    println!(
        "outer sizes: A=[{}×{}] B=[{}×{}] C=[{}×{}] (blocks)",
        args.l, args.m, args.m, args.n, args.l, args.n
    );
    println!(
        "scalar floats: A={} B={} C={}",
        args.l * args.m * matrix::BLOCK * matrix::BLOCK,
        args.m * args.n * matrix::BLOCK * matrix::BLOCK,
        args.l * args.n * matrix::BLOCK * matrix::BLOCK
    );
    println!("arch: {}", std::env::consts::ARCH);
    println!("timer: {}", cycles_label());
    println!();

    let t_auto = time_call(|| matmul_auto(&a, &b));
    println!(
        "C1 matmul_auto:  {:>12} {}  |  {:.6} s",
        t_auto.cycles,
        cycles_label(),
        t_auto.elapsed.as_secs_f64()
    );

    if args.skip_sse2 || cfg!(not(target_arch = "x86_64")) {
        if cfg!(not(target_arch = "x86_64")) {
            println!(
                "C2 matmul_sse2:  skipped (not x86_64 — use task run-vec-x86 / Windows Intel)"
            );
        } else {
            println!("C2 matmul_sse2:  skipped (--skip-sse2)");
        }
        println!();
        println!("match: n/a (SSE2 not run)");
        return;
    }

    let t_sse = time_call(|| matmul_sse2(&a, &b));
    println!(
        "C2 matmul_sse2:  {:>12} {}  |  {:.6} s",
        t_sse.cycles,
        cycles_label(),
        t_sse.elapsed.as_secs_f64()
    );

    let report = matrices_close(&t_auto.value, &t_sse.value, args.tol);
    println!();
    println!(
        "match: {}  (checked {} floats, max abs err = {:.6e}, tol = {:.1e})",
        if report.ok { "yes" } else { "NO" },
        report.checked_elements,
        report.max_abs_err,
        args.tol
    );

    if t_sse.cycles > t_auto.cycles {
        println!(
            "note: SSE2 cycles ({}) > auto ({}) — on defense SSE2 must not be slower than auto",
            t_sse.cycles, t_auto.cycles
        );
    }

    if !report.ok {
        std::process::exit(1);
    }
}
