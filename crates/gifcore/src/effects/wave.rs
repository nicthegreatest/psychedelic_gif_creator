//! Sine ripple. Horizontal shifts each row; vertical shifts each column.
//! The offset is `float(amp * sin(index * freq + phase))` added to the
//! integer coordinate, then `remap` linear with `BORDER_REPLICATE`.

use super::pad_bgr;
use crate::image::{bgra_to_bgr, Bgr8};
use crate::kernels::remap::remap;
use crate::kernels::warp::Border;
use crate::settings::{GifSettings, WaveDirection};

const TAU: f64 = 2.0 * std::f64::consts::PI;

/// Replace `bgr` with the rippled frame. A zero amplitude or frequency, or
/// direction `None`, leaves it unchanged.
pub fn apply(bgr: &mut Bgr8, settings: &GifSettings, scale: f64, t: f64) {
    if settings.wave_direction == WaveDirection::None
        || settings.wave_amplitude <= 0.0
        || settings.wave_frequency <= 0.0
    {
        return;
    }
    let amplitude = settings.wave_amplitude * scale;
    let frequency = settings.wave_frequency / scale;
    let phase = TAU * f64::from(settings.wave_cycles) * t;
    let (w, h) = (bgr.width, bgr.height);
    let n = w as usize * h as usize;
    let mut map_x = vec![0.0f32; n];
    let mut map_y = vec![0.0f32; n];
    let horizontal = settings.wave_direction == WaveDirection::Horizontal;
    for r in 0..h {
        let row_off = if horizontal {
            (amplitude * (f64::from(r) * frequency + phase).sin()) as f32
        } else {
            0.0
        };
        for c in 0..w {
            let col_off = if horizontal {
                0.0
            } else {
                (amplitude * (f64::from(c) * frequency + phase).sin()) as f32
            };
            let i = r as usize * w as usize + c as usize;
            map_x[i] = c as f32 + row_off;
            map_y[i] = r as f32 + col_off;
        }
    }
    let warped = remap(&pad_bgr(bgr), &map_x, &map_y, w, h, Border::Replicate);
    *bgr = bgra_to_bgr(&warped);
}
