//! Gaussian pyramid downsample (OpenCV `pyrDown`).

use crate::image::Bgra8;

/// Reflect a source index using OpenCV `BORDER_REFLECT_101` semantics
/// (`f(-1)=1`, `f(-2)=2`, `f(n)=n-2`, `f(n+1)=n-3`) and clamp into `0..=n-1`, so
/// one- and two-pixel images still yield a valid tap index.
fn reflect_101(i: i64, n: usize) -> usize {
    let n = n as i64;
    let i = if i < 0 {
        -i
    } else if i >= n {
        2 * n - 2 - i
    } else {
        i
    };
    i.clamp(0, n - 1) as usize
}

/// Five reflected/clamped tap indices for a destination coordinate `base`
/// (i.e. `base-2 .. base+2`), matching the kernel's `i,j-2` offset.
fn taps(base: i64, n: usize) -> [usize; 5] {
    [
        reflect_101(base - 2, n),
        reflect_101(base - 1, n),
        reflect_101(base, n),
        reflect_101(base + 1, n),
        reflect_101(base + 2, n),
    ]
}

/// Downsample `src` by a factor of two using OpenCV `pyrDown` semantics.
///
/// Output size is `((w + 1) / 2, (h + 1) / 2)`. Each destination pixel is the
/// separable 5-tap Gaussian
/// `dst(x, y) = Σ_{i,j} k[i]·k[j]·src(2x+i-2, 2y+j-2)` with `k = [1, 4, 6, 4, 1]`
/// (total weight 256) and `BORDER_REFLECT_101` borders. Each channel is
/// accumulated in `u32` and stored as `(sum + 128) >> 8`. An empty source
/// returns an empty image.
pub fn pyr_down(src: &Bgra8) -> Bgra8 {
    let (w, h) = (src.width, src.height);
    if w == 0 || h == 0 {
        return Bgra8::new(0, 0);
    }

    let (dst_w, dst_h) = (w.div_ceil(2), h.div_ceil(2));
    let mut dst = Bgra8::new(dst_w, dst_h);
    let k = [1i64, 4, 6, 4, 1];
    let src_stride = w as usize * Bgra8::CHANNELS;

    for y in 0..dst_h {
        let rows = taps(2 * y as i64, h as usize);
        let row_stride = rows.map(|r| r * src_stride);

        for x in 0..dst_w {
            let cols = taps(2 * x as i64, w as usize);
            let col_off = cols.map(|c| c * Bgra8::CHANNELS);

            let mut acc = [0u32; Bgra8::CHANNELS];
            for (rstride, wj) in row_stride.iter().zip(k.iter().copied()) {
                let base = *rstride;
                for (coff, wi) in col_off.iter().zip(k.iter().copied()) {
                    let weight = wi * wj;
                    let src_px = &src.data[base + coff..base + coff + Bgra8::CHANNELS];
                    for (a, v) in acc.iter_mut().zip(src_px) {
                        *a += weight as u32 * (*v as u32);
                    }
                }
            }

            let dst_off =
                (y as usize) * (dst_w as usize) * Bgra8::CHANNELS + (x as usize) * Bgra8::CHANNELS;
            let out = &mut dst.data[dst_off..dst_off + Bgra8::CHANNELS];
            for (o, a) in out.iter_mut().zip(acc) {
                *o = ((a + 128) >> 8) as u8;
            }
        }
    }

    dst
}
