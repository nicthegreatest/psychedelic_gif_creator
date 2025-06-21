// advancedsettingsdialog.cpp
#include "advancedsettingsdialog.h"
#include <random>
#include <QDebug>

AdvancedSettingsDialog::AdvancedSettingsDialog(GifSettings* settings, QWidget *parent)
    : QDialog(parent), settingsPtr(settings) {
    setWindowTitle("Advanced Cosmic Tweaks");
    setMinimumSize(500, 480);
    setModal(true);
    setupUi();
    setupConnections();
    updateDialogUiFromSettings();
}

void AdvancedSettingsDialog::setupUi() {
    auto mainLayout = new QVBoxLayout(this);
    mainLayout->setContentsMargins(20, 20, 20, 20);
    mainLayout->setSpacing(15);

    auto advancedSettingsGroup = new QGroupBox("Advanced Settings");
    auto grid = new QGridLayout(advancedSettingsGroup);
    grid->setColumnStretch(1, 1);

    int row = 0;
    
    grid->addWidget(new QLabel("Layers:"), row, 0);
    maxLayersSlider = new QSlider(Qt::Horizontal);
    maxLayersSlider->setRange(1, 20);
    grid->addWidget(maxLayersSlider, row, 1);
    maxLayersSpinBox = new QSpinBox();
    maxLayersSpinBox->setRange(1, 20);
    maxLayersSpinBox->setFixedWidth(80);
    grid->addWidget(maxLayersSpinBox, row, 2);
    row++;

    grid->addWidget(new QLabel("Haze (Blur):"), row, 0);
    blurRadiusSlider = new QSlider(Qt::Horizontal);
    blurRadiusSlider->setRange(0, 50);
    grid->addWidget(blurRadiusSlider, row, 1);
    blurRadiusSpinBox = new QDoubleSpinBox();
    blurRadiusSpinBox->setRange(0.0, 5.0);
    blurRadiusSpinBox->setSingleStep(0.1);
    blurRadiusSpinBox->setDecimals(1);
    blurRadiusSpinBox->setFixedWidth(80);
    grid->addWidget(blurRadiusSpinBox, row, 2);
    row++;

    grid->addWidget(new QLabel("Stars:"), row, 0);
    numStarsSlider = new QSlider(Qt::Horizontal);
    numStarsSlider->setRange(0, 500);
    grid->addWidget(numStarsSlider, row, 1);
    numStarsSpinBox = new QSpinBox();
    numStarsSpinBox->setRange(0, 500);
    numStarsSpinBox->setFixedWidth(80);
    grid->addWidget(numStarsSpinBox, row, 2);
    row++;
    
    grid->addWidget(new QLabel("Star Pattern:"), row, 0);
    starfieldPatternCombo = new QComboBox();
    starfieldPatternCombo->addItems({"None", "Random", "Spiral"});
    grid->addWidget(starfieldPatternCombo, row, 1, 1, 2);
    row++;

    grid->addWidget(new QLabel("Pixelation:"), row, 0);
    pixelationSlider = new QSlider(Qt::Horizontal);
    pixelationSlider->setRange(0, 10);
    grid->addWidget(pixelationSlider, row, 1);
    pixelationSpinBox = new QSpinBox();
    pixelationSpinBox->setRange(0, 10);
    pixelationSpinBox->setFixedWidth(80);
    grid->addWidget(pixelationSpinBox, row, 2);
    row++;

    grid->addWidget(new QLabel("Invert Freq:"), row, 0);
    colorInvertSlider = new QSlider(Qt::Horizontal);
    colorInvertSlider->setRange(0, 60);
    grid->addWidget(colorInvertSlider, row, 1);
    colorInvertSpinBox = new QSpinBox();
    colorInvertSpinBox->setRange(0, 60);
    colorInvertSpinBox->setFixedWidth(80);
    grid->addWidget(colorInvertSpinBox, row, 2);
    row++;

    grid->addWidget(new QLabel("Wave Amp:"), row, 0);
    waveAmplitudeSlider = new QSlider(Qt::Horizontal);
    waveAmplitudeSlider->setRange(0, 500);
    grid->addWidget(waveAmplitudeSlider, row, 1);
    waveAmplitudeSpinBox = new QDoubleSpinBox();
    waveAmplitudeSpinBox->setRange(0.0, 50.0);
    waveAmplitudeSpinBox->setSingleStep(0.1);
    waveAmplitudeSpinBox->setDecimals(1);
    waveAmplitudeSpinBox->setFixedWidth(80);
    grid->addWidget(waveAmplitudeSpinBox, row, 2);
    row++;
    
    grid->addWidget(new QLabel("Wave Freq:"), row, 0);
    waveFrequencySlider = new QSlider(Qt::Horizontal);
    waveFrequencySlider->setRange(0, 100);
    grid->addWidget(waveFrequencySlider, row, 1);
    waveFrequencySpinBox = new QDoubleSpinBox();
    waveFrequencySpinBox->setRange(0.0, 1.0);
    waveFrequencySpinBox->setSingleStep(0.01);
    waveFrequencySpinBox->setDecimals(2);
    waveFrequencySpinBox->setFixedWidth(80);
    grid->addWidget(waveFrequencySpinBox, row, 2);
    row++;
    
    grid->addWidget(new QLabel("Wave Dir:"), row, 0);
    waveDirectionCombo = new QComboBox();
    waveDirectionCombo->addItems({"None", "Horizontal", "Vertical"});
    grid->addWidget(waveDirectionCombo, row, 1, 1, 2);
    
    mainLayout->addWidget(advancedSettingsGroup);

    auto dialogActionButtonsLayout = new QHBoxLayout();
    randomizeButton = new QPushButton("Cosmic Chaos");
    randomizeButton->setObjectName("dialogButton");
    dialogActionButtonsLayout->addWidget(randomizeButton);
    defaultButton = new QPushButton("Default");
    defaultButton->setObjectName("dialogButton");
    dialogActionButtonsLayout->addWidget(defaultButton);
    dialogActionButtonsLayout->addStretch(1);
    auto closeButton = new QPushButton("Close");
    dialogActionButtonsLayout->addWidget(closeButton);
    mainLayout->addLayout(dialogActionButtonsLayout);
    connect(closeButton, &QPushButton::clicked, this, &AdvancedSettingsDialog::accept);
}

void AdvancedSettingsDialog::setupConnections() {
    auto connectIntSlider = [this](QSlider* slider, QSpinBox* spinBox, int& setting){
        connect(slider, &QSlider::valueChanged, spinBox, &QSpinBox::setValue);
        connect(spinBox, QOverload<int>::of(&QSpinBox::valueChanged), slider, &QSlider::setValue);
        connect(spinBox, QOverload<int>::of(&QSpinBox::valueChanged), this, [&setting](int val){ setting = val; });
    };

    auto connectDoubleSlider = [this](QSlider* slider, QDoubleSpinBox* spinBox, double& setting, double factor = 10.0){
        connect(slider, &QSlider::valueChanged, this, [=](int val){ spinBox->setValue(val / factor); });
        connect(spinBox, QOverload<double>::of(&QDoubleSpinBox::valueChanged), this, [=, &setting](double val){
            slider->blockSignals(true);
            slider->setValue(static_cast<int>(val * factor));
            slider->blockSignals(false);
            setting = val;
        });
    };

    connectIntSlider(maxLayersSlider, maxLayersSpinBox, settingsPtr->max_layers);
    connectIntSlider(numStarsSlider, numStarsSpinBox, settingsPtr->num_stars);
    connectIntSlider(pixelationSlider, pixelationSpinBox, settingsPtr->pixelation_level);
    connectIntSlider(colorInvertSlider, colorInvertSpinBox, settingsPtr->color_invert_frequency);
    
    connectDoubleSlider(blurRadiusSlider, blurRadiusSpinBox, settingsPtr->blur_radius);
    connectDoubleSlider(waveAmplitudeSlider, waveAmplitudeSpinBox, settingsPtr->wave_amplitude);
    connectDoubleSlider(waveFrequencySlider, waveFrequencySpinBox, settingsPtr->wave_frequency, 100.0);

    connect(starfieldPatternCombo, &QComboBox::currentTextChanged, this, [this](const QString& text){ settingsPtr->advanced_starfield_pattern = text.toStdString(); });
    connect(waveDirectionCombo, &QComboBox::currentTextChanged, this, [this](const QString& text){ settingsPtr->wave_direction = text.toStdString(); });
    
    connect(randomizeButton, &QPushButton::clicked, this, &AdvancedSettingsDialog::randomizeSettingsInDialog);
    connect(defaultButton, &QPushButton::clicked, this, &AdvancedSettingsDialog::resetToDefaultsInDialog);
}

void AdvancedSettingsDialog::randomizeSettingsInDialog() {
    std::random_device rd;
    std::mt19937 gen(rd());
    
    settingsPtr->rotation_speed = std::uniform_real_distribution<>(0.0, 10.0)(gen);
    settingsPtr->hue_speed = std::uniform_real_distribution<>(0.0, 15.0)(gen);
    settingsPtr->hue_intensity = std::uniform_real_distribution<>(0.0, 1.5)(gen);
    
    settingsPtr->max_layers = std::uniform_int_distribution<>(5, 20)(gen);
    settingsPtr->blur_radius = std::uniform_real_distribution<>(0.0, 3.0)(gen);
    settingsPtr->num_stars = std::uniform_int_distribution<>(0, 500)(gen);
    settingsPtr->advanced_starfield_pattern = starfieldPatternCombo->itemText(std::uniform_int_distribution<>(0, 2)(gen)).toStdString();
    settingsPtr->pixelation_level = std::uniform_int_distribution<>(0, 10)(gen);
    settingsPtr->color_invert_frequency = std::uniform_int_distribution<>(0, 40)(gen);
    settingsPtr->wave_amplitude = std::uniform_real_distribution<>(0.0, 25.0)(gen);
    settingsPtr->wave_frequency = std::uniform_real_distribution<>(0.0, 0.75)(gen);
    settingsPtr->wave_direction = waveDirectionCombo->itemText(std::uniform_int_distribution<>(0, 2)(gen)).toStdString();
    
    settingsPtr->oscillating_zoom_midpoint = std::uniform_real_distribution<>(0.8, 1.2)(gen);
    
    updateDialogUiFromSettings();
    emit settingsChanged();
}

void AdvancedSettingsDialog::resetToDefaultsInDialog() {
    *settingsPtr = GifSettings::getDefaultSettings();
    updateDialogUiFromSettings();
    emit settingsChanged();
}

void AdvancedSettingsDialog::updateDialogUiFromSettings() {
    // Block all signals to prevent infinite loops during UI refresh
    for(auto widget : this->findChildren<QWidget*>()) {
        widget->blockSignals(true);
    }

    maxLayersSlider->setValue(settingsPtr->max_layers);
    maxLayersSpinBox->setValue(settingsPtr->max_layers);
    blurRadiusSlider->setValue(static_cast<int>(settingsPtr->blur_radius * 10));
    blurRadiusSpinBox->setValue(settingsPtr->blur_radius);
    numStarsSlider->setValue(settingsPtr->num_stars);
    numStarsSpinBox->setValue(settingsPtr->num_stars);
    starfieldPatternCombo->setCurrentText(QString::fromStdString(settingsPtr->advanced_starfield_pattern));
    pixelationSlider->setValue(settingsPtr->pixelation_level);
    pixelationSpinBox->setValue(settingsPtr->pixelation_level);
    colorInvertSlider->setValue(settingsPtr->color_invert_frequency);
    colorInvertSpinBox->setValue(settingsPtr->color_invert_frequency);
    waveAmplitudeSlider->setValue(static_cast<int>(settingsPtr->wave_amplitude * 10));
    waveAmplitudeSpinBox->setValue(settingsPtr->wave_amplitude);
    waveFrequencySlider->setValue(static_cast<int>(settingsPtr->wave_frequency * 100));
    waveFrequencySpinBox->setValue(settingsPtr->wave_frequency);
    waveDirectionCombo->setCurrentText(QString::fromStdString(settingsPtr->wave_direction));

    // Unblock signals
    for(auto widget : this->findChildren<QWidget*>()) {
        widget->blockSignals(false);
    }
}