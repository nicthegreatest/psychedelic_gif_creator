//! Every user-tunable parameter, with the one set of defaults (port of v2 `gif_settings.h`).

use std::path::PathBuf;

/// Defines a choice enum plus its human-readable labels (index == variant order, as in v2).
macro_rules! choice_enum {
    (
        $(#[$meta:meta])*
        $name:ident { $( $(#[$vmeta:meta])* $variant:ident => $label:literal ),+ $(,)? }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
        pub enum $name {
            $( $(#[$vmeta])* $variant ),+
        }

        impl $name {
            /// Every variant, in label order.
            pub const ALL: &'static [$name] = &[$($name::$variant),+];
            /// UI / settings-file labels, indexed like [`Self::ALL`].
            pub const LABELS: &'static [&'static str] = &[$($label),+];

            /// Position of this variant in [`Self::ALL`].
            pub fn index(self) -> usize {
                self as usize
            }

            /// Variant at `index`, if any.
            pub fn from_index(index: usize) -> Option<Self> {
                Self::ALL.get(index).copied()
            }

            /// Human-readable label.
            pub fn label(self) -> &'static str {
                Self::LABELS[self.index()]
            }
        }
    };
}

choice_enum! {
    /// How the source image is fitted to the output size.
    FitMode {
        #[default]
        Fill => "Fill (crop)",
        Fit => "Fit (letterbox)",
        Stretch => "Stretch",
    }
}

choice_enum! {
    /// Direction the tunnel layers rotate.
    RotationDirection {
        #[default]
        Clockwise => "Clockwise",
        CounterClockwise => "Counter-clockwise",
        None => "None",
    }
}

choice_enum! {
    /// Zoom behaviour.
    ZoomMode {
        None => "None",
        #[default]
        Oscillating => "Oscillating",
        Tunnel => "Tunnel (infinite zoom)",
    }
}

choice_enum! {
    /// Star overlay pattern.
    StarPattern {
        None => "None",
        #[default]
        Random => "Random",
        Spiral => "Spiral",
    }
}

choice_enum! {
    /// Ripple distortion direction.
    WaveDirection {
        None => "None",
        #[default]
        Horizontal => "Horizontal",
        Vertical => "Vertical",
    }
}

choice_enum! {
    /// Kaleidoscope mirroring applied before the tunnel is built.
    KaleidoscopeMode {
        #[default]
        None => "None",
        QuadrantMirror => "Quadrant mirror",
        Radial3 => "Radial 3-fold",
        Radial4 => "Radial 4-fold",
        Radial6 => "Radial 6-fold",
    }
}

/// All render settings. "Per loop" values are whole cycles so the GIF loops seamlessly.
#[derive(Debug, Clone, PartialEq)]
pub struct GifSettings {
    /// Source image (not part of the schema table, like v2).
    pub image_path: PathBuf,

    // Output
    pub output_width: i32,
    pub output_height: i32,
    pub fit_mode: FitMode,
    pub num_frames: i32,
    /// Stored in the GIF as a 1/100 s delay, see [`GifSettings::frame_delay_cs`].
    pub fps: i32,
    pub dither: bool,

    // Tunnel layers
    pub max_layers: i32,
    /// Size of each layer relative to the previous one.
    pub scale_decay: f64,

    // Motion
    pub rotation_direction: RotationDirection,
    pub rotation_turns: f64,
    pub zoom_mode: ZoomMode,
    pub zoom_amplitude: f64,
    pub zoom_cycles: i32,
    pub zoom_midpoint: f64,
    pub tunnel_steps: i32,

    // Colour
    pub hue_cycles: i32,
    pub saturation_pulse: f64,
    pub saturation_pulses: i32,
    pub invert_flashes: i32,

    // Effects
    pub kaleidoscope_mode: KaleidoscopeMode,
    pub kaleidoscope_turns: i32,
    pub num_stars: i32,
    pub star_pattern: StarPattern,
    pub star_seed: i32,
    pub wave_direction: WaveDirection,
    /// Pixels at full output size.
    pub wave_amplitude: f64,
    /// Radians per pixel at full output size.
    pub wave_frequency: f64,
    pub wave_cycles: i32,
    /// Block size in pixels (0 or 1 = off).
    pub pixelation_level: i32,
    /// Gaussian sigma in pixels.
    pub blur_radius: f64,
    pub vignette_strength: f64,
}

impl Default for GifSettings {
    fn default() -> Self {
        Self {
            image_path: PathBuf::new(),
            output_width: 600,
            output_height: 600,
            fit_mode: FitMode::Fill,
            num_frames: 60,
            fps: 12,
            dither: false,
            max_layers: 12,
            scale_decay: 0.92,
            rotation_direction: RotationDirection::Clockwise,
            rotation_turns: 2.0,
            zoom_mode: ZoomMode::Oscillating,
            zoom_amplitude: 0.40,
            zoom_cycles: 2,
            zoom_midpoint: 0.82,
            tunnel_steps: 1,
            hue_cycles: 4,
            saturation_pulse: 0.5,
            saturation_pulses: 3,
            invert_flashes: 0,
            kaleidoscope_mode: KaleidoscopeMode::None,
            kaleidoscope_turns: 1,
            num_stars: 300,
            star_pattern: StarPattern::Random,
            star_seed: 1,
            wave_direction: WaveDirection::Horizontal,
            wave_amplitude: 3.6,
            wave_frequency: 0.10,
            wave_cycles: 1,
            pixelation_level: 0,
            blur_radius: 0.0,
            vignette_strength: 1.0,
        }
    }
}

impl GifSettings {
    /// Frame delay in 1/100 s: `clamp(round(100 / fps), 2, 100)` (fps below 1 counts as 1).
    pub fn frame_delay_cs(&self) -> i32 {
        let fps = f64::from(self.fps.max(1));
        // `f64::round` rounds half away from zero, like C++ `std::lround`.
        ((100.0 / fps).round() as i32).clamp(2, 100)
    }
}
