// mainwindow.cpp
#include "mainwindow.h"
#include "advancedsettingsdialog.h"
#include "gif_worker.h"

#include <QFileDialog>
#include <QMessageBox>
#include <QDir>
#include <QStatusBar>
#include <QDebug>
#include <QDesktopServices>
#include <QUrl>
#include <QFileInfo>
#include <QStyle>

#include <opencv2/opencv.hpp>

#ifndef M_PI
#define M_PI 3.14159265358979323846
#endif

MainWindow::MainWindow(QWidget *parent) : QMainWindow(parent),
    worker(nullptr), workerThread(nullptr)
{
    setWindowTitle("Psychedelic GIF Creator");
    resize(800, 800);
    currentSettings = GifSettings::getDefaultSettings();
    
    previewUpdateTimer = new QTimer(this);
    previewUpdateTimer->setSingleShot(true);
    connect(previewUpdateTimer, &QTimer::timeout, this, &MainWindow::generatePreviewFrame);

    setupUi();
    setupConnections();
    refreshCoreSettingsUi();
}

MainWindow::~MainWindow() {
    if (workerThread && workerThread->isRunning()) {
        workerThread->quit();
        workerThread->wait();
    }
}

void MainWindow::setupUi() {
    QWidget *centralWidget = new QWidget(this);
    setCentralWidget(centralWidget);

    auto mainLayout = new QVBoxLayout(centralWidget);
    mainLayout->setContentsMargins(20, 20, 20, 20);
    mainLayout->setSpacing(15);
    
    auto previewControlsLayout = new QHBoxLayout();
    previewCheckBox = new QCheckBox("Enable Real-time Preview");
    previewCheckBox->setChecked(true);
    previewControlsLayout->addStretch();
    previewControlsLayout->addWidget(previewCheckBox);
    mainLayout->addLayout(previewControlsLayout);
    m_controlsToManage.append(previewCheckBox);

    previewRenderLabel = new QLabel("Select an image to begin...");
    previewRenderLabel->setAlignment(Qt::AlignCenter);
    previewRenderLabel->setMinimumSize(400, 250);
    previewRenderLabel->setStyleSheet("background-color: #1A1A2E; color: #888888; border: 2px dashed #4A5763; border-radius: 10px;");
    previewRenderLabel->setWordWrap(true);
    mainLayout->addWidget(previewRenderLabel, 1, Qt::AlignCenter);
    m_controlsToManage.append(previewRenderLabel);


    auto inputGroup = new QGroupBox("Input Artifact");
    auto pathLayout = new QHBoxLayout(inputGroup);
    imagePathEdit = new QLineEdit();
    browseButton = new QPushButton("Browse");
    pathLayout->addWidget(imagePathEdit);
    pathLayout->addWidget(browseButton);
    mainLayout->addWidget(inputGroup);
    m_controlsToManage.append(inputGroup);

    auto coreSettingsGroup = new QGroupBox("Core Settings");
    auto coreLayout = new QGridLayout(coreSettingsGroup);
    coreLayout->setColumnStretch(1, 1);

    int row = 0;
    
    auto createSliderRow = [&](const QString& label, QSlider*& slider, QWidget*& spinBox, const QString& tooltip) {
        auto labelWidget = new QLabel(label);
        labelWidget->setToolTip(tooltip);
        coreLayout->addWidget(labelWidget, row, 0);
        slider = new QSlider(Qt::Horizontal);
        slider->setToolTip(tooltip);
        coreLayout->addWidget(slider, row, 1);
        spinBox->setToolTip(tooltip);
        spinBox->setFixedWidth(80);
        coreLayout->addWidget(spinBox, row, 2);
        
        m_controlsToManage.append(labelWidget);
        m_controlsToManage.append(slider);
        m_controlsToManage.append(spinBox);
        row++;
    };

    numFramesSpinBox = new QSpinBox();
    createSliderRow("Cycles:", numFramesSlider, reinterpret_cast<QWidget*&>(numFramesSpinBox), "Number of frames in the GIF.");
    numFramesSlider->setRange(10, 200); numFramesSpinBox->setRange(10, 200);

    scaleDecaySpinBox = new QDoubleSpinBox();
    createSliderRow("Warp:", scaleDecaySlider, reinterpret_cast<QWidget*&>(scaleDecaySpinBox), "Decay rate of layers (0.50-0.99). Smaller is faster.");
    scaleDecaySlider->setRange(50, 99); scaleDecaySpinBox->setRange(0.50, 0.99);
    scaleDecaySpinBox->setSingleStep(0.01); scaleDecaySpinBox->setDecimals(2);

    hueSpeedSpinBox = new QDoubleSpinBox();
    createSliderRow("Pulse Speed:", hueSpeedSlider, reinterpret_cast<QWidget*&>(hueSpeedSpinBox), "Controls the speed of the color shifting effect.");
    hueSpeedSlider->setRange(0, 200); hueSpeedSpinBox->setRange(0.0, 20.0);
    hueSpeedSpinBox->setSingleStep(0.1); hueSpeedSpinBox->setDecimals(1);
    
    hueIntensitySpinBox = new QDoubleSpinBox();
    createSliderRow("Pulse Intensity:", hueIntensitySlider, reinterpret_cast<QWidget*&>(hueIntensitySpinBox), "Controls the saturation of the color pulse effect. At 0, the effect is disabled.");
    hueIntensitySlider->setRange(0, 200); hueIntensitySpinBox->setRange(0.0, 2.0);
    hueIntensitySpinBox->setSingleStep(0.01); hueIntensitySpinBox->setDecimals(2);

    rotationSpeedSpinBox = new QDoubleSpinBox();
    createSliderRow("Spin Speed:", rotationSpeedSlider, reinterpret_cast<QWidget*&>(rotationSpeedSpinBox), "Determines the total number of full rotations.");
    rotationSpeedSlider->setRange(0, 100); rotationSpeedSpinBox->setRange(0.0, 10.0);
    rotationSpeedSpinBox->setSingleStep(0.1); rotationSpeedSpinBox->setDecimals(1);
    
    auto spinDirLabel = new QLabel("Spin Dir:");
    coreLayout->addWidget(spinDirLabel, row, 0);
    rotationDirectionCombo = new QComboBox();
    rotationDirectionCombo->addItems({"Clockwise", "Counter-Clockwise", "None"});
    coreLayout->addWidget(rotationDirectionCombo, row, 1, 1, 2);
    m_controlsToManage.append(spinDirLabel);
    m_controlsToManage.append(rotationDirectionCombo);
    row++;
    
    auto zoomModeLabel = new QLabel("Zoom Mode:");
    coreLayout->addWidget(zoomModeLabel, row, 0);
    zoomModeComboBox = new QComboBox();
    zoomModeComboBox->addItems({"Oscillating", "Linear", "None"});
    coreLayout->addWidget(zoomModeComboBox, row, 1, 1, 2);
    m_controlsToManage.append(zoomModeLabel);
    m_controlsToManage.append(zoomModeComboBox);
    row++;

    linearZoomStrengthLabel = new QLabel("Linear Strength:");
    coreLayout->addWidget(linearZoomStrengthLabel, row, 0);
    linearZoomStrengthSlider = new QSlider(Qt::Horizontal);
    coreLayout->addWidget(linearZoomStrengthSlider, row, 1);
    linearZoomStrengthSpinBox = new QDoubleSpinBox();
    linearZoomStrengthSpinBox->setFixedWidth(80);
    coreLayout->addWidget(linearZoomStrengthSpinBox, row, 2);
    linearZoomStrengthSlider->setRange(0, 200); linearZoomStrengthSpinBox->setRange(0.0, 2.0);
    linearZoomStrengthSpinBox->setSingleStep(0.01); linearZoomStrengthSpinBox->setDecimals(2);
    m_controlsToManage.append(linearZoomStrengthLabel);
    m_controlsToManage.append(linearZoomStrengthSlider);
    m_controlsToManage.append(linearZoomStrengthSpinBox);
    row++;

    oscillatingZoomStrengthLabel = new QLabel("Oscillating Strength:");
    coreLayout->addWidget(oscillatingZoomStrengthLabel, row, 0);
    oscillatingZoomStrengthSlider = new QSlider(Qt::Horizontal);
    coreLayout->addWidget(oscillatingZoomStrengthSlider, row, 1);
    oscillatingZoomStrengthSpinBox = new QDoubleSpinBox();
    oscillatingZoomStrengthSpinBox->setFixedWidth(80);
    coreLayout->addWidget(oscillatingZoomStrengthSpinBox, row, 2);
    oscillatingZoomStrengthSlider->setRange(0, 50); oscillatingZoomStrengthSpinBox->setRange(0.0, 0.5);
    oscillatingZoomStrengthSpinBox->setSingleStep(0.01); oscillatingZoomStrengthSpinBox->setDecimals(2);
    m_controlsToManage.append(oscillatingZoomStrengthLabel);
    m_controlsToManage.append(oscillatingZoomStrengthSlider);
    m_controlsToManage.append(oscillatingZoomStrengthSpinBox);
    row++;
    
    oscillatingZoomSpeedLabel = new QLabel("Oscillating Speed:");
    coreLayout->addWidget(oscillatingZoomSpeedLabel, row, 0);
    oscillatingZoomSpeedSlider = new QSlider(Qt::Horizontal);
    coreLayout->addWidget(oscillatingZoomSpeedSlider, row, 1);
    oscillatingZoomSpeedSpinBox = new QDoubleSpinBox();
    oscillatingZoomSpeedSpinBox->setFixedWidth(80);
    coreLayout->addWidget(oscillatingZoomSpeedSpinBox, row, 2);
    oscillatingZoomSpeedSlider->setRange(0, 500); oscillatingZoomSpeedSpinBox->setRange(0.0, 5.0);
    oscillatingZoomSpeedSpinBox->setSingleStep(0.01); oscillatingZoomSpeedSpinBox->setDecimals(2);
    m_controlsToManage.append(oscillatingZoomSpeedLabel);
    m_controlsToManage.append(oscillatingZoomSpeedSlider);
    m_controlsToManage.append(oscillatingZoomSpeedSpinBox);
    row++;
    
    oscillatingZoomMidpointLabel = new QLabel("Zoom Midpoint:");
    coreLayout->addWidget(oscillatingZoomMidpointLabel, row, 0);
    oscillatingZoomMidpointSlider = new QSlider(Qt::Horizontal);
    coreLayout->addWidget(oscillatingZoomMidpointSlider, row, 1);
    oscillatingZoomMidpointSpinBox = new QDoubleSpinBox();
    oscillatingZoomMidpointSpinBox->setFixedWidth(80);
    coreLayout->addWidget(oscillatingZoomMidpointSpinBox, row, 2);
    oscillatingZoomMidpointSlider->setRange(50, 150); 
    oscillatingZoomMidpointSpinBox->setRange(0.5, 1.5);
    oscillatingZoomMidpointSpinBox->setSingleStep(0.01); 
    oscillatingZoomMidpointSpinBox->setDecimals(2);
    m_controlsToManage.append(oscillatingZoomMidpointLabel);
    m_controlsToManage.append(oscillatingZoomMidpointSlider);
    m_controlsToManage.append(oscillatingZoomMidpointSpinBox);
    row++;
    
    mainLayout->addWidget(coreSettingsGroup);

    auto actionLayout = new QHBoxLayout();
    advancedButton = new QPushButton("Advanced Settings");
    actionLayout->addWidget(advancedButton);
    actionLayout->addStretch();
    cancelButton = new QPushButton("Cancel");
    cancelButton->setVisible(false);
    actionLayout->addWidget(cancelButton);
    generateButton = new QPushButton("Launch");
    generateButton->setObjectName("generateButton");
    actionLayout->addWidget(generateButton);
    mainLayout->addLayout(actionLayout);
    m_controlsToManage.append(advancedButton);
    m_controlsToManage.append(generateButton);

    progressBar = new QProgressBar();
    mainLayout->addWidget(progressBar);
    setStatusBar(new QStatusBar(this));
}

void MainWindow::setupConnections() {
    connect(browseButton, &QPushButton::clicked, this, &MainWindow::browseImage);
    connect(advancedButton, &QPushButton::clicked, this, &MainWindow::openAdvancedSettings);
    connect(generateButton, &QPushButton::clicked, this, &MainWindow::startGifGeneration);
    
    // Connect controls to the preview update trigger
    connect(previewCheckBox, &QCheckBox::toggled, this, &MainWindow::triggerPreviewUpdate);
    connect(rotationDirectionCombo, &QComboBox::currentTextChanged, this, &MainWindow::triggerPreviewUpdate);
    connect(zoomModeComboBox, &QComboBox::currentTextChanged, this, &MainWindow::triggerPreviewUpdate);

    connect(numFramesSlider, &QSlider::valueChanged, numFramesSpinBox, &QSpinBox::setValue);
    connect(numFramesSpinBox, QOverload<int>::of(&QSpinBox::valueChanged), numFramesSlider, &QSlider::setValue);
    connect(numFramesSpinBox, QOverload<int>::of(&QSpinBox::valueChanged), this, [this](int val){ 
        currentSettings.num_frames = val; 
        triggerPreviewUpdate();
    });
    
    auto connectDoubleSlider = [this](QSlider* slider, QDoubleSpinBox* spinBox, double& setting, double factor = 100.0){
        connect(slider, &QSlider::valueChanged, this, [=, &setting](int val){
            double newVal = val / factor;
            spinBox->blockSignals(true);
            spinBox->setValue(newVal);
            spinBox->blockSignals(false);
            setting = newVal;
            triggerPreviewUpdate();
        });
        connect(spinBox, QOverload<double>::of(&QDoubleSpinBox::valueChanged), this, [=, &setting](double val){
            slider->blockSignals(true);
            slider->setValue(static_cast<int>(val * factor));
            slider->blockSignals(false);
            setting = val;
            triggerPreviewUpdate();
        });
    };
    
    connectDoubleSlider(scaleDecaySlider, scaleDecaySpinBox, currentSettings.scale_decay);
    connectDoubleSlider(rotationSpeedSlider, rotationSpeedSpinBox, currentSettings.rotation_speed, 10.0);
    connectDoubleSlider(hueSpeedSlider, hueSpeedSpinBox, currentSettings.hue_speed, 10.0);
    connectDoubleSlider(hueIntensitySlider, hueIntensitySpinBox, currentSettings.hue_intensity);
    connectDoubleSlider(linearZoomStrengthSlider, linearZoomStrengthSpinBox, currentSettings.linear_zoom_speed);
    connectDoubleSlider(oscillatingZoomStrengthSlider, oscillatingZoomStrengthSpinBox, currentSettings.oscillating_zoom_amplitude);
    connectDoubleSlider(oscillatingZoomSpeedSlider, oscillatingZoomSpeedSpinBox, currentSettings.oscillating_zoom_frequency);
    connectDoubleSlider(oscillatingZoomMidpointSlider, oscillatingZoomMidpointSpinBox, currentSettings.oscillating_zoom_midpoint);
}

void MainWindow::browseImage() {
    QString filePath = QFileDialog::getOpenFileName(this, "Select Image", QDir::homePath(), "Image Files (*.png *.jpg *.jpeg *.bmp)");
    if (filePath.isEmpty()) return;
    
    imagePathEdit->setText(filePath);
    currentSettings.image_path = filePath.toStdString();
    
    showStaticPreview();
    triggerPreviewUpdate();
}

void MainWindow::openAdvancedSettings() {
    AdvancedSettingsDialog advDialog(&currentSettings, this);
    connect(&advDialog, &AdvancedSettingsDialog::settingsChanged, this, &MainWindow::refreshCoreSettingsUi);
    connect(&advDialog, &QDialog::finished, this, &MainWindow::triggerPreviewUpdate);
    advDialog.exec();
}

void MainWindow::startGifGeneration() {
    if (workerThread) { return; }
    if (currentSettings.image_path.empty()) { QMessageBox::warning(this, "Input Required", "Please select an input image first."); return; }
    QString outputFilePath = QFileDialog::getSaveFileName(this, "Save Generated GIF", QDir::homePath() + "/output.gif", "GIF Files (*.gif)");
    if (outputFilePath.isEmpty()) return;

    for (QWidget* w : m_controlsToManage) { w->setEnabled(false); }
    cancelButton->setVisible(true);
    statusBar()->showMessage("Launching Hyperspace...", 0);
    progressBar->setValue(0);
    workerThread = new QThread(this);
    worker = new GifWorker(currentSettings, outputFilePath.toStdString());
    worker->moveToThread(workerThread);
    connect(worker, &GifWorker::finished, workerThread, &QThread::quit);
    connect(workerThread, &QThread::finished, this, [this](){ worker->deleteLater(); workerThread->deleteLater(); worker = nullptr; workerThread = nullptr; });
    connect(workerThread, &QThread::started, worker, &GifWorker::process);
    connect(worker, &GifWorker::progressUpdated, this, &MainWindow::handleGenerationProgress);
    connect(worker, &GifWorker::finished, this, &MainWindow::handleGenerationFinished);
    connect(cancelButton, &QPushButton::clicked, worker, &GifWorker::cancel, Qt::DirectConnection);
    workerThread->start();
}

void MainWindow::handleGenerationProgress(int percentage, const QString& message) {
    progressBar->setValue(percentage);
    progressBar->setFormat(message + " (%p%)");
}

void MainWindow::handleGenerationFinished(bool success, const QString& pathOrMessage) {
    for (QWidget* w : m_controlsToManage) { w->setEnabled(true); }
    cancelButton->setVisible(false);
    if (!success) {
        if (pathOrMessage == "GIF generation cancelled.") {
            statusBar()->showMessage(pathOrMessage, 5000);
        } else {
            QMessageBox::critical(this, "Generation Failed", pathOrMessage);
            statusBar()->showMessage("Generation Failed!");
        }
    } else {
        statusBar()->showMessage("GIF Saved Successfully!", 5000);
        QMessageBox successMsgBox(this);
        successMsgBox.setIcon(QMessageBox::Information);
        successMsgBox.setText("GIF generation complete.");
        successMsgBox.setInformativeText(QString("Saved to:\n%1").arg(pathOrMessage));
        QPushButton *showButton = successMsgBox.addButton("Show in Folder", QMessageBox::ActionRole);
        successMsgBox.addButton(QMessageBox::Ok);
        successMsgBox.exec();
        if (successMsgBox.clickedButton() == showButton) {
            QDesktopServices::openUrl(QUrl::fromLocalFile(QFileInfo(pathOrMessage).absolutePath()));
        }
    }
}

void MainWindow::refreshCoreSettingsUi() {
    for (QWidget* w : m_controlsToManage) { w->blockSignals(true); }

    numFramesSlider->setValue(currentSettings.num_frames);
    numFramesSpinBox->setValue(currentSettings.num_frames);
    scaleDecaySlider->setValue(static_cast<int>(currentSettings.scale_decay * 100));
    scaleDecaySpinBox->setValue(currentSettings.scale_decay);
    rotationSpeedSlider->setValue(static_cast<int>(currentSettings.rotation_speed * 10));
    rotationSpeedSpinBox->setValue(currentSettings.rotation_speed);
    hueSpeedSlider->setValue(static_cast<int>(currentSettings.hue_speed * 10));
    hueSpeedSpinBox->setValue(currentSettings.hue_speed);
    hueIntensitySlider->setValue(static_cast<int>(currentSettings.hue_intensity * 100));
    hueIntensitySpinBox->setValue(currentSettings.hue_intensity);
    rotationDirectionCombo->setCurrentText(QString::fromStdString(currentSettings.rotation_direction));
    zoomModeComboBox->setCurrentText(QString::fromStdString(currentSettings.global_zoom_mode));
    linearZoomStrengthSlider->setValue(static_cast<int>(currentSettings.linear_zoom_speed * 100));
    linearZoomStrengthSpinBox->setValue(currentSettings.linear_zoom_speed);
    oscillatingZoomStrengthSlider->setValue(static_cast<int>(currentSettings.oscillating_zoom_amplitude * 100));
    oscillatingZoomStrengthSpinBox->setValue(currentSettings.oscillating_zoom_amplitude);
    oscillatingZoomSpeedSlider->setValue(static_cast<int>(currentSettings.oscillating_zoom_frequency * 100));
    oscillatingZoomSpeedSpinBox->setValue(currentSettings.oscillating_zoom_frequency);
    oscillatingZoomMidpointSlider->setValue(static_cast<int>(currentSettings.oscillating_zoom_midpoint * 100));
    oscillatingZoomMidpointSpinBox->setValue(currentSettings.oscillating_zoom_midpoint);

    for (QWidget* w : m_controlsToManage) { w->blockSignals(false); }
    updateZoomControlVisibility();
}

void MainWindow::on_zoomModeComboBox_currentIndexChanged(const QString& text) {
    currentSettings.global_zoom_mode = text.toStdString();
    triggerPreviewUpdate();
}

void MainWindow::updateZoomControlVisibility() {
    bool isLinear = (currentSettings.global_zoom_mode == "Linear");
    bool isOscillating = (currentSettings.global_zoom_mode == "Oscillating");
    linearZoomStrengthLabel->setVisible(isLinear);
    linearZoomStrengthSlider->setVisible(isLinear);
    linearZoomStrengthSpinBox->setVisible(isLinear);
    oscillatingZoomStrengthLabel->setVisible(isOscillating);
    oscillatingZoomStrengthSlider->setVisible(isOscillating);
    oscillatingZoomStrengthSpinBox->setVisible(isOscillating);
    oscillatingZoomSpeedLabel->setVisible(isOscillating);
    oscillatingZoomSpeedSlider->setVisible(isOscillating);
    oscillatingZoomSpeedSpinBox->setVisible(isOscillating);
    oscillatingZoomMidpointLabel->setVisible(isOscillating);
    oscillatingZoomMidpointSlider->setVisible(isOscillating);
    oscillatingZoomMidpointSpinBox->setVisible(isOscillating);
}

void MainWindow::showStaticPreview() {
    if (currentSettings.image_path.empty()) {
        previewRenderLabel->setText("Select an image to begin...");
        return;
    }
    QPixmap pixmap(QString::fromStdString(currentSettings.image_path));
    if (pixmap.isNull()) {
        previewRenderLabel->setText("Error: Could not load image file.");
        return;
    }
    previewRenderLabel->setPixmap(pixmap.scaled(previewRenderLabel->size(), Qt::KeepAspectRatio, Qt::SmoothTransformation));
}

void MainWindow::triggerPreviewUpdate() {
    previewUpdateTimer->start(200); 
}

void MainWindow::generatePreviewFrame() {
    if (!previewCheckBox->isChecked()) {
        showStaticPreview();
        return;
    }
    if (currentSettings.image_path.empty()) {
        previewRenderLabel->setText("Select an image to begin...");
        return;
    }

    const int PREVIEW_SIZE = 250;
    
    cv::Mat original_image_bgr = cv::imread(currentSettings.image_path, cv::IMREAD_UNCHANGED);
    if (original_image_bgr.empty()) {
        previewRenderLabel->setText("Error: Could not load image file.");
        return;
    }

    cv::Mat original_image_rgba;
    if (original_image_bgr.channels() < 4) {
         cv::cvtColor(original_image_bgr, original_image_rgba, cv::COLOR_BGR2BGRA);
    } else {
        original_image_rgba = original_image_bgr.clone();
    }
    cv::resize(original_image_rgba, original_image_rgba, cv::Size(PREVIEW_SIZE, PREVIEW_SIZE), 0, 0, cv::INTER_AREA);

    int width = original_image_rgba.cols;
    int height = original_image_rgba.rows;
    
    int i = currentSettings.num_frames / 2;

    cv::Mat frame = cv::Mat::zeros(height, width, CV_8UC4);
    double frame_progress = static_cast<double>(i) / currentSettings.num_frames;
    
    double num_rotations = std::round(currentSettings.rotation_speed / 2.0);
    double total_rotation_degrees = num_rotations * 360.0;
    if (currentSettings.rotation_direction == "Counter-Clockwise") total_rotation_degrees *= -1.0;
    else if (currentSettings.rotation_direction == "None") total_rotation_degrees = 0.0;
    double angle_per_frame = (currentSettings.num_frames > 0) ? (total_rotation_degrees / currentSettings.num_frames) : 0.0;

    double effective_max_scale = 1.0;
    double current_layer_scale = effective_max_scale;
        
    for (int layer = 0; layer < currentSettings.max_layers; ++layer) {
        int scaled_width = static_cast<int>(width * current_layer_scale);
        int scaled_height = static_cast<int>(height * current_layer_scale);
        if (scaled_width < 1 || scaled_height < 1) break;
        cv::Mat resized_image;
        cv::resize(original_image_rgba, resized_image, cv::Size(scaled_width, scaled_height), 0, 0, cv::INTER_AREA);
        double angle_degrees = angle_per_frame * i;
        cv::Point2f center(scaled_width / 2.0F, scaled_height / 2.0F);
        cv::Mat rot_mat = cv::getRotationMatrix2D(center, angle_degrees, 1.0);
        cv::Mat rotated_image;
        cv::warpAffine(resized_image, rotated_image, rot_mat, resized_image.size(), cv::INTER_LINEAR, cv::BORDER_CONSTANT, cv::Scalar(0,0,0,0));
        int paste_x = (width/2) - (scaled_width/2);
        int paste_y = (height/2) - (scaled_height/2);
        cv::Rect roi(paste_x, paste_y, rotated_image.cols, rotated_image.rows);
        cv::Rect frame_roi(0, 0, frame.cols, frame.rows);
        cv::Rect intersection = roi & frame_roi;
        if (intersection.empty()) continue;
        cv::Mat frame_sub_view = frame(intersection);
        cv::Mat rotated_sub_view = rotated_image(cv::Rect(intersection.x-roi.x, intersection.y-roi.y, intersection.width, intersection.height));
        for (int r = 0; r < intersection.height; ++r) {
            for (int c = 0; c < intersection.width; ++c) {
                cv::Vec4b& frame_pixel = frame_sub_view.at<cv::Vec4b>(r, c);
                cv::Vec4b& rotated_pixel = rotated_sub_view.at<cv::Vec4b>(r, c);
                if (rotated_pixel[3] == 0) continue;
                double alpha_rotated = rotated_pixel[3] / 255.0;
                double alpha_frame = frame_pixel[3] / 255.0;
                double new_alpha = alpha_rotated + alpha_frame * (1 - alpha_rotated);
                if (new_alpha > 0) {
                    for (int k = 0; k < 3; ++k) {
                        frame_pixel[k] = static_cast<uchar>((rotated_pixel[k] * alpha_rotated + frame_pixel[k] * alpha_frame * (1 - alpha_rotated)) / new_alpha);
                    }
                    frame_pixel[3] = static_cast<uchar>(new_alpha * 255);
                }
            }
        }
        current_layer_scale *= currentSettings.scale_decay;
    }
    
    double global_scale = 1.0;
    if (currentSettings.global_zoom_mode == "Linear") {
        global_scale = 1.0 + (currentSettings.linear_zoom_speed * frame_progress);
    } else if (currentSettings.global_zoom_mode == "Oscillating") {
        double sine_wave = sin(frame_progress * 2.0 * M_PI * currentSettings.oscillating_zoom_frequency);
        double zoom_center = currentSettings.oscillating_zoom_midpoint;
        global_scale = zoom_center + (currentSettings.oscillating_zoom_amplitude * sine_wave);
    }

    if (global_scale != 1.0) {
        cv::Mat zoom_matrix = cv::getRotationMatrix2D(cv::Point2f(width / 2.0f, height / 2.0f), 0.0, global_scale);
        cv::warpAffine(frame, frame, zoom_matrix, frame.size(), cv::INTER_LINEAR, cv::BORDER_REFLECT_101);
    }

    if (currentSettings.hue_speed > 0 && currentSettings.hue_intensity > 0) {
        cv::Mat hsv_frame, temp_bgr;
        cv::cvtColor(frame, temp_bgr, cv::COLOR_BGRA2BGR);
        cv::cvtColor(temp_bgr, hsv_frame, cv::COLOR_BGR2HSV);
        double saturation_pulse = sin(frame_progress * 2.0 * M_PI * (currentSettings.hue_speed / 4.0));
        double saturation_multiplier = 1.0 + (saturation_pulse * (currentSettings.hue_intensity - 1.0));
        for (int r = 0; r < hsv_frame.rows; ++r) {
            for (int c = 0; c < hsv_frame.cols; ++c) {
                auto& pixel = hsv_frame.at<cv::Vec3b>(r, c);
                pixel[0] = static_cast<uchar>(std::fmod((pixel[0] + (i * currentSettings.hue_speed)), 180.0));
                pixel[1] = cv::saturate_cast<uchar>(pixel[1] * saturation_multiplier);
            }
        }
        cv::cvtColor(hsv_frame, temp_bgr, cv::COLOR_HSV2BGR);
        cv::cvtColor(temp_bgr, frame, cv::COLOR_BGR2BGRA);
    }
    
    cv::cvtColor(frame, frame, cv::COLOR_BGRA2RGBA);
    QImage preview_qimage(frame.data, frame.cols, frame.rows, frame.step, QImage::Format_RGBA8888);
    previewRenderLabel->setPixmap(QPixmap::fromImage(preview_qimage.copy()).scaled(previewRenderLabel->size(), Qt::KeepAspectRatio, Qt::SmoothTransformation));
}