// gif_settings.h - every user-tunable parameter, with the ONE set of defaults
// (the member initialisers below). Plain C++ so the renderer has no Qt dependency.
#pragma once

#include <algorithm>
#include <cmath>
#include <string>
#include <vector>

enum class FitMode { Fill, Fit, Stretch };
enum class RotationDirection { Clockwise, CounterClockwise, None };
enum class ZoomMode { None, Oscillating, Tunnel };
enum class StarPattern { None, Random, Spiral };
enum class WaveDirection { None, Horizontal, Vertical };
enum class KaleidoscopeMode { None, QuadrantMirror, Radial3, Radial4, Radial6 };

// Human-readable labels, indexed by the enum's underlying value. Used for the
// combo boxes and for the (readable) QSettings file.
template <class E> const std::vector<const char*>& enumLabels();
template <> inline const std::vector<const char*>& enumLabels<FitMode>() {
    static const std::vector<const char*> v{"Fill (crop)", "Fit (letterbox)", "Stretch"};
    return v;
}
template <> inline const std::vector<const char*>& enumLabels<RotationDirection>() {
    static const std::vector<const char*> v{"Clockwise", "Counter-clockwise", "None"};
    return v;
}
template <> inline const std::vector<const char*>& enumLabels<ZoomMode>() {
    static const std::vector<const char*> v{"None", "Oscillating", "Tunnel (infinite zoom)"};
    return v;
}
template <> inline const std::vector<const char*>& enumLabels<StarPattern>() {
    static const std::vector<const char*> v{"None", "Random", "Spiral"};
    return v;
}
template <> inline const std::vector<const char*>& enumLabels<WaveDirection>() {
    static const std::vector<const char*> v{"None", "Horizontal", "Vertical"};
    return v;
}
template <> inline const std::vector<const char*>& enumLabels<KaleidoscopeMode>() {
    static const std::vector<const char*> v{"None", "Quadrant mirror", "Radial 3-fold", "Radial 4-fold", "Radial 6-fold"};
    return v;
}

struct GifSettings {
    std::string image_path;

    // Output
    int output_width = 600;
    int output_height = 600;
    FitMode fit_mode = FitMode::Fill;
    int num_frames = 60;
    int fps = 12;               // stored in the GIF as a 1/100 s delay
    bool dither = false;

    // Tunnel layers
    int max_layers = 12;
    double scale_decay = 0.92;  // size of each layer relative to the previous one

    // Motion (all "per loop" values are whole cycles so the GIF loops seamlessly)
    RotationDirection rotation_direction = RotationDirection::Clockwise;
    double rotation_turns = 2.0;
    ZoomMode zoom_mode = ZoomMode::Oscillating;
    double zoom_amplitude = 0.40;
    int zoom_cycles = 2;
    double zoom_midpoint = 0.82;
    int tunnel_steps = 1;

    // Colour
    int hue_cycles = 4;
    double saturation_pulse = 0.5;
    int saturation_pulses = 3;
    int invert_flashes = 0;

    // Effects
    KaleidoscopeMode kaleidoscope_mode = KaleidoscopeMode::None;
    int kaleidoscope_turns = 1;
    int num_stars = 300;
    StarPattern star_pattern = StarPattern::Random;
    int star_seed = 1;
    WaveDirection wave_direction = WaveDirection::Horizontal;
    double wave_amplitude = 3.6;   // pixels at full output size
    double wave_frequency = 0.10;  // radians per pixel at full output size
    int wave_cycles = 1;
    int pixelation_level = 0;      // block size in pixels (0/1 = off)
    double blur_radius = 0.0;      // Gaussian sigma in pixels
    double vignette_strength = 1.0;

    int frameDelayCs() const {
        const int f = std::max(1, fps);
        return std::clamp(static_cast<int>(std::lround(100.0 / f)), 2, 100);
    }
};
