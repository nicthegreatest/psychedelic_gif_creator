// gif_worker.h
#ifndef GIF_WORKER_H
#define GIF_WORKER_H

#include <QObject>
#include <string>
#include <atomic> // Required for std::atomic
#include "gif_settings.h"
#include <opencv2/core.hpp>

class GifWorker : public QObject
{
    Q_OBJECT

public:
    enum class Result { Success, Cancelled, Failed };
    Q_ENUM(Result)

    explicit GifWorker(const GifSettings& settings, const std::string& output_path);
    ~GifWorker() override = default;

signals:
    void progressUpdated(int percentage, const QString& message);
    void finished(GifWorker::Result result, const QString& pathOrMessage);

public slots:
    void process();
    void cancel(); // Thread-safe: may be called directly from any thread

private:
    GifSettings m_settings;
    std::string m_output_path;
    std::atomic<bool> m_isCancelled{false}; // Thread-safe cancellation flag
    bool m_fileCreated = false;            // true once GifBegin() created the output file

    void emitProgress(int percentage, const std::string& message);
    Result render(QString& message);
    void applyKaleidoscope(cv::Mat& image, int frame_idx); // Changed signature
};

#endif // GIF_WORKER_H