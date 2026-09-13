//! Auto-vectorizable blocked matrix multiply (C1).
//!
//! Compiler decides SIMD via Release + RUSTFLAGS (vec vs novec Task targets).

use crate::matrix::{BLOCK, BlockMatrix};

/// `C = A * B` where A is L×M, B is M×N, C is L×N (block counts).
/// No transpose; each tile is a 12×12 `f32` block.
pub fn matmul_auto(a: &BlockMatrix, b: &BlockMatrix) -> BlockMatrix {
    assert_eq!(a.cols, b.rows, "inner block dimensions must match");
    let l = a.rows;
    let m = a.cols;
    let n = b.cols;
    let mut c = BlockMatrix::zeros(l, n);

    for i in 0..l {
        for k in 0..m {
            // SAFETY: i < l, k < m by loop bounds.
            let a_ik = unsafe { a.get_unchecked(i, k) };
            for j in 0..n {
                // SAFETY: k < m, j < n, i < l by loop bounds.
                let b_kj = unsafe { b.get_unchecked(k, j) };
                let c_ij = unsafe { c.get_unchecked_mut(i, j) };
                mul_add_block_auto(c_ij, a_ik, b_kj);
            }
        }
    }
    c
}

/// Hot 12×12 kernel written so LLVM can auto-vectorize the inner `j` loop.
#[inline(never)]
pub fn mul_add_block_auto(
    c: &mut crate::matrix::Block,
    a: &crate::matrix::Block,
    b: &crate::matrix::Block,
) {
    for i in 0..BLOCK {
        for k in 0..BLOCK {
            let aik = a.data[i][k];
            // Contiguous stores along a row — good candidate for vectorization.
            for j in 0..BLOCK {
                c.data[i][j] += aik * b.data[k][j];
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::matrix::Block;
    use crate::verify::blocks_close;

    #[test]
    fn single_block_matches_scalar() {
        let a = Block::fill(1.5);
        let b = Block::fill(0.25);
        let mut expected = Block::zeros();
        expected.mul_add_scalar(&a, &b);

        let mut got = Block::zeros();
        mul_add_block_auto(&mut got, &a, &b);
        assert!(blocks_close(&expected, &got, 1e-5));
    }

    #[test]
    fn small_outer_matrix() {
        let a = BlockMatrix::from_seed(2, 2, 1);
        let b = BlockMatrix::from_seed(2, 3, 2);
        let c = matmul_auto(&a, &b);
        assert_eq!(c.rows, 2);
        assert_eq!(c.cols, 3);

        let mut ref_c = BlockMatrix::zeros(2, 3);
        for i in 0..2 {
            for k in 0..2 {
                for j in 0..3 {
                    ref_c.get_mut(i, j).mul_add_scalar(a.get(i, k), b.get(k, j));
                }
            }
        }
        for i in 0..2 {
            for j in 0..3 {
                assert!(blocks_close(ref_c.get(i, j), c.get(i, j), 1e-4));
            }
        }
    }
}
