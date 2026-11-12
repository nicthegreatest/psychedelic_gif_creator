//! GTK-free logic behind the app's window (port of the non-widget parts of v2
//! `mainwindow.cpp`, `settings_form.cpp` and `advancedsettingsdialog.cpp`).
//!
//! The GTK crate only wires widgets to these functions, so everything here is
//! unit-tested headlessly.

use std::path::{Path, PathBuf};

use crate::render::FrameRenderer;
use crate::schema::{self, FieldInfo};
use crate::settings::{
    GifSettings, KaleidoscopeMode, RotationDirection, StarPattern, WaveDirection, ZoomMode,
};

/// A titled group of setting keys, in display order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Group {
    pub title: &'static str,
    pub keys: &'static [&'static str],
}

/// Groups on the main settings panel (v2 `buildUi`).
pub const MAIN_GROUPS: &[Group] = &[
    Group {
        title: "Output",
        keys: &[
            "output_width",
            "output_height",
            "fit_mode",
            "num_frames",
            "fps",
            "dither",
        ],
    },
    Group {
        title: "Tunnel and motion",
        keys: &[
            "max_layers",
            "scale_decay",
            "rotation_direction",
            "rotation_turns",
            "zoom_mode",
        ],
    },
    Group {
        title: "Colour",
        keys: &["hue_cycles"],
    },
];

/// Groups in the Advanced Effects window (v2 `advancedsettingsdialog.cpp`).
pub const ADVANCED_GROUPS: &[Group] = &[
    Group {
        title: "Zoom",
        keys: &[
            "zoom_amplitude",
            "zoom_cycles",
            "zoom_midpoint",
            "tunnel_steps",
        ],
    },
    Group {
        title: "Colour",
        keys: &["saturation_pulse", "saturation_pulses", "invert_flashes"],
    },
    Group {
        title: "Kaleidoscope",
        keys: &["kaleidoscope_mode", "kaleidoscope_turns"],
    },
    Group {
        title: "Stars",
        keys: &["star_pattern", "num_stars", "star_seed"],
    },
    Group {
        title: "Wave",
        keys: &[
            "wave_direction",
            "wave_amplitude",
            "wave_frequency",
            "wave_cycles",
        ],
    },
    Group {
        title: "Texture",
        keys: &["pixelation_level", "blur_radius", "vignette_strength"],
    },
];

/// Every key shown in the Advanced Effects window.
pub fn advanced_keys() -> impl Iterator<Item = &'static str> {
    ADVANCED_GROUPS.iter().flat_map(|g| g.keys.iter().copied())
}

/// Number rows get a slider only up to this many steps (v2 `SliderSpin`).
pub const MAX_SLIDER_STEPS: f64 = 2000.0;

/// Whether a number field gets a slider next to its spin button.
pub fn has_slider(field: &FieldInfo) -> bool {
    ((field.max - field.min) / field.step).round() <= MAX_SLIDER_STEPS
}

/// Rounds a widget value to the field's decimals, as a Qt spin box would.
pub fn snap(field: &FieldInfo, v: f64) -> f64 {
    let scale = 10f64.powi(field.decimals as i32);
    (v * scale).round() / scale
}

/// Whether the row for `key` is shown for these settings (v2 `updateVisibility`).
/// Keys without a rule are always visible.
pub fn row_visible(s: &GifSettings, key: &str) -> bool {
    match key {
        "kaleidoscope_turns" => matches!(
            s.kaleidoscope_mode,
            KaleidoscopeMode::Radial3 | KaleidoscopeMode::Radial4 | KaleidoscopeMode::Radial6
        ),
        "num_stars" => s.star_pattern != StarPattern::None,
        "star_seed" => s.star_pattern == StarPattern::Random,
        "wave_amplitude" | "wave_frequency" | "wave_cycles" => {
            s.wave_direction != WaveDirection::None
        }
        "zoom_amplitude" | "zoom_cycles" | "zoom_midpoint" => s.zoom_mode == ZoomMode::Oscillating,
        "tunnel_steps" => s.zoom_mode == ZoomMode::Tunnel,
        "rotation_turns" => s.rotation_direction != RotationDirection::None,
        "saturation_pulses" => s.saturation_pulse > 0.0,
        _ => true,
    }
}

/// Formats a number the way Qt's `QString::arg(double)` does for these values
/// (at most 6 significant decimals, no trailing zeros).
pub fn fmt_num(v: f64) -> String {
    let s = format!("{v:.6}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" {
        "0".to_string()
    } else {
        s.to_string()
    }
}

/// The toast shown when spin is rounded to whole turns (v2 `onSettingChanged`),
/// or `None` when the value is used as typed. Call after `rotation_turns` or
/// `zoom_mode` changes.
pub fn spin_rounding_message(s: &GifSettings) -> Option<String> {
    if s.zoom_mode == ZoomMode::Tunnel || s.rotation_direction == RotationDirection::None {
        return None;
    }
    let used = FrameRenderer::effective_rotation_turns(s);
    (used != s.rotation_turns).then(|| {
        format!(
            "Spin {} is rounded to {} whole turn(s) so the loop is seamless (exact in Tunnel mode).",
            fmt_num(s.rotation_turns),
            fmt_num(used)
        )
    })
}

/// "Match image aspect": height = width × image height / width, clamped.
pub fn match_aspect(s: &mut GifSettings, image_width: u32, image_height: u32) {
    if image_width == 0 {
        return;
    }
    let h = f64::from(s.output_width) * f64::from(image_height) / f64::from(image_width);
    if let Some(f) = schema::find("output_height") {
        f.set_value(s, h);
    }
}

/// "Reset all": every setting back to its default, keeping the image path.
pub fn reset_all(s: &mut GifSettings) {
    let path = std::mem::take(&mut s.image_path);
    *s = GifSettings::default();
    s.image_path = path;
}

/// "Effect defaults": only the Advanced Effects keys back to their defaults.
pub fn reset_advanced(s: &mut GifSettings) {
    for key in advanced_keys() {
        if let Some(f) = schema::find(key) {
            f.set_value(s, f.default_value());
        }
    }
}

/// "Cosmic Chaos": randomises the advanced effects with v2's ranges.
///
/// `next` returns uniform samples in `[0, 1)`. The app feeds it a time-seeded
/// generator; this only sets settings, so frames stay deterministic.
pub fn cosmic_chaos(s: &mut GifSettings, mut next: impl FnMut() -> f64) {
    let mut uni = |lo: f64, hi: f64| lo + (hi - lo) * next();
    // Inclusive integer range, like QRandomGenerator::bounded(lo, hi + 1).
    let pick = |r: f64, lo: i32, hi: i32| {
        let span = f64::from(hi - lo + 1);
        (lo + (r * span).floor() as i32).min(hi)
    };
    macro_rules! pick {
        ($lo:expr, $hi:expr) => {
            pick(uni(0.0, 1.0), $lo, $hi)
        };
    }
    macro_rules! chance {
        ($p:expr) => {
            uni(0.0, 1.0) < $p
        };
    }

    s.zoom_amplitude = uni(0.10, 0.40);
    s.zoom_cycles = pick!(1, 3);
    s.zoom_midpoint = uni(0.85, 1.10);
    s.tunnel_steps = pick!(1, 3);

    s.saturation_pulse = uni(0.2, 0.8);
    s.saturation_pulses = pick!(1, 4);
    s.invert_flashes = if chance!(0.25) { pick!(1, 4) } else { 0 };

    s.kaleidoscope_mode = if chance!(0.4) {
        KaleidoscopeMode::None
    } else {
        KaleidoscopeMode::from_index(pick!(1, 4) as usize).unwrap_or_default()
    };
    s.kaleidoscope_turns = pick!(0, 2);

    s.star_pattern = StarPattern::from_index(pick!(0, 2) as usize).unwrap_or_default();
    s.num_stars = pick!(100, 500);
    s.star_seed = pick!(1, 99999);

    s.wave_direction = WaveDirection::from_index(pick!(0, 2) as usize).unwrap_or_default();
    s.wave_amplitude = uni(1.0, 10.0);
    s.wave_frequency = uni(0.03, 0.20);
    s.wave_cycles = pick!(1, 3);

    s.pixelation_level = if chance!(0.2) { pick!(2, 6) } else { 0 };
    s.blur_radius = if chance!(0.3) { uni(0.3, 1.5) } else { 0.0 };
    s.vignette_strength = uni(0.3, 1.0);

    schema::clamp(s);
}

/// Uniform `[0, 1)` samples from a seeded mt19937, for [`cosmic_chaos`].
pub fn chaos_source(seed: u32) -> impl FnMut() -> f64 {
    let mut rng = crate::rng::Mt19937::new(seed);
    move || f64::from(rng.next_u32()) / 4_294_967_296.0
}

/// Preview render scale for a preview area of `box_w × box_h` device pixels:
/// `min(1, box / output)`, at least 0.05 (v2 `startPreview`).
pub fn preview_scale(s: &GifSettings, box_w: f64, box_h: f64) -> f64 {
    let w = f64::from(s.output_width.max(1));
    let h = f64::from(s.output_height.max(1));
    (box_w / w).min(box_h / h).clamp(0.05, 1.0)
}

/// Milliseconds between preview frames while playing (`1000 / fps`, rounded).
pub fn play_interval_ms(fps: i32) -> u32 {
    let fps = fps.max(1) as u32;
    (2000 + fps) / (2 * fps)
}

/// Suggested export file name: `<input stem>_psychedelic.gif`.
pub fn suggested_output_name(input: &Path) -> String {
    let stem = input
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "output".to_string());
    format!("{stem}_psychedelic.gif")
}

/// Appends `.gif` unless the path already ends in it (any case).
pub fn ensure_gif_extension(path: PathBuf) -> PathBuf {
    let has = path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("gif"));
    if has {
        path
    } else {
        let mut s = path.into_os_string();
        s.push(".gif");
        PathBuf::from(s)
    }
}

/// Whether `path` lies in the Flatpak document portal (`/run/user/<uid>/doc/…`).
/// Such folders are per-file grants, so they make poor dialog start folders.
pub fn is_portal_path(path: &Path) -> bool {
    let mut parts = path
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned());
    parts.next().as_deref() == Some("/")
        && parts.next().as_deref() == Some("run")
        && parts.next().as_deref() == Some("user")
        && parts.next().is_some()
        && parts.next().as_deref() == Some("doc")
}

/// Whole percent done, as v2 shows it (`done * 100 / total`).
pub fn progress_percent(done: usize, total: usize) -> usize {
    (done.min(total) * 100).checked_div(total).unwrap_or(0)
}

/// Progress bar text, e.g. "Frame 3 of 60 (5%)".
pub fn progress_text(done: usize, total: usize) -> String {
    format!(
        "Frame {done} of {total} ({}%)",
        progress_percent(done, total)
    )
}

/// Export state of the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExportPhase {
    #[default]
    Idle,
    Running,
    Cancelling,
}

impl ExportPhase {
    /// Starts an export; `false` if one is already running.
    pub fn start(&mut self) -> bool {
        if *self == Self::Idle {
            *self = Self::Running;
            true
        } else {
            false
        }
    }

    /// Requests cancellation; `true` if a running export should be told.
    pub fn cancel(&mut self) -> bool {
        if *self == Self::Running {
            *self = Self::Cancelling;
            true
        } else {
            false
        }
    }

    /// The worker finished (success, error or cancel).
    pub fn finish(&mut self) {
        *self = Self::Idle;
    }

    /// Settings, input and Generate are locked while busy.
    pub fn is_busy(self) -> bool {
        self != Self::Idle
    }

    /// Status shown in the header while busy.
    pub fn status(self) -> Option<&'static str> {
        match self {
            Self::Idle => None,
            Self::Running => Some("Launching hyperspace…"),
            Self::Cancelling => Some("Cancelling…"),
        }
    }
}

/// One preview job at a time; requests made while one runs are coalesced into
/// a single re-render of the latest state (v2 `m_previewQueued`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PreviewQueue {
    running: bool,
    queued: bool,
}

impl PreviewQueue {
    /// A render was requested; `true` means start one now.
    pub fn request(&mut self) -> bool {
        if self.running {
            self.queued = true;
            false
        } else {
            self.running = true;
            true
        }
    }

    /// The running render finished; `true` means start the queued one now.
    pub fn finished(&mut self) -> bool {
        if self.queued {
            self.queued = false;
            true
        } else {
            self.running = false;
            false
        }
    }

    /// Whether a render is in flight (Play drops frames while it is).
    pub fn is_running(self) -> bool {
        self.running
    }
}

/// Lines kept by the console before the oldest are dropped.
pub const LOG_CAP: usize = 500;

/// A field's value as console text: choice labels, "On"/"Off", trimmed numbers.
pub fn field_text(field: &FieldInfo, s: &GifSettings) -> String {
    let v = field.value(s);
    match field.kind {
        schema::FieldKind::Bool => (if v >= 0.5 { "On" } else { "Off" }).to_string(),
        schema::FieldKind::Choice => field.format(s),
        schema::FieldKind::Int | schema::FieldKind::Double => fmt_num(v),
    }
}

/// One `Label: old → new` line per field that differs, in schema order.
pub fn settings_diff(old: &GifSettings, new: &GifSettings) -> Vec<String> {
    schema::FIELDS
        .iter()
        .filter(|f| f.value(old) != f.value(new))
        .map(|f| {
            format!(
                "{}: {} → {}",
                f.label,
                field_text(f, old),
                field_text(f, new)
            )
        })
        .collect()
}

/// Read-only console log: `HH:MM:SS  message` lines, capped at a fixed count.
#[derive(Debug, Clone)]
pub struct Log {
    lines: std::collections::VecDeque<String>,
    cap: usize,
}

impl Default for Log {
    fn default() -> Self {
        Self::with_cap(LOG_CAP)
    }
}

impl Log {
    /// A log keeping at most `cap` lines (at least 1).
    pub fn with_cap(cap: usize) -> Self {
        Self {
            lines: std::collections::VecDeque::new(),
            cap: cap.max(1),
        }
    }

    /// Formats one line: the time, two spaces, the message.
    pub fn format_line(time: &str, message: &str) -> String {
        format!("{time}  {message}")
    }

    /// Appends a message stamped with `time` (e.g. "10:53:07"), dropping the
    /// oldest line once the cap is reached.
    pub fn push(&mut self, time: &str, message: &str) {
        if self.lines.len() == self.cap {
            self.lines.pop_front();
        }
        self.lines.push_back(Self::format_line(time, message));
    }

    pub fn clear(&mut self) {
        self.lines.clear();
    }

    pub fn len(&self) -> usize {
        self.lines.len()
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    pub fn lines(&self) -> impl Iterator<Item = &str> {
        self.lines.iter().map(String::as_str)
    }

    /// All lines joined with newlines (no trailing newline).
    pub fn text(&self) -> String {
        self.lines().collect::<Vec<_>>().join("\n")
    }
}
