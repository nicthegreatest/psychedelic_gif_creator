// gif_worker.h
#ifndef GIF_WORKER_H
#define GIF_WORKER_H

#include <QObject>
#include <string>
#include <functional>
#include <atomic> // Required for std::atomic
#include "gif_settings.h"
#include <opencv2/opencv.hpp> // Add this line to include OpenCV types

class GifWorker : public QObject
{
    Q_OBJECT

public:
    explicit GifWorker(const GifSettings& settings, const std::string& output_path);
    ~GifWorker() override = default;

signals:
    void progressUpdated(int percentage, const QString& message);
    void finished(bool success, const QString& pathOrMessage);

public slots:
    void process();
    void cancel(); // Slot to trigger cancellation

private:
    GifSettings m_settings;
    std::string m_output_path;
    std::atomic<bool> m_isCancelled{false}; // Thread-safe cancellation flag

    void emitProgress(int percentage, const std::string& message);
    void applyKaleidoscope(cv::Mat& image, int frame_idx); // Changed signature
};

#endif // GIF_WORKER_H