use crate::cache_params::{TileHierarchy, TileShape};
use crate::matrix::{Block, BlockMatrix};
use crate::mul_neon::mul_add_block_neon;

pub fn matmul_l3(a: &BlockMatrix, b: &BlockMatrix, tiles: &TileHierarchy) -> BlockMatrix {
    assert_eq!(a.cols, b.rows, "внутренние размеры блоков не совпадают");
    let l = a.rows;
    let m = a.cols;
    let n = b.cols;
    let mut c = BlockMatrix::zeros(l, n);

    let t3 = tiles.l3;
    let t2 = tiles.l2;
    let t1 = tiles.l1;

    let mut a_pack: Vec<Block> = Vec::with_capacity(t2.mr * t2.kc);
    let mut b_pack: Vec<Block> = Vec::with_capacity(t2.kc * t2.nc);

    let mut i3 = 0;
    while i3 < l {
        let i3_end = (i3 + t3.mr).min(l);
        let mut k3 = 0;
        while k3 < m {
            let k3_end = (k3 + t3.kc).min(m);
            let mut j3 = 0;
            while j3 < n {
                let j3_end = (j3 + t3.nc).min(n);
                matmul_l2(
                    a,
                    b,
                    &mut c,
                    t2,
                    t1,
                    i3,
                    i3_end,
                    k3,
                    k3_end,
                    j3,
                    j3_end,
                    &mut a_pack,
                    &mut b_pack,
                );
                j3 = j3_end;
            }
            k3 = k3_end;
        }
        i3 = i3_end;
    }
    c
}

fn matmul_l2(
    a: &BlockMatrix,
    b: &BlockMatrix,
    c: &mut BlockMatrix,
    t2: TileShape,
    t1: TileShape,
    i3: usize,
    i3_end: usize,
    k3: usize,
    k3_end: usize,
    j3: usize,
    j3_end: usize,
    a_pack: &mut Vec<Block>,
    b_pack: &mut Vec<Block>,
) {
    let mut i2 = i3;
    while i2 < i3_end {
        let i2_end = (i2 + t2.mr).min(i3_end);
        let mr = i2_end - i2;
        let mut k2 = k3;
        while k2 < k3_end {
            let k2_end = (k2 + t2.kc).min(k3_end);
            let kc = k2_end - k2;

            pack_a(a, i2, i2_end, k2, k2_end, a_pack);

            let mut j2 = j3;
            while j2 < j3_end {
                let j2_end = (j2 + t2.nc).min(j3_end);
                let nc = j2_end - j2;
                pack_b(b, k2, k2_end, j2, j2_end, b_pack);
                matmul_l1(c, a_pack, b_pack, t1, i2, j2, mr, kc, nc);
                j2 = j2_end;
            }
            k2 = k2_end;
        }
        i2 = i2_end;
    }
}

#[inline(never)]
fn pack_a(
    a: &BlockMatrix,
    i0: usize,
    i1: usize,
    k0: usize,
    k1: usize,
    buf: &mut Vec<Block>,
) {
    let mr = i1 - i0;
    let kc = k1 - k0;
    buf.clear();
    buf.reserve(mr * kc);
    for i in i0..i1 {
        for k in k0..k1 {
            buf.push(unsafe { *a.get_unchecked(i, k) });
        }
    }
}

#[inline(never)]
fn pack_b(
    b: &BlockMatrix,
    k0: usize,
    k1: usize,
    j0: usize,
    j1: usize,
    buf: &mut Vec<Block>,
) {
    let kc = k1 - k0;
    let nc = j1 - j0;
    buf.clear();
    buf.reserve(kc * nc);
    for k in k0..k1 {
        for j in j0..j1 {
            buf.push(unsafe { *b.get_unchecked(k, j) });
        }
    }
}

fn matmul_l1(
    c: &mut BlockMatrix,
    a_pack: &[Block],
    b_pack: &[Block],
    t1: TileShape,
    i0: usize,
    j0: usize,
    mr: usize,
    kc: usize,
    nc: usize,
) {
    let mut i1 = 0;
    while i1 < mr {
        let i1_end = (i1 + t1.mr).min(mr);
        let mut k1 = 0;
        while k1 < kc {
            let k1_end = (k1 + t1.kc).min(kc);
            let mut j1 = 0;
            while j1 < nc {
                let j1_end = (j1 + t1.nc).min(nc);
                for i in i1..i1_end {
                    for k in k1..k1_end {
                        let a_ik = unsafe { a_pack.get_unchecked(i * kc + k) };
                        for j in j1..j1_end {
                            let b_kj = unsafe { b_pack.get_unchecked(k * nc + j) };
                            let c_ij = unsafe { c.get_unchecked_mut(i0 + i, j0 + j) };
                            unsafe { mul_add_block_neon(c_ij, a_ik, b_kj) };
                        }
                    }
                }
                j1 = j1_end;
            }
            k1 = k1_end;
        }
        i1 = i1_end;
    }
}
