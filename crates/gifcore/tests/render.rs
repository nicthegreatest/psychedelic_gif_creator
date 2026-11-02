//! Ports of the loop, determinism, stars and size checks in v2
//! `tests/render_tests.cpp`. Loader and schema checks live in their own files.

use gifcore::image::{Bgr8, Bgra8};
use gifcore::render::FrameRenderer;
use gifcore::settings::{
    FitMode, GifSettings, KaleidoscopeMode, RotationDirection, StarPattern, WaveDirection, ZoomMode,
};

fn test_image(w: u32, h: u32, with_alpha: bool) -> Bgra8 {
    let mut img = Bgra8::new(w, h);
    for r in 0..h {
        for c in 0..w {
            let px = img.pixel_mut(c, r);
            px[0] = ((255 * c) / w) as u8;
            px[1] = ((255 * r) / h) as u8;
            px[2] = ((c * r) % 256) as u8;
            px[3] = 255;
        }
    }
    // Same shapes as v2's test image, with a plain rasteriser. The loop checks
    // compare a renderer with itself, so the circle does not have to match OpenCV.
    fill_circle(
        &mut img,
        (w / 3) as i32,
        (h / 3) as i32,
        (h / 5) as i32,
        [30, 220, 250, 255],
    );
    fill_rect(
        &mut img,
        w / 2,
        h / 2,
        w.saturating_sub(10),
        h.saturating_sub(10),
        [200, 40, 160, 255],
    );
    if with_alpha {
        fill_circle(
            &mut img,
            (w / 2) as i32,
            (h / 2) as i32,
            (h / 6) as i32,
            [0, 0, 0, 0],
        );
    }
    img
}

fn fill_rect(img: &mut Bgra8, x0: u32, y0: u32, x1: u32, y1: u32, px: [u8; 4]) {
    for y in y0..y1.min(img.height) {
        for x in x0..x1.min(img.width) {
            img.pixel_mut(x, y).copy_from_slice(&px);
        }
    }
}

fn fill_circle(img: &mut Bgra8, cx: i32, cy: i32, radius: i32, px: [u8; 4]) {
    if radius <= 0 {
        return;
    }
    let r2 = radius * radius;
    let x0 = (cx - radius).max(0) as u32;
    let y0 = (cy - radius).max(0) as u32;
    let x1 = (cx + radius + 1).clamp(0, img.width as i32) as u32;
    let y1 = (cy + radius + 1).clamp(0, img.height as i32) as u32;
    for y in y0..y1 {
        for x in x0..x1 {
            let dx = x as i32 - cx;
            let dy = y as i32 - cy;
            if dx * dx + dy * dy <= r2 {
                img.pixel_mut(x, y).copy_from_slice(&px);
            }
        }
    }
}

fn base_settings() -> GifSettings {
    GifSettings {
        output_width: 160,
        output_height: 120,
        num_frames: 24,
        ..GifSettings::default()
    }
}

/// Mean absolute difference per channel, matching v2 `meanAbsDiff`.
fn mean_abs_diff(a: &Bgr8, b: &Bgr8) -> f64 {
    assert_eq!(a.width, b.width);
    assert_eq!(a.height, b.height);
    let sum: u64 = a
        .data
        .iter()
        .zip(&b.data)
        .map(|(p, q)| u64::from(p.abs_diff(*q)))
        .sum();
    sum as f64 / a.data.len() as f64
}

fn check_loop(name: &str, settings: &GifSettings, src: &Bgra8) {
    let renderer = FrameRenderer::new(settings, src);
    let expect = FrameRenderer::output_size(settings, 1.0);
    let f0 = renderer.render_frame(0);
    let f1 = renderer.render_frame(1);
    let f_n = renderer.render_frame(renderer.frame_count());
    let f0_again = renderer.render_frame(0);
    assert_eq!((f0.width, f0.height), expect, "{name}: size");
    assert_eq!(f0.data.len(), expect.0 as usize * expect.1 as usize * 3);
    let seam = mean_abs_diff(&f0, &f_n);
    let step = mean_abs_diff(&f0, &f1);
    println!("[loop] {name}: seam {seam:.4}, frame-to-frame {step:.4}");
    assert!(seam < 0.05, "{name}: loop seam too visible ({seam:.4})");
    assert!(
        step > 10.0 * seam || step < 1e-9,
        "{name}: seam ({seam:.4}) not clearly below normal motion ({step:.4})"
    );
    assert_eq!(
        f0.data, f0_again.data,
        "{name}: render is not deterministic"
    );
    let again = FrameRenderer::new(settings, src).render_frame(0);
    assert_eq!(f0.data, again.data, "{name}: two renderers disagree");
    let mut i = 2;
    while i < renderer.frame_count() {
        let _ = renderer.render_frame(i);
        i += 7;
    }
}

#[test]
fn loop_defaults() {
    check_loop("defaults", &base_settings(), &test_image(320, 200, false));
}

#[test]
fn loop_tunnel_fractional_spin() {
    let mut s = base_settings();
    s.zoom_mode = ZoomMode::Tunnel;
    s.tunnel_steps = 2;
    s.rotation_turns = 0.7;
    assert_eq!(FrameRenderer::effective_rotation_turns(&s), 0.7);
    check_loop(
        "tunnel zoom, fractional spin 0.7",
        &s,
        &test_image(320, 200, false),
    );
}

#[test]
fn loop_oscillating_spin_rounds_up() {
    let mut s = base_settings();
    s.rotation_turns = 0.3;
    s.rotation_direction = RotationDirection::CounterClockwise;
    assert_eq!(FrameRenderer::effective_rotation_turns(&s), 1.0);
    check_loop(
        "oscillating zoom, spin 0.3 (rounded)",
        &s,
        &test_image(320, 200, false),
    );
}

#[test]
fn loop_radial6_two_turns() {
    let mut s = base_settings();
    s.kaleidoscope_mode = KaleidoscopeMode::Radial6;
    s.kaleidoscope_turns = 2;
    check_loop(
        "radial 6-fold kaleidoscope, 2 turns",
        &s,
        &test_image(320, 200, false),
    );
}

#[test]
fn stars_visible_on_a_still_frame() {
    let mut s = base_settings();
    s.vignette_strength = 0.0;
    s.hue_cycles = 0;
    s.saturation_pulse = 0.0;
    s.wave_direction = WaveDirection::None;
    s.zoom_mode = ZoomMode::None;
    let mut no_stars = s.clone();
    no_stars.star_pattern = StarPattern::None;
    let src = test_image(320, 200, false);
    let with = FrameRenderer::new(&s, &src).render_frame(3);
    let without = FrameRenderer::new(&no_stars, &src).render_frame(3);
    let diff = mean_abs_diff(&with, &without);
    println!("[stars] frame 3 mean abs diff {diff:.4}");
    assert!(diff > 0.2, "stars not visible (diff {diff:.4})");
}

#[test]
fn loop_everything_on_letterboxed() {
    let mut s = base_settings();
    s.kaleidoscope_mode = KaleidoscopeMode::QuadrantMirror;
    s.star_pattern = StarPattern::Spiral;
    s.wave_direction = WaveDirection::Vertical;
    s.wave_cycles = 3;
    s.pixelation_level = 4;
    s.invert_flashes = 3;
    s.blur_radius = 1.0;
    s.fit_mode = FitMode::Fit;
    check_loop(
        "everything on, letterboxed",
        &s,
        &test_image(320, 200, false),
    );
}

#[test]
fn loop_tunnel_transparent_source() {
    let mut s = base_settings();
    s.zoom_mode = ZoomMode::Tunnel;
    s.fit_mode = FitMode::Fit;
    s.rotation_turns = 1.5;
    check_loop(
        "tunnel with transparent source",
        &s,
        &test_image(200, 200, true),
    );
}

#[test]
fn aspect_16_9_and_preview_scale() {
    let mut s = base_settings();
    s.output_width = 320;
    s.output_height = 180;
    // Output size does not depend on the source pixels. v2 builds a 1280×720
    // image here only to check the frame size.
    let renderer = FrameRenderer::new(&s, &test_image(64, 36, false));
    assert_eq!(renderer.frame_size(), (320, 180));
    let preview = FrameRenderer::with_scale(&s, &test_image(320, 200, false), 0.5);
    assert_eq!(preview.frame_size(), (160, 90));
    assert_eq!(FrameRenderer::output_size(&s, 0.5), (160, 90));
}

#[test]
fn non_positive_preview_scale_is_full_size() {
    let s = base_settings();
    let src = test_image(40, 30, false);
    let full = FrameRenderer::new(&s, &src);
    let rewritten = FrameRenderer::with_scale(&s, &src, 0.0);
    assert_eq!(rewritten.frame_size(), full.frame_size());
}

#[test]
fn opaque_same_size_one_layer_is_the_source() {
    // No hand-computed pixels: with a single unrotated layer and the effects
    // that change pixels switched off, frame 0 is the source itself.
    let mut s = base_settings();
    s.output_width = 48;
    s.output_height = 32;
    s.fit_mode = FitMode::Stretch;
    s.max_layers = 1;
    s.zoom_mode = ZoomMode::None;
    s.vignette_strength = 0.0;
    s.rotation_direction = RotationDirection::None;
    s.star_pattern = StarPattern::None;
    s.wave_direction = WaveDirection::None;
    s.hue_cycles = 0;
    s.saturation_pulse = 0.0;
    let src = test_image(48, 32, false);
    let frame = FrameRenderer::new(&s, &src).render_frame(0);
    assert_eq!(frame.width, 48);
    assert_eq!(frame.height, 32);
    for y in 0..32 {
        for x in 0..48 {
            let bgra = src.pixel(x, y);
            assert_eq!(frame.pixel(x, y), &bgra[..3], "pixel {x},{y}");
        }
    }
}
