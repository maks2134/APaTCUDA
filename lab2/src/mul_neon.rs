use crate::matrix::{BLOCK, Block, BlockMatrix};
use std::arch::aarch64::{
    float32x4_t, vaddq_f32, vdupq_n_f32, vld1q_f32, vmulq_f32, vst1q_f32,
};

pub fn matmul_neon(a: &BlockMatrix, b: &BlockMatrix) -> BlockMatrix {
    assert_eq!(a.cols, b.rows, "внутренние размеры блоков не совпадают");
    let l = a.rows;
    let m = a.cols;
    let n = b.cols;
    let mut c = BlockMatrix::zeros(l, n);

    for i in 0..l {
        for j in 0..n {
            let c_ij = unsafe { c.get_unchecked_mut(i, j) };
            for k in 0..m {
                let a_ik = unsafe { a.get_unchecked(i, k) };
                let b_kj = unsafe { b.get_unchecked(k, j) };
                unsafe { mul_add_block_neon(c_ij, a_ik, b_kj) };
            }
        }
    }
    c
}

#[target_feature(enable = "neon")]
#[inline(never)]
pub unsafe fn mul_add_block_neon(c: &mut Block, a: &Block, b: &Block) {
    unsafe {
        for i in 0..BLOCK {
            for k in 0..BLOCK {
                let aik: float32x4_t = vdupq_n_f32(a.data[i][k]);
                for j0 in (0..BLOCK).step_by(4) {
                    let c_ptr = c.data[i].as_mut_ptr().add(j0);
                    let b_ptr = b.data[k].as_ptr().add(j0);
                    let vc = vld1q_f32(c_ptr);
                    let vb = vld1q_f32(b_ptr);
                    let prod = vmulq_f32(aik, vb);
                    let sum = vaddq_f32(vc, prod);
                    vst1q_f32(c_ptr, sum);
                }
            }
        }
    }
}
