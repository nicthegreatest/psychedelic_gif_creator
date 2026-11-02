//! Post-composite effects from v2 `frame_renderer.cpp`.
//!
//! Kaleidoscope runs on the fitted image before the pyramid. Stars, wave,
//! pixelation, hue, saturation, invert and blur run inside `render_frame`,
//! in that order.

pub mod colour;
pub mod kaleidoscope;
pub mod stars;
pub mod texture;
pub mod wave;

use crate::image::{Bgr8, Bgra8};

/// BGR padded with alpha 255. Channel-independent filters (remap, nearest
/// resize) then match a 3-channel OpenCV call, because a constant 255 stays 255.
pub(crate) fn pad_bgr(src: &Bgr8) -> Bgra8 {
    let mut out = Bgra8::new(src.width, src.height);
    for (dst, src) in out
        .data
        .as_chunks_mut::<4>()
        .0
        .iter_mut()
        .zip(src.data.as_chunks::<3>().0)
    {
        dst[0] = src[0];
        dst[1] = src[1];
        dst[2] = src[2];
        dst[3] = 255;
    }
    out
}
