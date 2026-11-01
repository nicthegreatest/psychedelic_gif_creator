//! One table describing every setting (port of v2 `settings_schema.cpp`): key, label, tooltip,
//! range, plus clamped get/set and a `key=value` text format for presets and tests.

use crate::settings::{
    FitMode, GifSettings, KaleidoscopeMode, RotationDirection, StarPattern, WaveDirection, ZoomMode,
};

/// Value type of a field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldKind {
    Int,
    Double,
    Bool,
    Choice,
}

/// Description of one setting. Defaults live in [`GifSettings::default`].
#[derive(Debug)]
pub struct FieldInfo {
    /// Persistence key (same as v2).
    pub key: &'static str,
    pub label: &'static str,
    pub tooltip: &'static str,
    pub kind: FieldKind,
    pub min: f64,
    pub max: f64,
    pub step: f64,
    pub decimals: u32,
    /// Choice labels, index == enum variant (empty unless [`FieldKind::Choice`]).
    pub choices: &'static [&'static str],
    read: fn(&GifSettings) -> f64,
    write: fn(&mut GifSettings, f64),
}

impl FieldInfo {
    /// Current value as a number (choices as index, bools as 0/1).
    pub fn value(&self, s: &GifSettings) -> f64 {
        (self.read)(s)
    }

    /// Default value as a number.
    pub fn default_value(&self) -> f64 {
        self.value(&GifSettings::default())
    }

    /// Sets a numeric value, rounding and clamping like v2. Non-finite doubles become `min`
    /// (as in v2); non-finite values for other kinds are ignored.
    pub fn set_value(&self, s: &mut GifSettings, v: f64) {
        match self.kind {
            FieldKind::Double => {
                let v = if v.is_finite() {
                    v.clamp(self.min, self.max)
                } else {
                    self.min
                };
                (self.write)(s, v);
            }
            FieldKind::Bool => (self.write)(s, if v >= 0.5 { 1.0 } else { 0.0 }),
            FieldKind::Int | FieldKind::Choice => {
                if v.is_finite() {
                    (self.write)(s, v.round().clamp(self.min, self.max));
                }
            }
        }
    }

    /// Value as settings-file text (choices as label, bools as `true`/`false`).
    pub fn format(&self, s: &GifSettings) -> String {
        let v = self.value(s);
        match self.kind {
            FieldKind::Int => format!("{}", v as i64),
            FieldKind::Double => format!("{v}"),
            FieldKind::Bool => (if v >= 0.5 { "true" } else { "false" }).to_string(),
            FieldKind::Choice => self
                .choices
                .get(v as usize)
                .copied()
                .unwrap_or_default()
                .to_string(),
        }
    }

    /// Parses settings-file text and stores it (clamped). Returns `false` if the text is invalid.
    pub fn parse_into(&self, s: &mut GifSettings, text: &str) -> bool {
        let text = text.trim();
        let v = match self.kind {
            FieldKind::Choice => match self.choices.iter().position(|c| *c == text) {
                Some(i) => i as f64,
                None => return false,
            },
            FieldKind::Bool => match text.to_ascii_lowercase().as_str() {
                "true" | "1" => 1.0,
                "false" | "0" => 0.0,
                _ => return false,
            },
            FieldKind::Int => match text.parse::<f64>() {
                Ok(v) if v.is_finite() => v,
                _ => return false,
            },
            FieldKind::Double => match text.parse::<f64>() {
                Ok(v) => v,
                Err(_) => return false,
            },
        };
        self.set_value(s, v);
        true
    }
}

macro_rules! int_field {
    ($field:ident, $label:literal, $tip:literal, $lo:literal, $hi:literal) => {
        FieldInfo {
            key: stringify!($field),
            label: $label,
            tooltip: $tip,
            kind: FieldKind::Int,
            min: $lo as f64,
            max: $hi as f64,
            step: 1.0,
            decimals: 0,
            choices: &[],
            read: |s| f64::from(s.$field),
            write: |s, v| s.$field = v as i32,
        }
    };
}

macro_rules! dbl_field {
    ($field:ident, $label:literal, $tip:literal, $lo:literal, $hi:literal, $step:literal, $dec:literal) => {
        FieldInfo {
            key: stringify!($field),
            label: $label,
            tooltip: $tip,
            kind: FieldKind::Double,
            min: $lo,
            max: $hi,
            step: $step,
            decimals: $dec,
            choices: &[],
            read: |s| s.$field,
            write: |s, v| s.$field = v,
        }
    };
}

macro_rules! bool_field {
    ($field:ident, $label:literal, $tip:literal) => {
        FieldInfo {
            key: stringify!($field),
            label: $label,
            tooltip: $tip,
            kind: FieldKind::Bool,
            min: 0.0,
            max: 1.0,
            step: 1.0,
            decimals: 0,
            choices: &[],
            read: |s| if s.$field { 1.0 } else { 0.0 },
            write: |s, v| s.$field = v >= 0.5,
        }
    };
}

macro_rules! choice_field {
    ($field:ident, $ty:ident, $label:literal, $tip:literal) => {
        FieldInfo {
            key: stringify!($field),
            label: $label,
            tooltip: $tip,
            kind: FieldKind::Choice,
            min: 0.0,
            max: ($ty::LABELS.len() - 1) as f64,
            step: 1.0,
            decimals: 0,
            choices: $ty::LABELS,
            read: |s| s.$field.index() as f64,
            write: |s, v| {
                if let Some(e) = $ty::from_index(v as usize) {
                    s.$field = e;
                }
            },
        }
    };
}

/// Every setting, in v2's table order (also the UI and text-file order).
pub static FIELDS: &[FieldInfo] = &[
    // Output
    int_field!(output_width, "Width (px)", "Width of the generated GIF in pixels.", 64, 1600),
    int_field!(output_height, "Height (px)", "Height of the generated GIF in pixels.", 64, 1600),
    choice_field!(fit_mode, FitMode, "Image fit", "How the image is fitted to the output size: Fill crops, Fit letterboxes, Stretch distorts."),
    int_field!(num_frames, "Frames per loop", "Number of frames in one loop of the GIF.", 10, 240),
    int_field!(fps, "Frame rate (fps)", "Playback speed. GIF timing is stored in 1/100 s, so the real rate is 100 / round(100 / fps).", 2, 50),
    bool_field!(dither, "Dithering", "Floyd-Steinberg dithering: smoother gradients, slower export, usually a larger file."),
    // Tunnel
    int_field!(max_layers, "Tunnel layers", "Number of nested copies of the image that form the tunnel.", 1, 24),
    dbl_field!(scale_decay, "Layer size ratio", "Size of each tunnel layer relative to the one around it (0.50-0.99). Higher = more, tighter layers.", 0.50, 0.99, 0.01, 2),
    // Motion
    choice_field!(rotation_direction, RotationDirection, "Spin direction", "Direction the tunnel layers rotate."),
    dbl_field!(rotation_turns, "Spin (turns per loop)", "Full rotations per loop. Outside Tunnel zoom mode this is rounded to whole turns so the loop is seamless; in Tunnel mode fractional values are exact.", 0.0, 10.0, 0.1, 1),
    choice_field!(zoom_mode, ZoomMode, "Zoom mode", "Oscillating: the whole frame breathes in and out. Tunnel: endless flight into the tunnel."),
    dbl_field!(zoom_amplitude, "Zoom amount", "How far the oscillating zoom swings either side of the zoom centre.", 0.0, 0.6, 0.01, 2),
    int_field!(zoom_cycles, "Zoom pulses per loop", "Number of in-out zoom pulses per loop.", 1, 8),
    dbl_field!(zoom_midpoint, "Zoom centre", "Average zoom level (1 = none). Below 1 zooms out and mirrors the edges.", 0.5, 1.5, 0.01, 2),
    int_field!(tunnel_steps, "Tunnel speed (layers per loop)", "How many layers the camera flies through per loop.", 1, 6),
    // Colour
    int_field!(hue_cycles, "Colour cycles per loop", "How many times the colours go all the way round the colour wheel per loop (0 = off).", 0, 12),
    dbl_field!(saturation_pulse, "Saturation pulse", "Strength of the saturation pulsing (0 = off).", 0.0, 1.0, 0.01, 2),
    int_field!(saturation_pulses, "Saturation pulses per loop", "Number of saturation pulses per loop.", 1, 12),
    int_field!(invert_flashes, "Invert flashes per loop", "Number of evenly spaced frames per loop shown with inverted colours (0 = off).", 0, 30),
    // Effects
    choice_field!(kaleidoscope_mode, KaleidoscopeMode, "Kaleidoscope", "Mirror the image into a kaleidoscope before building the tunnel."),
    int_field!(kaleidoscope_turns, "Kaleidoscope turns per loop", "How many times the image turns under the kaleidoscope mirrors per loop (0 = static).", 0, 6),
    choice_field!(star_pattern, StarPattern, "Star pattern", "Twinkling stars drawn over the tunnel."),
    int_field!(num_stars, "Stars", "Number of stars.", 0, 1000),
    int_field!(star_seed, "Star layout seed", "Different numbers give different random star layouts.", 0, 99999),
    choice_field!(wave_direction, WaveDirection, "Wave direction", "Ripple distortion direction."),
    dbl_field!(wave_amplitude, "Wave height (px)", "How far the ripple displaces pixels.", 0.0, 50.0, 0.1, 1),
    dbl_field!(wave_frequency, "Wave density", "How tightly packed the ripples are.", 0.0, 1.0, 0.01, 2),
    int_field!(wave_cycles, "Wave cycles per loop", "How many times the ripple travels its full length per loop (0 = static).", 0, 10),
    int_field!(pixelation_level, "Pixelation (px)", "Pixel block size (0 or 1 = off).", 0, 20),
    dbl_field!(blur_radius, "Blur (px)", "Soft-focus haze (Gaussian blur radius).", 0.0, 10.0, 0.1, 1),
    dbl_field!(vignette_strength, "Vignette", "Darkens the corners (0 = off).", 0.0, 1.0, 0.01, 2),
];

/// Looks up a field by key.
pub fn find(key: &str) -> Option<&'static FieldInfo> {
    FIELDS.iter().find(|f| f.key == key)
}

/// Value of `key` as settings-file text, or `None` for an unknown key.
pub fn get(s: &GifSettings, key: &str) -> Option<String> {
    find(key).map(|f| f.format(s))
}

/// Sets `key` from text, clamped to its range. Returns `false` for an unknown key or invalid text
/// (the setting is then unchanged).
pub fn set(s: &mut GifSettings, key: &str, text: &str) -> bool {
    find(key).is_some_and(|f| f.parse_into(s, text))
}

/// Clamps every field into its range (v2 `clampSettings`).
pub fn clamp(s: &mut GifSettings) {
    for f in FIELDS {
        let v = f.value(s);
        f.set_value(s, v);
    }
}

/// All fields as `key=value` lines in table order (no image path).
pub fn to_text(s: &GifSettings) -> String {
    FIELDS
        .iter()
        .map(|f| format!("{}={}\n", f.key, f.format(s)))
        .collect()
}

/// Parses `key=value` lines onto the defaults. Blank lines, `#` comments, unknown keys and invalid
/// values are ignored; out-of-range numbers are clamped.
pub fn from_text(text: &str) -> GifSettings {
    let mut s = GifSettings::default();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            set(&mut s, key.trim(), value);
        }
    }
    s
}
