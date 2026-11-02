//! Kaleidoscope, applied to the fitted image before the pyramid is built.
//!
//! Quadrant mirror and a static radial fold replace the base. A radial fold
//! with a non-zero turn count keeps the polar maps and rebuilds the base on
//! every frame (`kaleidoscopeFrame`).

use crate::image::Bgra8;
use crate::kernels::remap::{remap, remap_nearest};
use crate::kernels::warp::Border;
use crate::settings::{GifSettings, KaleidoscopeMode};

const PI: f64 = std::f64::consts::PI;
const TAU: f64 = 2.0 * PI;

/// Polar maps for a radial kaleidoscope, plus the image they sample.
pub struct AnimatedRadial {
    pub base: Bgra8,
    pub radius: Vec<f32>,
    pub angle: Vec<f32>,
    pub turns: i32,
}

/// What `prepare` did with the fitted image.
pub enum Prepared {
    /// Pyramid base (mirrored, or untouched when the effect is off).
    Static(Bgra8),
    /// Sample `base` through the maps on every frame.
    Animated(AnimatedRadial),
}

/// Mirror `base` if the settings ask for it.
pub fn prepare(base: Bgra8, settings: &GifSettings) -> Prepared {
    match settings.kaleidoscope_mode {
        KaleidoscopeMode::None => Prepared::Static(base),
        KaleidoscopeMode::QuadrantMirror => Prepared::Static(quadrant_mirror(&base)),
        KaleidoscopeMode::Radial3 | KaleidoscopeMode::Radial4 | KaleidoscopeMode::Radial6 => {
            let folds = match settings.kaleidoscope_mode {
                KaleidoscopeMode::Radial3 => 3,
                KaleidoscopeMode::Radial4 => 4,
                _ => 6,
            };
            let (radius, angle) = radial_maps(base.width, base.height, folds);
            if settings.kaleidoscope_turns == 0 {
                let mirrored = radial_frame(&base, &radius, &angle, 0, 0.0);
                Prepared::Static(mirrored)
            } else {
                Prepared::Animated(AnimatedRadial {
                    base,
                    radius,
                    angle,
                    turns: settings.kaleidoscope_turns,
                })
            }
        }
    }
}

/// One animated frame. `t` is the loop position in `0..=1`.
pub fn frame_at(state: &AnimatedRadial, t: f64) -> Bgra8 {
    radial_frame(&state.base, &state.radius, &state.angle, state.turns, t)
}

fn quadrant_mirror(base: &Bgra8) -> Bgra8 {
    let (w, h) = (base.width, base.height);
    let n = w as usize * h as usize;
    let mut map_x = vec![0.0f32; n];
    let mut map_y = vec![0.0f32; n];
    for r in 0..h {
        for c in 0..w {
            let i = r as usize * w as usize + c as usize;
            map_x[i] = c.min(w - 1 - c) as f32;
            map_y[i] = r.min(h - 1 - r) as f32;
        }
    }
    remap_nearest(base, &map_x, &map_y, w, h, Border::Reflect101)
}

fn radial_maps(width: u32, height: u32, folds: i32) -> (Vec<f32>, Vec<f32>) {
    let sector = TAU / f64::from(folds);
    let cx = (f64::from(width) - 1.0) / 2.0;
    let cy = (f64::from(height) - 1.0) / 2.0;
    let n = width as usize * height as usize;
    let mut radius = vec![0.0f32; n];
    let mut angle = vec![0.0f32; n];
    for r in 0..height {
        for c in 0..width {
            let dx = f64::from(c) - cx;
            let dy = f64::from(r) - cy;
            let mut a = dy.atan2(dx) + PI / 2.0;
            a %= sector;
            if a < 0.0 {
                a += sector;
            }
            if a > sector / 2.0 {
                a = sector - a;
            }
            let i = r as usize * width as usize + c as usize;
            radius[i] = (dx * dx + dy * dy).sqrt() as f32;
            angle[i] = (a - PI / 2.0 - sector / 4.0) as f32;
        }
    }
    (radius, angle)
}

fn radial_frame(base: &Bgra8, radius: &[f32], angle: &[f32], turns: i32, t: f64) -> Bgra8 {
    let phase = (TAU * f64::from(turns) * t) as f32;
    let cx = ((f64::from(base.width) - 1.0) / 2.0) as f32;
    let cy = ((f64::from(base.height) - 1.0) / 2.0) as f32;
    let map_x: Vec<f32> = radius
        .iter()
        .zip(angle)
        .map(|(&rad, &ang)| {
            let theta = ang + phase;
            rad * theta.cos() + cx
        })
        .collect();
    let map_y: Vec<f32> = radius
        .iter()
        .zip(angle)
        .map(|(&rad, &ang)| {
            let theta = ang + phase;
            rad * theta.sin() + cy
        })
        .collect();
    remap(
        base,
        &map_x,
        &map_y,
        base.width,
        base.height,
        Border::Reflect101,
    )
}
