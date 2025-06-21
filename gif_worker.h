// gif_worker.h
#ifndef GIF_WORKER_H
#define GIF_WORKER_H

#include <QObject>
#include <string>
#include <functional>
#include <atomic> // Required for std::atomic
#include "gif_settings.h"

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
};

#endif // GIF_WORKER_H