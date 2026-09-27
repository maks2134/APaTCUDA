pub const BLOCK: usize = 12;

#[repr(C, align(16))]
#[derive(Clone, Copy)]
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
}

#[derive(Clone)]
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
                    let bits = (state >> 9) & 0x007F_FFFF;
                    block.data[i][j] = (bits as f32) * (1.0 / 8_388_608.0) - 0.5;
                }
            }
        }
        m
    }

    #[inline]
    pub unsafe fn get_unchecked(&self, row: usize, col: usize) -> &Block {
        unsafe { self.data.get_unchecked(row * self.cols + col) }
    }

    #[inline]
    pub unsafe fn get_unchecked_mut(&mut self, row: usize, col: usize) -> &mut Block {
        unsafe { self.data.get_unchecked_mut(row * self.cols + col) }
    }
}
