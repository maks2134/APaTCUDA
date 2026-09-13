use crate::matrix::{BLOCK, Block, BlockMatrix};

pub fn matmul_auto(a: &BlockMatrix, b: &BlockMatrix) -> BlockMatrix {
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
                mul_add_block_auto(c_ij, a_ik, b_kj);
            }
        }
    }
    c
}

#[inline(never)]
pub fn mul_add_block_auto(c: &mut Block, a: &Block, b: &Block) {
    for i in 0..BLOCK {
        for k in 0..BLOCK {
            let aik = a.data[i][k];
            for j in 0..BLOCK {
                c.data[i][j] += aik * b.data[k][j];
            }
        }
    }
}
