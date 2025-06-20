// mainwindow.cpp
#include "mainwindow.h" // Include the declaration of MainWindow
#include "gif_generator.h" // Include the declaration of createPsychedelicGif
#include "advancedsettingsdialog.h" // Include the advanced dialog header

#include <QVBoxLayout>
#include <QHBoxLayout>
#include <QGridLayout>
#include <QFileDialog>
#include <QMessageBox>
#include <QDir>
#include <QStatusBar>
#include <QtConcurrent/QtConcurrent> // For QtConcurrent::run
#include <random> // For std::random_device, std::mt19937, std::uniform_int_distribution
#include <string> // For std::string conversions
#include <QDebug> // For debugging output

// For Image Preview (OpenCV to QPixmap conversion)
#include <opencv2/imgcodecs.hpp> // For cv::imread
#include <opencv2/imgproc.hpp>   // For cv::resize
#include <QPixmap>
#include <QImage>

// --- Constructor ---
MainWindow::MainWindow(QWidget *parent) : QMainWindow(parent) {
    setWindowTitle("Psychedelic GIF Creator");
    resize(800, 700); // Set a default window size

    // Initialize our settings object using the centralized default settings
    currentSettings = GifSettings::getDefaultSettings();

    setupUi();         // Build the GUI elements
    setupConnections(); // Connect signals and slots
    setupQtConcurrent(); // Setup for QFutureWatcher

    // At startup, set the core UI controls to match the default values
    refreshCoreSettingsUi();
}

// --- setupUi Method ---
void MainWindow::setupUi() {
    QWidget *centralWidget = new QWidget(this);
    setCentralWidget(centralWidget);

    // --- Apply Global Qt Style Sheet (QSS) ---
    // This defines the overall look and feel, including colors, fonts, and basic reactivity
    QString styleSheet = R"(
        /* Global Background and Text Colors */
        QMainWindow, QWidget {
            background-color: #1A1A2E; /* Deep dark blue/purple */
            color: #E0E0E0; /* Light gray for general text */
            font-family: "Segoe UI", "Roboto", "Open Sans", sans-serif;
            font-size: 10pt;
        }

        /* Group Boxes */
        QGroupBox {
            background-color: #24293D; /* Slightly lighter dark */
            border: 1px solid #3A4750; /* Subtle border */
            border-radius: 8px;
            margin-top: 1ex; /* Space for title */
            padding: 10px;
        }
        QGroupBox::title {
            subcontrol-origin: margin;
            subcontrol-position: top left; /* Position at top left */
            padding: 0 5px;
            color: #90CAF9; /* Light blue accent for titles */
            font-size: 10pt;
            font-weight: bold;
        }

        /* Labels */
        QLabel {
            color: #E0E0E0; /* Consistent light text */
            padding: 2px;
        }

        /* Line Edits (Text Inputs) */
        QLineEdit {
            background-color: #1A1A2E; /* Same as main window background */
            border: 1px solid #3A4750;
            border-radius: 5px;
            padding: 5px;
            color: #64FFDA; /* Teal accent for input text */
            selection-background-color: #64FFDA;
            selection-color: #1A1A2E;
        }
        QLineEdit:focus {
            border: 1px solid #64FFDA; /* Accent border on focus */
        }

        /* Push Buttons (General) */
        QPushButton {
            background-color: #3A4750; /* Muted dark button */
            color: #E0E0E0;
            border: none;
            border-radius: 5px;
            padding: 8px 15px;
            min-height: 28px; /* Consistent height */
            font-weight: bold;
        }
        QPushButton:hover {
            background-color: #4A5763; /* Lighter on hover */
            color: #FFFFFF;
        }
        QPushButton:pressed {
            background-color: #303A45; /* Darker on press */
            border-style: inset;
        }
        QPushButton:disabled {
            background-color: #2A2A3A;
            color: #888888;
        }

        /* Accent Buttons (e.g., Generate Button) */
        QPushButton#generateButton { /* Using an objectName for specific styling */
            background-color: #64FFDA; /* Bright Teal */
            color: #1A1A2E; /* Dark text on bright button */
        }
        QPushButton#generateButton:hover {
            background-color: #7BFFED;
        }
        QPushButton#generateButton:pressed {
            background-color: #58E0C4;
        }
        /* Default Button specific style (small, distinct) */
        /* This style is now for the default button *inside* the Advanced Settings dialog */
        QPushButton#dialogButton {
            background-color: #555555; /* Darker grey */
            color: #BBBBBB;
            font-size: 8pt; /* Smaller font */
            padding: 4px 8px; /* Smaller padding */
            min-height: 20px; /* Smaller height */
            border-radius: 3px;
        }
        QPushButton#dialogButton:hover {
            background-color: #666666;
        }
        QPushButton#dialogButton:pressed {
            background-color: #444444;
        }


        /* Sliders */
        QSlider::groove:horizontal {
            border: 1px solid #3A4750;
            height: 8px;
            background: #2A2A3A; /* Dark track */
            margin: 2px 0;
            border-radius: 4px;
        }
        QSlider::handle:horizontal {
            background: #64FFDA; /* Accent thumb */
            border: 1px solid #56B6C2;
            width: 18px;
            margin: -5px 0; /* Center handle vertically */
            border-radius: 9px;
        }
        QSlider::sub-page:horizontal {
            background: #4A5763; /* Filled portion of track */
            border-radius: 4px;
        }
        QSlider::add-page:horizontal {
            background: #2A2A3A;
            border-radius: 4px;
        }

        /* Combo Boxes (Dropdowns) */
        QComboBox {
            background-color: #1A1A2E;
            border: 1px solid #3A4750;
            border-radius: 5px;
            padding: 5px;
            color: #E0E0E0;
            selection-background-color: #64FFDA; /* Accent for selected item in dropdown list */
            selection-color: #1A1A2E;
        }
        QComboBox::drop-down {
            border: none; /* No default border */
            subcontrol-origin: padding;
            subcontrol-position: top right;
            width: 20px; /* Size of the dropdown arrow area */
        }
        QComboBox::down-arrow {
            /* Placeholder for an actual SVG/icon. For now, system default */
            image: url(:/qt-project.org/styles/commonstyle/images/down-arrow.png); /* Example */
        }
        QComboBox QAbstractItemView { /* Style for the dropdown list itself */
            background-color: #24293D;
            border: 1px solid #3A4750;
            border-radius: 5px;
            selection-background-color: #64FFDA;
            selection-color: #1A1A2E;
        }

        /* Progress Bar */
        QProgressBar {
            border: 1px solid #3A4750;
            border-radius: 5px;
            text-align: center;
            color: #E0E0E0;
            background-color: #2A2A3A;
        }
        QProgressBar::chunk {
            background-color: qlineargradient(x1:0, y1:0, x2:1, y2:0, stop:0 #64FFDA, stop:1 #4A90E2); /* Gradient fill */
            border-radius: 5px;
        }

        /* Status Bar */
        QStatusBar {
            background-color: #24293D;
            color: #E0E0E0;
            border-top: 1px solid #3A4750;
        }
        QStatusBar::item {
            border: none;
        }
        QToolTip { /* Tooltip global style */
            background-color: #3A4750;
            color: #E0E0E0;
            border: 1px solid #64FFDA;
            border-radius: 5px;
            padding: 5px;
            opacity: 200; /* Semi-transparent */
            font-size: 9pt;
        }
    )";
    centralWidget->setStyleSheet(styleSheet);


    QVBoxLayout *mainLayout = new QVBoxLayout(centralWidget);
    mainLayout->setContentsMargins(20, 20, 20, 20); // Add padding around the main layout
    mainLayout->setSpacing(15); // Add spacing between major sections

    // --- Image Preview/Placeholder (Phase 3.4) ---
    imagePreviewLabel = new QLabel("Select an image to begin your cosmic journey...");
    imagePreviewLabel->setAlignment(Qt::AlignCenter);
    imagePreviewLabel->setFixedSize(400, 250); // Larger fixed size for preview
    imagePreviewLabel->setStyleSheet("background-color: #1A1A2E; color: #888888; border: 2px dashed #4A5763; border-radius: 10px;");
    imagePreviewLabel->setWordWrap(true); // Allow text to wrap
    mainLayout->addWidget(imagePreviewLabel, 2); // Stretch factor 2 for preview

    // --- Input Section ---
    QGroupBox *inputGroup = new QGroupBox("Input Artifact");
    // QGroupBox style is applied globally via stylesheet
    QVBoxLayout *inputLayout = new QVBoxLayout(inputGroup);
    inputLayout->setContentsMargins(10, 20, 10, 10); // Adjust padding within group box
    inputLayout->setSpacing(8);

    QHBoxLayout *imagePathLayout = new QHBoxLayout();
    imagePathLayout->setSpacing(5);

    imagePathEdit = new QLineEdit(QString::fromStdString(currentSettings.image_path));
    imagePathEdit->setPlaceholderText("Path to your image file (e.g., /home/user/image.png)");
    imagePathEdit->setToolTip("Enter or browse for the image file to transform.");
    imagePathLayout->addWidget(imagePathEdit);

    browseButton = new QPushButton("Browse");
    browseButton->setToolTip("Select the input image file.");
    imagePathLayout->addWidget(browseButton);

    inputLayout->addLayout(imagePathLayout);
    mainLayout->addWidget(inputGroup);


    // --- Core Settings Section ---
    QGroupBox *coreSettingsGroup = new QGroupBox("Core Settings");
    QVBoxLayout *coreSettingsMainLayout = new QVBoxLayout(coreSettingsGroup); // New VBox for internal padding
    coreSettingsMainLayout->setContentsMargins(10, 20, 10, 10);
    coreSettingsMainLayout->setSpacing(10);

    QGridLayout *coreSettingsLayout = new QGridLayout(); // Grid for sliders within the VBox
    coreSettingsLayout->setColumnStretch(1, 1); // Make slider column stretchable
    coreSettingsLayout->setHorizontalSpacing(10);
    coreSettingsLayout->setVerticalSpacing(5);

    // Helper to add slider rows (reduces repetition)
    // Modified: Removed initialVal parameter, as refreshCoreSettingsUi will set values
    auto addSliderRow = [&](const QString& labelText, QSlider*& slider, QLabel*& valueLabel, int row, int minVal, int maxVal, const QString& tooltipText) {
        coreSettingsLayout->addWidget(new QLabel(labelText), row, 0);
        slider = new QSlider(Qt::Horizontal);
        slider->setRange(minVal, maxVal);
        // slider->setValue() will be set by refreshCoreSettingsUi()
        slider->setTickPosition(QSlider::TicksBelow);
        slider->setTickInterval( (maxVal - minVal) / 10 ); // Approx 10 ticks
        coreSettingsLayout->addWidget(slider, row, 1);
        valueLabel = new QLabel(""); // Initialize with empty string, refreshCoreSettingsUi will set actual value
        valueLabel->setMinimumWidth(50); // Ensure enough space for value
        coreSettingsLayout->addWidget(valueLabel, row, 2);
        slider->setToolTip(tooltipText); // Tooltip for slider
        valueLabel->setToolTip(tooltipText); // Tooltip for value label
    };

    // Cycles Slider
    addSliderRow("Cycles:", numFramesSlider, numFramesValueLabel, 0, 10, 100, // No initialVal here
                 "Number of frames in the GIF. More frames result in a smoother but larger GIF.");

    // Warp Slider
    addSliderRow("Warp:", scaleDecaySlider, scaleDecayValueLabel, 1, 50, 95, // No initialVal here
                 "Controls how quickly inner layers shrink (0.50-0.95). Lower values create a more intense tunnel effect.");

    // Spin Speed Slider
    addSliderRow("Spin Speed:", rotationSpeedSlider, rotationSpeedValueLabel, 2, 0, 30, // No initialVal here
                 "Determines the rotational speed of the psychedelic layers.");

    // Pulse Speed Slider
    addSliderRow("Pulse Speed:", hueSpeedSlider, hueSpeedValueLabel, 3, 0, 200, // No initialVal here
                 "Controls the speed of the color shifting (hue rotation) effect.");


    // Spin Direction Dropdown
    coreSettingsLayout->addWidget(new QLabel("Spin Dir:"), 4, 0);
    rotationDirectionCombo = new QComboBox();
    rotationDirectionCombo->addItem("Clockwise");
    rotationDirectionCombo->addItem("Counter-Clockwise");
    rotationDirectionCombo->addItem("None");
    // The current index will be set by refreshCoreSettingsUi()
    rotationDirectionCombo->setToolTip("Choose the direction of the rotational animation.");
    coreSettingsLayout->addWidget(rotationDirectionCombo, 4, 1, 1, 2); // Span 2 columns

    coreSettingsMainLayout->addLayout(coreSettingsLayout); // Add the grid to the main VBox of settings group
    mainLayout->addWidget(coreSettingsGroup);


    // --- Action Buttons ---
    QHBoxLayout *actionButtonsLayout = new QHBoxLayout();
    actionButtonsLayout->setSpacing(10); // Spacing between buttons
    actionButtonsLayout->setContentsMargins(0, 5, 0, 5); // Padding around buttons

    advancedButton = new QPushButton("Advanced Settings");
    advancedButton->setToolTip("Configure fractal types, starfield patterns, and more detailed options.");
    actionButtonsLayout->addWidget(advancedButton);

    generateButton = new QPushButton("Launch");
    generateButton->setToolTip("Generate the psychedelic GIF with current settings.");
    generateButton->setObjectName("generateButton"); // Set object name for specific QSS styling
    actionButtonsLayout->addWidget(generateButton);

    mainLayout->addLayout(actionButtonsLayout);


    // --- Progress Bar and Status ---
    progressBar = new QProgressBar();
    progressBar->setRange(0, 100);
    progressBar->setValue(0);
    progressBar->setTextVisible(true);
    progressBar->setFormat("Ready to Launch...");
    progressBar->setToolTip("Shows the progress of GIF generation.");
    mainLayout->addWidget(progressBar);

    // QStatusBar provides built-in message display
    setStatusBar(new QStatusBar(this));
    statusBar()->showMessage("Welcome, Commander! Stand by for hyperspace jump.", 5000); // Initial message

    // Make central widget expandable
    mainLayout->addStretch(1); // Push content to top if window is very large
}

// --- setupConnections Method ---
void MainWindow::setupConnections() {
    // Input
    connect(browseButton, &QPushButton::clicked, this, &MainWindow::browseImage);

    // Core Sliders
    // Connect valueChanged signal to a lambda that updates the label and currentSettings
    connect(numFramesSlider, &QSlider::valueChanged, this, [this](int value){
        currentSettings.num_frames = value;
        updateSliderValueLabel(value, numFramesValueLabel);
    });
    connect(scaleDecaySlider, &QSlider::valueChanged, this, [this](int value){
        currentSettings.scale_decay = static_cast<double>(value) / 100.0;
        updateSliderValueLabel(value, scaleDecayValueLabel, 100.0, 2);
    });
    connect(rotationSpeedSlider, &QSlider::valueChanged, this, [this](int value){
        currentSettings.rotation_speed = value;
        updateSliderValueLabel(value, rotationSpeedValueLabel);
    });
    connect(hueSpeedSlider, &QSlider::valueChanged, this, [this](int value){
        currentSettings.hue_speed = static_cast<double>(value) / 10.0;
        updateSliderValueLabel(value, hueSpeedValueLabel, 10.0, 1);
    });

    connect(rotationDirectionCombo, QOverload<int>::of(&QComboBox::currentIndexChanged), this, &MainWindow::onRotationDirectionChanged);

    // Action Buttons
    connect(advancedButton, &QPushButton::clicked, this, &MainWindow::openAdvancedSettings);
    connect(generateButton, &QPushButton::clicked, this, &MainWindow::startGifGeneration);

    // Connect our custom signal for progress updates from the worker thread to the main thread
    connect(this, &MainWindow::generationProgress, this,
            [this](int percentage, const QString& message) {
                progressBar->setValue(percentage);
                progressBar->setFormat(message + " %p%"); // Show percentage in format
                statusBar()->showMessage(message); // Also update status bar
            });
}

// --- setupQtConcurrent Method ---
void MainWindow::setupQtConcurrent() {
    // No specific initial setup needed here, just the QFutureWatcher member
}

// --- setUiEnabled Method ---
void MainWindow::setUiEnabled(bool enabled) {
    imagePathEdit->setEnabled(enabled);
    browseButton->setEnabled(enabled);
    numFramesSlider->setEnabled(enabled);
    numFramesValueLabel->setEnabled(enabled);
    scaleDecaySlider->setEnabled(enabled);
    scaleDecayValueLabel->setEnabled(enabled);
    rotationSpeedSlider->setEnabled(enabled);
    rotationSpeedValueLabel->setEnabled(enabled);
    hueSpeedSlider->setEnabled(enabled);
    hueSpeedValueLabel->setEnabled(enabled);
    rotationDirectionCombo->setEnabled(enabled);
    advancedButton->setEnabled(enabled);
    generateButton->setEnabled(enabled);
    // Add other widgets as needed (e.g., the advanced settings dialog's controls will be enabled/disabled when it's open/closed)
}

// --- browseImage Slot ---
void MainWindow::browseImage() {
    QString filePath = QFileDialog::getOpenFileName(this,
                                                    "Select Cosmic Artifact",
                                                    QDir::homePath(),
                                                    "Image Files (*.png *.jpg *.jpeg *.gif *.bmp)");
    if (!filePath.isEmpty()) {
        imagePathEdit->setText(filePath);
        currentSettings.image_path = filePath.toStdString();

        // --- Phase 3.4: Implement Image Preview ---
        try {
            cv::Mat img = cv::imread(filePath.toStdString(), cv::IMREAD_UNCHANGED);
            if (img.empty()) {
                imagePreviewLabel->setText("Error: Could not load image preview.");
                return;
            }

            // Ensure RGBA for preview if not already
            if (img.channels() < 4) {
                 cv::cvtColor(img, img, cv::COLOR_BGR2BGRA);
            }

            // Convert BGR to RGB (for Qt compatibility if original is BGR)
            cv::cvtColor(img, img, cv::COLOR_BGRA2RGBA); // Ensure proper channel order for QImage

            // Scale image for preview without distorting aspect ratio
            QSize labelSize = imagePreviewLabel->size();
            QImage qImage(img.data, img.cols, img.rows, img.step, QImage::Format_RGBA8888);
            QPixmap pixmap = QPixmap::fromImage(qImage);
            pixmap = pixmap.scaled(labelSize, Qt::KeepAspectRatio, Qt::SmoothTransformation);

            imagePreviewLabel->setPixmap(pixmap);
            imagePreviewLabel->setText(""); // Clear placeholder text
            imagePreviewLabel->setAlignment(Qt::AlignCenter); // Center the image
            imagePreviewLabel->setStyleSheet("background-color: #1A1A2E; border: none;"); // Remove border after image load

        } catch (const cv::Exception& e) {
            imagePreviewLabel->setText(QString("Error loading image: %1").arg(e.what()));
            imagePreviewLabel->setStyleSheet("background-color: #1A1A2E; color: #E74C3C; border: 2px dashed #E74C3C; border-radius: 10px;");
        } catch (const std::exception& e) {
            imagePreviewLabel->setText(QString("Error: %1").arg(e.what()));
            imagePreviewLabel->setStyleSheet("background-color: #1A1A2E; color: #E74C3C; border: 2px dashed #E74C3C; border-radius: 10px;");
        }
    }
}

// --- updateSliderValueLabel Method ---
void MainWindow::updateSliderValueLabel(int value, QLabel* label, double factor, int precision) {
    label->setText(QString("(%1)").arg(static_cast<double>(value) / factor, 0, 'f', precision));
}

// --- onRotationDirectionChanged Slot ---
void MainWindow::onRotationDirectionChanged(int index) {
    currentSettings.rotation_direction = rotationDirectionCombo->itemText(index).toStdString();
}

// --- openAdvancedSettings Slot ---
void MainWindow::openAdvancedSettings() {
    AdvancedSettingsDialog advDialog(&currentSettings, this); // Pass pointer to settings
    // Connect dialog's settingsChanged signal to MainWindow's refresh slot
    connect(&advDialog, &AdvancedSettingsDialog::settingsChanged, this, &MainWindow::refreshCoreSettingsUi);
    advDialog.exec(); // Show the dialog modally
    // The refreshCoreSettingsUi() slot will now be called by the dialog when needed.
}

// --- startGifGeneration Slot (Threading) ---
void MainWindow::startGifGeneration() {
    if (currentSettings.image_path.empty()) {
        QMessageBox::warning(this, "Input Required", "Please select an input image first.");
        return;
    }

    QString outputFilePath = QFileDialog::getSaveFileName(this,
                                                          "Save Generated GIF",
                                                          QDir::homePath() + "/output.gif",
                                                          "GIF Files (*.gif)");
    if (outputFilePath.isEmpty()) {
        return; // User cancelled
    }

    setUiEnabled(false); // Disable UI during generation
    statusBar()->showMessage("Launching Hyperspace...", 0);
    progressBar->setValue(0);
    progressBar->setFormat("Initializing...");

    // Create a QFutureWatcher to monitor the background task
    QFuture<std::string> future = QtConcurrent::run(
        [this, outputFilePath]() {
            // This lambda runs in a separate thread
            // We pass a local lambda for progress updates to the core function
            auto progress_cb = [this](int percentage, const std::string& message) {
                // Emit a signal to update GUI from the main thread
                emit generationProgress(percentage, QString::fromStdString(message));
            };
            return createPsychedelicGif(this->currentSettings, outputFilePath.toStdString(), progress_cb);
        });

    futureWatcher.setFuture(future);
    connect(&futureWatcher, &QFutureWatcher<std::string>::finished, this, &MainWindow::gifGenerationFinished);
    connect(&futureWatcher, &QFutureWatcher<std::string>::progressValueChanged, progressBar, &QProgressBar::setValue);
    connect(&futureWatcher, &QFutureWatcher<std::string>::progressTextChanged, progressBar, &QProgressBar::setFormat);
}

// --- gifGenerationFinished Slot ---
void MainWindow::gifGenerationFinished() {
    std::string result = futureWatcher.result();
    setUiEnabled(true); // Re-enable UI

    if (result.rfind("Error:", 0) == 0 || result.rfind("OpenCV Error:", 0) == 0 || result.rfind("Standard Error:", 0) == 0) {
        QMessageBox::critical(this, "Generation Failed", QString::fromStdString(result));
        statusBar()->showMessage("Generation Failed!");
        progressBar->setValue(0);
        progressBar->setFormat("Failed!");
    } else {
        statusBar()->showMessage("GIF Saved Successfully!", 5000);
        progressBar->setValue(100);
        progressBar->setFormat("Complete!");
    }
}

// --- refreshCoreSettingsUi Slot ---
void MainWindow::refreshCoreSettingsUi() {
    qDebug() << "MainWindow::refreshCoreSettingsUi() called.";

    // Block signals to prevent immediate re-triggering of slots during refresh
    numFramesSlider->blockSignals(true);
    scaleDecaySlider->blockSignals(true);
    rotationSpeedSlider->blockSignals(true);
    hueSpeedSlider->blockSignals(true);
    rotationDirectionCombo->blockSignals(true);

    // Update UI elements based on currentSettings
    numFramesSlider->setValue(currentSettings.num_frames);
    updateSliderValueLabel(currentSettings.num_frames, numFramesValueLabel);

    // For scale_decay, it's a double in settings (0.xx) but an int in slider (50-95)
    scaleDecaySlider->setValue(static_cast<int>(currentSettings.scale_decay * 100));
    updateSliderValueLabel(static_cast<int>(currentSettings.scale_decay * 100), scaleDecayValueLabel, 100.0, 2);

    rotationSpeedSlider->setValue(currentSettings.rotation_speed);
    updateSliderValueLabel(currentSettings.rotation_speed, rotationSpeedValueLabel);

    // For hue_speed, it's a double in settings (X.X) but an int in slider (0-200)
    hueSpeedSlider->setValue(static_cast<int>(currentSettings.hue_speed * 10));
    updateSliderValueLabel(static_cast<int>(currentSettings.hue_speed * 10), hueSpeedValueLabel, 10.0, 1);

    int rotationIndex = rotationDirectionCombo->findText(QString::fromStdString(currentSettings.rotation_direction));
    if (rotationIndex != -1) {
        rotationDirectionCombo->setCurrentIndex(rotationIndex);
    }

    // Unblock signals
    numFramesSlider->blockSignals(false);
    scaleDecaySlider->blockSignals(false);
    rotationSpeedSlider->blockSignals(false);
    hueSpeedSlider->blockSignals(false);
    rotationDirectionCombo->blockSignals(false);

    statusBar()->showMessage("Core settings refreshed.", 2000); // Short message
}