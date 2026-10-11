use crate::matrix::MatrixI32;

/// Deinterleave rows: A1[k,j]=B[2k,j], A2[k,j]=B[2k+1,j].
pub fn deinterleave(b: &MatrixI32) -> (MatrixI32, MatrixI32) {
    assert_eq!(b.rows % 2, 0, "B.rows must be even (2K)");
    let k = b.rows / 2;
    let m = b.cols;
    let mut a1 = MatrixI32::zeros(k, m);
    let mut a2 = MatrixI32::zeros(k, m);
    for ki in 0..k {
        let row_even = 2 * ki;
        let row_odd = row_even + 1;
        let src_even = row_even * m;
        let src_odd = row_odd * m;
        let dst = ki * m;
        a1.data[dst..dst + m].copy_from_slice(&b.data[src_even..src_even + m]);
        a2.data[dst..dst + m].copy_from_slice(&b.data[src_odd..src_odd + m]);
    }
    (a1, a2)
}
