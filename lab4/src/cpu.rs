use crate::array::Percentiles;

pub fn percentiles(a: &[f32]) -> Percentiles {
    let mut sorted = a.to_vec();
    sorted.sort_by(|x, y| x.total_cmp(y));
    Percentiles::from_sorted(&sorted)
}
