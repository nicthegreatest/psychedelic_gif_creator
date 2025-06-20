// advancedsettingsdialog.cpp
#include "advancedsettingsdialog.h"

#include <QApplication>
#include <QMessageBox>
#include <QDoubleValidator>
#include <QIntValidator> // For integer inputs
#include <random> // For std::random_device etc.
#include <algorithm> // For std::min, std::max
#include <QDebug> // For debugging output

// --- Constructor ---
AdvancedSettingsDialog::AdvancedSettingsDialog(GifSettings* settings, QWidget *parent)
    : QDialog(parent), settingsPtr(settings) {
    setWindowTitle("Advanced Cosmic Tweaks");
    setMinimumSize(500, 600);
    setModal(true);

    // --- Apply Dialog-specific Qt Style Sheet (QSS) ---
    // Inherits most styles from mainwindow.cpp's global stylesheet
    // Add specific overrides if needed.
    setStyleSheet(R"(
        QDialog {
            background-color: #1A1A2E; /* Same as main window background */
            color: #E0E0E0;
            font-family: "Segoe UI", "Roboto", "Open Sans", sans-serif;
            font-size: 10pt;
        }
        QGroupBox {
            background-color: #24293D;
            border: 1px solid #3A4750;
            border-radius: 8px;
            margin-top: 1ex;
            padding: 10px;
        }
        QGroupBox::title {
            subcontrol-origin: margin;
            subcontrol-position: top left;
            padding: 0 5px;
            color: #90CAF9;
            font-size: 10pt;
            font-weight: bold;
        }
        QLabel {
            color: #E0E0E0;
            padding: 2px;
        }
        QSlider::groove:horizontal {
            border: 1px solid #3A4750;
            height: 8px;
            background: #2A2A3A;
            margin: 2px 0;
            border-radius: 4px;
        }
        QSlider::handle:horizontal {
            background: #64FFDA;
            border: 1px solid #56B6C2;
            width: 18px;
            margin: -5px 0;
            border-radius: 9px;
        }
        QSlider::sub-page:horizontal {
            background: #4A5763;
            border-radius: 4px;
        }
        QSlider::add-page:horizontal {
            background: #2A2A3A;
            border-radius: 4px;
        }
        QLineEdit {
            background-color: #1A1A2E;
            border: 1px solid #3A4750;
            border-radius: 5px;
            padding: 5px;
            color: #64FFDA;
            selection-background-color: #64FFDA;
            selection-color: #1A1A2E;
        }
        QLineEdit:focus {
            border: 1px solid #64FFDA;
        }
        QComboBox {
            background-color: #1A1A2E;
            border: 1px solid #3A4750;
            border-radius: 5px;
            padding: 5px;
            color: #E0E0E0;
            selection-background-color: #64FFDA;
            selection-color: #1A1A2E;
        }
        QComboBox::drop-down {
            border: none;
            subcontrol-origin: padding;
            subcontrol-position: top right;
            width: 20px;
        }
        QComboBox QAbstractItemView {
            background-color: #24293D;
            border: 1px solid #3A4750;
            border-radius: 5px;
            selection-background-color: #64FFDA;
            selection-color: #1A1A2E;
        }
        QPushButton {
            background-color: #3A4750;
            color: #E0E0E0;
            border: none;
            border-radius: 5px;
            padding: 8px 15px;
            min-height: 28px;
            font-weight: bold;
        }
        QPushButton:hover {
            background-color: #4A5763;
            color: #FFFFFF;
        }
        QPushButton:pressed {
            background-color: #303A45;
            border-style: inset;
        }
        /* Style for buttons that are inside the dialog (Randomize, Default) */
        QPushButton#dialogButton { /* Unique objectName for these buttons */
            background-color: #555555;
            color: #BBBBBB;
            font-size: 8pt;
            padding: 4px 8px;
            min-height: 20px;
            border-radius: 3px;
        }
        QPushButton#dialogButton:hover {
            background-color: #666666;
        }
        QPushButton#dialogButton:pressed {
            background-color: #444444;
        }
        QToolTip {
            background-color: #3A4750;
            color: #E0E0E0;
            border: 1px solid #64FFDA;
            border-radius: 5px;
            padding: 5px;
            opacity: 200;
            font-size: 9pt;
        }
    )");

    setupUi();
    setupConnections();

    // Initialize UI elements with current settings passed from MainWindow
    updateDialogUiFromSettings();
}

// --- setupUi Method ---
void AdvancedSettingsDialog::setupUi() {
    QVBoxLayout *mainLayout = new QVBoxLayout(this);
    mainLayout->setContentsMargins(20, 20, 20, 20);
    mainLayout->setSpacing(15);

    QGroupBox *advancedSettingsGroup = new QGroupBox("Advanced Settings");
    QVBoxLayout *advSettingsMainLayout = new QVBoxLayout(advancedSettingsGroup);
    advSettingsMainLayout->setContentsMargins(10, 20, 10, 10);
    advSettingsMainLayout->setSpacing(10);

    QGridLayout *grid = new QGridLayout();
    grid->setColumnStretch(1, 1); // Slider column
    grid->setColumnStretch(2, 0); // Value label column (fixed width)
    grid->setColumnStretch(3, 0); // Line edit column (fixed width)
    grid->setHorizontalSpacing(10);
    grid->setVerticalSpacing(5);

    // Helper to add a row for a numeric control (Label, Slider, Value Label, Line Edit)
    // This lambda is for DOUBLE values (explicitly handles precision)
    auto addNumericControlRowDouble = [&](const QString& labelText, QSlider*& slider, QLineEdit** edit_ptr, QLabel*& valueLabel, int row, int minSlider, int maxSlider, double initialVal, double guiScaleFactor, int precision, const QString& tooltipText) {
        grid->addWidget(new QLabel(labelText), row, 0); // Label
        slider = new QSlider(Qt::Horizontal);
        slider->setRange(minSlider, maxSlider);
        slider->setTickPosition(QSlider::TicksBelow);
        slider->setTickInterval((maxSlider - minSlider) / 10);
        slider->setToolTip(tooltipText);
        grid->addWidget(slider, row, 1); // Slider

        valueLabel = new QLabel(QString("(%1)").arg(initialVal, 0, 'f', precision));
        valueLabel->setMinimumWidth(50); // Ensure space
        valueLabel->setToolTip(tooltipText);
        grid->addWidget(valueLabel, row, 2); // Value Label

        if (edit_ptr) { // Only create and add QLineEdit if a pointer to it is provided
            *edit_ptr = new QLineEdit();
            QDoubleValidator* validator = new QDoubleValidator(*edit_ptr);
            validator->setNotation(QDoubleValidator::StandardNotation); // Allow scientific notation
            validator->setRange(static_cast<double>(minSlider) / guiScaleFactor, static_cast<double>(maxSlider) / guiScaleFactor, precision);
            (*edit_ptr)->setValidator(validator);

            (*edit_ptr)->setMinimumWidth(60);
            (*edit_ptr)->setMaximumWidth(80);
            (*edit_ptr)->setToolTip(tooltipText);
            grid->addWidget(*edit_ptr, row, 3); // Line Edit
        } else {
            // Add an empty widget to fill the column for alignment if no edit is needed
            QWidget* emptyWidget = new QWidget();
            emptyWidget->setFixedWidth(80); // Match width of QLineEdit
            grid->addWidget(emptyWidget, row, 3);
        }
    };

    // Helper to add a row for an INTEGER numeric control (Label, Slider, Value Label, Line Edit)
    auto addNumericControlRowInt = [&](const QString& labelText, QSlider*& slider, QLineEdit** edit_ptr, QLabel*& valueLabel, int row, int minSlider, int maxSlider, int initialVal, const QString& tooltipText) {
        grid->addWidget(new QLabel(labelText), row, 0); // Label
        slider = new QSlider(Qt::Horizontal);
        slider->setRange(minSlider, maxSlider);
        slider->setTickPosition(QSlider::TicksBelow);
        slider->setTickInterval((maxSlider - minSlider) / 10);
        slider->setToolTip(tooltipText);
        grid->addWidget(slider, row, 1); // Slider

        valueLabel = new QLabel(QString("(%1)").arg(initialVal));
        valueLabel->setMinimumWidth(50); // Ensure space
        valueLabel->setToolTip(tooltipText);
        grid->addWidget(valueLabel, row, 2); // Value Label

        if (edit_ptr) { // Only create and add QLineEdit if a pointer to it is provided
            *edit_ptr = new QLineEdit();
            QIntValidator* validator = new QIntValidator(*edit_ptr);
            validator->setRange(minSlider, maxSlider);
            (*edit_ptr)->setValidator(validator);

            (*edit_ptr)->setMinimumWidth(60);
            (*edit_ptr)->setMaximumWidth(80);
            (*edit_ptr)->setToolTip(tooltipText);
            grid->addWidget(*edit_ptr, row, 3); // Line Edit
        } else {
            // Add an empty widget to fill the column for alignment if no edit is needed
            QWidget* emptyWidget = new QWidget();
            emptyWidget->setFixedWidth(80); // Match width of QLineEdit
            grid->addWidget(emptyWidget, row, 3);
        }
    };


    // --- Core Advanced Sliders ---
    addNumericControlRowInt("Layers:", maxLayersSlider, nullptr, maxLayersValueLabel, 0, 5, 20, settingsPtr->max_layers,
                         "Determines the maximum number of original image layers used to create the tunnel effect. More layers create a deeper, more complex visual.");

    addNumericControlRowDouble("Haze (Blur):", blurRadiusSlider, nullptr, blurRadiusValueLabel, 1, 0, 50, settingsPtr->blur_radius, 10.0, 1,
                         "Applies a Gaussian blur to each frame. Higher values result in a dreamier, hazier effect (0.0-5.0).");

    addNumericControlRowInt("Stars:", numStarsSlider, nullptr, numStarsValueLabel, 2, 0, 200, settingsPtr->num_stars,
                         "Sets the number of stars in the background starfield. A higher count creates a denser starscape.");


    // --- Global Zoom Controls ---
    addNumericControlRowDouble("Global Zoom:", globalZoomSlider, &globalZoomEdit, globalZoomValueLabel, 3, 0, 100, settingsPtr->global_zoom_speed, 1000.0, 3,
                         "Controls the intensity of the zoom effect.");

    // NEW: Global Zoom Mode Dropdown
    grid->addWidget(new QLabel("Zoom Type:"), 4, 0); // Use next available row (e.g., row 4)
    globalZoomModeCombo = new QComboBox();
    globalZoomModeCombo->addItem("Linear");
    globalZoomModeCombo->addItem("Oscillating");
    globalZoomModeCombo->setToolTip("Choose between continuous zoom-in (Linear) or in-and-out (Oscillating) zoom.");
    grid->addWidget(globalZoomModeCombo, 4, 1, 1, 3); // Span 3 columns


    // --- Other Psychedelic Effect Controls (Adjusted row numbers) ---
    // Pixelation (Int)
    addNumericControlRowInt("Pixelation:", pixelationSlider, &pixelationEdit, pixelationValueLabel, 5, 0, 50, settingsPtr->pixelation_level,
                         "Applies a blocky pixelation effect. Higher values create larger blocks.");

    // Color Invert Frequency (Int)
    addNumericControlRowInt("Invert Freq:", colorInvertSlider, &colorInvertEdit, colorInvertValueLabel, 6, 0, 60, settingsPtr->color_invert_frequency,
                         "Inverts colors every N frames. Set to 0 for no inversion. (e.g., 5 for inversion every 5th frame)");

    // Wave Amplitude (Double)
    addNumericControlRowDouble("Wave Amp:", waveAmplitudeSlider, &waveAmplitudeEdit, waveAmplitudeValueLabel, 7, 0, 500, settingsPtr->wave_amplitude, 10.0, 1,
                         "Controls the strength of the wave distortion effect.");

    // Wave Frequency (Double)
    addNumericControlRowDouble("Wave Freq:", waveFrequencySlider, &waveFrequencyEdit, waveFrequencyValueLabel, 8, 0, 100, settingsPtr->wave_frequency, 100.0, 2,
                         "Controls how many waves appear across the image. Higher values mean more frequent waves.");

    // Wave Direction Dropdown
    grid->addWidget(new QLabel("Wave Dir:"), 9, 0);
    waveDirectionCombo = new QComboBox();
    waveDirectionCombo->addItem("None");
    waveDirectionCombo->addItem("Horizontal");
    waveDirectionCombo->addItem("Vertical");
    waveDirectionCombo->setToolTip("Choose the direction of the wave distortion (Horizontal, Vertical, None).");
    grid->addWidget(waveDirectionCombo, 9, 1, 1, 3);

    // Fractal Type Dropdown
    grid->addWidget(new QLabel("Fractal Type:"), 10, 0);
    fractalTypeCombo = new QComboBox();
    fractalTypeCombo->addItem("Sierpinski");
    fractalTypeCombo->addItem("None"); // Placeholder for other fractals
    fractalTypeCombo->setToolTip("Choose a fractal pattern to integrate into the GIF animation.");
    grid->addWidget(fractalTypeCombo, 10, 1, 1, 3);

    // Starfield Pattern Dropdown
    grid->addWidget(new QLabel("Starfield Pattern:"), 11, 0);
    starfieldPatternCombo = new QComboBox();
    starfieldPatternCombo->addItem("Random");
    starfieldPatternCombo->addItem("Spiral");
    starfieldPatternCombo->addItem("None"); // Placeholder for other patterns
    starfieldPatternCombo->setToolTip("Select a pattern for how the background stars are arranged (Random, Spiral).");
    grid->addWidget(starfieldPatternCombo, 11, 1, 1, 3);

    advSettingsMainLayout->addLayout(grid);
    mainLayout->addWidget(advancedSettingsGroup);

    // --- Dialog-specific Action Buttons (Randomize, Default, Close) ---
    QHBoxLayout *dialogActionButtonsLayout = new QHBoxLayout();
    dialogActionButtonsLayout->setSpacing(10);
    dialogActionButtonsLayout->setContentsMargins(0, 5, 0, 5);

    randomizeButton = new QPushButton("Cosmic Chaos");
    randomizeButton->setToolTip("Randomizes all advanced settings.");
    randomizeButton->setObjectName("dialogButton"); // Apply specific style
    dialogActionButtonsLayout->addWidget(randomizeButton);

    defaultButton = new QPushButton("Default");
    defaultButton->setToolTip("Resets all advanced settings to their recommended default values.");
    defaultButton->setObjectName("dialogButton"); // Apply specific style
    dialogActionButtonsLayout->addWidget(defaultButton);

    // Spacer to push Close button to the right
    dialogActionButtonsLayout->addStretch(1);

    QPushButton* closeButton = new QPushButton("Close");
    closeButton->setToolTip("Close this dialog and return to the main application.");
    dialogActionButtonsLayout->addWidget(closeButton);

    mainLayout->addLayout(dialogActionButtonsLayout);

    // Connect close button
    connect(closeButton, &QPushButton::clicked, this, &AdvancedSettingsDialog::accept);
}

// --- setupConnections Method ---
void AdvancedSettingsDialog::setupConnections() {
    // Existing advanced settings connections
    connect(maxLayersSlider, &QSlider::valueChanged, this, &AdvancedSettingsDialog::onMaxLayersChanged);
    connect(blurRadiusSlider, &QSlider::valueChanged, this, &AdvancedSettingsDialog::onBlurRadiusChanged);
    connect(numStarsSlider, &QSlider::valueChanged, this, &AdvancedSettingsDialog::onNumStarsChanged);
    connect(fractalTypeCombo, QOverload<int>::of(&QComboBox::currentIndexChanged), this, &AdvancedSettingsDialog::onFractalTypeChanged);
    connect(starfieldPatternCombo, QOverload<int>::of(&QComboBox::currentIndexChanged), this, &AdvancedSettingsDialog::onStarfieldPatternChanged);

    // Effect Control Connections
    connect(globalZoomSlider, &QSlider::valueChanged, this, &AdvancedSettingsDialog::onGlobalZoomSliderChanged);
    connect(globalZoomEdit, &QLineEdit::editingFinished, this, &AdvancedSettingsDialog::onGlobalZoomEditFinished);

    // NEW: Connect Global Zoom Mode Combo Box
    connect(globalZoomModeCombo, QOverload<int>::of(&QComboBox::currentIndexChanged), this, &AdvancedSettingsDialog::onGlobalZoomModeChanged);

    connect(pixelationSlider, &QSlider::valueChanged, this, &AdvancedSettingsDialog::onPixelationSliderChanged);
    connect(pixelationEdit, &QLineEdit::editingFinished, this, &AdvancedSettingsDialog::onPixelationEditFinished);

    connect(colorInvertSlider, &QSlider::valueChanged, this, &AdvancedSettingsDialog::onColorInvertSliderChanged);
    connect(colorInvertEdit, &QLineEdit::editingFinished, this, &AdvancedSettingsDialog::onColorInvertEditFinished);

    connect(waveAmplitudeSlider, &QSlider::valueChanged, this, &AdvancedSettingsDialog::onWaveAmplitudeSliderChanged);
    connect(waveAmplitudeEdit, &QLineEdit::editingFinished, this, &AdvancedSettingsDialog::onWaveAmplitudeEditFinished);

    connect(waveFrequencySlider, &QSlider::valueChanged, this, &AdvancedSettingsDialog::onWaveFrequencySliderChanged);
    connect(waveFrequencyEdit, &QLineEdit::editingFinished, this, &AdvancedSettingsDialog::onWaveFrequencyEditFinished);

    connect(waveDirectionCombo, QOverload<int>::of(&QComboBox::currentIndexChanged), this, &AdvancedSettingsDialog::onWaveDirectionChanged);

    // Connect Randomize and Default buttons
    connect(randomizeButton, &QPushButton::clicked, this, &AdvancedSettingsDialog::randomizeSettingsInDialog);
    connect(defaultButton, &QPushButton::clicked, this, &AdvancedSettingsDialog::resetToDefaultsInDialog);
}

// --- Helper function for updating numeric controls (slider <-> edit <-> label) for DOUBLE values ---
void AdvancedSettingsDialog::updateNumericControl(int sliderValue, QSlider* slider, QLineEdit* edit, QLabel* label,
                                                  double settingsMin, double settingsMax, double guiScaleFactor,
                                                  double& settingsVar, int precision) {
    QObject* senderObj = sender();

    if (senderObj == slider) {
        settingsVar = static_cast<double>(sliderValue) / guiScaleFactor;
        settingsVar = std::min(settingsMax, std::max(settingsMin, settingsVar)); // Clamp after scaling

        if (edit) {
            edit->blockSignals(true); // Prevent QLineEdit from re-triggering this function
            edit->setText(QString::number(settingsVar, 'f', precision));
            edit->blockSignals(false);
        }
        if (label) label->setText(QString("(%1)").arg(settingsVar, 0, 'f', precision));
    }
    else if (senderObj == edit) {
        bool ok;
        double val = edit->text().toDouble(&ok);
        if (ok) {
            settingsVar = std::min(settingsMax, std::max(settingsMin, val));
            slider->blockSignals(true); // Prevent slider from re-triggering this function
            slider->setValue(static_cast<int>(settingsVar * guiScaleFactor));
            slider->blockSignals(false);
            if (label) label->setText(QString("(%1)").arg(settingsVar, 0, 'f', precision));
            edit->setText(QString::number(settingsVar, 'f', precision)); // Update text for clamping
        } else {
            edit->setText(QString::number(settingsVar, 'f', precision)); // Revert
            if (label) label->setText(QString("(%1)").arg(settingsVar, 0, 'f', precision));
        }
    }
}

// --- Helper function for updating numeric controls (slider <-> edit <-> label) for INTEGER values ---
void AdvancedSettingsDialog::updateNumericControl(int sliderValue, QSlider* slider, QLineEdit* edit, QLabel* label,
                                                  int settingsMin, int settingsMax, double guiScaleFactor,
                                                  int& settingsVar) {
    QObject* senderObj = sender();

    if (senderObj == slider) {
        settingsVar = static_cast<int>(sliderValue / guiScaleFactor);
        settingsVar = std::min(settingsMax, std::max(settingsMin, settingsVar));

        if (edit) {
            edit->blockSignals(true);
            edit->setText(QString::number(settingsVar));
            edit->blockSignals(false);
        }
        if (label) label->setText(QString("(%1)").arg(settingsVar));
    }
    else if (senderObj == edit) {
        bool ok;
        int val = edit->text().toInt(&ok);
        if (ok) {
            settingsVar = std::min(settingsMax, std::max(settingsMin, val));
            slider->blockSignals(true);
            slider->setValue(static_cast<int>(settingsVar * guiScaleFactor));
            slider->blockSignals(false);
            if (label) label->setText(QString("(%1)").arg(settingsVar));
            edit->setText(QString::number(settingsVar)); // Update text for clamping
        } else {
            edit->setText(QString::number(settingsVar)); // Revert
            if (label) label->setText(QString("(%1)").arg(settingsVar));
        }
    }
}


// --- Slots for updating GifSettings and UI synchronization ---

// Existing Advanced Settings Sliders
void AdvancedSettingsDialog::onMaxLayersChanged(int value) {
    updateNumericControl(value, maxLayersSlider, nullptr, maxLayersValueLabel,
                         5, 20, 1.0, settingsPtr->max_layers);
}

void AdvancedSettingsDialog::onBlurRadiusChanged(int value) {
    updateNumericControl(value, blurRadiusSlider, nullptr, blurRadiusValueLabel,
                         0.0, 5.0, 10.0, settingsPtr->blur_radius, 1);
}

void AdvancedSettingsDialog::onNumStarsChanged(int value) {
    updateNumericControl(value, numStarsSlider, nullptr, numStarsValueLabel,
                         0, 200, 1.0, settingsPtr->num_stars);
}

void AdvancedSettingsDialog::onFractalTypeChanged(int index) {
    settingsPtr->advanced_fractal_type = fractalTypeCombo->itemText(index).toStdString();
}

void AdvancedSettingsDialog::onStarfieldPatternChanged(int index) {
    settingsPtr->advanced_starfield_pattern = starfieldPatternCombo->itemText(index).toStdString();
}


// --- New Effect Control Slots Implementations ---

// Global Zoom
void AdvancedSettingsDialog::onGlobalZoomSliderChanged(int value) {
    updateNumericControl(value, globalZoomSlider, globalZoomEdit, globalZoomValueLabel,
                         0.0, 0.1, 1000.0, settingsPtr->global_zoom_speed, 3);
}
void AdvancedSettingsDialog::onGlobalZoomEditFinished() {
    updateNumericControl(0, globalZoomSlider, globalZoomEdit, globalZoomValueLabel,
                         0.0, 0.1, 1000.0, settingsPtr->global_zoom_speed, 3);
}

// NEW: Global Zoom Mode Slot
void AdvancedSettingsDialog::onGlobalZoomModeChanged(int index) {
    settingsPtr->global_zoom_mode = globalZoomModeCombo->itemText(index).toStdString();
    emit settingsChanged(); // Notify MainWindow that settings have changed
}

// Pixelation
void AdvancedSettingsDialog::onPixelationSliderChanged(int value) {
    updateNumericControl(value, pixelationSlider, pixelationEdit, pixelationValueLabel,
                         0, 50, 1.0, settingsPtr->pixelation_level);
}
void AdvancedSettingsDialog::onPixelationEditFinished() {
    updateNumericControl(0, pixelationSlider, pixelationEdit, pixelationValueLabel,
                         0, 50, 1.0, settingsPtr->pixelation_level);
}

// Color Invert Frequency
void AdvancedSettingsDialog::onColorInvertSliderChanged(int value) {
    updateNumericControl(value, colorInvertSlider, colorInvertEdit, colorInvertValueLabel,
                         0, 60, 1.0, settingsPtr->color_invert_frequency);
}
void AdvancedSettingsDialog::onColorInvertEditFinished() {
    updateNumericControl(0, colorInvertSlider, colorInvertEdit, colorInvertValueLabel,
                         0, 60, 1.0, settingsPtr->color_invert_frequency);
}

// Wave Amplitude
void AdvancedSettingsDialog::onWaveAmplitudeSliderChanged(int value) {
    updateNumericControl(value, waveAmplitudeSlider, waveAmplitudeEdit, waveAmplitudeValueLabel,
                         0.0, 50.0, 10.0, settingsPtr->wave_amplitude, 1);
}
void AdvancedSettingsDialog::onWaveAmplitudeEditFinished() {
    updateNumericControl(0, waveAmplitudeSlider, waveAmplitudeEdit, waveAmplitudeValueLabel,
                         0.0, 50.0, 10.0, settingsPtr->wave_amplitude, 1);
}

// Wave Frequency
void AdvancedSettingsDialog::onWaveFrequencySliderChanged(int value) {
    updateNumericControl(value, waveFrequencySlider, waveFrequencyEdit, waveFrequencyValueLabel,
                         0.0, 1.0, 100.0, settingsPtr->wave_frequency, 2);
}
void AdvancedSettingsDialog::onWaveFrequencyEditFinished() {
    updateNumericControl(0, waveFrequencySlider, waveFrequencyEdit, waveFrequencyValueLabel,
                         0.0, 1.0, 100.0, settingsPtr->wave_frequency, 2);
}

// Wave Direction
void AdvancedSettingsDialog::onWaveDirectionChanged(int index) {
    settingsPtr->wave_direction = waveDirectionCombo->itemText(index).toStdString();
}

// --- randomizeSettingsInDialog Slot ---
void AdvancedSettingsDialog::randomizeSettingsInDialog() {
    std::random_device rd;
    std::mt19937 gen(rd());

    // Distributions for Core Settings
    std::uniform_int_distribution<> dist_num_frames(10, 100);
    std::uniform_int_distribution<> dist_scale_decay_int(50, 95); // For int 50 to 95 for slider mapping
    std::uniform_int_distribution<> dist_rotation_speed(0, 30);
    std::uniform_int_distribution<> dist_hue_speed_int(0, 200); // For int 0 to 200 for slider mapping
    std::vector<std::string> rotationDirections = {"Clockwise", "Counter-Clockwise", "None"};

    // Distributions for Advanced Settings
    std::uniform_int_distribution<> dist_layers(5, 20);
    std::uniform_int_distribution<> dist_blur_int(0, 50); // For int slider value
    std::uniform_int_distribution<> dist_stars(0, 200);
    std::uniform_int_distribution<> dist_global_zoom_int(0, 100);
    std::uniform_int_distribution<> dist_pixelation(0, 50);
    std::uniform_int_distribution<> dist_color_invert(0, 60);
    std::uniform_int_distribution<> dist_wave_amp_int(0, 500);
    std::uniform_int_distribution<> dist_wave_freq_int(0, 100);
    std::vector<std::string> fractalTypes = {"Sierpinski", "None"};
    std::vector<std::string> starfieldPatterns = {"Random", "Spiral", "None"};
    std::vector<std::string> waveDirections = {"None", "Horizontal", "Vertical"};
    std::vector<std::string> zoomModes = {"Linear", "Oscillating"}; // NEW: For randomizing zoom mode

    // Update settingsPtr directly
    // Core Settings Randomization
    settingsPtr->num_frames = dist_num_frames(gen);
    settingsPtr->scale_decay = static_cast<double>(dist_scale_decay_int(gen)) / 100.0;
    settingsPtr->rotation_speed = dist_rotation_speed(gen);
    settingsPtr->hue_speed = static_cast<double>(dist_hue_speed_int(gen)) / 10.0;
    settingsPtr->rotation_direction = rotationDirections[std::uniform_int_distribution<>(0, rotationDirections.size() - 1)(gen)];

    // Advanced Settings Randomization
    settingsPtr->max_layers = dist_layers(gen);
    settingsPtr->blur_radius = static_cast<double>(dist_blur_int(gen)) / 10.0;
    settingsPtr->num_stars = dist_stars(gen);
    settingsPtr->global_zoom_speed = static_cast<double>(dist_global_zoom_int(gen)) / 1000.0;
    settingsPtr->pixelation_level = dist_pixelation(gen);
    settingsPtr->color_invert_frequency = dist_color_invert(gen);
    settingsPtr->wave_amplitude = static_cast<double>(dist_wave_amp_int(gen)) / 10.0;
    settingsPtr->wave_frequency = static_cast<double>(dist_wave_freq_int(gen)) / 100.0;
    settingsPtr->wave_direction = waveDirections[std::uniform_int_distribution<>(0, waveDirections.size() - 1)(gen)];
    settingsPtr->advanced_fractal_type = fractalTypes[std::uniform_int_distribution<>(0, fractalTypes.size() - 1)(gen)];
    settingsPtr->advanced_starfield_pattern = starfieldPatterns[std::uniform_int_distribution<>(0, starfieldPatterns.size() - 1)(gen)];
    settingsPtr->global_zoom_mode = zoomModes[std::uniform_int_distribution<>(0, zoomModes.size() - 1)(gen)]; // NEW: Randomize zoom mode

    // Update the UI elements in the dialog
    updateDialogUiFromSettings();
    emit settingsChanged(); // Notify MainWindow that settings have changed
    qDebug() << "AdvancedSettingsDialog: Randomized settings and emitted settingsChanged().";
}

// --- resetToDefaultsInDialog Slot ---
void AdvancedSettingsDialog::resetToDefaultsInDialog() {
    // Update settingsPtr directly with the centralized default values
    *settingsPtr = GifSettings::getDefaultSettings(); // Uses the static method from gif_settings.h

    // Update the UI elements in the dialog
    updateDialogUiFromSettings();
    emit settingsChanged(); // Notify MainWindow that settings have changed
    qDebug() << "AdvancedSettingsDialog: Reset to defaults and emitted settingsChanged().";
}

// --- Helper to update all dialog UI elements from settingsPtr ---
void AdvancedSettingsDialog::updateDialogUiFromSettings() {
    // Block signals to prevent slots from being triggered during UI update
    maxLayersSlider->blockSignals(true);
    blurRadiusSlider->blockSignals(true);
    numStarsSlider->blockSignals(true);
    globalZoomSlider->blockSignals(true); globalZoomEdit->blockSignals(true);
    globalZoomModeCombo->blockSignals(true); // NEW: Block this combo box
    pixelationSlider->blockSignals(true); pixelationEdit->blockSignals(true);
    colorInvertSlider->blockSignals(true); colorInvertEdit->blockSignals(true);
    waveAmplitudeSlider->blockSignals(true); waveAmplitudeEdit->blockSignals(true);
    waveFrequencySlider->blockSignals(true); waveFrequencyEdit->blockSignals(true);
    waveDirectionCombo->blockSignals(true);
    fractalTypeCombo->blockSignals(true);
    starfieldPatternCombo->blockSignals(true);


    // Update sliders and labels
    maxLayersSlider->setValue(settingsPtr->max_layers);
    maxLayersValueLabel->setText(QString("(%1)").arg(settingsPtr->max_layers));

    blurRadiusSlider->setValue(static_cast<int>(settingsPtr->blur_radius * 10));
    blurRadiusValueLabel->setText(QString("(%1)").arg(settingsPtr->blur_radius, 0, 'f', 1));

    numStarsSlider->setValue(settingsPtr->num_stars);
    numStarsValueLabel->setText(QString("(%1)").arg(settingsPtr->num_stars));

    globalZoomSlider->setValue(static_cast<int>(settingsPtr->global_zoom_speed * 1000));
    globalZoomEdit->setText(QString::number(settingsPtr->global_zoom_speed, 'f', 3));
    globalZoomValueLabel->setText(QString("(%1)").arg(settingsPtr->global_zoom_speed, 0, 'f', 3));

    pixelationSlider->setValue(settingsPtr->pixelation_level);
    pixelationEdit->setText(QString::number(settingsPtr->pixelation_level));
    pixelationValueLabel->setText(QString("(%1)").arg(settingsPtr->pixelation_level));

    colorInvertSlider->setValue(settingsPtr->color_invert_frequency);
    colorInvertEdit->setText(QString::number(settingsPtr->color_invert_frequency));
    colorInvertValueLabel->setText(QString("(%1)").arg(settingsPtr->color_invert_frequency));

    waveAmplitudeSlider->setValue(static_cast<int>(settingsPtr->wave_amplitude * 10));
    waveAmplitudeEdit->setText(QString::number(settingsPtr->wave_amplitude, 'f', 1));
    waveAmplitudeValueLabel->setText(QString("(%1)").arg(settingsPtr->wave_amplitude, 0, 'f', 1));

    waveFrequencySlider->setValue(static_cast<int>(settingsPtr->wave_frequency * 100));
    waveFrequencyEdit->setText(QString::number(settingsPtr->wave_frequency, 'f', 2));
    waveFrequencyValueLabel->setText(QString("(%1)").arg(settingsPtr->wave_frequency, 0, 'f', 2));

    // Update dropdowns
    int waveDirIndex = waveDirectionCombo->findText(QString::fromStdString(settingsPtr->wave_direction));
    if (waveDirIndex != -1) waveDirectionCombo->setCurrentIndex(waveDirIndex);

    int fractalIndex = fractalTypeCombo->findText(QString::fromStdString(settingsPtr->advanced_fractal_type));
    if (fractalIndex != -1) fractalTypeCombo->setCurrentIndex(fractalIndex);

    int starfieldIndex = starfieldPatternCombo->findText(QString::fromStdString(settingsPtr->advanced_starfield_pattern));
    if (starfieldIndex != -1) starfieldPatternCombo->setCurrentIndex(starfieldIndex);

    // NEW: Update Global Zoom Mode dropdown
    int zoomModeIndex = globalZoomModeCombo->findText(QString::fromStdString(settingsPtr->global_zoom_mode));
    if (zoomModeIndex != -1) globalZoomModeCombo->setCurrentIndex(zoomModeIndex);


    // Unblock signals
    maxLayersSlider->blockSignals(false);
    blurRadiusSlider->blockSignals(false);
    numStarsSlider->blockSignals(false);
    globalZoomSlider->blockSignals(false); globalZoomEdit->blockSignals(false);
    globalZoomModeCombo->blockSignals(false); // NEW: Unblock this combo box
    pixelationSlider->blockSignals(false); pixelationEdit->blockSignals(false);
    colorInvertSlider->blockSignals(false); colorInvertEdit->blockSignals(false);
    waveAmplitudeSlider->blockSignals(false); waveAmplitudeEdit->blockSignals(false);
    waveFrequencySlider->blockSignals(false); waveFrequencyEdit->blockSignals(false);
    waveDirectionCombo->blockSignals(false);
    fractalTypeCombo->blockSignals(false);
    starfieldPatternCombo->blockSignals(false);
}
