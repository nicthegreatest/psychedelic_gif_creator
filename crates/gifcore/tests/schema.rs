//! Settings + schema tests: defaults match v2 `gif_settings.h`, text round trip, clamping.

use std::collections::HashSet;

use gifcore::schema::{self, FieldKind, FIELDS};
use gifcore::settings::{
    FitMode, GifSettings, KaleidoscopeMode, RotationDirection, StarPattern, WaveDirection, ZoomMode,
};

/// Keys in v2 `buildFields` order.
const V2_KEYS: [&str; 31] = [
    "output_width",
    "output_height",
    "fit_mode",
    "num_frames",
    "fps",
    "dither",
    "max_layers",
    "scale_decay",
    "rotation_direction",
    "rotation_turns",
    "zoom_mode",
    "zoom_amplitude",
    "zoom_cycles",
    "zoom_midpoint",
    "tunnel_steps",
    "hue_cycles",
    "saturation_pulse",
    "saturation_pulses",
    "invert_flashes",
    "kaleidoscope_mode",
    "kaleidoscope_turns",
    "star_pattern",
    "num_stars",
    "star_seed",
    "wave_direction",
    "wave_amplitude",
    "wave_frequency",
    "wave_cycles",
    "pixelation_level",
    "blur_radius",
    "vignette_strength",
];

#[test]
fn defaults_match_v2() {
    let s = GifSettings::default();
    assert!(s.image_path.as_os_str().is_empty());
    assert_eq!(s.output_width, 600);
    assert_eq!(s.output_height, 600);
    assert_eq!(s.fit_mode, FitMode::Fill);
    assert_eq!(s.num_frames, 60);
    assert_eq!(s.fps, 12);
    assert!(!s.dither);
    assert_eq!(s.max_layers, 12);
    assert_eq!(s.scale_decay, 0.92);
    assert_eq!(s.rotation_direction, RotationDirection::Clockwise);
    assert_eq!(s.rotation_turns, 2.0);
    assert_eq!(s.zoom_mode, ZoomMode::Oscillating);
    assert_eq!(s.zoom_amplitude, 0.40);
    assert_eq!(s.zoom_cycles, 2);
    assert_eq!(s.zoom_midpoint, 0.82);
    assert_eq!(s.tunnel_steps, 1);
    assert_eq!(s.hue_cycles, 4);
    assert_eq!(s.saturation_pulse, 0.5);
    assert_eq!(s.saturation_pulses, 3);
    assert_eq!(s.invert_flashes, 0);
    assert_eq!(s.kaleidoscope_mode, KaleidoscopeMode::None);
    assert_eq!(s.kaleidoscope_turns, 1);
    assert_eq!(s.num_stars, 300);
    assert_eq!(s.star_pattern, StarPattern::Random);
    assert_eq!(s.star_seed, 1);
    assert_eq!(s.wave_direction, WaveDirection::Horizontal);
    assert_eq!(s.wave_amplitude, 3.6);
    assert_eq!(s.wave_frequency, 0.10);
    assert_eq!(s.wave_cycles, 1);
    assert_eq!(s.pixelation_level, 0);
    assert_eq!(s.blur_radius, 0.0);
    assert_eq!(s.vignette_strength, 1.0);
}

#[test]
fn enum_defaults_and_labels() {
    assert_eq!(FitMode::default(), FitMode::Fill);
    assert_eq!(ZoomMode::default(), ZoomMode::Oscillating);
    assert_eq!(StarPattern::default(), StarPattern::Random);
    assert_eq!(WaveDirection::default(), WaveDirection::Horizontal);
    assert_eq!(FitMode::Fit.label(), "Fit (letterbox)");
    assert_eq!(
        RotationDirection::CounterClockwise.label(),
        "Counter-clockwise"
    );
    assert_eq!(ZoomMode::Tunnel.label(), "Tunnel (infinite zoom)");
    assert_eq!(KaleidoscopeMode::Radial6.label(), "Radial 6-fold");
    assert_eq!(
        KaleidoscopeMode::from_index(1),
        Some(KaleidoscopeMode::QuadrantMirror)
    );
    assert_eq!(KaleidoscopeMode::from_index(5), None);
}

#[test]
fn keys_unique_and_in_v2_order() {
    assert_eq!(FIELDS.len(), 31);
    let keys: Vec<&str> = FIELDS.iter().map(|f| f.key).collect();
    assert_eq!(keys, V2_KEYS);
    let unique: HashSet<&str> = keys.iter().copied().collect();
    assert_eq!(unique.len(), 31);
}

#[test]
fn defaults_are_within_range() {
    for f in FIELDS {
        let d = f.default_value();
        assert!(
            d >= f.min && d <= f.max,
            "{} default {d} out of range",
            f.key
        );
        if f.kind == FieldKind::Choice {
            assert_eq!(f.max as usize + 1, f.choices.len(), "{}", f.key);
        } else {
            assert!(f.choices.is_empty(), "{}", f.key);
        }
    }
}

#[test]
fn field_metadata_spot_checks() {
    let f = schema::find("scale_decay").unwrap();
    assert_eq!(f.label, "Layer size ratio");
    assert_eq!(f.kind, FieldKind::Double);
    assert_eq!((f.min, f.max, f.step, f.decimals), (0.50, 0.99, 0.01, 2));

    let f = schema::find("star_seed").unwrap();
    assert_eq!(f.label, "Star layout seed");
    assert_eq!(f.kind, FieldKind::Int);
    assert_eq!((f.min, f.max), (0.0, 99999.0));

    let f = schema::find("fps").unwrap();
    assert_eq!(
        f.tooltip,
        "Playback speed. GIF timing is stored in 1/100 s, so the real rate is 100 / round(100 / fps)."
    );

    let f = schema::find("kaleidoscope_mode").unwrap();
    assert_eq!(f.kind, FieldKind::Choice);
    assert_eq!(f.choices, KaleidoscopeMode::LABELS);
    assert_eq!(f.max, 4.0);

    assert_eq!(schema::find("dither").unwrap().kind, FieldKind::Bool);
    assert!(schema::find("image_path").is_none());
}

#[test]
fn get_formats_like_v2_save() {
    let s = GifSettings::default();
    assert_eq!(schema::get(&s, "fit_mode").as_deref(), Some("Fill (crop)"));
    assert_eq!(schema::get(&s, "dither").as_deref(), Some("false"));
    assert_eq!(schema::get(&s, "fps").as_deref(), Some("12"));
    assert_eq!(schema::get(&s, "scale_decay").as_deref(), Some("0.92"));
    assert_eq!(schema::get(&s, "rotation_turns").as_deref(), Some("2"));
    assert_eq!(schema::get(&s, "nope"), None);
}

#[test]
fn round_trip_defaults() {
    let s = GifSettings::default();
    let text = schema::to_text(&s);
    assert_eq!(text.lines().count(), 31);
    assert!(text.starts_with("output_width=600\noutput_height=600\nfit_mode=Fill (crop)\n"));
    assert_eq!(schema::from_text(&text), s);
}

#[test]
fn round_trip_modified() {
    let s = GifSettings {
        output_width: 1280,
        output_height: 720,
        fit_mode: FitMode::Fit,
        fps: 25,
        dither: true,
        scale_decay: 0.73,
        rotation_direction: RotationDirection::CounterClockwise,
        rotation_turns: 0.7,
        zoom_mode: ZoomMode::Tunnel,
        tunnel_steps: 3,
        kaleidoscope_mode: KaleidoscopeMode::Radial6,
        star_pattern: StarPattern::Spiral,
        star_seed: 4242,
        wave_direction: WaveDirection::Vertical,
        wave_frequency: 0.123456789,
        blur_radius: 2.5,
        vignette_strength: 0.0,
        ..GifSettings::default()
    };
    assert_eq!(schema::from_text(&schema::to_text(&s)), s);
}

#[test]
fn clamps_out_of_range_values() {
    let s = schema::from_text(
        "num_frames=100000\noutput_width=1\nfps=-5\nscale_decay=2.5\nzoom_midpoint=0.1\nstar_seed=1e9\n",
    );
    assert_eq!(s.num_frames, 240);
    assert_eq!(s.output_width, 64);
    assert_eq!(s.fps, 2);
    assert_eq!(s.scale_decay, 0.99);
    assert_eq!(s.zoom_midpoint, 0.5);
    assert_eq!(s.star_seed, 99999);

    let mut s = GifSettings::default();
    assert!(schema::set(&mut s, "max_layers", "12.6"));
    assert_eq!(s.max_layers, 13);
    // Non-finite doubles become the minimum, as in v2.
    assert!(schema::set(&mut s, "wave_amplitude", "NaN"));
    assert_eq!(s.wave_amplitude, 0.0);

    let mut s = GifSettings {
        blur_radius: 99.0,
        hue_cycles: -3,
        ..GifSettings::default()
    };
    schema::clamp(&mut s);
    assert_eq!(s.blur_radius, 10.0);
    assert_eq!(s.hue_cycles, 0);
}

#[test]
fn ignores_unknown_keys_and_garbage() {
    let text = "\
# comment
bogus_key=5
fps=fast
dither=maybe
fit_mode=Squash
fit_mode=fill (crop)
max_layers=inf
no equals sign here

  hue_cycles = 7  
";
    let s = schema::from_text(text);
    let expected = GifSettings {
        hue_cycles: 7,
        ..GifSettings::default()
    };
    assert_eq!(s, expected);

    let mut s = GifSettings::default();
    assert!(!schema::set(&mut s, "bogus_key", "1"));
    assert!(!schema::set(&mut s, "num_stars", ""));
    assert!(!schema::set(&mut s, "zoom_mode", "2"));
    assert_eq!(s, GifSettings::default());
    assert!(schema::set(&mut s, "zoom_mode", "None"));
    assert_eq!(s.zoom_mode, ZoomMode::None);
    assert!(schema::set(&mut s, "dither", "TRUE"));
    assert!(s.dither);
}

#[test]
fn frame_delay_cs_examples() {
    let delay = |fps| {
        GifSettings {
            fps,
            ..GifSettings::default()
        }
        .frame_delay_cs()
    };
    assert_eq!(delay(12), 8); // 8.33
    assert_eq!(delay(8), 13); // 12.5 rounds half away from zero
    assert_eq!(delay(10), 10);
    assert_eq!(delay(2), 50);
    assert_eq!(delay(50), 2);
    assert_eq!(delay(200), 2); // 0.5 -> 1 -> clamped to 2
    assert_eq!(delay(1), 100);
    assert_eq!(delay(0), 100); // fps < 1 treated as 1
}
