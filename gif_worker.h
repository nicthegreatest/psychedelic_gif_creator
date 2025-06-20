// gif_worker.h
#ifndef GIF_WORKER_H
#define GIF_WORKER_H

#include <QObject>
#include <string>
#include <functional> // For std::function

// Forward declare GifSettings to avoid circular dependency if gif_settings.h includes QObject
// However, GifSettings does not include QObject, so direct include is fine.
#include "gif_settings.h"

// #include "gif.h" // Include gif.h for GIF encoding functionality

class GifWorker : public QObject
{
    Q_OBJECT // Required for Qt's meta-object system

public:
    explicit GifWorker(const GifSettings& settings, const std::string& output_path);
    ~GifWorker() override = default;

signals:
    void progressUpdated(int percentage, const QString& message);
    void finished(const QString& result_message);

public slots:
    void process(); // This method will contain the main GIF generation logic
    
private:
    GifSettings m_settings;
    std::string m_output_path;

    // Helper to allow createPsychedelicGif's progress callback to emit our signal
    void emitProgress(int percentage, const std::string& message);
};

#endif // GIF_WORKER_H