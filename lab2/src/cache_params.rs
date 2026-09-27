use crate::matrix::BLOCK;
use std::process::Command;

pub const DEFAULT_L3_BYTES: usize = 8 * 1024 * 1024;
pub const WIDTH_ALIGN_BLOCKS: usize = 4;

#[derive(Clone, Copy, Debug)]
pub struct CacheSizes {
    pub l1: usize,
    pub l2: usize,
    pub l3: usize,
    pub l3_from_sysctl: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TileShape {
    pub mr: usize,
    pub kc: usize,
    pub nc: usize,
}

impl TileShape {
    pub fn a_bytes(self) -> usize {
        self.mr * self.kc * microblock_bytes()
    }

    pub fn b_bytes(self) -> usize {
        self.kc * self.nc * microblock_bytes()
    }

    pub fn c_bytes(self) -> usize {
        self.mr * self.nc * microblock_bytes()
    }

    pub fn width_a_bytes(self) -> usize {
        self.kc * BLOCK * 4
    }

    pub fn width_b_bytes(self) -> usize {
        self.nc * BLOCK * 4
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TileHierarchy {
    pub l1: TileShape,
    pub l2: TileShape,
    pub l3: TileShape,
}

#[inline]
pub fn microblock_bytes() -> usize {
    BLOCK * BLOCK * 4
}

pub fn block_bytes(cache_size: usize) -> usize {
    ((cache_size as f64) / 3.0 * 0.9).floor() as usize
}

pub fn detect_cache_sizes(l3_override: Option<usize>) -> CacheSizes {
    let l1 = sysctl_usize("hw.l1dcachesize").unwrap_or(64 * 1024);
    let l2 = sysctl_usize("hw.l2cachesize").unwrap_or(4 * 1024 * 1024);
    let (l3, l3_from_sysctl) = match l3_override {
        Some(v) => (v, false),
        None => match sysctl_usize("hw.l3cachesize") {
            Some(v) if v > 0 => (v, true),
            _ => (DEFAULT_L3_BYTES, false),
        },
    };
    CacheSizes {
        l1,
        l2,
        l3,
        l3_from_sysctl,
    }
}

fn sysctl_usize(name: &str) -> Option<usize> {
    let out = Command::new("sysctl")
        .args(["-n", name])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout);
    s.trim().parse().ok()
}

pub fn choose_tiles(caches: &CacheSizes) -> TileHierarchy {
    let budget_l1 = block_bytes(caches.l1);
    let budget_l2 = block_bytes(caches.l2);
    let budget_l3 = block_bytes(caches.l3);

    let l1 = best_tile_l1(budget_l1);
    let l2 = grow_tile_gebp(l1, budget_l2, side_cap(budget_l2), 16);
    let l3 = grow_tile_gebp(l2, budget_l3, side_cap(budget_l3), 32);

    debug_assert!(fits_budget(l1, budget_l1));
    debug_assert!(fits_budget(l2, budget_l2));
    debug_assert!(fits_budget(l3, budget_l3));

    TileHierarchy { l1, l2, l3 }
}

fn side_cap(budget: usize) -> usize {
    let mb = microblock_bytes().max(1);
    let max_panels = (budget / mb).max(1);
    let side = ((max_panels as f64).sqrt().floor() as usize).max(WIDTH_ALIGN_BLOCKS);
    round_down_align(side).max(WIDTH_ALIGN_BLOCKS)
}

fn fits_budget(t: TileShape, budget: usize) -> bool {
    t.a_bytes() <= budget && t.b_bytes() <= budget && t.c_bytes() <= budget
}

fn tile_score_gebp(t: TileShape) -> (usize, i64, usize) {
    let a_aspect = (t.mr as i64 - t.kc as i64).abs();
    let vol = t.a_bytes() + t.b_bytes() + t.c_bytes();
    (t.a_bytes(), -a_aspect, vol)
}

fn tile_score_cubic(t: TileShape) -> (i64, usize) {
    let dims = [t.mr as i64, t.kc as i64, t.nc as i64];
    let mx = *dims.iter().max().unwrap();
    let mn = *dims.iter().min().unwrap();
    let vol = t.a_bytes() + t.b_bytes() + t.c_bytes();
    (-(mx - mn), vol)
}

fn best_tile_l1(budget: usize) -> TileShape {
    let floor = TileShape {
        mr: 1,
        kc: WIDTH_ALIGN_BLOCKS,
        nc: WIDTH_ALIGN_BLOCKS,
    };
    let mb = microblock_bytes();
    if mb == 0 || budget < mb {
        return floor;
    }
    let max_panels = (budget / mb).max(1);
    let mut best = floor;
    let mut best_score = tile_score_cubic(best);

    let mut kc = WIDTH_ALIGN_BLOCKS;
    let kc_max = round_down_align(max_panels).max(WIDTH_ALIGN_BLOCKS);
    while kc <= kc_max {
        let mr_max = (max_panels / kc).max(1);
        for mr in 1..=mr_max {
            let nc_max = round_down_align((max_panels / kc).min(max_panels / mr));
            if nc_max < WIDTH_ALIGN_BLOCKS {
                continue;
            }
            let mut nc = WIDTH_ALIGN_BLOCKS;
            while nc <= nc_max {
                let t = TileShape { mr, kc, nc };
                if fits_budget(t, budget) {
                    let score = tile_score_cubic(t);
                    if score > best_score {
                        best_score = score;
                        best = t;
                    }
                }
                nc += WIDTH_ALIGN_BLOCKS;
            }
        }
        kc += WIDTH_ALIGN_BLOCKS;
    }
    best
}

fn grow_tile_gebp(
    base: TileShape,
    budget: usize,
    max_mr_kc: usize,
    max_nc: usize,
) -> TileShape {
    let mb = microblock_bytes().max(1);
    let max_panels = (budget / mb).max(1);
    let mut best = base;
    let mut best_score = tile_score_gebp(best);

    let max_mi = (max_panels / (base.mr.max(1) * base.kc.max(1)))
        .min(max_mr_kc / base.mr.max(1))
        .max(1);
    for mi in 1..=max_mi {
        let mr = base.mr * mi;
        if mr > max_mr_kc {
            break;
        }
        let max_ki = (max_panels / (mr * base.kc.max(1)))
            .min(max_mr_kc / base.kc.max(1))
            .max(1);
        for ki in 1..=max_ki {
            let kc = base.kc * ki;
            if kc > max_mr_kc {
                break;
            }
            let nc_lim = (max_panels / kc.max(1))
                .min(max_panels / mr.max(1))
                .min(max_nc);
            let max_ni = (nc_lim / base.nc.max(1)).max(1);
            for ni in 1..=max_ni {
                let nc = base.nc * ni;
                if nc > max_nc {
                    break;
                }
                let t = TileShape { mr, kc, nc };
                if !fits_budget(t, budget) {
                    break;
                }
                let score = tile_score_gebp(t);
                if score > best_score {
                    best_score = score;
                    best = t;
                }
            }
        }
    }
    best
}

fn round_down_align(x: usize) -> usize {
    (x / WIDTH_ALIGN_BLOCKS) * WIDTH_ALIGN_BLOCKS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_bytes_formula() {
        assert_eq!(block_bytes(30), 9);
        assert_eq!(block_bytes(65536), ((65536.0_f64 / 3.0) * 0.9).floor() as usize);
    }

    #[test]
    fn widths_multiple_of_64() {
        let caches = CacheSizes {
            l1: 64 * 1024,
            l2: 4 * 1024 * 1024,
            l3: 8 * 1024 * 1024,
            l3_from_sysctl: false,
        };
        let t = choose_tiles(&caches);
        for shape in [t.l1, t.l2, t.l3] {
            assert_eq!(shape.width_a_bytes() % 64, 0);
            assert_eq!(shape.width_b_bytes() % 64, 0);
            assert_eq!(shape.kc % WIDTH_ALIGN_BLOCKS, 0);
            assert_eq!(shape.nc % WIDTH_ALIGN_BLOCKS, 0);
        }
        assert_eq!(t.l2.mr % t.l1.mr, 0);
        assert_eq!(t.l3.mr % t.l2.mr, 0);
        assert!(t.l2.a_bytes() <= block_bytes(caches.l2));
        assert!(t.l3.a_bytes() <= block_bytes(caches.l3));
    }
}
