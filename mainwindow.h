// mainwindow.h
#ifndef MAINWINDOW_H
#define MAINWINDOW_H

#include <QMainWindow>
#include <QLabel>
#include <QLineEdit>
#include <QPushButton>
#include <QSlider>
#include <QComboBox>
#include <QProgressBar>
#include <QThread>
#include <QGroupBox>
#include <QDoubleSpinBox>
#include <QSpinBox>
#include <QVBoxLayout>
#include <QHBoxLayout>
#include <QGridLayout>
#include <QList>
#include <QCheckBox>
#include <QTimer>

#include "gif_settings.h"

class GifWorker;
class AdvancedSettingsDialog;

class MainWindow : public QMainWindow {
    Q_OBJECT

public:
    explicit MainWindow(QWidget *parent = nullptr);
    ~MainWindow() override;

private slots:
    void browseImage();
    void openAdvancedSettings();
    void startGifGeneration();
    void handleGenerationProgress(int percentage, const QString& message);
    void handleGenerationFinished(bool success, const QString& pathOrMessage);
    void refreshCoreSettingsUi();
    void on_zoomModeComboBox_currentIndexChanged(const QString& text);
    void triggerPreviewUpdate(); // New slot to trigger preview updates
    void generatePreviewFrame(); // New slot to perform the preview render

private:
    GifSettings currentSettings;
    GifWorker* worker = nullptr;
    QThread* workerThread = nullptr;

    // --- GUI Widgets ---
    QCheckBox* previewCheckBox; // New checkbox to enable/disable preview
    QLabel* previewRenderLabel; // Replaces the old static image label
    QLineEdit* imagePathEdit;
    QPushButton* browseButton;
    
    QSlider* numFramesSlider;
    QSpinBox* numFramesSpinBox;
    QSlider* scaleDecaySlider;
    QDoubleSpinBox* scaleDecaySpinBox;
    QSlider* rotationSpeedSlider;
    QDoubleSpinBox* rotationSpeedSpinBox;
    QSlider* hueSpeedSlider;
    QDoubleSpinBox* hueSpeedSpinBox;
    QSlider* hueIntensitySlider;
    QDoubleSpinBox* hueIntensitySpinBox;
    
    QComboBox* rotationDirectionCombo;
    QPushButton* advancedButton;
    QPushButton* generateButton;
    QPushButton* cancelButton;
    QProgressBar* progressBar;

    QComboBox* zoomModeComboBox;
    
    QLabel* linearZoomStrengthLabel;
    QSlider* linearZoomStrengthSlider;
    QDoubleSpinBox* linearZoomStrengthSpinBox;
    
    QLabel* oscillatingZoomStrengthLabel;
    QSlider* oscillatingZoomStrengthSlider;
    QDoubleSpinBox* oscillatingZoomStrengthSpinBox;
    
    QLabel* oscillatingZoomSpeedLabel;
    QSlider* oscillatingZoomSpeedSlider;
    QDoubleSpinBox* oscillatingZoomSpeedSpinBox;
    
    QLabel* oscillatingZoomMidpointLabel;
    QSlider* oscillatingZoomMidpointSlider;
    QDoubleSpinBox* oscillatingZoomMidpointSpinBox;

    QList<QWidget*> m_controlsToManage;
    QTimer* previewUpdateTimer; // New timer for debouncing UI updates

    void setupUi();
    void setupConnections();
    void updateZoomControlVisibility();
    void showStaticPreview(); // New helper to show the original image
};

#endif // MAINWINDOW_H