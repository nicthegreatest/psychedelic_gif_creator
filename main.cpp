// main.cpp (GUI Entry Point)
#include <QApplication>
#include "mainwindow.h"

int main(int argc, char *argv[]) {
    QApplication app(argc, argv);

    // --- GLOBAL STYLESHEET ---
    QString styleSheet = R"(
        /* Global Background and Text Colors */
        QMainWindow, QWidget, QDialog {
            background-color: #1A1A2E;
            color: #E0E0E0;
            font-family: "Segoe UI", "Roboto", "Open Sans", sans-serif;
            font-size: 10pt;
        }

        /* Group Boxes */
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

        /* Labels */
        QLabel {
            color: #E0E0E0;
            padding: 2px;
            background-color: transparent;
        }

        /* Line Edits (Text Inputs) */
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
        
        /* Spin Boxes */
        QSpinBox, QDoubleSpinBox {
            background-color: #1A1A2E;
            border: 1px solid #3A4750;
            border-radius: 5px;
            padding: 5px;
            color: #64FFDA;
        }
        QSpinBox:focus, QDoubleSpinBox:focus {
            border: 1px solid #64FFDA;
        }

        /* --- MODIFICATION: Style the up/down buttons on SpinBoxes --- */
        QSpinBox::up-button, QDoubleSpinBox::up-button,
        QSpinBox::down-button, QDoubleSpinBox::down-button {
            background-color: #3A4750;
            border: none;
        }
        QSpinBox::up-button:hover, QDoubleSpinBox::up-button:hover,
        QSpinBox::down-button:hover, QDoubleSpinBox::down-button:hover {
            background-color: #4A5763;
        }
        QSpinBox::up-arrow, QDoubleSpinBox::up-arrow {
            border-left: 3px solid transparent;
            border-right: 3px solid transparent;
            border-bottom: 3px solid #E0E0E0;
            width: 0px; height: 0px;
        }
        QSpinBox::down-arrow, QDoubleSpinBox::down-arrow {
            border-left: 3px solid transparent;
            border-right: 3px solid transparent;
            border-top: 3px solid #E0E0E0;
            width: 0px; height: 0px;
        }
        /* --- END MODIFICATION --- */


        /* Push Buttons (General) */
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
        QPushButton:disabled {
            background-color: #2A2A3A;
            color: #888888;
        }

        /* Accent Buttons (e.g., Generate Button) */
        QPushButton#generateButton {
            background-color: #64FFDA;
            color: #1A1A2E;
        }
        QPushButton#generateButton:hover {
            background-color: #7BFFED;
        }
        QPushButton#generateButton:pressed {
            background-color: #58E0C4;
        }
        
        /* Buttons inside the Advanced Dialog */
        QPushButton#dialogButton {
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

        /* Sliders */
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

        /* Combo Boxes (Dropdowns) */
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
        }
        QComboBox QAbstractItemView {
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
            background-color: qlineargradient(x1:0, y1:0, x2:1, y2:0, stop:0 #64FFDA, stop:1 #4A90E2);
            border-radius: 5px;
        }

        /* Status Bar */
        QStatusBar {
            background-color: #24293D;
            color: #E0E0E0;
            border-top: 1px solid #3A4750;
        }
        
        /* Tooltip global style */
        QToolTip {
            background-color: #3A4750;
            color: #E0E0E0;
            border: 1px solid #64FFDA;
            border-radius: 5px;
            padding: 5px;
            opacity: 200;
            font-size: 9pt;
        }
    )";
    app.setStyleSheet(styleSheet);
    
    MainWindow mainWindow;
    mainWindow.show();
    return app.exec();
}