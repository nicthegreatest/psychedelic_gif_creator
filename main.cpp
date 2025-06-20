// main.cpp (GUI Entry Point)
#include <QApplication>
#include "mainwindow.h" // Include the declaration of our MainWindow class

// The main entry point for the Qt GUI application
int main(int argc, char *argv[]) {
    // Correct order of arguments for QApplication: argc first, then argv
    QApplication app(argc, argv);

    // Create an instance of our MainWindow
    MainWindow mainWindow;

    // Show the main window
    mainWindow.show();

    // Start the Qt event loop
    return app.exec();
}
