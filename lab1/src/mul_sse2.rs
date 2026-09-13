#[cfg(target_arch = "x86_64")]
use crate::matrix::{BLOCK, Block, BlockMatrix};

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::{__m128, _mm_add_ps, _mm_loadu_ps, _mm_mul_ps, _mm_set1_ps, _mm_storeu_ps};

#[cfg(target_arch = "x86_64")]
pub fn matmul_sse2(a: &BlockMatrix, b: &BlockMatrix) -> BlockMatrix {
    assert_eq!(a.cols, b.rows, "inner block dimensions must match");
    let l = a.rows;
    let m = a.cols;
    let n = b.cols;
    let mut c = BlockMatrix::zeros(l, n);

    for i in 0..l {
        for k in 0..m {
            let a_ik = unsafe { a.get_unchecked(i, k) };
            for j in 0..n {
                let b_kj = unsafe { b.get_unchecked(k, j) };
                let c_ij = unsafe { c.get_unchecked_mut(i, j) };
                unsafe { mul_add_block_sse2(c_ij, a_ik, b_kj) };
            }
        }
    }
    c
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "sse2")]
#[inline(never)]
pub unsafe fn mul_add_block_sse2(c: &mut Block, a: &Block, b: &Block) {
    debug_assert_eq!(BLOCK, 12);
    unsafe {
        for i in 0..BLOCK {
            for k in 0..BLOCK {
                let aik: __m128 = _mm_set1_ps(a.data[i][k]);
                for j0 in (0..BLOCK).step_by(4) {
                    let c_ptr = c.data[i].as_mut_ptr().add(j0);
                    let b_ptr = b.data[k].as_ptr().add(j0);
                    let vc = _mm_loadu_ps(c_ptr);
                    let vb = _mm_loadu_ps(b_ptr);
                    let prod = _mm_mul_ps(aik, vb);
                    let sum = _mm_add_ps(vc, prod);
                    _mm_storeu_ps(c_ptr, sum);
                }
            }
        }
    }
}

#[cfg(not(target_arch = "x86_64"))]
pub fn matmul_sse2(
    _a: &crate::matrix::BlockMatrix,
    _b: &crate::matrix::BlockMatrix,
) -> crate::matrix::BlockMatrix {
    panic!(
        "matmul_sse2 requires x86_64 (SSE2); rebuild with --target x86_64-apple-darwin or run on Windows/Intel"
    );
}

#[cfg(all(test, target_arch = "x86_64"))]
mod tests {
    use super::*;
    use crate::matrix::Block;
    use crate::mul_auto::{matmul_auto, mul_add_block_auto};
    use crate::verify::{blocks_close, matrices_close};

    #[test]
    fn sse2_block_matches_auto() {
        let a = Block::fill(1.25);
        let b = Block::fill(-0.5);
        let mut auto = Block::zeros();
        let mut sse = Block::zeros();
        mul_add_block_auto(&mut auto, &a, &b);
        unsafe { mul_add_block_sse2(&mut sse, &a, &b) };
        assert!(blocks_close(&auto, &sse, 1e-5));
    }

    #[test]
    fn sse2_matrix_matches_auto() {
        let a = BlockMatrix::from_seed(2, 2, 7);
        let b = BlockMatrix::from_seed(2, 2, 9);
        let c1 = matmul_auto(&a, &b);
        let c2 = matmul_sse2(&a, &b);
        let report = matrices_close(&c1, &c2, 1e-4);
        assert!(report.ok, "max abs err = {}", report.max_abs_err);
    }
}
