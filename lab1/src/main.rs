#[cfg(not(target_arch = "x86_64"))]
compile_error!("lab1 требует x86_64 (Intel/AMD). Собирайте и запускайте на Windows Intel.");

mod matrix;
mod mul_auto;
mod mul_sse2;
mod timing;
mod verify;

use matrix::BlockMatrix;
use mul_auto::matmul_auto;
use mul_sse2::matmul_sse2;
use timing::time_call;
use verify::matrices_close;

const SEED: u32 = 42;
const TOL: f32 = 1e-4;

struct Args {
    l: usize,
    m: usize,
    n: usize,
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args {
        l: 16,
        m: 16,
        n: 16,
    };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--l" => args.l = parse_usize(&mut it, "--l")?,
            "--m" => args.m = parse_usize(&mut it, "--m")?,
            "--n" => args.n = parse_usize(&mut it, "--n")?,
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
        "блок {}×{} float | внешние A={}×{}  B={}×{}",
        matrix::BLOCK,
        matrix::BLOCK,
        args.l,
        args.m,
        args.m,
        args.n
    );

    let t_auto = time_call(|| matmul_auto(&a, &b));
    println!(
        "C1 авто:  {:>14} тактов  {:.3} с",
        t_auto.cycles,
        t_auto.elapsed.as_secs_f64()
    );

    let t_sse = time_call(|| matmul_sse2(&a, &b));
    println!(
        "C2 SSE2:  {:>14} тактов  {:.3} с",
        t_sse.cycles,
        t_sse.elapsed.as_secs_f64()
    );

    let report = matrices_close(&t_auto.value, &t_sse.value, TOL);
    println!(
        "совпадение: {}  (макс. ошибка {:.2e})",
        if report.ok { "да" } else { "нет" },
        report.max_abs_err
    );

    if !report.ok {
        std::process::exit(1);
    }
}
