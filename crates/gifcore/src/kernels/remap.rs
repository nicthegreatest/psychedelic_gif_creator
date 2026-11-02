//! Generic geometric remap (OpenCV `remap` with two `CV_32FC1` maps).
//!
//! v2 uses `INTER_LINEAR` with `BORDER_REFLECT_101` (animated kaleidoscope)
//! and `BORDER_REPLICATE` (wave), and `INTER_NEAREST` with
//! `BORDER_REFLECT_101` (quadrant-mirror kaleidoscope), so both [`remap`] and
//! [`remap_nearest`] are provided. The bilinear path shares the fixed-point
//! sampler with [`super::warp`].

use super::warp::{border_index, fill_empty, sample_bilinear_fixed, Border};
use crate::image::Bgra8;

/// Sub-pixel steps per pixel (`INTER_TAB_SIZE`).
const INTER_TAB_SIZE: f32 = 32.0;

/// `cvRound(float)`: round half to even, saturating to `i32`.
fn cv_round_f32(v: f32) -> i32 {
    v.round_ties_even() as i32
}

/// Check the map lengths and return the destination row length in bytes.
fn check_maps(map_x: &[f32], map_y: &[f32], dst_w: u32, dst_h: u32) -> usize {
    let n = dst_w as usize * dst_h as usize;
    assert_eq!(map_x.len(), n, "map_x must have dst_w * dst_h entries");
    assert_eq!(map_y.len(), n, "map_y must have dst_w * dst_h entries");
    dst_w as usize * Bgra8::CHANNELS
}

/// Remap with bilinear interpolation (`INTER_LINEAR`):
/// `dst(x, y) = src(map_x[i], map_y[i])` with `i = y * dst_w + x`.
///
/// Map coordinates are rounded to 1/32 pixel (half to even) and sampled with
/// OpenCV's fixed-point bilinear weights. Panics if a map does not have
/// `dst_w * dst_h` entries. An empty source yields an image filled with the
/// constant border colour, or zeros for the other border modes.
pub fn remap(
    src: &Bgra8,
    map_x: &[f32],
    map_y: &[f32],
    dst_w: u32,
    dst_h: u32,
    border: Border,
) -> Bgra8 {
    let row_len = check_maps(map_x, map_y, dst_w, dst_h);
    let mut dst = Bgra8::new(dst_w, dst_h);
    if src.width == 0 || src.height == 0 {
        fill_empty(&mut dst, border);
        return dst;
    }
    if row_len == 0 {
        return dst;
    }
    for ((out, mx), my) in dst
        .data
        .as_chunks_mut::<{ Bgra8::CHANNELS }>()
        .0
        .iter_mut()
        .zip(map_x)
        .zip(map_y)
    {
        let fx = cv_round_f32(mx * INTER_TAB_SIZE);
        let fy = cv_round_f32(my * INTER_TAB_SIZE);
        sample_bilinear_fixed(src, fx, fy, border, out);
    }
    dst
}

/// Remap with nearest-neighbour sampling (`INTER_NEAREST`).
///
/// Each map coordinate is rounded half to even to a whole pixel (OpenCV's
/// `saturate_cast<short>`), then out-of-range pixels follow `border`.
/// Same size rules and empty-source behaviour as [`remap`].
pub fn remap_nearest(
    src: &Bgra8,
    map_x: &[f32],
    map_y: &[f32],
    dst_w: u32,
    dst_h: u32,
    border: Border,
) -> Bgra8 {
    let row_len = check_maps(map_x, map_y, dst_w, dst_h);
    let mut dst = Bgra8::new(dst_w, dst_h);
    if src.width == 0 || src.height == 0 {
        fill_empty(&mut dst, border);
        return dst;
    }
    if row_len == 0 {
        return dst;
    }
    let (sw, sh) = (src.width as i32, src.height as i32);
    let stride = src.width as usize * Bgra8::CHANNELS;
    let to_i16 = |v: f32| cv_round_f32(v).clamp(i32::from(i16::MIN), i32::from(i16::MAX));
    for ((out, mx), my) in dst
        .data
        .as_chunks_mut::<{ Bgra8::CHANNELS }>()
        .0
        .iter_mut()
        .zip(map_x)
        .zip(map_y)
    {
        let x = border_index(to_i16(*mx), sw, border);
        let y = border_index(to_i16(*my), sh, border);
        match (x, y, border) {
            (Some(x), Some(y), _) => {
                let off = y as usize * stride + x as usize * Bgra8::CHANNELS;
                out.copy_from_slice(&src.data[off..off + Bgra8::CHANNELS]);
            }
            (_, _, Border::Constant(c)) => out.copy_from_slice(&c),
            _ => unreachable!("only BORDER_CONSTANT yields an out-of-range tap"),
        }
    }
    dst
}
