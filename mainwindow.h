// mainwindow.h
#ifndef MAINWINDOW_H
#define MAINWINDOW_H

#include <QMainWindow>
#include <QLabel>       // For QLabel
#include <QLineEdit>    // For QLineEdit
#include <QPushButton>  // For QPushButton
#include <QSlider>      // For QSlider
#include <QComboBox>    // For QComboBox
#include <QProgressBar> // For QProgressBar
#include <QThread>      // NEW: For QThread
#include <string>       // For std::string
#include <functional>   // For std::function
#include <vector>       // For std::vector
#include <random>       // For random number generation
#include <QGroupBox>    // For QGroupBox
#include <QDoubleSpinBox> // For oscillating zoom amplitude/frequency
#include <QVBoxLayout> // For layouts
#include <QHBoxLayout> // For layouts
#include <QGridLayout> // For layouts


// Include our GifSettings struct
#include "gif_settings.h"

// Include the new advanced settings dialog header
#include "advancedsettingsdialog.h"

// Include the new worker class
#include "gif_worker.h"

// Forward declare the GIF generation function (if still needed, though its logic moved)
// extern std::string createPsychedelicGif(const GifSettings& settings, const std::string& output_gif_path,
//                                         std::function<void(int, const std::string&)> progress_callback);


class MainWindow : public QMainWindow {
    Q_OBJECT // Required for Qt's meta-object system

public:
    explicit MainWindow(QWidget *parent = nullptr);
    ~MainWindow() override; // Non-default destructor for QThread cleanup

private slots:
    // Slots for GUI interactions
    void browseImage();
    void onRotationDirectionChanged(int index);
    void openAdvancedSettings();
    void startGifGeneration();

    // NEW Slots for worker thread communication
    void handleGenerationProgress(int percentage, const QString& message);
    void handleGenerationFinished(const QString& result_message);

    void refreshCoreSettingsUi();

    // NEW: Slot for handling zoom mode changes
    void on_zoomModeComboBox_currentIndexChanged(int index);

// No `signals:` block here if no signals are emitted *from MainWindow*
// (We removed the `generationProgress` signal from MainWindow in the last step,
// as the worker now emits it directly.)

private:
    GifSettings currentSettings; // Instance of our settings struct

    GifWorker* worker;     // Pointer to our worker object
    QThread* workerThread; // Pointer to the thread

    // GUI Widgets (member pointers)
    QLabel* imagePreviewLabel;
    QLineEdit* imagePathEdit;
    QPushButton* browseButton;
    QSlider* numFramesSlider;
    QLabel* numFramesValueLabel;
    QSlider* scaleDecaySlider;
    QLabel* scaleDecayValueLabel;
    QSlider* rotationSpeedSlider;
    QLabel* rotationSpeedValueLabel;
    QSlider* hueSpeedSlider;
    QLabel* hueSpeedValueLabel;
    QComboBox* rotationDirectionCombo;
    QPushButton* advancedButton;
    QPushButton* generateButton;
    QProgressBar* progressBar;

    // --- NEW: Zoom Type Controls ---
    QLabel* zoomModeLabel;
    QComboBox* zoomModeComboBox;

    // --- NEW: Oscillating Zoom Specific Controls ---
    QLabel* oscillatingZoomAmplitudeLabel;
    QDoubleSpinBox* oscillatingZoomAmplitudeSpinBox;

    QLabel* oscillatingZoomFrequencyLabel;
    QDoubleSpinBox* oscillatingZoomFrequencySpinBox;

    // Helper functions for UI setup and state management
    void setupUi();
    void setupConnections();
    // void setupQtConcurrent(); // This function declaration is no longer needed
    void setUiEnabled(bool enabled);
    void updateSliderValueLabel(int value, QLabel* label, double factor = 1.0, int precision = 0);

    // NEW: Helper to manage visibility of zoom-specific controls
    void updateZoomControlVisibility();
};

#endif // MAINWINDOW_H