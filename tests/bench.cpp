// bench.cpp - headless export benchmark.
// usage: gif_bench <image> <out.gif> [kaleidoscope: none|radial6] [threads]
#include "frame_renderer.h"
#include "gif_worker.h"
#include "image_loader.h"
#include <QCoreApplication>
#include <QElapsedTimer>
#include <QFileInfo>
#include <cstdio>
#include <cstring>

int main(int argc, char** argv) {
    QCoreApplication app(argc, argv);
    if (argc < 3) {
        std::fprintf(stderr, "usage: %s <image> <out.gif> [none|radial6] [threads]\n", argv[0]);
        return 2;
    }
    GifSettings s;
    s.image_path = argv[1];
    if (argc > 3 && std::strcmp(argv[3], "radial6") == 0) s.kaleidoscope_mode = KaleidoscopeMode::Radial6;
    {
        // Render-only timing (single thread), to separate rendering from encoding.
        const cv::Mat src = loadImageBGRA(s.image_path);
        if (!src.empty()) {
            QElapsedTimer rt;
            rt.start();
            FrameRenderer r(s, src);
            for (int i = 0; i < r.frameCount(); ++i) (void)r.renderFrame(i);
            std::printf("render only, 1 thread: %lld ms for %d frames\n", static_cast<long long>(rt.elapsed()), r.frameCount());
        }
    }
    GifWorker w(s, argv[2]);
    if (argc > 4) w.setThreadCount(std::atoi(argv[4]));
    GifWorker::Result result = GifWorker::Result::Failed;
    QString msg;
    QObject::connect(&w, &GifWorker::finished, [&](GifWorker::Result r, const QString& m) { result = r; msg = m; });
    QElapsedTimer t;
    t.start();
    w.process();
    const qint64 ms = t.elapsed();
    std::printf("%s: %lld ms, %lld bytes (%s)\n", result == GifWorker::Result::Success ? "ok" : "FAILED",
                static_cast<long long>(ms), static_cast<long long>(QFileInfo(argv[2]).size()), qPrintable(msg));
    return result == GifWorker::Result::Success ? 0 : 1;
}
