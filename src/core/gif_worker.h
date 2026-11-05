// gif_worker.h - exports a GIF on a background thread (frames rendered in
// parallel, encoded in order).
#pragma once

#include "gif_settings.h"
#include <QObject>
#include <QString>
#include <atomic>
#include <string>

class GifWorker : public QObject {
    Q_OBJECT
public:
    enum class Result { Success, Cancelled, Failed };
    Q_ENUM(Result)

    GifWorker(const GifSettings& settings, const std::string& outputPath, QObject* parent = nullptr);

    // Number of render threads (0 = one per CPU core).
    void setThreadCount(int threads) { m_threads = threads; }

signals:
    void progressUpdated(int percentage, const QString& message);
    void finished(GifWorker::Result result, const QString& pathOrMessage);

public slots:
    void process();
    void cancel(); // thread-safe; may be called directly from any thread

private:
    Result render(QString& message);

    GifSettings m_settings;
    std::string m_outputPath;
    std::atomic<bool> m_cancelled{false};
    bool m_fileCreated = false;
    int m_threads = 0;
};
