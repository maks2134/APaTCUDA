//! Row-major i32 matrices for deinterleave lab.

pub const MAX_DIM: usize = 4096;
/// Allowed for `--gpu-variant oom` (out-of-core / oversized demos).
pub const MAX_DIM_LARGE: usize = 16384;
pub const SEED: u32 = 42;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FillMode {
    /// Pseudo-random (stress / timing).
    Random,
    /// Visible patterns: even rows = «картинка A1», odd = «картинка A2».
    Pattern,
}

#[derive(Clone)]
pub struct MatrixI32 {
    pub rows: usize,
    pub cols: usize,
    pub data: Vec<i32>,
}

impl MatrixI32 {
    pub fn zeros(rows: usize, cols: usize) -> Self {
        Self {
            rows,
            cols,
            data: vec![0; rows * cols],
        }
    }

    /// Fill with deterministic pseudo-random i32 values.
    pub fn from_seed(rows: usize, cols: usize, seed: u32) -> Self {
        let mut m = Self::zeros(rows, cols);
        let mut state = seed;
        for v in &mut m.data {
            state = state.wrapping_mul(1664525).wrapping_add(1013904223);
            *v = state as i32;
        }
        m
    }

    /// Pattern for visualization (values 0..255).
    /// Even output-row k → soft radial blob; odd → diagonal stripes.
    /// Interleaved into B so the combined image looks striped; A1/A2 look clean.
    pub fn from_pattern_interleaved(k: usize, m: usize) -> Self {
        let mut b = Self::zeros(2 * k, m);
        for ki in 0..k {
            for j in 0..m {
                let fx = (j as f64 + 0.5) / m as f64;
                let fy = (ki as f64 + 0.5) / k as f64;

                // A1: bright circle + horizontal gradient
                let dx = fx - 0.35;
                let dy = fy - 0.45;
                let r = (dx * dx + dy * dy).sqrt();
                let circle = ((1.0 - (r / 0.35).min(1.0)) * 200.0) as i32;
                let grad = (fx * 80.0) as i32;
                let a1 = (circle + grad).clamp(0, 255);

                // A2: diagonal stripes + vertical wash
                let stripe = if ((fx * 18.0 + fy * 18.0) as i32) % 2 == 0 {
                    220
                } else {
                    40
                };
                let wash = (fy * 60.0) as i32;
                let a2 = (stripe + wash).clamp(0, 255);

                b.data[(2 * ki) * m + j] = a1;
                b.data[(2 * ki + 1) * m + j] = a2;
            }
        }
        b
    }

    #[inline]
    pub fn idx(&self, r: usize, c: usize) -> usize {
        r * self.cols + c
    }

    #[inline]
    pub fn get(&self, r: usize, c: usize) -> i32 {
        self.data[self.idx(r, c)]
    }
}

/// Input B is (2K)×M.
pub fn generate_b(k: usize, m: usize, fill: FillMode) -> MatrixI32 {
    match fill {
        FillMode::Random => MatrixI32::from_seed(2 * k, m, SEED),
        FillMode::Pattern => MatrixI32::from_pattern_interleaved(k, m),
    }
}
