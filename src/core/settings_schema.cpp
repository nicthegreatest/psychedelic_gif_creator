// settings_schema.cpp
#include "settings_schema.h"

#include <QSettings>
#include <cmath>

namespace {
using Kind = FieldInfo::Kind;

FieldInfo intField(const char* key, const char* label, const char* tip, int GifSettings::*m, int lo, int hi) {
    FieldInfo f;
    f.key = key; f.label = label; f.tooltip = tip; f.kind = Kind::Int;
    f.min = lo; f.max = hi; f.step = 1; f.decimals = 0;
    f.get = [m](const GifSettings& s) { return double(s.*m); };
    f.set = [m, lo, hi](GifSettings& s, double v) { s.*m = std::clamp(int(std::lround(v)), lo, hi); };
    return f;
}

FieldInfo dblField(const char* key, const char* label, const char* tip, double GifSettings::*m,
                   double lo, double hi, double step, int decimals) {
    FieldInfo f;
    f.key = key; f.label = label; f.tooltip = tip; f.kind = Kind::Double;
    f.min = lo; f.max = hi; f.step = step; f.decimals = decimals;
    f.get = [m](const GifSettings& s) { return s.*m; };
    f.set = [m, lo, hi](GifSettings& s, double v) { s.*m = std::isfinite(v) ? std::clamp(v, lo, hi) : lo; };
    return f;
}

FieldInfo boolField(const char* key, const char* label, const char* tip, bool GifSettings::*m) {
    FieldInfo f;
    f.key = key; f.label = label; f.tooltip = tip; f.kind = Kind::Bool;
    f.get = [m](const GifSettings& s) { return s.*m ? 1.0 : 0.0; };
    f.set = [m](GifSettings& s, double v) { s.*m = v >= 0.5; };
    return f;
}

template <class E>
FieldInfo choiceField(const char* key, const char* label, const char* tip, E GifSettings::*m) {
    FieldInfo f;
    f.key = key; f.label = label; f.tooltip = tip; f.kind = Kind::Choice;
    for (const char* c : enumLabels<E>()) f.choices << QString::fromUtf8(c);
    const int n = int(f.choices.size());
    f.min = 0; f.max = n - 1;
    f.get = [m](const GifSettings& s) { return double(static_cast<int>(s.*m)); };
    f.set = [m, n](GifSettings& s, double v) { s.*m = static_cast<E>(std::clamp(int(std::lround(v)), 0, n - 1)); };
    return f;
}

std::vector<FieldInfo> buildFields() {
    using S = GifSettings;
    return {
        // Output
        intField("output_width", "Width (px)", "Width of the generated GIF in pixels.", &S::output_width, 64, 1600),
        intField("output_height", "Height (px)", "Height of the generated GIF in pixels.", &S::output_height, 64, 1600),
        choiceField("fit_mode", "Image fit", "How the image is fitted to the output size: Fill crops, Fit letterboxes, Stretch distorts.", &S::fit_mode),
        intField("num_frames", "Frames per loop", "Number of frames in one loop of the GIF.", &S::num_frames, 10, 240),
        intField("fps", "Frame rate (fps)", "Playback speed. GIF timing is stored in 1/100 s, so the real rate is 100 / round(100 / fps).", &S::fps, 2, 50),
        boolField("dither", "Dithering", "Floyd-Steinberg dithering: smoother gradients, slower export, usually a larger file.", &S::dither),

        // Tunnel
        intField("max_layers", "Tunnel layers", "Number of nested copies of the image that form the tunnel.", &S::max_layers, 1, 24),
        dblField("scale_decay", "Layer size ratio", "Size of each tunnel layer relative to the one around it (0.50-0.99). Higher = more, tighter layers.", &S::scale_decay, 0.50, 0.99, 0.01, 2),

        // Motion
        choiceField("rotation_direction", "Spin direction", "Direction the tunnel layers rotate.", &S::rotation_direction),
        dblField("rotation_turns", "Spin (turns per loop)", "Full rotations per loop. Outside Tunnel zoom mode this is rounded to whole turns so the loop is seamless; in Tunnel mode fractional values are exact.", &S::rotation_turns, 0.0, 10.0, 0.1, 1),
        choiceField("zoom_mode", "Zoom mode", "Oscillating: the whole frame breathes in and out. Tunnel: endless flight into the tunnel.", &S::zoom_mode),
        dblField("zoom_amplitude", "Zoom amount", "How far the oscillating zoom swings either side of the zoom centre.", &S::zoom_amplitude, 0.0, 0.6, 0.01, 2),
        intField("zoom_cycles", "Zoom pulses per loop", "Number of in-out zoom pulses per loop.", &S::zoom_cycles, 1, 8),
        dblField("zoom_midpoint", "Zoom centre", "Average zoom level (1 = none). Below 1 zooms out and mirrors the edges.", &S::zoom_midpoint, 0.5, 1.5, 0.01, 2),
        intField("tunnel_steps", "Tunnel speed (layers per loop)", "How many layers the camera flies through per loop.", &S::tunnel_steps, 1, 6),

        // Colour
        intField("hue_cycles", "Colour cycles per loop", "How many times the colours go all the way round the colour wheel per loop (0 = off).", &S::hue_cycles, 0, 12),
        dblField("saturation_pulse", "Saturation pulse", "Strength of the saturation pulsing (0 = off).", &S::saturation_pulse, 0.0, 1.0, 0.01, 2),
        intField("saturation_pulses", "Saturation pulses per loop", "Number of saturation pulses per loop.", &S::saturation_pulses, 1, 12),
        intField("invert_flashes", "Invert flashes per loop", "Number of evenly spaced frames per loop shown with inverted colours (0 = off).", &S::invert_flashes, 0, 30),

        // Effects
        choiceField("kaleidoscope_mode", "Kaleidoscope", "Mirror the image into a kaleidoscope before building the tunnel.", &S::kaleidoscope_mode),
        intField("kaleidoscope_turns", "Kaleidoscope turns per loop", "How many times the image turns under the kaleidoscope mirrors per loop (0 = static).", &S::kaleidoscope_turns, 0, 6),
        choiceField("star_pattern", "Star pattern", "Twinkling stars drawn over the tunnel.", &S::star_pattern),
        intField("num_stars", "Stars", "Number of stars.", &S::num_stars, 0, 1000),
        intField("star_seed", "Star layout seed", "Different numbers give different random star layouts.", &S::star_seed, 0, 99999),
        choiceField("wave_direction", "Wave direction", "Ripple distortion direction.", &S::wave_direction),
        dblField("wave_amplitude", "Wave height (px)", "How far the ripple displaces pixels.", &S::wave_amplitude, 0.0, 50.0, 0.1, 1),
        dblField("wave_frequency", "Wave density", "How tightly packed the ripples are.", &S::wave_frequency, 0.0, 1.0, 0.01, 2),
        intField("wave_cycles", "Wave cycles per loop", "How many times the ripple travels its full length per loop (0 = static).", &S::wave_cycles, 0, 10),
        intField("pixelation_level", "Pixelation (px)", "Pixel block size (0 or 1 = off).", &S::pixelation_level, 0, 20),
        dblField("blur_radius", "Blur (px)", "Soft-focus haze (Gaussian blur radius).", &S::blur_radius, 0.0, 10.0, 0.1, 1),
        dblField("vignette_strength", "Vignette", "Darkens the corners (0 = off).", &S::vignette_strength, 0.0, 1.0, 0.01, 2),
    };
}
} // namespace

const std::vector<FieldInfo>& settingsFields() {
    static const std::vector<FieldInfo> fields = buildFields();
    return fields;
}

const FieldInfo* findField(const QString& key) {
    for (const auto& f : settingsFields())
        if (f.key == key) return &f;
    return nullptr;
}

void clampSettings(GifSettings& s) {
    for (const auto& f : settingsFields()) f.set(s, f.get(s));
}

void saveSettings(QSettings& store, const GifSettings& s) {
    store.beginGroup("settings");
    store.setValue("image_path", QString::fromStdString(s.image_path));
    for (const auto& f : settingsFields()) {
        const double v = f.get(s);
        switch (f.kind) {
        case Kind::Choice: store.setValue(f.key, f.choices.value(int(v))); break;
        case Kind::Bool: store.setValue(f.key, v >= 0.5); break;
        case Kind::Int: store.setValue(f.key, int(std::lround(v))); break;
        case Kind::Double: store.setValue(f.key, v); break;
        }
    }
    store.endGroup();
}

GifSettings loadSettings(QSettings& store) {
    GifSettings s;
    store.beginGroup("settings");
    s.image_path = store.value("image_path").toString().toStdString();
    for (const auto& f : settingsFields()) {
        if (!store.contains(f.key)) continue;
        const QVariant v = store.value(f.key);
        if (f.kind == Kind::Choice) {
            const int idx = int(f.choices.indexOf(v.toString()));
            if (idx >= 0) f.set(s, idx);
        } else if (f.kind == Kind::Bool) {
            f.set(s, v.toBool() ? 1.0 : 0.0);
        } else {
            bool ok = false;
            const double d = v.toDouble(&ok);
            if (ok) f.set(s, d);
        }
    }
    store.endGroup();
    return s;
}
