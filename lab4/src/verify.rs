use crate::array::Percentiles;

const EPS: f32 = 1e-5;

pub fn percentiles_equal(a: &Percentiles, b: &Percentiles) -> bool {
    (a.p25 - b.p25).abs() <= EPS
        && (a.p50 - b.p50).abs() <= EPS
        && (a.p75 - b.p75).abs() <= EPS
}

pub fn print_percentiles(label: &str, p: &Percentiles) {
    println!(
        "  {label}: P25={:.6}  P50={:.6}  P75={:.6}",
        p.p25, p.p50, p.p75
    );
}
