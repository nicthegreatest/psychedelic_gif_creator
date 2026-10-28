// image_loader.h
#pragma once
#include <opencv2/core.hpp>
#include <string>

// Loads any image OpenCV (or, as a fallback, Qt) can read and normalises it to
// 8-bit BGRA with straight alpha: 16-bit/float are rescaled, grey/grey+alpha/BGR
// are expanded. Returns an empty Mat and sets *error on failure.
cv::Mat loadImageBGRA(const std::string& path, std::string* error = nullptr);

// Normalises an already decoded image (any depth, 1-4 channels) the same way.
cv::Mat normaliseToBGRA8(const cv::Mat& input);
