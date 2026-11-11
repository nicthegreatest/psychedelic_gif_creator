//! Mapping a frame onto its palette (gif-h `GifThresholdImage` and
//! `GifDitherImage`), with delta transparency: a pixel whose wanted colour
//! equals the previous *output* frame is written as the transparent index.

use super::palette::{Palette, TRANSPARENT_INDEX};

/// A palettised frame: the indices to encode and the colours a viewer shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mapped {
    pub indices: Vec<u8>,
    /// What the canvas shows after this frame (input to the next frame's delta).
    pub shown: Vec<[u8; 3]>,
}

/// Nearest-colour mapping with no dithering.
pub fn threshold(prev: Option<&[[u8; 3]]>, next: &[[u8; 3]], pal: &Palette) -> Mapped {
    let (indices, shown) = next
        .iter()
        .enumerate()
        .map(|(i, &px)| match prev.map(|p| p[i]) {
            Some(old) if old == px => (TRANSPARENT_INDEX, old),
            _ => {
                let (ind, _) = pal.closest(px.map(i32::from), (1, 1_000_000));
                (ind, pal.colors[usize::from(ind)])
            }
        })
        .unzip();
    Mapped { indices, shown }
}

/// Floyd–Steinberg dithering in 8.8 fixed point, error clamped so no channel
/// goes negative. Neighbour offsets are flat indices, exactly as gif-h (so
/// row-end error wraps onto the next row, as in v2).
pub fn dither(prev: Option<&[[u8; 3]]>, next: &[[u8; 3]], width: usize, pal: &Palette) -> Mapped {
    let n = next.len();
    let mut quant: Vec<[i32; 3]> = next.iter().map(|p| p.map(|c| i32::from(c) * 256)).collect();
    let mut indices = vec![TRANSPARENT_INDEX; n];

    for i in 0..n {
        let want = quant[i].map(|q| (q + 127) / 256);
        if let Some(p) = prev {
            if p[i].map(i32::from) == want {
                quant[i] = want;
                indices[i] = TRANSPARENT_INDEX;
                continue;
            }
        }
        let (ind, _) = pal.closest(want, (TRANSPARENT_INDEX, 1_000_000));
        let color = pal.colors[usize::from(ind)].map(i32::from);
        let err = [0, 1, 2].map(|c| quant[i][c] - color[c] * 256);
        quant[i] = color;
        indices[i] = ind;

        let (x, row) = (i % width, i - i % width);
        // (flat index, weight/16), computed like gif-h's int arithmetic.
        let targets = [
            (row + x + 1, 7),
            ((row + width + x).wrapping_sub(1), 3),
            (row + width + x, 5),
            (row + width + x + 1, 1),
        ];
        for (t, w) in targets {
            if t < n {
                let q = &mut quant[t];
                for c in 0..3 {
                    q[c] += (-q[c]).max(err[c] * w / 16);
                }
            }
        }
    }
    let shown = quant.iter().map(|q| q.map(|c| c as u8)).collect();
    Mapped { indices, shown }
}
