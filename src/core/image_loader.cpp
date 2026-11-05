// image_loader.cpp
#include "image_loader.h"

#include <QFileInfo>
#include <QImage>
#include <QString>
#include <opencv2/imgcodecs.hpp>
#include <opencv2/imgproc.hpp>
#include <stdexcept>

cv::Mat normaliseToBGRA8(const cv::Mat& input) {
    if (input.empty()) return {};

    cv::Mat img8;
    switch (input.depth()) {
    case CV_8U:
        img8 = input;
        break;
    case CV_16U:
        input.convertTo(img8, CV_8U, 1.0 / 257.0);
        break;
    case CV_32F:
    case CV_64F: {
        double lo = 0, hi = 0;
        cv::minMaxLoc(input.reshape(1), &lo, &hi);
        if (lo >= 0.0 && hi <= 1.0) {
            input.convertTo(img8, CV_8U, 255.0);
        } else {
            const double range = (hi - lo) > 0 ? (hi - lo) : 1.0;
            input.convertTo(img8, CV_8U, 255.0 / range, -lo * 255.0 / range);
        }
        break;
    }
    default: { // signed integer types: stretch to 0..255
        double lo = 0, hi = 0;
        cv::minMaxLoc(input.reshape(1), &lo, &hi);
        const double range = (hi - lo) > 0 ? (hi - lo) : 1.0;
        input.convertTo(img8, CV_8U, 255.0 / range, -lo * 255.0 / range);
        break;
    }
    }

    cv::Mat out;
    switch (img8.channels()) {
    case 1:
        cv::cvtColor(img8, out, cv::COLOR_GRAY2BGRA);
        break;
    case 2: {
        cv::Mat ch[2];
        cv::split(img8, ch);
        cv::Mat four[4] = {ch[0], ch[0], ch[0], ch[1]};
        cv::merge(four, 4, out);
        break;
    }
    case 3:
        cv::cvtColor(img8, out, cv::COLOR_BGR2BGRA);
        break;
    case 4:
        out = img8.clone();
        break;
    default:
        throw std::runtime_error("Unsupported number of image channels: " + std::to_string(img8.channels()));
    }
    return out;
}

static cv::Mat loadWithQt(const std::string& path) {
    QImage img(QString::fromStdString(path));
    if (img.isNull()) return {};
    img = img.convertToFormat(QImage::Format_RGBA8888);
    cv::Mat rgba(img.height(), img.width(), CV_8UC4, const_cast<uchar*>(img.constBits()),
                 static_cast<size_t>(img.bytesPerLine()));
    cv::Mat bgra;
    cv::cvtColor(rgba, bgra, cv::COLOR_RGBA2BGRA); // deep copy
    return bgra;
}

cv::Mat loadImageBGRA(const std::string& path, std::string* error) {
    if (path.empty()) {
        if (error) *error = "No input image selected.";
        return {};
    }
    const QFileInfo info(QString::fromStdString(path));
    if (!info.isFile() || !info.isReadable()) {
        if (error) *error = "Cannot open \"" + path + "\": the file does not exist or is not readable.";
        return {};
    }
    cv::Mat raw;
    try {
        // UNCHANGED keeps 16-bit depth and alpha but ignores EXIF orientation,
        // so re-read images without alpha in a mode that applies it.
        raw = cv::imread(path, cv::IMREAD_UNCHANGED);
        if (!raw.empty() && raw.channels() < 4) {
            cv::Mat oriented = cv::imread(path, cv::IMREAD_ANYDEPTH | cv::IMREAD_ANYCOLOR);
            if (!oriented.empty()) raw = oriented;
        }
    } catch (const cv::Exception&) {
        raw.release();
    }
    if (raw.empty()) raw = loadWithQt(path); // e.g. formats this OpenCV build lacks
    if (raw.empty()) {
        if (error) *error = "Could not read \"" + path + "\" (missing file or unsupported format).";
        return {};
    }
    try {
        return normaliseToBGRA8(raw);
    } catch (const std::exception& e) {
        if (error) *error = e.what();
        return {};
    }
}
