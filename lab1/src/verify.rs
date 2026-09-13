//! Full-matrix comparison for C1 vs C2 (no fragment printing).

use crate::matrix::{BLOCK, Block, BlockMatrix};

#[derive(Clone, Debug)]
pub struct CompareReport {
    pub ok: bool,
    pub max_abs_err: f32,
    pub checked_elements: usize,
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn blocks_close(a: &Block, b: &Block, abs_tol: f32) -> bool {
    block_max_abs_err(a, b) <= abs_tol
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn block_max_abs_err(a: &Block, b: &Block) -> f32 {
    let mut max_err = 0.0_f32;
    for i in 0..BLOCK {
        for j in 0..BLOCK {
            let err = (a.data[i][j] - b.data[i][j]).abs();
            if err > max_err {
                max_err = err;
            }
        }
    }
    max_err
}

pub fn matrices_close(a: &BlockMatrix, b: &BlockMatrix, abs_tol: f32) -> CompareReport {
    assert_eq!(a.rows, b.rows);
    assert_eq!(a.cols, b.cols);
    let mut max_abs_err = 0.0_f32;
    let mut checked = 0usize;
    for (ba, bb) in a.data.iter().zip(b.data.iter()) {
        for i in 0..BLOCK {
            for j in 0..BLOCK {
                let err = (ba.data[i][j] - bb.data[i][j]).abs();
                if err > max_abs_err {
                    max_abs_err = err;
                }
                checked += 1;
            }
        }
    }
    CompareReport {
        ok: max_abs_err <= abs_tol,
        max_abs_err,
        checked_elements: checked,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_blocks() {
        let a = Block::fill(3.0);
        let b = Block::fill(3.0);
        assert!(blocks_close(&a, &b, 0.0));
    }
}
