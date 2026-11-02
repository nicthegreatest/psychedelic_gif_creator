//! OpenCV 8-bit `COLOR_BGR2HSV` / `COLOR_HSV2BGR` (H in 0..180).
//!
//! BGR to HSV is the integer table path (`hsv_shift = 12`); the SIMD and
//! scalar loops use the same arithmetic. HSV to BGR is `HSV2RGB_b`. On this
//! machine OpenCV 4.13 builds that with AVX2, so `v_float32` has 8 lanes and
//! each row converts `4 * 8 = 32` pixels at a time with `v_trunc` (toward
//! zero). The tail of the row uses `saturate_cast<uchar>`, which rounds half
//! to even. A row shorter than 32 pixels is entirely the scalar tail, which
//! is why a 23-wide probe matches rounding for every pixel.

use crate::image::Bgr8;

const HSV_SHIFT: i32 = 12;
/// Pixels per AVX2 `HSV2RGB_b` iteration: 4 × 8 `f32` lanes, applied per row.
const SIMD_PIXELS: usize = 32;

fn cv_round_i32(v: f64) -> i32 {
    v.round_ties_even() as i32
}

fn div_tables() -> ([i32; 256], [i32; 256]) {
    let mut sdiv = [0i32; 256];
    let mut hdiv = [0i32; 256];
    for i in 1..256 {
        sdiv[i] = cv_round_i32(f64::from(255 << HSV_SHIFT) / (i as f64));
        hdiv[i] = cv_round_i32(f64::from(180 << HSV_SHIFT) / (6.0 * (i as f64)));
    }
    (sdiv, hdiv)
}

/// `COLOR_BGR2HSV`. The result is still a 3-channel image: H, S, V.
pub fn bgr_to_hsv(src: &Bgr8) -> Bgr8 {
    let (sdiv, hdiv) = div_tables();
    let mut dst = Bgr8::new(src.width, src.height);
    for (px, out) in src
        .data
        .as_chunks::<3>()
        .0
        .iter()
        .zip(dst.data.as_chunks_mut::<3>().0)
    {
        let b = i32::from(px[0]);
        let g = i32::from(px[1]);
        let r = i32::from(px[2]);
        let v = b.max(g).max(r);
        let vmin = b.min(g).min(r);
        let diff = (v - vmin) as u8;
        let diff_i = i32::from(diff);
        let vr = if v == r { -1 } else { 0 };
        let vg = if v == g { -1 } else { 0 };
        let s = (diff_i * sdiv[v as usize] + (1 << (HSV_SHIFT - 1))) >> HSV_SHIFT;
        let mut h =
            (vr & (g - b)) + (!vr & ((vg & (b - r + 2 * diff_i)) + ((!vg) & (r - g + 4 * diff_i))));
        h = (h * hdiv[diff as usize] + (1 << (HSV_SHIFT - 1))) >> HSV_SHIFT;
        if h < 0 {
            h += 180;
        }
        out[0] = h as u8;
        out[1] = s as u8;
        out[2] = v as u8;
    }
    dst
}

fn sector_rgb(h_byte: u8, s_byte: u8, v_byte: u8) -> (f32, f32, f32) {
    let mut h = f32::from(h_byte);
    let s = f32::from(s_byte) * (1.0 / 255.0);
    let v = f32::from(v_byte) * (1.0 / 255.0);
    if s == 0.0 {
        return (v, v, v);
    }
    const HSCALE: f32 = 6.0 / 180.0;
    h *= HSCALE;
    let mut sector = h.floor() as i32;
    h -= sector as f32;
    sector %= 6;
    if sector < 0 {
        sector += 6;
    }
    let tab = [
        v,
        v * (1.0 - s),
        v * (1.0 - s * h),
        v * (1.0 - s * (1.0 - h)),
    ];
    const MAP: [[usize; 3]; 6] = [
        [1, 3, 0],
        [1, 0, 2],
        [3, 0, 1],
        [0, 2, 1],
        [0, 1, 3],
        [2, 1, 0],
    ];
    let m = MAP[sector as usize];
    (tab[m[0]], tab[m[1]], tab[m[2]])
}

fn to_u8(value: f32, truncate: bool) -> u8 {
    let scaled = value * 255.0;
    let n = if truncate {
        scaled.trunc()
    } else {
        scaled.round_ties_even()
    };
    n.clamp(0.0, 255.0) as u8
}

/// `COLOR_HSV2BGR`. H above 179 is passed through the same float formula.
pub fn hsv_to_bgr(src: &Bgr8) -> Bgr8 {
    let width = src.width as usize;
    let simd = (width / SIMD_PIXELS) * SIMD_PIXELS;
    let mut dst = Bgr8::new(src.width, src.height);
    let src_rows = src.data.as_chunks::<3>().0.chunks(width);
    let dst_rows = dst.data.as_chunks_mut::<3>().0.chunks_mut(width);
    for (src_row, dst_row) in src_rows.zip(dst_rows) {
        for (x, (px, out)) in src_row.iter().zip(dst_row).enumerate() {
            let (b, g, r) = sector_rgb(px[0], px[1], px[2]);
            let truncate = x < simd;
            out[0] = to_u8(b, truncate);
            out[1] = to_u8(g, truncate);
            out[2] = to_u8(r, truncate);
        }
    }
    dst
}
