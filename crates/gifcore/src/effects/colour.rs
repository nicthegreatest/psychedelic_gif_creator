//! Hue shift, saturation pulse and invert flashes.
//!
//! Hue and saturation go through OpenCV's 8-bit HSV and a 256-entry LUT.
//! Invert is `bitwise_not` on `k` evenly spaced frames of the loop.

use crate::image::Bgr8;
use crate::kernels::hsv::{bgr_to_hsv, hsv_to_bgr};
use crate::settings::GifSettings;

const TAU: f64 = 2.0 * std::f64::consts::PI;

/// Apply the colour effects in place.
pub fn apply(bgr: &mut Bgr8, settings: &GifSettings, t: f64, index: i32) {
    let hue_shift = (180.0 * f64::from(settings.hue_cycles) * t).round() as i32 % 180;
    let sat_mul =
        1.0 + settings.saturation_pulse * (TAU * f64::from(settings.saturation_pulses) * t).sin();
    if hue_shift != 0 || (sat_mul - 1.0).abs() > 1e-9 {
        let hsv = bgr_to_hsv(bgr);
        let mut shifted = Bgr8::new(bgr.width, bgr.height);
        for (px, out) in hsv
            .data
            .as_chunks::<3>()
            .0
            .iter()
            .zip(shifted.data.as_chunks_mut::<3>().0)
        {
            let v = i32::from(px[0]);
            let h = if v < 180 {
                (v + hue_shift + 180) % 180
            } else {
                v
            };
            out[0] = sat_u8(h);
            out[1] = sat_u8_f64(f64::from(px[1]) * sat_mul);
            out[2] = px[2];
        }
        *bgr = hsv_to_bgr(&shifted);
    }

    let n = settings.num_frames.max(1);
    let k = settings.invert_flashes;
    if k > 0 {
        let i = index.rem_euclid(n);
        let flash = (i64::from(i) * i64::from(k)) % i64::from(n) < i64::from(k);
        if flash {
            for byte in &mut bgr.data {
                *byte = !*byte;
            }
        }
    }
}

fn sat_u8(v: i32) -> u8 {
    v.clamp(0, 255) as u8
}

/// `saturate_cast<uchar>(cvRound(v))`.
fn sat_u8_f64(v: f64) -> u8 {
    v.round_ties_even().clamp(0.0, 255.0) as u8
}
