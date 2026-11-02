//! Affine warp (OpenCV `warpAffine`, `INTER_LINEAR`) and the fixed-point
//! bilinear sampler shared with [`super::remap`].
//!
//! v2 calls `warpAffine` with `INTER_LINEAR` only, either with
//! `BORDER_CONSTANT` (value 0) for tunnel layers or `BORDER_REFLECT_101` with
//! `getRotationMatrix2D` for the oscillating zoom. It never passes
//! `WARP_INVERSE_MAP`, so the matrix maps source to destination and is
//! inverted here exactly like OpenCV does.
//!
//! The maths mirrors OpenCV 4.13's scalar C++ path (no IPP, no custom HAL):
//! source coordinates are quantised to 1/32 pixel (`INTER_BITS = 5`), the four
//! bilinear weights come from OpenCV's 32×32 integer table scaled by 2^15
//! (`INTER_REMAP_COEF_BITS`), and each channel is `(Σ w·v + 2^14) >> 15`.

use std::sync::OnceLock;

use crate::image::Bgra8;

/// Sub-pixel bits for interpolation (`INTER_BITS`).
const INTER_BITS: u32 = 5;
/// Sub-pixel steps per pixel (`INTER_TAB_SIZE`).
const INTER_TAB_SIZE: i32 = 1 << INTER_BITS;
/// Fixed-point bits of the bilinear weights (`INTER_REMAP_COEF_BITS`).
const COEF_BITS: u32 = 15;
/// Fixed-point scale of the bilinear weights (`INTER_REMAP_COEF_SCALE`).
const COEF_SCALE: i32 = 1 << COEF_BITS;
/// Fixed-point bits of the warp coordinate accumulator (`AB_BITS`).
const AB_BITS: u32 = 10;

/// How samples outside the source image are produced (OpenCV border modes
/// used by v2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Border {
    /// `BORDER_CONSTANT`: outside taps read this BGRA value.
    Constant([u8; 4]),
    /// `BORDER_REPLICATE`: clamp to the nearest edge pixel (`aaa|abcd|ddd`).
    Replicate,
    /// `BORDER_REFLECT_101`: mirror without repeating the edge (`dcb|abcd|cba`).
    Reflect101,
}

/// Build the 2×3 matrix of OpenCV `getRotationMatrix2D`.
///
/// A positive `angle_deg` rotates counter-clockwise as displayed (y down);
/// `scale` is an isotropic zoom about `center`. OpenCV takes the centre as a
/// `Point2f`, so callers wanting bit parity should pass `f32`-representable
/// values (v2 uses `(w - 1) / 2`, which always is).
pub fn rotation_matrix(center: (f64, f64), angle_deg: f64, scale: f64) -> [[f64; 3]; 2] {
    let angle = angle_deg * (std::f64::consts::PI / 180.0);
    let alpha = angle.cos() * scale;
    let beta = angle.sin() * scale;
    let (cx, cy) = center;
    [
        [alpha, beta, (1.0 - alpha) * cx - beta * cy],
        [-beta, alpha, beta * cx + (1.0 - alpha) * cy],
    ]
}

/// Warp `src` into a `dst_w × dst_h` image with OpenCV `warpAffine`
/// semantics (`INTER_LINEAR`, no `WARP_INVERSE_MAP`).
///
/// `m` maps source coordinates to destination coordinates; it is inverted
/// (a singular matrix inverts to all zeros, as in OpenCV) and every
/// destination pixel samples the source at the mapped position using the
/// fixed-point bilinear sampler. An empty source yields an image filled with
/// the constant border colour, or zeros for the other border modes.
pub fn warp_affine(
    src: &Bgra8,
    m: &[[f64; 3]; 2],
    dst_w: u32,
    dst_h: u32,
    border: Border,
) -> Bgra8 {
    let mut dst = Bgra8::new(dst_w, dst_h);
    if src.width == 0 || src.height == 0 {
        fill_empty(&mut dst, border);
        return dst;
    }

    let inv = invert_affine(m);
    let ab_scale = f64::from(1u32 << AB_BITS);
    let round_delta = (1i32 << AB_BITS) / INTER_TAB_SIZE / 2;
    let adelta: Vec<i32> = (0..dst_w)
        .map(|x| cv_round(inv[0] * f64::from(x) * ab_scale))
        .collect();
    let bdelta: Vec<i32> = (0..dst_w)
        .map(|x| cv_round(inv[3] * f64::from(x) * ab_scale))
        .collect();
    let row_len = dst_w as usize * Bgra8::CHANNELS;
    if row_len == 0 {
        return dst;
    }

    for (y, row) in dst.data.chunks_exact_mut(row_len).enumerate() {
        let yf = y as f64;
        let x0 = cv_round((inv[1] * yf + inv[2]) * ab_scale).wrapping_add(round_delta);
        let y0 = cv_round((inv[4] * yf + inv[5]) * ab_scale).wrapping_add(round_delta);
        for ((out, ad), bd) in row
            .as_chunks_mut::<{ Bgra8::CHANNELS }>()
            .0
            .iter_mut()
            .zip(&adelta)
            .zip(&bdelta)
        {
            let fx = x0.wrapping_add(*ad) >> (AB_BITS - INTER_BITS);
            let fy = y0.wrapping_add(*bd) >> (AB_BITS - INTER_BITS);
            sample_bilinear_fixed(src, fx, fy, border, out);
        }
    }
    dst
}

/// Invert a 2×3 affine matrix exactly as `cv::warpAffine` does, returned as
/// OpenCV's flat `M[0..6]`.
fn invert_affine(m: &[[f64; 3]; 2]) -> [f64; 6] {
    let [[m0, m1, m2], [m3, m4, m5]] = *m;
    let d = m0 * m4 - m1 * m3;
    let d = if d != 0.0 { 1.0 / d } else { 0.0 };
    let a11 = m4 * d;
    let a22 = m0 * d;
    let (n0, n1, n3, n4) = (a11, m1 * -d, m3 * -d, a22);
    let b1 = -n0 * m2 - n1 * m5;
    let b2 = -n3 * m2 - n4 * m5;
    [n0, n1, b1, n3, n4, b2]
}

/// `cvRound`/`saturate_cast<int>(double)`: round half to even, saturating.
pub(crate) fn cv_round(v: f64) -> i32 {
    v.round_ties_even() as i32
}

/// `saturate_cast<short>(int)`.
fn sat_i16(v: i32) -> i32 {
    v.clamp(i32::from(i16::MIN), i32::from(i16::MAX))
}

/// Fill an output for an empty source.
pub(crate) fn fill_empty(dst: &mut Bgra8, border: Border) {
    if let Border::Constant(c) = border {
        dst.data
            .as_chunks_mut::<{ Bgra8::CHANNELS }>()
            .0
            .iter_mut()
            .for_each(|p| p.copy_from_slice(&c));
    }
}

/// OpenCV's fixed-point bilinear weight table: for each of the 32×32
/// sub-pixel positions `(fy, fx)` (index `fy * 32 + fx`), the weights of the
/// taps `[(x, y), (x+1, y), (x, y+1), (x+1, y+1)]`, summing to 2^15.
///
/// Built like `initInterTab2D(INTER_LINEAR, true)`: `f32` products scaled by
/// 2^15, rounded half to even, then the sum corrected on one tap. The
/// correction reproduces OpenCV's search window (which for a 2×2 kernel
/// peeks past the cell); in practice only position (0, 0) is corrected,
/// giving `[32767, 0, 0, 1]`.
fn bilinear_table() -> &'static [[i32; 4]] {
    static TABLE: OnceLock<Vec<[i32; 4]>> = OnceLock::new();
    TABLE.get_or_init(|| {
        let n = INTER_TAB_SIZE as usize;
        let scale = 1.0f32 / INTER_TAB_SIZE as f32;
        let tab1: Vec<[f32; 2]> = (0..n)
            .map(|i| {
                let x = i as f32 * scale;
                [1.0 - x, x]
            })
            .collect();
        // Flat i16 storage with padding, mirroring OpenCV's out-of-cell reads.
        let mut itab = vec![0i16; n * n * 4 + 8];
        for i in 0..n {
            for j in 0..n {
                let base = (i * n + j) * 4;
                let mut isum = 0i32;
                for k1 in 0..2 {
                    for k2 in 0..2 {
                        let v = tab1[i][k1] * tab1[j][k2];
                        let w = (v * COEF_SCALE as f32).round_ties_even();
                        let w = w.clamp(f32::from(i16::MIN), f32::from(i16::MAX)) as i16;
                        itab[base + k1 * 2 + k2] = w;
                        isum += i32::from(w);
                    }
                }
                if isum != COEF_SCALE {
                    let diff = isum - COEF_SCALE;
                    let (mut big, mut small) = (base + 3, base + 3);
                    for k1 in 1..3 {
                        for k2 in 1..3 {
                            let idx = base + k1 * 2 + k2;
                            if itab[idx] < itab[small] {
                                small = idx;
                            } else if itab[idx] > itab[big] {
                                big = idx;
                            }
                        }
                    }
                    let idx = if diff < 0 { big } else { small };
                    itab[idx] = (i32::from(itab[idx]) - diff) as i16;
                }
            }
        }
        itab.as_chunks::<4>()
            .0
            .iter()
            .take(n * n)
            .map(|c| c.map(i32::from))
            .collect()
    })
}

/// `borderInterpolate`: map an out-of-range index into `0..len`, or `None`
/// for `BORDER_CONSTANT`.
pub(crate) fn border_index(p: i32, len: i32, border: Border) -> Option<i32> {
    if (p as u32) < (len as u32) {
        return Some(p);
    }
    match border {
        Border::Constant(_) => None,
        Border::Replicate => Some(p.clamp(0, len - 1)),
        Border::Reflect101 => {
            if len == 1 {
                return Some(0);
            }
            let mut p = i64::from(p);
            let len = i64::from(len);
            loop {
                p = if p < 0 { -p } else { 2 * len - 2 - p };
                if (0..len).contains(&p) {
                    return Some(p as i32);
                }
            }
        }
    }
}

/// Sample `src` at a fixed-point coordinate in 1/32 pixel units (OpenCV's
/// `XY = coord >> 5` plus table index `(fy & 31) * 32 + (fx & 31)`) and write
/// the four channels to `out`. Matches `remapBilinear` for 8-bit images,
/// including its border handling.
pub(crate) fn sample_bilinear_fixed(src: &Bgra8, fx: i32, fy: i32, border: Border, out: &mut [u8]) {
    const CN: usize = Bgra8::CHANNELS;
    let sx = sat_i16(fx >> INTER_BITS);
    let sy = sat_i16(fy >> INTER_BITS);
    let a = ((fy & (INTER_TAB_SIZE - 1)) * INTER_TAB_SIZE + (fx & (INTER_TAB_SIZE - 1))) as usize;
    let w = bilinear_table()[a];
    let (sw, sh) = (src.width as i32, src.height as i32);
    let stride = src.width as usize * CN;

    let combine = |taps: [Option<&[u8]>; 4], cval: [u8; 4], out: &mut [u8]| {
        for (c, o) in out.iter_mut().enumerate() {
            let sum: i32 = taps
                .iter()
                .zip(w)
                .map(|(t, wt)| i32::from(t.map_or(cval[c], |p| p[c])) * wt)
                .sum();
            *o = ((sum + (1 << (COEF_BITS - 1))) >> COEF_BITS).clamp(0, 255) as u8;
        }
    };

    if (sx as u32) < (sw - 1).max(0) as u32 && (sy as u32) < (sh - 1).max(0) as u32 {
        let off = sy as usize * stride + sx as usize * CN;
        let r0 = &src.data[off..off + 2 * CN];
        let r1 = &src.data[off + stride..off + stride + 2 * CN];
        combine(
            [
                Some(&r0[..CN]),
                Some(&r0[CN..]),
                Some(&r1[..CN]),
                Some(&r1[CN..]),
            ],
            [0; 4],
            out,
        );
        return;
    }

    let cval = match border {
        Border::Constant(c) => {
            if sx >= sw || sx + 1 < 0 || sy >= sh || sy + 1 < 0 {
                out.copy_from_slice(&c);
                return;
            }
            c
        }
        _ => [0; 4],
    };
    let x0 = border_index(sx, sw, border);
    let x1 = border_index(sx + 1, sw, border);
    let y0 = border_index(sy, sh, border);
    let y1 = border_index(sy + 1, sh, border);
    let tap = |x: Option<i32>, y: Option<i32>| -> Option<&[u8]> {
        let off = y? as usize * stride + x? as usize * CN;
        Some(&src.data[off..off + CN])
    };
    combine(
        [tap(x0, y0), tap(x1, y0), tap(x0, y1), tap(x1, y1)],
        cval,
        out,
    );
}
