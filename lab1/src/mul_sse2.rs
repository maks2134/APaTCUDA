use crate::matrix::{BLOCK, Block, BlockMatrix};
use std::arch::x86_64::{__m128, _mm_add_ps, _mm_loadu_ps, _mm_mul_ps, _mm_set1_ps, _mm_storeu_ps};

pub fn matmul_sse2(a: &BlockMatrix, b: &BlockMatrix) -> BlockMatrix {
    assert_eq!(a.cols, b.rows, "внутренние размеры блоков не совпадают");
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

#[target_feature(enable = "sse2")]
#[inline(never)]
pub unsafe fn mul_add_block_sse2(c: &mut Block, a: &Block, b: &Block) {
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
