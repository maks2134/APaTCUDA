//! 12×12 float blocks and outer block matrices (row-major).

pub const BLOCK: usize = 12;

/// One 12×12 tile, 16-byte aligned for SSE2 loads/stores.
#[repr(C, align(16))]
#[derive(Clone, Copy, Debug)]
pub struct Block {
    pub data: [[f32; BLOCK]; BLOCK],
}

impl Block {
    #[inline]
    pub const fn zeros() -> Self {
        Self {
            data: [[0.0; BLOCK]; BLOCK],
        }
    }

    #[inline]
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn fill(value: f32) -> Self {
        Self {
            data: [[value; BLOCK]; BLOCK],
        }
    }

    /// Scalar multiply-accumulate: `self += a * b` (no transpose).
    #[inline]
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn mul_add_scalar(&mut self, a: &Block, b: &Block) {
        for i in 0..BLOCK {
            for k in 0..BLOCK {
                let aik = a.data[i][k];
                for j in 0..BLOCK {
                    self.data[i][j] += aik * b.data[k][j];
                }
            }
        }
    }
}

impl Default for Block {
    fn default() -> Self {
        Self::zeros()
    }
}

/// Outer matrix whose elements are [`Block`] tiles. Layout: `data[i * cols + j]`.
#[derive(Clone, Debug)]
pub struct BlockMatrix {
    pub rows: usize,
    pub cols: usize,
    pub data: Vec<Block>,
}

impl BlockMatrix {
    pub fn zeros(rows: usize, cols: usize) -> Self {
        Self {
            rows,
            cols,
            data: vec![Block::zeros(); rows * cols],
        }
    }

    pub fn from_seed(rows: usize, cols: usize, seed: u32) -> Self {
        let mut m = Self::zeros(rows, cols);
        let mut state = seed;
        for block in &mut m.data {
            for i in 0..BLOCK {
                for j in 0..BLOCK {
                    state = state.wrapping_mul(1664525).wrapping_add(1013904223);
                    // Keep values in a modest range so float error stays bounded.
                    let bits = (state >> 9) & 0x007F_FFFF;
                    block.data[i][j] = (bits as f32) * (1.0 / 8_388_608.0) - 0.5;
                }
            }
        }
        m
    }

    #[inline]
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn get(&self, row: usize, col: usize) -> &Block {
        debug_assert!(row < self.rows && col < self.cols);
        &self.data[row * self.cols + col]
    }

    #[inline]
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn get_mut(&mut self, row: usize, col: usize) -> &mut Block {
        debug_assert!(row < self.rows && col < self.cols);
        &mut self.data[row * self.cols + col]
    }

    /// # Safety
    /// `row < rows` and `col < cols`.
    #[inline]
    pub unsafe fn get_unchecked(&self, row: usize, col: usize) -> &Block {
        unsafe { self.data.get_unchecked(row * self.cols + col) }
    }

    /// # Safety
    /// `row < rows` and `col < cols`.
    #[inline]
    pub unsafe fn get_unchecked_mut(&mut self, row: usize, col: usize) -> &mut Block {
        unsafe { self.data.get_unchecked_mut(row * self.cols + col) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_alignment() {
        assert_eq!(std::mem::align_of::<Block>(), 16);
        assert_eq!(std::mem::size_of::<Block>(), BLOCK * BLOCK * 4);
    }

    #[test]
    fn scalar_times_identity() {
        let mut c = Block::zeros();
        let a = Block::fill(2.0);
        let mut b = Block::zeros();
        for i in 0..BLOCK {
            b.data[i][i] = 1.0;
        }
        c.mul_add_scalar(&a, &b);
        for i in 0..BLOCK {
            for j in 0..BLOCK {
                assert!((c.data[i][j] - a.data[i][j]).abs() < 1e-5);
            }
        }
    }
}
