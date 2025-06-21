// advancedsettingsdialog.h
#ifndef ADVANCEDSETTINGSDIALOG_H
#define ADVANCEDSETTINGSDIALOG_H

#include <QDialog>
#include <QLabel>
#include <QSlider>
#include <QComboBox>
#include <QPushButton>
#include <QDoubleSpinBox>
#include <QSpinBox>
#include <QVBoxLayout>
#include <QGridLayout>
#include <QGroupBox>
#include "gif_settings.h"

class AdvancedSettingsDialog : public QDialog {
    Q_OBJECT

public:
    explicit AdvancedSettingsDialog(GifSettings* settings, QWidget *parent = nullptr);
    ~AdvancedSettingsDialog() override = default;

signals:
    void settingsChanged();

private slots:
    void randomizeSettingsInDialog();
    void resetToDefaultsInDialog();

private:
    GifSettings* settingsPtr;

    // Widgets
    QSlider* maxLayersSlider;
    QSpinBox* maxLayersSpinBox;
    QSlider* blurRadiusSlider;
    QDoubleSpinBox* blurRadiusSpinBox;
    QSlider* numStarsSlider;
    QSpinBox* numStarsSpinBox;
    QComboBox* starfieldPatternCombo; // Added control for star patterns
    QSlider* pixelationSlider;
    QSpinBox* pixelationSpinBox;
    QSlider* colorInvertSlider;
    QSpinBox* colorInvertSpinBox;
    QSlider* waveAmplitudeSlider;
    QDoubleSpinBox* waveAmplitudeSpinBox;
    QSlider* waveFrequencySlider;
    QDoubleSpinBox* waveFrequencySpinBox;
    QComboBox* waveDirectionCombo;

    QPushButton* randomizeButton;
    QPushButton* defaultButton;

    void setupUi();
    void setupConnections();
    void updateDialogUiFromSettings();
};

#endif // ADVANCEDSETTINGSDIALOG_H