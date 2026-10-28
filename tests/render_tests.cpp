// render_tests.cpp - headless checks for the shared renderer and exporter.
// Run via `ctest` (or directly). Returns non-zero on failure.
#include "frame_renderer.h"
#include "gif_worker.h"
#include "image_loader.h"
#include "settings_schema.h"

#include <QCoreApplication>
#include <QDir>
#include <QFile>
#include <QSettings>
#include <QTemporaryDir>
#include <opencv2/imgcodecs.hpp>
#include <opencv2/imgproc.hpp>

#include <cstdio>
#include <functional>
#include <string>

static int g_failures = 0;
#define CHECK(cond, ...)                                                    \
    do {                                                                    \
        if (!(cond)) {                                                      \
            ++g_failures;                                                   \
            std::printf("  FAIL %s:%d: %s ", __FILE__, __LINE__, #cond);    \
            std::printf(__VA_ARGS__);                                       \
            std::printf("\n");                                              \
        }                                                                   \
    } while (0)

static cv::Mat testImage(int w, int h, bool withAlpha = false) {
    cv::Mat img(h, w, CV_8UC3);
    for (int r = 0; r < h; ++r)
        for (int c = 0; c < w; ++c)
            img.at<cv::Vec3b>(r, c) = cv::Vec3b(uchar(255 * c / w), uchar(255 * r / h), uchar((c * r) % 256));
    cv::circle(img, {w / 3, h / 3}, h / 5, cv::Scalar(30, 220, 250), cv::FILLED);
    cv::rectangle(img, {w / 2, h / 2}, {w - 10, h - 10}, cv::Scalar(200, 40, 160), cv::FILLED);
    cv::Mat out;
    cv::cvtColor(img, out, cv::COLOR_BGR2BGRA);
    if (withAlpha) cv::circle(out, {w / 2, h / 2}, h / 6, cv::Scalar(0, 0, 0, 0), cv::FILLED);
    return out;
}

static double meanAbsDiff(const cv::Mat& a, const cv::Mat& b) {
    cv::Mat d;
    cv::absdiff(a, b, d);
    const cv::Scalar m = cv::mean(d);
    return (m[0] + m[1] + m[2]) / 3.0;
}

static void checkLoop(const char* name, const GifSettings& s, const cv::Mat& src) {
    std::printf("[loop] %s\n", name);
    try {
        FrameRenderer r(s, src);
        const cv::Size expect = FrameRenderer::outputSize(s);
        cv::Mat f0 = r.renderFrame(0), f1 = r.renderFrame(1), fN = r.renderFrame(r.frameCount());
        cv::Mat f0again = r.renderFrame(0);
        CHECK(f0.size() == expect && f0.type() == CV_8UC3, "size %dx%d", f0.cols, f0.rows);
        const double seam = meanAbsDiff(f0, fN), step = meanAbsDiff(f0, f1);
        std::printf("  seam diff %.4f, frame-to-frame diff %.4f\n", seam, step);
        CHECK(seam < 0.05, "loop seam too visible (%.4f)", seam);
        CHECK(step > 10.0 * seam || step < 1e-9, "seam (%.4f) not clearly below normal motion (%.4f)", seam, step);
        CHECK(meanAbsDiff(f0, f0again) == 0.0, "render is not deterministic");
        for (int i = 2; i < r.frameCount(); i += 7) (void)r.renderFrame(i);
    } catch (const std::exception& e) {
        CHECK(false, "exception: %s", e.what());
    }
}

int main(int argc, char** argv) {
    QCoreApplication app(argc, argv);
    QTemporaryDir tmp;
    const cv::Mat src = testImage(320, 200);

    GifSettings base; // defaults
    base.output_width = 160;
    base.output_height = 120;
    base.num_frames = 24;

    checkLoop("defaults", base, src);
    {
        GifSettings s = base;
        s.kaleidoscope_mode = KaleidoscopeMode::Radial6;
        s.kaleidoscope_turns = 2;
        checkLoop("radial 6-fold kaleidoscope, 2 turns", s, src);
    }
    {
        GifSettings s = base;
        s.zoom_mode = ZoomMode::Tunnel;
        s.tunnel_steps = 2;
        s.rotation_turns = 0.7; // fractional: exact in tunnel mode
        checkLoop("tunnel zoom, fractional spin 0.7", s, src);
    }
    {
        GifSettings s = base;
        s.rotation_turns = 0.3; // fractional outside tunnel => rounded to 1
        s.rotation_direction = RotationDirection::CounterClockwise;
        CHECK(FrameRenderer::effectiveRotationTurns(s) == 1.0, "0.3 turns should round up to 1");
        checkLoop("oscillating zoom, spin 0.3 (rounded)", s, src);
    }
    {
        GifSettings s = base;
        s.kaleidoscope_mode = KaleidoscopeMode::QuadrantMirror;
        s.star_pattern = StarPattern::Spiral;
        s.wave_direction = WaveDirection::Vertical;
        s.wave_cycles = 3;
        s.pixelation_level = 4;
        s.invert_flashes = 3;
        s.blur_radius = 1.0;
        s.fit_mode = FitMode::Fit;
        checkLoop("everything on, letterboxed", s, src);
    }
    {
        GifSettings s = base;
        s.zoom_mode = ZoomMode::Tunnel;
        s.fit_mode = FitMode::Fit;
        s.rotation_turns = 1.5;
        checkLoop("tunnel with transparent source", s, testImage(200, 200, true));
    }

    std::printf("[stars] visible on top of the tunnel\n");
    {
        GifSettings s = base;
        s.vignette_strength = 0;
        s.hue_cycles = 0;
        s.saturation_pulse = 0;
        s.wave_direction = WaveDirection::None;
        s.zoom_mode = ZoomMode::None;
        GifSettings noStars = s;
        noStars.star_pattern = StarPattern::None;
        const cv::Mat a = FrameRenderer(s, src).renderFrame(3), b = FrameRenderer(noStars, src).renderFrame(3);
        CHECK(meanAbsDiff(a, b) > 0.2, "stars not visible (diff %.4f)", meanAbsDiff(a, b));
    }

    std::printf("[aspect] 16:9 output keeps aspect\n");
    {
        GifSettings s = base;
        s.output_width = 320;
        s.output_height = 180;
        FrameRenderer r(s, testImage(1280, 720));
        CHECK(r.frameSize() == cv::Size(320, 180), "got %dx%d", r.frameSize().width, r.frameSize().height);
        CHECK(FrameRenderer(s, src, 0.5).frameSize() == cv::Size(160, 90), "preview scale");
    }

    std::printf("[loader] 16-bit, grey, alpha input\n");
    {
        cv::Mat bgr16, bgr8;
        cv::cvtColor(src, bgr8, cv::COLOR_BGRA2BGR);
        bgr8.convertTo(bgr16, CV_16UC3, 257.0);
        const std::string p16 = QDir(tmp.path()).filePath("rgb16.png").toStdString();
        const std::string pGray = QDir(tmp.path()).filePath("gray.png").toStdString();
        cv::imwrite(p16, bgr16);
        cv::Mat gray;
        cv::cvtColor(bgr8, gray, cv::COLOR_BGR2GRAY);
        cv::imwrite(pGray, gray);
        std::string err;
        cv::Mat a = loadImageBGRA(p16, &err);
        CHECK(a.type() == CV_8UC4, "16-bit load: %s", err.c_str());
        if (a.type() == CV_8UC4) {
            cv::Mat a3;
            cv::cvtColor(a, a3, cv::COLOR_BGRA2BGR);
            CHECK(meanAbsDiff(a3, bgr8) < 1.0, "16-bit content wrong (%.3f)", meanAbsDiff(a3, bgr8));
        }
        CHECK(loadImageBGRA(pGray, &err).type() == CV_8UC4, "grey load: %s", err.c_str());
        cv::Mat f32;
        src.convertTo(f32, CV_32FC4, 1.0 / 255.0);
        CHECK(normaliseToBGRA8(f32).type() == CV_8UC4, "float normalise");
        CHECK(loadImageBGRA(QDir(tmp.path()).filePath("missing.png").toStdString(), &err).empty() && !err.empty(),
              "missing file must fail cleanly");
    }

    std::printf("[settings] QSettings round trip + clamping\n");
    {
        QSettings store(QDir(tmp.path()).filePath("test.ini"), QSettings::IniFormat);
        GifSettings s;
        s.kaleidoscope_mode = KaleidoscopeMode::Radial4;
        s.wave_amplitude = 12.5;
        s.dither = true;
        s.image_path = "/tmp/x.png";
        saveSettings(store, s);
        store.setValue("settings/num_frames", 100000); // out of range
        store.sync();
        GifSettings l = loadSettings(store);
        CHECK(l.kaleidoscope_mode == KaleidoscopeMode::Radial4 && l.wave_amplitude == 12.5 && l.dither &&
                  l.image_path == "/tmp/x.png", "round trip");
        CHECK(l.num_frames == 240, "clamped to %d", l.num_frames);
    }

    std::printf("[export] GIF export, cancel, failure\n");
    {
        const std::string inPath = QDir(tmp.path()).filePath("in.png").toStdString();
        cv::imwrite(inPath, src);
        GifSettings s = base;
        s.image_path = inPath;
        s.num_frames = 12;
        const QString out = QDir(tmp.path()).filePath("out.gif");
        GifWorker w(s, out.toStdString());
        w.setThreadCount(3);
        GifWorker::Result res = GifWorker::Result::Failed;
        int lastProgress = -1;
        QObject::connect(&w, &GifWorker::finished, [&](GifWorker::Result r, const QString&) { res = r; });
        QObject::connect(&w, &GifWorker::progressUpdated, [&](int p, const QString&) { lastProgress = p; });
        w.process();
        QFile f(out);
        CHECK(res == GifWorker::Result::Success && f.open(QIODevice::ReadOnly) && f.read(6) == "GIF89a", "export");
        CHECK(lastProgress == 100, "progress reached %d", lastProgress);

        const QString out2 = QDir(tmp.path()).filePath("cancelled.gif");
        GifWorker c(s, out2.toStdString());
        QObject::connect(&c, &GifWorker::finished, [&](GifWorker::Result r, const QString&) { res = r; });
        c.cancel();
        c.process();
        CHECK(res == GifWorker::Result::Cancelled && !QFile::exists(out2), "cancel removes partial file");

        GifSettings bad = s;
        bad.image_path = "/nonexistent/image.png";
        GifWorker b(bad, QDir(tmp.path()).filePath("bad.gif").toStdString());
        QObject::connect(&b, &GifWorker::finished, [&](GifWorker::Result r, const QString&) { res = r; });
        b.process();
        CHECK(res == GifWorker::Result::Failed, "missing input must fail, not crash");
    }

    std::printf(g_failures ? "\n%d FAILURE(S)\n" : "\nALL TESTS PASSED\n", g_failures);
    return g_failures ? 1 : 0;
}
