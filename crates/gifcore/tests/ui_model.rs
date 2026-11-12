//! Headless tests for the window logic in `gifcore::ui_model`.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use gifcore::render::FrameRenderer;
use gifcore::schema::{self, FieldKind, FIELDS};
use gifcore::settings::{
    GifSettings, KaleidoscopeMode, RotationDirection, StarPattern, WaveDirection, ZoomMode,
};
use gifcore::ui_model::*;

#[test]
fn groups_cover_every_field_once() {
    let mut seen = BTreeSet::new();
    for g in MAIN_GROUPS.iter().chain(ADVANCED_GROUPS) {
        for k in g.keys {
            assert!(schema::find(k).is_some(), "unknown key {k}");
            assert!(seen.insert(*k), "duplicate key {k}");
        }
    }
    let all: BTreeSet<_> = FIELDS.iter().map(|f| f.key).collect();
    assert_eq!(seen, all);
    assert_eq!(advanced_keys().count(), 19);
}

#[test]
fn slider_only_up_to_2000_steps() {
    assert!(has_slider(schema::find("output_width").unwrap()));
    assert!(has_slider(schema::find("num_stars").unwrap()));
    assert!(!has_slider(schema::find("star_seed").unwrap()));
}

#[test]
fn snap_rounds_to_field_decimals() {
    let f = schema::find("rotation_turns").unwrap();
    assert_eq!(f.decimals, 1);
    assert_eq!(snap(f, 0.1 + 0.2), 0.3);
    let w = schema::find("output_width").unwrap();
    assert_eq!(snap(w, 600.0), 600.0);
}

#[test]
fn visibility_follows_v2_rules() {
    let mut s = GifSettings::default();
    // Defaults: kaleidoscope None, stars Random, wave on, Oscillating, Clockwise, pulse 0.5.
    assert!(!row_visible(&s, "kaleidoscope_turns"));
    assert!(row_visible(&s, "num_stars"));
    assert!(row_visible(&s, "star_seed"));
    assert!(row_visible(&s, "wave_amplitude"));
    assert!(row_visible(&s, "zoom_amplitude"));
    assert!(!row_visible(&s, "tunnel_steps"));
    assert!(row_visible(&s, "rotation_turns"));
    assert!(row_visible(&s, "saturation_pulses"));
    assert!(row_visible(&s, "output_width"));

    s.kaleidoscope_mode = KaleidoscopeMode::Radial4;
    assert!(row_visible(&s, "kaleidoscope_turns"));
    s.kaleidoscope_mode = KaleidoscopeMode::QuadrantMirror;
    assert!(!row_visible(&s, "kaleidoscope_turns"));
    s.star_pattern = StarPattern::Spiral;
    assert!(row_visible(&s, "num_stars"));
    assert!(!row_visible(&s, "star_seed"));
    s.star_pattern = StarPattern::None;
    assert!(!row_visible(&s, "num_stars"));
    s.wave_direction = WaveDirection::None;
    assert!(!row_visible(&s, "wave_cycles"));
    s.zoom_mode = ZoomMode::Tunnel;
    assert!(!row_visible(&s, "zoom_midpoint"));
    assert!(row_visible(&s, "tunnel_steps"));
    s.rotation_direction = RotationDirection::None;
    assert!(!row_visible(&s, "rotation_turns"));
    s.saturation_pulse = 0.0;
    assert!(!row_visible(&s, "saturation_pulses"));
}

#[test]
fn fmt_num_trims_like_qt() {
    assert_eq!(fmt_num(3.0), "3");
    assert_eq!(fmt_num(2.5), "2.5");
    assert_eq!(fmt_num(0.1 + 0.2), "0.3");
    assert_eq!(fmt_num(0.0), "0");
}

#[test]
fn spin_rounding_message_only_when_rounded() {
    let mut s = GifSettings::default();
    assert_eq!(spin_rounding_message(&s), None); // 2 turns is whole
    s.rotation_turns = 2.5;
    let used = FrameRenderer::effective_rotation_turns(&s);
    assert_ne!(used, 2.5);
    assert_eq!(
        spin_rounding_message(&s).unwrap(),
        format!(
            "Spin 2.5 is rounded to {} whole turn(s) so the loop is seamless (exact in Tunnel mode).",
            fmt_num(used)
        )
    );
    s.zoom_mode = ZoomMode::Tunnel;
    assert_eq!(spin_rounding_message(&s), None);
    s.zoom_mode = ZoomMode::Oscillating;
    s.rotation_direction = RotationDirection::None;
    assert_eq!(spin_rounding_message(&s), None);
}

#[test]
fn match_aspect_sets_height_and_clamps() {
    let mut s = GifSettings::default(); // width 600
    match_aspect(&mut s, 400, 200);
    assert_eq!(s.output_height, 300);
    match_aspect(&mut s, 100, 100);
    assert_eq!(s.output_height, 600);
    match_aspect(&mut s, 10, 1000);
    assert_eq!(s.output_height, 1600); // schema max
    assert_eq!(s.output_width, 600);
}

#[test]
fn reset_all_keeps_image_path() {
    let mut s = GifSettings {
        image_path: PathBuf::from("/tmp/a.png"),
        output_width: 100,
        hue_cycles: 0,
        ..GifSettings::default()
    };
    reset_all(&mut s);
    let expected = GifSettings {
        image_path: PathBuf::from("/tmp/a.png"),
        ..GifSettings::default()
    };
    assert_eq!(s, expected);
}

#[test]
fn reset_advanced_touches_only_advanced_keys() {
    let mut s = GifSettings {
        output_width: 100,
        hue_cycles: 0,
        blur_radius: 5.0,
        num_stars: 7,
        ..GifSettings::default()
    };
    reset_advanced(&mut s);
    let d = GifSettings::default();
    assert_eq!(s.blur_radius, d.blur_radius);
    assert_eq!(s.num_stars, d.num_stars);
    assert_eq!(s.output_width, 100);
    assert_eq!(s.hue_cycles, 0);
}

#[test]
fn cosmic_chaos_low_samples_give_range_minimums() {
    let mut s = GifSettings::default();
    cosmic_chaos(&mut s, || 0.0);
    assert_eq!(s.zoom_amplitude, 0.10);
    assert_eq!(s.zoom_cycles, 1);
    assert_eq!(s.zoom_midpoint, 0.85);
    assert_eq!(s.tunnel_steps, 1);
    assert_eq!(s.saturation_pulse, 0.2);
    assert_eq!(s.saturation_pulses, 1);
    assert_eq!(s.invert_flashes, 1); // chance(0.25) is true for 0.0
    assert_eq!(s.kaleidoscope_mode, KaleidoscopeMode::None);
    assert_eq!(s.kaleidoscope_turns, 0);
    assert_eq!(s.star_pattern, StarPattern::None);
    assert_eq!(s.num_stars, 100);
    assert_eq!(s.star_seed, 1);
    assert_eq!(s.wave_direction, WaveDirection::None);
    assert_eq!(s.wave_amplitude, 1.0);
    assert_eq!(s.wave_frequency, 0.03);
    assert_eq!(s.wave_cycles, 1);
    assert_eq!(s.pixelation_level, 2);
    assert_eq!(s.blur_radius, 0.3);
    assert_eq!(s.vignette_strength, 0.3);
}

#[test]
fn cosmic_chaos_high_samples_turn_chance_effects_off() {
    let mut s = GifSettings::default();
    cosmic_chaos(&mut s, || 0.999_999);
    assert_eq!(s.zoom_cycles, 3);
    assert_eq!(s.invert_flashes, 0);
    assert_eq!(s.kaleidoscope_turns, 2);
    assert_eq!(s.star_pattern, StarPattern::Spiral);
    assert_eq!(s.num_stars, 500);
    assert_eq!(s.pixelation_level, 0);
    assert_eq!(s.blur_radius, 0.0);
    assert!(s.vignette_strength <= 1.0);
}

#[test]
fn cosmic_chaos_stays_in_range_and_leaves_main_keys() {
    let start = GifSettings {
        output_width: 320,
        fps: 20,
        rotation_turns: 3.0,
        ..GifSettings::default()
    };
    let advanced: BTreeSet<_> = advanced_keys().collect();
    let mut next = chaos_source(12345);
    for _ in 0..200 {
        let mut s = start.clone();
        cosmic_chaos(&mut s, &mut next);
        for f in FIELDS {
            let v = f.value(&s);
            assert!(v >= f.min && v <= f.max, "{} out of range: {v}", f.key);
            if !advanced.contains(f.key) {
                assert_eq!(v, f.value(&start), "{} changed", f.key);
            }
        }
        assert!((0.10..=0.40).contains(&s.zoom_amplitude));
        assert!((1..=4).contains(&s.saturation_pulses));
        assert!((100..=500).contains(&s.num_stars));
        assert!((1..=99999).contains(&s.star_seed));
    }
}

#[test]
fn chaos_source_is_in_unit_interval() {
    let mut next = chaos_source(1);
    for _ in 0..1000 {
        let r = next();
        assert!((0.0..1.0).contains(&r));
    }
}

#[test]
fn preview_scale_is_clamped() {
    let s = GifSettings::default(); // 600×600
    assert_eq!(preview_scale(&s, 300.0, 300.0), 0.5);
    assert_eq!(preview_scale(&s, 300.0, 1200.0), 0.5);
    assert_eq!(preview_scale(&s, 5000.0, 5000.0), 1.0);
    assert_eq!(preview_scale(&s, 1.0, 1.0), 0.05);
}

#[test]
fn play_interval_follows_fps() {
    assert_eq!(play_interval_ms(10), 100);
    assert_eq!(play_interval_ms(50), 20);
    assert_eq!(play_interval_ms(0), 1000);
}

#[test]
fn output_name_and_extension() {
    assert_eq!(
        suggested_output_name(Path::new("/a/b/photo.jpg")),
        "photo_psychedelic.gif"
    );
    assert_eq!(
        suggested_output_name(Path::new("archive.tar.gz")),
        "archive.tar_psychedelic.gif"
    );
    assert_eq!(
        suggested_output_name(Path::new("")),
        "output_psychedelic.gif"
    );
    assert_eq!(
        ensure_gif_extension(PathBuf::from("/tmp/x")),
        PathBuf::from("/tmp/x.gif")
    );
    assert_eq!(
        ensure_gif_extension(PathBuf::from("/tmp/x.GIF")),
        PathBuf::from("/tmp/x.GIF")
    );
    assert_eq!(
        ensure_gif_extension(PathBuf::from("/tmp/x.png")),
        PathBuf::from("/tmp/x.png.gif")
    );
}

#[test]
fn progress_text_and_percent() {
    assert_eq!(progress_percent(0, 60), 0);
    assert_eq!(progress_percent(30, 60), 50);
    assert_eq!(progress_percent(60, 60), 100);
    assert_eq!(progress_percent(1, 0), 0);
    assert_eq!(progress_text(30, 60), "Frame 30 of 60 (50%)");
}

#[test]
fn export_phase_transitions() {
    let mut p = ExportPhase::default();
    assert!(!p.is_busy());
    assert_eq!(p.status(), None);
    assert!(!p.cancel());
    assert!(p.start());
    assert!(!p.start());
    assert!(p.is_busy());
    assert_eq!(p.status(), Some("Launching hyperspace…"));
    assert!(p.cancel());
    assert!(!p.cancel());
    assert_eq!(p.status(), Some("Cancelling…"));
    p.finish();
    assert_eq!(p, ExportPhase::Idle);
}

#[test]
fn preview_queue_coalesces() {
    let mut q = PreviewQueue::default();
    assert!(q.request());
    assert!(q.is_running());
    assert!(!q.request());
    assert!(!q.request()); // still one queued render
    assert!(q.finished()); // run the queued one
    assert!(q.is_running());
    assert!(!q.finished());
    assert!(!q.is_running());
}

#[test]
fn kinds_have_expected_widgets() {
    // The UI maps Choice to a combo, Bool to a switch, numbers to slider+spin.
    assert_eq!(schema::find("fit_mode").unwrap().kind, FieldKind::Choice);
    assert_eq!(schema::find("dither").unwrap().kind, FieldKind::Bool);
}

#[test]
fn portal_paths_are_detected() {
    assert!(is_portal_path(Path::new(
        "/run/user/1000/doc/ab12/photo.png"
    )));
    assert!(!is_portal_path(Path::new("/home/nic/Pictures/photo.png")));
    assert!(!is_portal_path(Path::new("/run/user/1000/photo.png")));
    assert!(!is_portal_path(Path::new("relative/doc/x.png")));
}
