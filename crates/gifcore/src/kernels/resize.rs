//! Nearest-neighbour resize (OpenCV `INTER_NEAREST`).

use crate::image::Bgra8;

/// Area-average resize (OpenCV `INTER_AREA`), downscaling only.
///
/// For each destination pixel `(x, y)` the source box is
/// `[x * scale_x, (x + 1) * scale_x) × [y * scale_y, (y + 1) * scale_y)` with
/// `scale_x = src_w / dst_w` and `scale_y = src_h / dst_h`. Every source pixel
/// overlapping that box contributes its channel values weighted by the overlap
/// area; the weighted sum is divided by `scale_x * scale_y` (the box area),
/// rounded to the nearest even, clamped to `0..=255` and stored as `u8`.
///
/// Upscaling is delegated to [`resize_nearest`]; the proper OpenCV upscale path
/// is left for a later milestone. If `dst_w` or `dst_h` is zero an empty image
/// of the requested size is returned.
pub fn resize_area(src: &Bgra8, dst_w: u32, dst_h: u32) -> Bgra8 {
    if dst_w == 0 || dst_h == 0 {
        return Bgra8::new(dst_w, dst_h);
    }
    if dst_w > src.width || dst_h > src.height {
        // TODO(3c): OpenCV upscale path
        return resize_nearest(src, dst_w, dst_h);
    }

    let scale_x = src.width as f64 / dst_w as f64;
    let scale_y = src.height as f64 / dst_h as f64;
    let last_x = src.width.saturating_sub(1) as usize;
    let last_y = src.height.saturating_sub(1) as usize;
    let inv_area = 1.0 / (scale_x * scale_y);

    // Per-column overlap lists: for each destination column `x`, the source
    // columns that intersect its box together with their x-overlap widths.
    let mut col: Vec<Vec<(usize, f64)>> = vec![Vec::new(); dst_w as usize];
    for x in 0..dst_w {
        let x0 = (x as f64) * scale_x;
        let x1 = (x as f64 + 1.0) * scale_x;
        let mut s = (x0.floor() as i64).clamp(0, last_x as i64);
        while s <= last_x as i64 && (s as f64) < x1 {
            let w = x1.min((s + 1) as f64) - x0.max(s as f64);
            col[x as usize].push((s as usize, w));
            s += 1;
        }
    }

    let dst_stride = dst_w as usize * Bgra8::CHANNELS;
    let src_stride = src.width as usize * Bgra8::CHANNELS;
    let mut dst = Bgra8::new(dst_w, dst_h);

    for y in 0..dst_h {
        let y0 = (y as f64) * scale_y;
        let y1 = (y as f64 + 1.0) * scale_y;
        let mut s = (y0.floor() as i64).clamp(0, last_y as i64);
        let mut row: Vec<(usize, f64)> = Vec::new();
        while s <= last_y as i64 && (s as f64) < y1 {
            let w = y1.min((s + 1) as f64) - y0.max(s as f64);
            row.push((s as usize, w));
            s += 1;
        }

        let dst_row = (y as usize) * dst_stride;
        for x in 0..dst_w {
            let mut acc = [0f64; Bgra8::CHANNELS];
            for &(sy, wy) in &row {
                let src_row = sy * src_stride;
                for &(sx, wx) in &col[x as usize] {
                    let p = sx * Bgra8::CHANNELS;
                    let src_px = &src.data[src_row + p..src_row + p + Bgra8::CHANNELS];
                    let w = wx * wy;
                    for c in 0..Bgra8::CHANNELS {
                        acc[c] += w * src_px[c] as f64;
                    }
                }
            }
            let dst_off = (x as usize) * Bgra8::CHANNELS;
            let out = &mut dst.data[dst_row + dst_off..dst_row + dst_off + Bgra8::CHANNELS];
            for (o, a) in out.iter_mut().zip(acc) {
                *o = (a * inv_area).round_ties_even().clamp(0.0, 255.0) as u8;
            }
        }
    }

    dst
}

/// Nearest-neighbour resize of `src` to `dst_w × dst_h` (OpenCV `INTER_NEAREST`).
///
/// For each destination pixel `(x, y)` the source coordinate is
/// `sx = min(floor(x * src_w / dst_w), src_w - 1)` (and `sy` likewise), i.e. the
/// top-left source pixel the destination pixel falls into. All four BGRA
/// channels are copied verbatim. If `dst_w` or `dst_h` is zero an empty image of
/// the requested size is returned.
pub fn resize_nearest(src: &Bgra8, dst_w: u32, dst_h: u32) -> Bgra8 {
    let mut dst = Bgra8::new(dst_w, dst_h);
    if dst_w == 0 || dst_h == 0 {
        return dst;
    }

    let src_w = u64::from(src.width);
    let src_h = u64::from(src.height);
    let dst_w64 = u64::from(dst_w);
    let dst_h64 = u64::from(dst_h);
    // Clamp so a coordinate never reads past the last source pixel.
    let last_x = src_w.saturating_sub(1);
    let last_y = src_h.saturating_sub(1);
    let src_stride = (src.width as usize) * Bgra8::CHANNELS;

    for y in 0..dst_h {
        let sy = ((u64::from(y) * src_h) / dst_h64).min(last_y) as usize;
        let src_row = sy * src_stride;
        let dst_row = (y as usize) * (dst_w as usize) * Bgra8::CHANNELS;
        for x in 0..dst_w {
            let sx = ((u64::from(x) * src_w) / dst_w64).min(last_x) as usize;
            // Destination offset is keyed by the destination coordinate; the
            // source offset is keyed by the mapped source coordinate. These
            // differ whenever several destination pixels sample one source pixel.
            let src_offset = sx * Bgra8::CHANNELS;
            let dst_offset = (x as usize) * Bgra8::CHANNELS;
            dst.data[dst_row + dst_offset..dst_row + dst_offset + 4]
                .copy_from_slice(&src.data[src_row + src_offset..src_row + src_offset + 4]);
        }
    }

    dst
}

/// Bicubic resize of `src` to `dst_w × dst_h` (OpenCV `INTER_CUBIC`).
///
/// For each destination pixel `(x, y)` the source coordinate is mapped with a
/// centre-aligned formula
/// `fx = (x + 0.5) * src_w / dst_w - 0.5` (and `fy` likewise). The base source
/// index is the floor of that coordinate; the four cubic taps are taken at
/// `ix-1, ix, ix+1, ix+2` using the generalised cubic kernel with `A = -0.75`
/// (Catmull–Rom), whose weights sum to one. Tap indices are clamped to the last
/// valid source index, which replicates the edge pixel; every BGRA channel is
/// interpolated the same way and rounded half-to-even. If `dst_w` or `dst_h` is
/// zero, or the source is empty, an empty image of the requested size is
/// returned.
pub fn resize_cubic(src: &Bgra8, dst_w: u32, dst_h: u32) -> Bgra8 {
    if dst_w == 0 || dst_h == 0 {
        return Bgra8::new(dst_w, dst_h);
    }
    if src.width == 0 || src.height == 0 {
        return Bgra8::new(dst_w, dst_h);
    }

    let src_w = src.width as f64;
    let src_h = src.height as f64;
    let last_x = (src.width - 1) as f64;
    let last_y = (src.height - 1) as f64;
    let src_stride = (src.width as usize) * Bgra8::CHANNELS;
    let dst_stride = (dst_w as usize) * Bgra8::CHANNELS;

    let mut dst = Bgra8::new(dst_w, dst_h);

    for y in 0..dst_h {
        let fy = ((y as f64 + 0.5) * src_h / dst_h as f64) - 0.5;
        let iy = fy.floor();
        let ty = fy - iy;
        let wy = cubic_weights(ty);
        // Clamp each of the four vertical taps (offsets -1, 0, +1, +2) to the
        // last valid row, replicating the edge pixel.
        let row_idx = [
            (iy - 1.0).clamp(0.0, last_y) as usize,
            iy.clamp(0.0, last_y) as usize,
            (iy + 1.0).clamp(0.0, last_y) as usize,
            (iy + 2.0).clamp(0.0, last_y) as usize,
        ];
        let dst_row = y as usize * dst_stride;

        for x in 0..dst_w {
            let fx = ((x as f64 + 0.5) * src_w / dst_w as f64) - 0.5;
            let ix = fx.floor();
            let tx = fx - ix;
            let wx = cubic_weights(tx);
            // Clamp each of the four horizontal taps to the last valid column.
            let col_idx = [
                (ix - 1.0).clamp(0.0, last_x) as usize,
                ix.clamp(0.0, last_x) as usize,
                (ix + 1.0).clamp(0.0, last_x) as usize,
                (ix + 2.0).clamp(0.0, last_x) as usize,
            ];

            let mut acc = [0f64; Bgra8::CHANNELS];
            for (sy, wy_j) in row_idx.iter().zip(wy.iter().copied()) {
                let src_row = *sy * src_stride;
                for (sx, wx_i) in col_idx.iter().zip(wx.iter().copied()) {
                    let p = *sx * Bgra8::CHANNELS;
                    let src_px = &src.data[src_row + p..src_row + p + Bgra8::CHANNELS];
                    for (a, sv) in acc.iter_mut().zip(src_px) {
                        *a += wx_i * wy_j * (*sv as f64);
                    }
                }
            }

            let dst_off = x as usize * Bgra8::CHANNELS;
            let out = &mut dst.data[dst_row + dst_off..dst_row + dst_off + Bgra8::CHANNELS];
            for (o, a) in out.iter_mut().zip(acc) {
                *o = a.round_ties_even().clamp(0.0, 255.0) as u8;
            }
        }
    }

    dst
}

/// Generalised cubic convolution weights with `A = -0.75` (Catmull–Rom), for
/// fraction `t ∈ [0, 1)` and taps at offsets -1, 0, +1, +2. The weights sum to
/// one, so a uniform input reproduces that value exactly.
fn cubic_weights(t: f64) -> [f64; 4] {
    const A: f64 = -0.75;
    let w0 = ((A * (t + 1.0) - 5.0 * A) * (t + 1.0) + 8.0 * A) * (t + 1.0) - 4.0 * A;
    let w1 = ((A + 2.0) * t - (A + 3.0)) * t * t + 1.0;
    let w2 = ((A + 2.0) * (1.0 - t) - (A + 3.0)) * (1.0 - t) * (1.0 - t) + 1.0;
    let w3 = 1.0 - w0 - w1 - w2;
    [w0, w1, w2, w3]
}
