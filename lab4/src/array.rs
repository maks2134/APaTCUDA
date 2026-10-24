pub const MIN_N: usize = 100;
pub const MAX_N: usize = 10_000_000;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FillMode {
    Pattern,
    Random,
}

pub fn next_pow2(n: usize) -> usize {
    if n <= 1 {
        return 1;
    }
    1usize << (usize::BITS - (n - 1).leading_zeros())
}

pub fn percentile_index(n: usize, q: f64) -> usize {
    debug_assert!(n > 0);
    let i = (q * n as f64).floor() as usize;
    i.min(n - 1)
}

pub fn generate_a(n: usize, fill: FillMode) -> Vec<f32> {
    assert!(n >= MIN_N && n <= MAX_N);
    match fill {
        FillMode::Random => {
            let mut state = 42u32;
            (0..n)
                .map(|_| {
                    state = state.wrapping_mul(1664525).wrapping_add(1013904223);
                    (state as f32) * (1.0 / 4294967296.0) * 1000.0
                })
                .collect()
        }
        FillMode::Pattern => (0..n)
            .map(|i| {
                let x = (i as f32) * 0.017;
                (x.sin() * 100.0) + ((i % 97) as f32) * 0.3
            })
            .collect(),
    }
}

#[derive(Clone, Debug)]
pub struct Percentiles {
    pub p25: f32,
    pub p50: f32,
    pub p75: f32,
}

impl Percentiles {
    pub fn from_sorted(sorted: &[f32]) -> Self {
        let n = sorted.len();
        Self {
            p25: sorted[percentile_index(n, 0.25)],
            p50: sorted[percentile_index(n, 0.50)],
            p75: sorted[percentile_index(n, 0.75)],
        }
    }
}
