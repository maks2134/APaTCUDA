use crate::matrix::MatrixI32;

pub struct CompareReport {
    pub ok: bool,
    #[allow(dead_code)]
    pub mismatches: usize,
}

pub fn matrices_equal(a: &MatrixI32, b: &MatrixI32) -> CompareReport {
    assert_eq!(a.rows, b.rows);
    assert_eq!(a.cols, b.cols);
    let mut mismatches = 0usize;
    for (x, y) in a.data.iter().zip(b.data.iter()) {
        if x != y {
            mismatches += 1;
        }
    }
    CompareReport {
        ok: mismatches == 0,
        mismatches,
    }
}

/// Compact preview for pattern data (0..255). Skips dumping huge random ints.
#[allow(dead_code)]
pub fn print_fragment_compact(label: &str, m: &MatrixI32, max_r: usize, max_c: usize) {
    let r = m.rows.min(max_r);
    let c = m.cols.min(max_c);
    println!("  {label} (угол {r}×{c} из {}×{}):", m.rows, m.cols);
    for i in 0..r {
        print!("   ");
        for j in 0..c {
            print!("{:>4}", m.get(i, j));
        }
        println!();
    }
}
