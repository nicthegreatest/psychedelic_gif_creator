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
#include <QFutureWatcher>// For QFutureWatcher
#include <string>       // For std::string
#include <functional>   // For std::function
#include <vector>       // For std::vector
#include <random>       // For random number generation
#include <QGroupBox>    // For QGroupBox

// Include our GifSettings struct
#include "gif_settings.h"

// Include the new advanced settings dialog header
#include "advancedsettingsdialog.h"

// Forward declare the GIF generation function
extern std::string createPsychedelicGif(const GifSettings& settings, const std::string& output_gif_path,
                                        std::function<void(int, const std::string&)> progress_callback);

class MainWindow : public QMainWindow {
    Q_OBJECT // Required for Qt's meta-object system

public:
    explicit MainWindow(QWidget *parent = nullptr);
    ~MainWindow() override = default;

private slots:
    // Slots for GUI interactions
    void browseImage();
    void onRotationDirectionChanged(int index);

    void openAdvancedSettings();
    void startGifGeneration();
    void gifGenerationFinished();
    void refreshCoreSettingsUi(); // <--- CRITICAL: Slot to refresh main window UI

signals:
    // Signal emitted from the worker thread to update GUI
    void generationProgress(int percentage, const QString& message);

private:
    GifSettings currentSettings; // Instance of our settings struct
    QFutureWatcher<std::string> futureWatcher; // To monitor background task

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

    // Helper functions for UI setup and state management
    void setupUi();
    void setupConnections();
    void setupQtConcurrent();
    void setUiEnabled(bool enabled);
    void updateSliderValueLabel(int value, QLabel* label, double factor = 1.0, int precision = 0);
};

#endif // MAINWINDOW_H
