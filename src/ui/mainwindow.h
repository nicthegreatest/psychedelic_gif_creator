// mainwindow.h
#pragma once

#include "gif_settings.h"
#include "gif_worker.h"

#include <QFutureWatcher>
#include <QImage>
#include <QMainWindow>
#include <QPointer>
#include <QThreadPool>
#include <QTimer>
#include <memory>
#include <optional>

class QCheckBox;
class QLabel;
class QLineEdit;
class QProgressBar;
class QPushButton;
class QSlider;
class QThread;
class SettingsForm;
struct SourceCache;

struct PreviewResult {
    QImage image;
    QString error;
};

class MainWindow : public QMainWindow {
    Q_OBJECT
public:
    explicit MainWindow(QWidget* parent = nullptr);
    ~MainWindow() override;

protected:
    void closeEvent(QCloseEvent* event) override;
    void resizeEvent(QResizeEvent* event) override;

private:
    void buildUi();
    void setImagePath(const QString& path);
    void browseForImage();
    void matchImageAspect();
    void openAdvanced();
    void resetAll();
    void onSettingChanged(const QString& key);

    // Preview: debounced, rendered off the UI thread with the shared renderer.
    void schedulePreview(int delayMs = 150);
    void startPreview();
    void previewFinished();
    void showPreviewImage();
    void togglePlay(bool play);

    void startExport();
    void exportProgress(int percentage, const QString& message);
    void exportFinished(GifWorker::Result result, const QString& pathOrMessage);
    void setBusy(bool busy);
    void saveState();

    GifSettings m_settings;
    std::optional<GifSettings> m_previewOverride; // advanced dialog's working copy
    SettingsForm* m_form = nullptr;
    QPointer<class AdvancedSettingsDialog> m_advanced; // non-modal so GNOME lets it move

    QLineEdit* m_pathEdit = nullptr;
    QLabel* m_preview = nullptr;
    QSlider* m_frameSlider = nullptr;
    QPushButton* m_playButton = nullptr;
    QCheckBox* m_previewEnabled = nullptr;
    QPushButton* m_generateButton = nullptr;
    QPushButton* m_cancelButton = nullptr;
    QList<QWidget*> m_lockWhileBusy;
    QProgressBar* m_progress = nullptr;

    QTimer m_previewTimer;
    QTimer m_playTimer;
    QThreadPool m_previewPool;
    QFutureWatcher<PreviewResult> m_previewWatcher;
    bool m_previewQueued = false;
    std::shared_ptr<SourceCache> m_sourceCache;
    QImage m_previewImage;

    QPointer<GifWorker> m_worker;
    QPointer<QThread> m_thread;
};
