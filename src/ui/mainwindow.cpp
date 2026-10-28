// mainwindow.cpp
#include "mainwindow.h"

#include "advancedsettingsdialog.h"
#include "frame_renderer.h"
#include "image_loader.h"
#include "settings_form.h"
#include "settings_schema.h"

#include <QCheckBox>
#include <QCloseEvent>
#include <QDesktopServices>
#include <QDir>
#include <QFileDialog>
#include <QFileInfo>
#include <QGridLayout>
#include <QGroupBox>
#include <QHBoxLayout>
#include <QLabel>
#include <QLineEdit>
#include <QMessageBox>
#include <QPixmap>
#include <QProgressBar>
#include <QPushButton>
#include <QScrollArea>
#include <QSettings>
#include <QSlider>
#include <QStatusBar>
#include <QThread>
#include <QUrl>
#include <QVBoxLayout>
#include <QtConcurrent/QtConcurrentRun>
#include <mutex>
#include <opencv2/imgproc.hpp>

// Decoded source image shared with preview jobs, so moving a slider does not
// re-read the file from disk.
struct SourceCache {
    std::mutex mutex;
    std::string path;
    cv::Mat image;
    std::string error;

    cv::Mat get(const std::string& p, std::string* err) {
        std::lock_guard<std::mutex> lock(mutex);
        if (p != path) {
            path = p;
            error.clear();
            image = loadImageBGRA(p, &error);
        }
        if (err) *err = error;
        return image;
    }
};

namespace {
const char* kImageFilter = "Images (*.png *.jpg *.jpeg *.bmp *.webp *.tif *.tiff *.gif *.jp2 *.pnm *.ppm *.pgm);;All files (*)";

QGroupBox* makeGroup(const QString& title, SettingsForm* form, const QStringList& keys) {
    auto* box = new QGroupBox(title);
    auto* grid = new QGridLayout(box);
    grid->setColumnStretch(1, 1);
    for (const auto& k : keys) form->addRow(grid, k);
    return box;
}
} // namespace

MainWindow::MainWindow(QWidget* parent) : QMainWindow(parent), m_sourceCache(std::make_shared<SourceCache>()) {
    setWindowTitle("Psychedelic GIF Creator");
    QSettings store;
    m_settings = loadSettings(store);

    m_previewPool.setMaxThreadCount(1);
    m_previewTimer.setSingleShot(true);
    connect(&m_previewTimer, &QTimer::timeout, this, &MainWindow::startPreview);
    connect(&m_previewWatcher, &QFutureWatcher<PreviewResult>::finished, this, &MainWindow::previewFinished);
    connect(&m_playTimer, &QTimer::timeout, this, [this] {
        if (m_previewWatcher.isRunning()) return; // drop frames rather than queue up
        m_frameSlider->setValue((m_frameSlider->value() + 1) % m_settings.num_frames);
    });

    buildUi();
    if (!restoreGeometry(store.value("ui/geometry").toByteArray())) resize(1100, 760);
    m_pathEdit->setText(QString::fromStdString(m_settings.image_path));
    statusBar()->showMessage(m_settings.image_path.empty() ? "Choose an image to begin." : "Ready.");
    schedulePreview(0);
}

MainWindow::~MainWindow() {
    m_playTimer.stop();
    m_previewWatcher.waitForFinished();
    if (m_worker) m_worker->cancel();
    if (m_thread) m_thread->wait();
}

void MainWindow::buildUi() {
    auto* central = new QWidget;
    setCentralWidget(central);
    auto* root = new QHBoxLayout(central);
    root->setContentsMargins(16, 16, 16, 16);
    root->setSpacing(16);

    // --- Left: preview ---------------------------------------------------
    auto* left = new QVBoxLayout;
    m_preview = new QLabel("No preview");
    m_preview->setObjectName("previewLabel");
    m_preview->setAlignment(Qt::AlignCenter);
    m_preview->setMinimumSize(320, 320);
    m_preview->setSizePolicy(QSizePolicy::Ignored, QSizePolicy::Ignored);
    left->addWidget(m_preview, 1);

    auto* scrub = new QHBoxLayout;
    m_playButton = new QPushButton("Play");
    m_playButton->setCheckable(true);
    m_playButton->setToolTip("Play the preview (renders as fast as it can; the exported GIF plays at the chosen fps).");
    m_frameSlider = new QSlider(Qt::Horizontal);
    m_frameSlider->setRange(0, m_settings.num_frames - 1);
    m_frameSlider->setToolTip("Preview frame");
    m_previewEnabled = new QCheckBox("Live preview");
    m_previewEnabled->setChecked(true);
    m_previewEnabled->setToolTip("Re-render the preview whenever a setting changes.");
    scrub->addWidget(m_playButton);
    scrub->addWidget(m_frameSlider, 1);
    scrub->addWidget(m_previewEnabled);
    left->addLayout(scrub);
    root->addLayout(left, 3);

    connect(m_frameSlider, &QSlider::valueChanged, this, [this] { schedulePreview(0); });
    connect(m_playButton, &QPushButton::toggled, this, &MainWindow::togglePlay);
    connect(m_previewEnabled, &QCheckBox::toggled, this, [this](bool on) {
        if (on) schedulePreview(0);
    });

    // --- Right: settings --------------------------------------------------
    m_form = new SettingsForm(&m_settings, this);
    auto* panel = new QWidget;
    auto* panelLayout = new QVBoxLayout(panel);
    panelLayout->setContentsMargins(0, 0, 8, 0);

    auto* input = new QGroupBox("Input image");
    auto* inputLayout = new QHBoxLayout(input);
    m_pathEdit = new QLineEdit;
    m_pathEdit->setPlaceholderText("Path to an image (PNG, JPEG, WebP, TIFF, GIF...)");
    m_pathEdit->setToolTip("Type or paste a path and press Enter, or use Browse.");
    auto* browse = new QPushButton("Browse...");
    inputLayout->addWidget(m_pathEdit, 1);
    inputLayout->addWidget(browse);
    panelLayout->addWidget(input);
    connect(browse, &QPushButton::clicked, this, &MainWindow::browseForImage);
    connect(m_pathEdit, &QLineEdit::editingFinished, this, [this] { setImagePath(m_pathEdit->text().trimmed()); });

    auto* output = makeGroup("Output", m_form, {"output_width", "output_height", "fit_mode", "num_frames", "fps", "dither"});
    auto* aspect = new QPushButton("Match image aspect");
    aspect->setObjectName("dialogButton");
    aspect->setToolTip("Set the height so the GIF has the same shape as the input image.");
    static_cast<QGridLayout*>(output->layout())->addWidget(aspect, output->layout()->count(), 1, Qt::AlignRight);
    connect(aspect, &QPushButton::clicked, this, &MainWindow::matchImageAspect);
    panelLayout->addWidget(output);
    panelLayout->addWidget(makeGroup("Tunnel and motion", m_form,
                                     {"max_layers", "scale_decay", "rotation_direction", "rotation_turns", "zoom_mode"}));
    panelLayout->addWidget(makeGroup("Colour", m_form, {"hue_cycles"}));
    panelLayout->addStretch();

    auto* scroll = new QScrollArea;
    scroll->setWidgetResizable(true);
    scroll->setWidget(panel);
    scroll->setMinimumWidth(430);

    auto* advanced = new QPushButton("Advanced effects...");
    advanced->setToolTip("Zoom details, kaleidoscope, stars, wave, pixelation, blur and vignette.");
    auto* reset = new QPushButton("Reset all");
    reset->setToolTip("Restore every setting to its default (the input image is kept).");
    m_generateButton = new QPushButton("Generate GIF");
    m_generateButton->setObjectName("generateButton");
    m_cancelButton = new QPushButton("Cancel");
    m_cancelButton->setVisible(false);
    m_progress = new QProgressBar;
    m_progress->setRange(0, 100);
    m_progress->setValue(0);
    m_progress->setVisible(false);

    auto* buttons = new QHBoxLayout;
    buttons->addWidget(advanced);
    buttons->addWidget(reset);
    buttons->addStretch();
    buttons->addWidget(m_cancelButton);
    buttons->addWidget(m_generateButton);

    auto* right = new QVBoxLayout;
    right->addWidget(scroll, 1);
    right->addLayout(buttons);
    right->addWidget(m_progress);
    root->addLayout(right, 2);

    m_lockWhileBusy = {input, output, advanced, reset, m_generateButton};
    for (auto* g : panel->findChildren<QGroupBox*>()) m_lockWhileBusy << g;

    connect(m_form, &SettingsForm::changed, this, &MainWindow::onSettingChanged);
    connect(advanced, &QPushButton::clicked, this, &MainWindow::openAdvanced);
    connect(reset, &QPushButton::clicked, this, &MainWindow::resetAll);
    connect(m_generateButton, &QPushButton::clicked, this, &MainWindow::startExport);
}

void MainWindow::onSettingChanged(const QString& key) {
    if (key == "num_frames") m_frameSlider->setRange(0, m_settings.num_frames - 1);
    if ((key == "rotation_turns" || key == "zoom_mode") && m_settings.zoom_mode != ZoomMode::Tunnel &&
        m_settings.rotation_direction != RotationDirection::None) {
        const double used = FrameRenderer::effectiveRotationTurns(m_settings);
        if (used != m_settings.rotation_turns)
            statusBar()->showMessage(QString("Spin %1 is rounded to %2 whole turn(s) so the loop is seamless (exact in Tunnel mode).")
                                         .arg(m_settings.rotation_turns).arg(used), 6000);
    }
    if (key == "fps" && m_playTimer.isActive()) m_playTimer.setInterval(10 * m_settings.frameDelayCs());
    schedulePreview();
}

void MainWindow::setImagePath(const QString& path) {
    if (path.toStdString() == m_settings.image_path) return;
    m_settings.image_path = path.toStdString();
    if (m_pathEdit->text() != path) m_pathEdit->setText(path);
    if (!path.isEmpty()) statusBar()->showMessage("Loading " + QFileInfo(path).fileName() + "...", 3000);
    schedulePreview(0);
}

void MainWindow::browseForImage() {
    QString start = m_settings.image_path.empty() ? QDir::homePath()
                                                 : QFileInfo(QString::fromStdString(m_settings.image_path)).absolutePath();
    const QString file = QFileDialog::getOpenFileName(this, "Choose an image", start, kImageFilter);
    if (!file.isEmpty()) setImagePath(file);
}

void MainWindow::matchImageAspect() {
    std::string err;
    const cv::Mat src = m_sourceCache->get(m_settings.image_path, &err);
    if (src.empty()) {
        statusBar()->showMessage(err.empty() ? "No image loaded." : QString::fromStdString(err), 5000);
        return;
    }
    const FieldInfo* h = findField("output_height");
    h->set(m_settings, m_settings.output_width * double(src.rows) / src.cols);
    m_form->load();
    statusBar()->showMessage(QString("Output set to %1 x %2.").arg(m_settings.output_width).arg(m_settings.output_height), 4000);
    schedulePreview();
}

void MainWindow::openAdvanced() {
    AdvancedSettingsDialog dialog(m_settings, this);
    connect(&dialog, &AdvancedSettingsDialog::settingsChanged, this, [this](const GifSettings& s) {
        m_previewOverride = s;
        schedulePreview();
    });
    const bool accepted = dialog.exec() == QDialog::Accepted;
    if (accepted) {
        m_settings = dialog.settings();
        m_form->load();
    }
    m_previewOverride.reset();
    schedulePreview(0);
}

void MainWindow::resetAll() {
    const std::string path = m_settings.image_path;
    m_settings = GifSettings{};
    m_settings.image_path = path;
    m_form->load();
    m_frameSlider->setRange(0, m_settings.num_frames - 1);
    statusBar()->showMessage("All settings restored to defaults.", 4000);
    schedulePreview(0);
}

// --- Preview ------------------------------------------------------------------

void MainWindow::schedulePreview(int delayMs) {
    if (!m_previewEnabled || !m_previewEnabled->isChecked()) return;
    m_previewTimer.start(delayMs);
}

void MainWindow::startPreview() {
    if (m_previewWatcher.isRunning()) { // coalesce: render the latest state once the current job ends
        m_previewQueued = true;
        return;
    }
    const GifSettings s = m_previewOverride.value_or(m_settings);
    if (s.image_path.empty()) {
        m_previewImage = QImage();
        m_preview->setPixmap(QPixmap());
        m_preview->setText("Choose an image to see a preview");
        return;
    }
    const int frame = std::min(m_frameSlider->value(), s.num_frames - 1);
    const qreal dpr = devicePixelRatioF();
    const QSize box = m_preview->size() * dpr;
    auto cache = m_sourceCache;
    m_previewWatcher.setFuture(QtConcurrent::run(&m_previewPool, [s, frame, box, cache]() {
        PreviewResult result;
        try {
            std::string err;
            const cv::Mat src = cache->get(s.image_path, &err);
            if (src.empty()) {
                result.error = QString::fromStdString(err);
                return result;
            }
            const double scale = std::min({1.0, double(box.width()) / s.output_width, double(box.height()) / s.output_height});
            const FrameRenderer renderer(s, src, std::max(scale, 0.05));
            cv::Mat rgb;
            cv::cvtColor(renderer.renderFrame(frame), rgb, cv::COLOR_BGR2RGB);
            result.image = QImage(rgb.data, rgb.cols, rgb.rows, int(rgb.step), QImage::Format_RGB888).copy();
        } catch (const std::exception& e) {
            result.error = QString("Preview failed: %1").arg(e.what());
        }
        return result;
    }));
}

void MainWindow::previewFinished() {
    const PreviewResult r = m_previewWatcher.result();
    if (!r.error.isEmpty()) {
        m_previewImage = QImage();
        m_preview->setPixmap(QPixmap());
        m_preview->setText(r.error);
        statusBar()->showMessage(r.error, 6000);
    } else {
        m_previewImage = r.image;
        showPreviewImage();
    }
    if (m_previewQueued) {
        m_previewQueued = false;
        startPreview();
    }
}

void MainWindow::showPreviewImage() {
    if (m_previewImage.isNull()) return;
    const qreal dpr = devicePixelRatioF();
    QPixmap pm = QPixmap::fromImage(m_previewImage.scaled(m_preview->size() * dpr, Qt::KeepAspectRatio, Qt::SmoothTransformation));
    pm.setDevicePixelRatio(dpr);
    m_preview->setPixmap(pm);
}

void MainWindow::togglePlay(bool play) {
    m_playButton->setText(play ? "Pause" : "Play");
    if (play) m_playTimer.start(10 * m_settings.frameDelayCs());
    else m_playTimer.stop();
}

void MainWindow::resizeEvent(QResizeEvent* event) {
    QMainWindow::resizeEvent(event);
    showPreviewImage();        // instant rescale of the last frame
    schedulePreview(250);      // then re-render at the new size
}

// --- Export -------------------------------------------------------------------

void MainWindow::startExport() {
    if (m_thread) return;
    if (m_settings.image_path.empty()) {
        QMessageBox::warning(this, "Input required", "Please choose an input image first.");
        return;
    }
    std::string err;
    if (m_sourceCache->get(m_settings.image_path, &err).empty()) {
        QMessageBox::warning(this, "Cannot read image", QString::fromStdString(err));
        return;
    }
    QSettings store;
    const QFileInfo in(QString::fromStdString(m_settings.image_path));
    const QString dir = store.value("ui/lastOutputDir", in.absolutePath()).toString();
    const QString suggested = QDir(dir).filePath(in.completeBaseName() + "_psychedelic.gif");
    QString out = QFileDialog::getSaveFileName(this, "Save GIF", suggested, "GIF images (*.gif)");
    if (out.isEmpty()) return;
    if (!out.endsWith(".gif", Qt::CaseInsensitive)) out += ".gif";
    store.setValue("ui/lastOutputDir", QFileInfo(out).absolutePath());
    saveState();

    m_playButton->setChecked(false);
    setBusy(true);
    statusBar()->showMessage("Launching hyperspace...");

    m_thread = new QThread(this);
    m_worker = new GifWorker(m_settings, out.toStdString());
    m_worker->moveToThread(m_thread);
    // Delete the worker in its own thread, then stop and delete the thread.
    connect(m_worker, &GifWorker::finished, m_worker, &QObject::deleteLater);
    connect(m_worker, &QObject::destroyed, m_thread, &QThread::quit);
    connect(m_thread, &QThread::finished, m_thread, &QObject::deleteLater);
    connect(m_thread, &QThread::started, m_worker, &GifWorker::process);
    connect(m_worker, &GifWorker::progressUpdated, this, &MainWindow::exportProgress);
    connect(m_worker, &GifWorker::finished, this, &MainWindow::exportFinished);
    m_thread->start();
}

void MainWindow::exportProgress(int percentage, const QString& message) {
    m_progress->setValue(percentage);
    m_progress->setFormat(message + "  (%p%)");
}

void MainWindow::exportFinished(GifWorker::Result result, const QString& pathOrMessage) {
    setBusy(false);
    switch (result) {
    case GifWorker::Result::Cancelled:
        statusBar()->showMessage(pathOrMessage, 5000);
        break;
    case GifWorker::Result::Failed:
        statusBar()->showMessage("GIF generation failed.", 5000);
        QMessageBox::critical(this, "Generation failed", pathOrMessage);
        break;
    case GifWorker::Result::Success: {
        statusBar()->showMessage("Saved " + pathOrMessage, 8000);
        QMessageBox box(this);
        box.setIcon(QMessageBox::Information);
        box.setText("GIF generation complete.");
        box.setInformativeText("Saved to:\n" + pathOrMessage);
        QPushButton* show = box.addButton("Show in folder", QMessageBox::ActionRole);
        box.addButton(QMessageBox::Ok);
        box.exec();
        if (box.clickedButton() == show) QDesktopServices::openUrl(QUrl::fromLocalFile(QFileInfo(pathOrMessage).absolutePath()));
        break;
    }
    }
}

void MainWindow::setBusy(bool busy) {
    for (QWidget* w : std::as_const(m_lockWhileBusy)) w->setEnabled(!busy);
    m_cancelButton->setVisible(busy);
    m_cancelButton->setEnabled(busy);
    m_progress->setVisible(busy);
    m_progress->setValue(0);
    if (busy) {
        // Direct call: cancel() only sets an atomic flag, and the worker's own
        // thread is busy rendering so a queued call would never run.
        connect(m_cancelButton, &QPushButton::clicked, this, [this] {
            if (m_worker) m_worker->cancel();
            m_cancelButton->setEnabled(false);
            statusBar()->showMessage("Cancelling...");
        }, Qt::UniqueConnection);
    }
}

void MainWindow::saveState() {
    QSettings store;
    saveSettings(store, m_settings);
    store.setValue("ui/geometry", saveGeometry());
}

void MainWindow::closeEvent(QCloseEvent* event) {
    if (m_thread) {
        const auto answer = QMessageBox::question(this, "Export in progress", "A GIF is still being generated. Stop it and quit?");
        if (answer != QMessageBox::Yes) {
            event->ignore();
            return;
        }
        if (m_worker) m_worker->cancel();
        m_thread->wait(); // the worker checks the flag every frame
    }
    saveState();
    event->accept();
}
