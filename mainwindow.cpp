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

MainWindow::MainWindow(QWidget *parent) : QMainWindow(parent),
    worker(nullptr), workerThread(nullptr)
{
    setWindowTitle("Psychedelic GIF Creator");
    resize(800, 800);
    currentSettings = GifSettings::getDefaultSettings();
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

    imagePreviewLabel = new QLabel("Select an image to begin...");
    imagePreviewLabel->setAlignment(Qt::AlignCenter);
    imagePreviewLabel->setMinimumSize(400, 250);
    imagePreviewLabel->setStyleSheet("background-color: #1A1A2E; color: #888888; border: 2px dashed #4A5763; border-radius: 10px;");
    imagePreviewLabel->setWordWrap(true);
    mainLayout->addWidget(imagePreviewLabel, 1, Qt::AlignCenter);
    m_controlsToManage.append(imagePreviewLabel);


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
    
    connect(rotationDirectionCombo, &QComboBox::currentTextChanged, this, [this](const QString& text){ currentSettings.rotation_direction = text.toStdString(); });
    connect(zoomModeComboBox, &QComboBox::currentTextChanged, this, &MainWindow::on_zoomModeComboBox_currentIndexChanged);

    connect(numFramesSlider, &QSlider::valueChanged, numFramesSpinBox, &QSpinBox::setValue);
    connect(numFramesSpinBox, QOverload<int>::of(&QSpinBox::valueChanged), numFramesSlider, &QSlider::setValue);
    connect(numFramesSpinBox, QOverload<int>::of(&QSpinBox::valueChanged), this, [this](int val){ currentSettings.num_frames = val; });
    
    auto connectDoubleSlider = [this](QSlider* slider, QDoubleSpinBox* spinBox, double& setting, double factor = 100.0){
        connect(slider, &QSlider::valueChanged, this, [=, &setting](int val){
            double newVal = val / factor;
            spinBox->blockSignals(true);
            spinBox->setValue(newVal);
            spinBox->blockSignals(false);
            setting = newVal;
        });
        connect(spinBox, QOverload<double>::of(&QDoubleSpinBox::valueChanged), this, [=, &setting](double val){
            slider->blockSignals(true);
            slider->setValue(static_cast<int>(val * factor));
            slider->blockSignals(false);
            setting = val;
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
    
    QPixmap pixmap(filePath);
    imagePreviewLabel->setPixmap(pixmap.scaled(imagePreviewLabel->size(), Qt::KeepAspectRatio, Qt::SmoothTransformation));
}

// --- THIS FUNCTION WAS MISSING ---
void MainWindow::openAdvancedSettings() {
    AdvancedSettingsDialog advDialog(&currentSettings, this);
    connect(&advDialog, &AdvancedSettingsDialog::settingsChanged, this, &MainWindow::refreshCoreSettingsUi);
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
    updateZoomControlVisibility();
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