//! Star overlay. Positions come from [`crate::rng`] once per renderer.
//! Circles are OpenCV filled `LINE_AA`, then a per-channel max (lighten).

use crate::image::Bgra8;
use crate::kernels::aa_circle::circle_filled_aa;
use crate::rng::{Mt19937, UniformReal};
use crate::settings::StarPattern;

const TAU: f64 = 2.0 * std::f64::consts::PI;

/// One star, in the order v2 pushes them (x, y, phase, brightness).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Star {
    pub x: f32,
    pub y: f32,
    pub phase: f32,
    pub brightness: f32,
}

/// `num_stars` samples from `mt19937(seed)`: three uniforms in `[0, 1)` and
/// one brightness in `[0.5, 1)`. An empty pattern or a non-positive count
/// yields no stars.
pub fn layout(count: i32, seed: i32, pattern: StarPattern) -> Vec<Star> {
    if pattern == StarPattern::None || count <= 0 {
        return Vec::new();
    }
    let mut rng = Mt19937::new(seed as u32);
    let unit = UniformReal::new(0.0, 1.0);
    let brightness = UniformReal::new(0.5, 1.0);
    (0..count)
        .map(|_| Star {
            x: unit.sample(&mut rng),
            y: unit.sample(&mut rng),
            phase: unit.sample(&mut rng),
            brightness: brightness.sample(&mut rng),
        })
        .collect()
}

/// Draw `stars` onto `frame` and keep the brighter channel at each pixel.
pub fn draw(frame: &mut Bgra8, stars: &[Star], pattern: StarPattern, t: f64) {
    if stars.is_empty() || pattern == StarPattern::None {
        return;
    }
    let (w, h) = (frame.width, frame.height);
    let unit = f64::from(w.min(h)) / 600.0;
    let radius = (1.2 * unit).round().max(1.0) as i32;
    let mut layer = Bgra8::new(w, h);
    if pattern == StarPattern::Random {
        for star in stars {
            let v = 255.0
                * f64::from(star.brightness)
                * (0.55 + 0.45 * (TAU * (2.0 * t + f64::from(star.phase))).sin());
            let x = (star.x * w as f32) as i32;
            let y = (star.y * h as f32) as i32;
            circle_filled_aa(&mut layer, (x, y), radius, grey(v));
        }
    } else {
        for (j, star) in stars.iter().enumerate() {
            let angle = 0.1 * j as f64 + TAU * t;
            let reach = 2.0 * j as f64 * unit;
            let x = (f64::from(w) / 2.0 + reach * angle.cos()) as i32;
            let y = (f64::from(h) / 2.0 + reach * angle.sin()) as i32;
            if x < 0 || x >= w as i32 || y < 0 || y >= h as i32 {
                continue;
            }
            let v = 255.0 * (0.6 + 0.4 * (TAU * (t + f64::from(star.phase))).sin());
            circle_filled_aa(&mut layer, (x, y), radius, grey(v));
        }
    }
    for (dst, src) in frame.data.iter_mut().zip(&layer.data) {
        *dst = (*dst).max(*src);
    }
}

/// `saturate_cast<uchar>(cvRound(v))` for the grey BGRA star colour.
fn grey(v: f64) -> [u8; 4] {
    let c = v.round_ties_even().clamp(0.0, 255.0) as u8;
    [c, c, c, 255]
}
