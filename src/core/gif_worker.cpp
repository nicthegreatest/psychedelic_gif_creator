// gif_worker.cpp
#include "gif_worker.h"

#include "frame_renderer.h"
#include "image_loader.h"
#include "settings_schema.h"

#include <QFile>
#include <QFuture>
#include <QThread>
#include <QThreadPool>
#include <QtConcurrent/QtConcurrentRun>
#include <cstdio>
#include <deque>
#include <opencv2/imgproc.hpp>

#include "gif.h"

namespace {
struct FrameResult {
    cv::Mat rgba;       // rendered frame, RGBA as gif-h expects
    GifPalette palette; // built from the whole frame on the worker thread
    std::string error;
};

// Closes the GIF on every exit path (including exceptions).
struct GifFile {
    GifWriter writer{};
    bool open = false;
    bool close() {
        if (!open) return true;
        open = false;
        return GifEnd(&writer);
    }
    ~GifFile() { close(); }
};
} // namespace

GifWorker::GifWorker(const GifSettings& settings, const std::string& outputPath, QObject* parent)
    : QObject(parent), m_settings(settings), m_outputPath(outputPath) {
    clampSettings(m_settings);
}

void GifWorker::cancel() { m_cancelled = true; }

void GifWorker::process() {
    QString message;
    Result result = Result::Failed;
    try {
        result = render(message);
    } catch (const cv::Exception& e) {
        message = QString("Image processing error: %1").arg(QString::fromStdString(e.what()));
    } catch (const std::exception& e) {
        message = QString("Unexpected error: %1").arg(QString::fromStdString(e.what()));
    } catch (...) {
        message = "Unknown error during GIF generation.";
    }

    if (result != Result::Success && m_fileCreated) QFile::remove(QString::fromStdString(m_outputPath));
    if (result == Result::Success) message = QString::fromStdString(m_outputPath);
    emit finished(result, message);
}

GifWorker::Result GifWorker::render(QString& message) {
    std::string error;
    const cv::Mat source = loadImageBGRA(m_settings.image_path, &error);
    if (source.empty()) {
        message = QString::fromStdString(error);
        return Result::Failed;
    }

    const FrameRenderer renderer(m_settings, source);
    const cv::Size size = renderer.frameSize();
    const int frames = renderer.frameCount();
    const int delay = m_settings.frameDelayCs();

    GifFile gif;
    if (!GifBegin(&gif.writer, m_outputPath.c_str(), uint32_t(size.width), uint32_t(size.height), uint32_t(delay), 8,
                  m_settings.dither)) {
        message = "Could not open the output file for writing.";
        return Result::Failed;
    }
    gif.open = true;
    m_fileCreated = true;

    // Declared after `renderer` so it is destroyed (and waits for its threads)
    // before the renderer goes away, even if an exception unwinds the stack.
    QThreadPool pool;
    pool.setMaxThreadCount(m_threads > 0 ? m_threads : std::max(1, QThread::idealThreadCount()));
    const size_t window = size_t(pool.maxThreadCount()) * 2;

    std::deque<QFuture<FrameResult>> pending;
    int nextToSubmit = 0;
    auto submit = [&] {
        while (nextToSubmit < frames && pending.size() < window) {
            const int index = nextToSubmit++;
            const bool dither = m_settings.dither;
            pending.push_back(QtConcurrent::run(&pool, [&renderer, index, size, dither] {
                FrameResult r;
                try {
                    cv::cvtColor(renderer.renderFrame(index), r.rgba, cv::COLOR_BGR2RGBA);
                    // The palette is the costly part of encoding and does not
                    // depend on the previous frame, so build it in parallel too.
                    GifMakePalette(nullptr, r.rgba.data, uint32_t(size.width), uint32_t(size.height), 8, dither,
                                   &r.palette);
                } catch (const std::exception& e) {
                    r.error = e.what();
                } catch (...) {
                    r.error = "unknown error";
                }
                return r;
            }));
        }
    };

    Result result = Result::Success;
    for (int i = 0; i < frames; ++i) {
        if (m_cancelled) {
            result = Result::Cancelled;
            message = "GIF generation cancelled.";
            break;
        }
        submit();
        FrameResult fr = pending.front().result(); // frames are encoded strictly in order
        pending.pop_front();
        if (!fr.error.empty()) {
            message = QString("Rendering frame %1 failed: %2").arg(i + 1).arg(QString::fromStdString(fr.error));
            result = Result::Failed;
            break;
        }
        // Same as gif-h's GifWriteFrame(), but with the precomputed palette.
        // Mapping to the palette is sequential because unchanged pixels are
        // encoded as transparent relative to the previous output frame.
        GifWriter& w = gif.writer;
        const uint8_t* previous = w.firstFrame ? nullptr : w.oldImage;
        w.firstFrame = false;
        if (m_settings.dither)
            GifDitherImage(previous, fr.rgba.data, w.oldImage, uint32_t(size.width), uint32_t(size.height), &fr.palette);
        else
            GifThresholdImage(previous, fr.rgba.data, w.oldImage, uint32_t(size.width), uint32_t(size.height), &fr.palette);
        GifWriteLzwImage(w.f, w.oldImage, 0, 0, uint32_t(size.width), uint32_t(size.height), uint32_t(delay), &fr.palette);
        if (std::ferror(w.f)) {
            message = "Failed to write to the GIF file (disk full?).";
            result = Result::Failed;
            break;
        }
        emit progressUpdated((i + 1) * 100 / frames, QString("Frame %1 of %2").arg(i + 1).arg(frames));
    }
    for (auto& f : pending) f.waitForFinished();

    if (!gif.close() && result == Result::Success) {
        message = "Failed to finish writing the GIF file.";
        result = Result::Failed;
    }
    return result;
}
