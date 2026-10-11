use std::path::Path;

use image::{GrayImage, ImageBuffer, Luma, Rgb, RgbImage};

use crate::matrix::MatrixI32;

const MAX_SIDE: u32 = 1024;

fn to_gray_buffer(m: &MatrixI32) -> GrayImage {
    let mut min_v = m.data[0];
    let mut max_v = m.data[0];
    for &v in &m.data {
        min_v = min_v.min(v);
        max_v = max_v.max(v);
    }
    // Pattern data is already 0..255 — keep it sharp; random still stretches.
    let use_direct = min_v >= 0 && max_v <= 255;
    let span = (max_v as i64 - min_v as i64).max(1) as f64;

    let src_w = m.cols as u32;
    let src_h = m.rows as u32;
    let scale = (MAX_SIDE as f64 / src_w.max(src_h) as f64).min(1.0);
    let dst_w = ((src_w as f64 * scale).round() as u32).max(1);
    let dst_h = ((src_h as f64 * scale).round() as u32).max(1);

    let mut img: GrayImage = ImageBuffer::new(dst_w, dst_h);
    for y in 0..dst_h {
        let sy = ((y as f64 + 0.5) / dst_h as f64 * src_h as f64) as usize;
        let sy = sy.min(m.rows - 1);
        for x in 0..dst_w {
            let sx = ((x as f64 + 0.5) / dst_w as f64 * src_w as f64) as usize;
            let sx = sx.min(m.cols - 1);
            let v = m.get(sy, sx);
            let g = if use_direct {
                v.clamp(0, 255) as u8
            } else {
                (((v as i64 - min_v as i64) as f64 / span) * 255.0).round() as u8
            };
            img.put_pixel(x, y, Luma([g]));
        }
    }
    img
}

/// Save matrix as grayscale PNG (downscaled if needed).
pub fn save_matrix_png(m: &MatrixI32, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if m.rows == 0 || m.cols == 0 || m.data.is_empty() {
        return Err("пустая матрица".into());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    to_gray_buffer(m).save(path)?;
    Ok(())
}

/// Side-by-side RGB panel: B | A1 | A2 with labels as colored borders.
pub fn save_panel_png(
    b: &MatrixI32,
    a1: &MatrixI32,
    a2: &MatrixI32,
    path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let gb = to_gray_buffer(b);
    let g1 = to_gray_buffer(a1);
    let g2 = to_gray_buffer(a2);

    // Normalize heights to max, scale widths proportionally already done per image.
    let h = gb.height().max(g1.height()).max(g2.height());
    let gap = 8u32;
    let border = 4u32;
    let w = gb.width() + g1.width() + g2.width() + gap * 2 + border * 6;
    let mut panel: RgbImage = ImageBuffer::from_pixel(w, h + border * 2, Rgb([30, 30, 30]));

    let blit = |dst: &mut RgbImage, src: &GrayImage, x0: u32, border_rgb: [u8; 3]| {
        let y0 = border;
        // colored frame
        for y in y0.saturating_sub(border)..(y0 + src.height() + border).min(dst.height()) {
            for x in x0.saturating_sub(border)..(x0 + src.width() + border).min(dst.width()) {
                let on_frame = x < x0
                    || x >= x0 + src.width()
                    || y < y0
                    || y >= y0 + src.height();
                if on_frame {
                    dst.put_pixel(x, y, Rgb(border_rgb));
                }
            }
        }
        for y in 0..src.height() {
            for x in 0..src.width() {
                let g = src.get_pixel(x, y)[0];
                dst.put_pixel(x0 + x, y0 + y, Rgb([g, g, g]));
            }
        }
    };

    let mut x = border;
    blit(&mut panel, &gb, x, [220, 80, 80]); // B — red frame (striped mix)
    x += gb.width() + gap + border * 2;
    blit(&mut panel, &g1, x, [80, 200, 100]); // A1 — green
    x += g1.width() + gap + border * 2;
    blit(&mut panel, &g2, x, [80, 140, 220]); // A2 — blue

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    panel.save(path)?;
    Ok(())
}

pub fn save_deinterleave_set(
    b: &MatrixI32,
    a1: &MatrixI32,
    a2: &MatrixI32,
    out_dir: &Path,
) -> Result<Vec<std::path::PathBuf>, Box<dyn std::error::Error>> {
    std::fs::create_dir_all(out_dir)?;
    let k = a1.rows;
    let m = a1.cols;
    let paths = [
        out_dir.join(format!("B_{}x{}.png", b.rows, b.cols)),
        out_dir.join(format!("A1_{k}x{m}.png")),
        out_dir.join(format!("A2_{k}x{m}.png")),
        out_dir.join(format!("panel_B_A1_A2_{k}x{m}.png")),
    ];
    save_matrix_png(b, &paths[0])?;
    save_matrix_png(a1, &paths[1])?;
    save_matrix_png(a2, &paths[2])?;
    save_panel_png(b, a1, a2, &paths[3])?;
    Ok(paths.to_vec())
}
