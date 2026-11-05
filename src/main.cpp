// main.cpp - GUI entry point.
#include "mainwindow.h"

#include <QApplication>
#include <QFile>
#include <QIcon>

int main(int argc, char* argv[]) {
    QApplication app(argc, argv);
    QApplication::setOrganizationName("nicthegreatest");
    QApplication::setApplicationName("psychedelic-gif-creator");
    QApplication::setApplicationDisplayName("Psychedelic GIF Creator");
    QApplication::setApplicationVersion(PGC_VERSION);
    QGuiApplication::setDesktopFileName("psychedelic-gif-creator");
    QApplication::setWindowIcon(QIcon(":/psychedelic-gif-creator.png"));

    QFile style(":/style.qss");
    if (style.open(QIODevice::ReadOnly | QIODevice::Text)) app.setStyleSheet(QString::fromUtf8(style.readAll()));

    MainWindow window;
    window.show();
    return app.exec();
}
