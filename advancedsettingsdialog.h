// advancedsettingsdialog.h
#ifndef ADVANCEDSETTINGSDIALOG_H
#define ADVANCEDSETTINGSDIALOG_H

#include <QDialog>
#include <QLabel>
#include <QSlider>
#include <QComboBox>
#include <QPushButton>
#include <QLineEdit>
#include <QVBoxLayout>
#include <QGridLayout>
#include <QGroupBox>

#include "gif_settings.h" // Include our GifSettings struct


class AdvancedSettingsDialog : public QDialog {
    Q_OBJECT

public:
    explicit AdvancedSettingsDialog(GifSettings* settings, QWidget *parent = nullptr);
    ~AdvancedSettingsDialog() override = default;

signals: // <--- ADDED: Signals section for custom signals
    void settingsChanged(); // <--- ADDED: New signal to notify parent of setting changes

private slots:
    // Slots for the advanced settings controls
    void onMaxLayersChanged(int value);
    void onBlurRadiusChanged(int value);
    void onNumStarsChanged(int value);
    void onFractalTypeChanged(int index);
    void onStarfieldPatternChanged(int index);

    // New Effect Control Slots
    void onGlobalZoomSliderChanged(int value);
    void onGlobalZoomEditFinished();

    void onPixelationSliderChanged(int value);
    void onPixelationEditFinished();

    void onColorInvertSliderChanged(int value);
    void onColorInvertEditFinished();

    void onWaveAmplitudeSliderChanged(int value);
    void onWaveAmplitudeEditFinished();

    void onWaveFrequencySliderChanged(int value);
    void onWaveFrequencyEditFinished();

    void onWaveDirectionChanged(int index);

    void randomizeSettingsInDialog();
    void resetToDefaultsInDialog();


private:
    GifSettings* settingsPtr;

    // Widgets for advanced settings
    QSlider* maxLayersSlider;
    QLabel* maxLayersValueLabel;
    QSlider* blurRadiusSlider;
    QLabel* blurRadiusValueLabel;
    QSlider* numStarsSlider;
    QLabel* numStarsValueLabel;
    QComboBox* fractalTypeCombo;
    QComboBox* starfieldPatternCombo;

    // New Effect Control Widgets
    QSlider* globalZoomSlider;
    QLineEdit* globalZoomEdit;
    QLabel* globalZoomValueLabel;

    QSlider* pixelationSlider;
    QLineEdit* pixelationEdit;
    QLabel* pixelationValueLabel;

    QSlider* colorInvertSlider;
    QLineEdit* colorInvertEdit;
    QLabel* colorInvertValueLabel;

    QSlider* waveAmplitudeSlider;
    QLineEdit* waveAmplitudeEdit;
    QLabel* waveAmplitudeValueLabel;

    QSlider* waveFrequencySlider;
    QLineEdit* waveFrequencyEdit;
    QLabel* waveFrequencyValueLabel;

    QComboBox* waveDirectionCombo;

    QPushButton* randomizeButton;
    QPushButton* defaultButton;


    void setupUi();
    void setupConnections();

    // Overload for double settings
    void updateNumericControl(int sliderValue, QSlider* slider, QLineEdit* edit, QLabel* label,
                              double settingsMin, double settingsMax, double guiScaleFactor,
                              double& settingsVar, int precision);

    // NEW Overload for integer settings
    void updateNumericControl(int sliderValue, QSlider* slider, QLineEdit* edit, QLabel* label,
                              int settingsMin, int settingsMax, double guiScaleFactor,
                              int& settingsVar); // No precision for int, int& for settingsVar


    void updateDialogUiFromSettings();
};

#endif // ADVANCEDSETTINGSDIALOG_H