//! Pixelation (nearest down, then nearest up) and Gaussian blur.

use super::pad_bgr;
use crate::image::{bgra_to_bgr, Bgr8};
use crate::kernels::gaussian::gaussian_blur;
use crate::kernels::resize::resize_nearest;

/// Block size `max(1, lround(level * scale))`, then nearest resize down and up.
/// Levels of 1 or less, and a block of 1, are no-ops.
pub fn pixelate(bgr: &mut Bgr8, level: i32, scale: f64) {
    if level <= 1 {
        return;
    }
    let block = (f64::from(level) * scale).round().max(1.0) as u32;
    if block <= 1 {
        return;
    }
    let small_w = (bgr.width / block).max(1);
    let small_h = (bgr.height / block).max(1);
    let down = resize_nearest(&pad_bgr(bgr), small_w, small_h);
    let up = resize_nearest(&down, bgr.width, bgr.height);
    *bgr = bgra_to_bgr(&up);
}

/// `GaussianBlur` with sigma `radius * scale`, skipped at or below 0.05.
pub fn blur(bgr: &mut Bgr8, radius: f64, scale: f64) {
    if radius <= 0.0 {
        return;
    }
    let sigma = radius * scale;
    if sigma > 0.05 {
        *bgr = gaussian_blur(bgr, sigma);
    }
}
