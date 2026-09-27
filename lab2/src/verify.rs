use crate::matrix::{BLOCK, BlockMatrix};

pub struct CompareReport {
    pub ok: bool,
    pub max_abs_err: f32,
}

pub fn matrices_close(a: &BlockMatrix, b: &BlockMatrix, abs_tol: f32) -> CompareReport {
    assert_eq!(a.rows, b.rows);
    assert_eq!(a.cols, b.cols);
    let mut max_abs_err = 0.0_f32;
    for (ba, bb) in a.data.iter().zip(b.data.iter()) {
        for i in 0..BLOCK {
            for j in 0..BLOCK {
                let err = (ba.data[i][j] - bb.data[i][j]).abs();
                if err > max_abs_err {
                    max_abs_err = err;
                }
            }
        }
    }
    CompareReport {
        ok: max_abs_err <= abs_tol,
        max_abs_err,
    }
}
