// Writes v2 reference frames for the M2 renderer.
//
// Kaleidoscope, stars, wave, pixelation, hue, saturation, invert and blur are
// forced off. The Rust renderer still has those as no-op hooks (M3); leaving
// them on would make the golden comparison measure effects that are not ported
// yet. Everything else is v2's own FrameRenderer.
//
// Output layout (under the directory passed on the command line):
//   sources/<name>.bgra          raw 8-bit BGRA, tightly packed
//   <config>/settings.txt        key=value overrides (gifcore schema text)
//   <config>/meta.txt            sizes, source path, frame indices
//   <config>/<index>.rgba        raw 8-bit RGBA of that frame (alpha 255)
//
// PNG previews and v2 GIFs are not written: together with the raw frames they
// were about 3.5 MB, over the 2 MB cap in PLAN.md. `gif_worker.cpp` is still
// compiled into this binary (see CMakeLists.txt).

#include "frame_renderer.h"

#include <opencv2/imgproc.hpp>

#include <cstdlib>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <limits>
#include <sstream>
#include <string>
#include <vector>

namespace fs = std::filesystem;

static void fail(const std::string& message) {
    std::cerr << message << "\n";
    std::exit(1);
}

static cv::Mat testImage(int w, int h, bool withAlpha) {
    cv::Mat img(h, w, CV_8UC3);
    for (int r = 0; r < h; ++r) {
        for (int c = 0; c < w; ++c) {
            img.at<cv::Vec3b>(r, c) =
                cv::Vec3b(uchar(255 * c / w), uchar(255 * r / h), uchar((c * r) % 256));
        }
    }
    cv::circle(img, {w / 3, h / 3}, h / 5, cv::Scalar(30, 220, 250), cv::FILLED);
    cv::rectangle(img, {w / 2, h / 2}, {w - 10, h - 10}, cv::Scalar(200, 40, 160), cv::FILLED);
    cv::Mat out;
    cv::cvtColor(img, out, cv::COLOR_BGR2BGRA);
    if (withAlpha) {
        cv::circle(out, {w / 2, h / 2}, h / 6, cv::Scalar(0, 0, 0, 0), cv::FILLED);
    }
    return out;
}

static void writeRaw(const fs::path& path, const cv::Mat& image) {
    if (!image.isContinuous()) fail("expected a continuous Mat: " + path.string());
    std::ofstream out(path, std::ios::binary);
    if (!out) fail("cannot write " + path.string());
    out.write(reinterpret_cast<const char*>(image.data),
              static_cast<std::streamsize>(image.total() * image.elemSize()));
    if (!out) fail("short write " + path.string());
}

static void writeText(const fs::path& path, const std::string& text) {
    std::ofstream out(path);
    if (!out) fail("cannot write " + path.string());
    out << text;
}

// Only the lines that differ from GifSettings' defaults. Rust `schema::from_text`
// starts from those same defaults, so untouched values are not re-serialised
// (printing a double and parsing it back can change the last bit).
static std::string settingsText(const GifSettings& s) {
    std::string text;
    text += "# M2 golden frames from v2 FrameRenderer.\n";
    text += "# Kaleidoscope, stars, wave, pixelation, hue, saturation, invert and\n";
    text += "# blur are off: the Rust side still has those as M3 no-op hooks.\n";
    auto line = [&](const std::string& key, const std::string& value) {
        text += key + "=" + value + "\n";
    };
    line("output_width", std::to_string(s.output_width));
    line("output_height", std::to_string(s.output_height));
    line("num_frames", std::to_string(s.num_frames));
    line("fit_mode", enumLabels<FitMode>()[int(s.fit_mode)]);
    line("zoom_mode", enumLabels<ZoomMode>()[int(s.zoom_mode)]);
    line("rotation_direction", enumLabels<RotationDirection>()[int(s.rotation_direction)]);
    // max_digits10 so Rust parses the same double the C++ literal produced.
    std::ostringstream turns;
    turns.precision(std::numeric_limits<double>::max_digits10);
    turns << s.rotation_turns;
    line("rotation_turns", turns.str());
    line("tunnel_steps", std::to_string(s.tunnel_steps));
    line("hue_cycles", "0");
    line("saturation_pulse", "0");
    line("star_pattern", "None");
    line("wave_direction", "None");
    line("kaleidoscope_mode", "None");
    line("pixelation_level", "0");
    line("blur_radius", "0");
    line("invert_flashes", "0");
    return text;
}

static GifSettings m2Base() {
    GifSettings s;
    s.output_width = 160;
    s.output_height = 120;
    s.num_frames = 24;
    s.hue_cycles = 0;
    s.saturation_pulse = 0;
    s.star_pattern = StarPattern::None;
    s.wave_direction = WaveDirection::None;
    s.kaleidoscope_mode = KaleidoscopeMode::None;
    s.pixelation_level = 0;
    s.blur_radius = 0;
    s.invert_flashes = 0;
    return s;
}

struct Config {
    const char* name;
    GifSettings settings;
    bool transparent;
};

static void renderConfig(const fs::path& root, const Config& config, const cv::Mat& gradient,
                         const cv::Mat& punched) {
    const cv::Mat& source = config.transparent ? punched : gradient;
    const fs::path dir = root / config.name;
    fs::create_directories(dir);

    const char* sourceName =
        config.transparent ? "transparent_200x200.bgra" : "gradient_320x200.bgra";
    writeText(dir / "settings.txt", settingsText(config.settings));

    FrameRenderer renderer(config.settings, source);
    const int frames = renderer.frameCount();
    const int i0 = 0;
    const int i1 = frames / 4;
    const int i2 = frames / 2;
    const int indices[3] = {i0, i1, i2};

    std::string meta;
    meta += "source: ../sources/" + std::string(sourceName) + "\n";
    meta += "source_width: " + std::to_string(source.cols) + "\n";
    meta += "source_height: " + std::to_string(source.rows) + "\n";
    meta += "width: " + std::to_string(renderer.frameSize().width) + "\n";
    meta += "height: " + std::to_string(renderer.frameSize().height) + "\n";
    meta += "frames: " + std::to_string(frames) + "\n";
    meta += "indices: " + std::to_string(indices[0]) + " " + std::to_string(indices[1]) + " " +
            std::to_string(indices[2]) + "\n";
    writeText(dir / "meta.txt", meta);

    for (int index : indices) {
        cv::Mat bgr = renderer.renderFrame(index);
        if (bgr.type() != CV_8UC3) fail("renderFrame did not return BGR");
        cv::Mat rgba;
        cv::cvtColor(bgr, rgba, cv::COLOR_BGR2RGBA);
        writeRaw(dir / (std::to_string(index) + ".rgba"), rgba);
    }

    std::cout << config.name << " frames " << indices[0] << ", " << indices[1] << ", " << indices[2]
              << "\n";
}

int main(int argc, char** argv) {
    if (argc != 2) fail("usage: v2_golden <output-dir>");
    const fs::path root = argv[1];
    fs::create_directories(root / "sources");

    const cv::Mat gradient = testImage(320, 200, false);
    const cv::Mat punched = testImage(200, 200, true);
    writeRaw(root / "sources" / "gradient_320x200.bgra", gradient);
    writeRaw(root / "sources" / "transparent_200x200.bgra", punched);

    GifSettings defaults = m2Base();
    GifSettings tunnel = m2Base();
    tunnel.zoom_mode = ZoomMode::Tunnel;
    tunnel.tunnel_steps = 2;
    tunnel.rotation_turns = 0.7;
    GifSettings oscillating = m2Base();
    oscillating.rotation_turns = 0.3;
    oscillating.rotation_direction = RotationDirection::CounterClockwise;
    GifSettings letterboxed = m2Base();
    letterboxed.fit_mode = FitMode::Fit;
    GifSettings transparent = m2Base();
    transparent.zoom_mode = ZoomMode::Tunnel;
    transparent.fit_mode = FitMode::Fit;
    transparent.rotation_turns = 1.5;

    const Config configs[] = {
        {"defaults", defaults, false},
        {"tunnel 0.7 spin", tunnel, false},
        {"oscillating 0.3 spin (rounded)", oscillating, false},
        {"letterboxed", letterboxed, false},
        {"transparent source", transparent, true},
    };
    for (const Config& config : configs) renderConfig(root, config, gradient, punched);
    return 0;
}
