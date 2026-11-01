//! Nearest-neighbour resize (OpenCV `INTER_NEAREST`).

use crate::image::Bgra8;

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
