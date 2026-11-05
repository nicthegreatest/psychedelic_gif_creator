// frame_renderer.h - the single render pipeline used by both export and preview.
#pragma once

#include "gif_settings.h"
#include <opencv2/core.hpp>
#include <vector>

class FrameRenderer {
public:
    // source: 8-bit BGRA from loadImageBGRA(). scale: 1.0 for export, < 1 for a
    // preview; every pixel-based parameter is scaled so the preview matches.
    FrameRenderer(const GifSettings& settings, const cv::Mat& source, double scale = 1.0);

    // Output size for the given settings at the given scale.
    static cv::Size outputSize(const GifSettings& settings, double scale = 1.0);

    // Whole turns actually used when not in Tunnel mode (fractional speeds are
    // rounded so the loop is seamless; any non-zero speed gives at least 1 turn).
    static double effectiveRotationTurns(const GifSettings& settings);

    cv::Size frameSize() const { return m_size; }
    int frameCount() const { return m_settings.num_frames; }

    // Renders frame `index` as 8-bit BGR. Thread-safe (const, no shared mutable
    // state) and deterministic. index == frameCount() renders t = 1.0, which
    // must equal frame 0 for a seamless loop (used by the tests).
    cv::Mat renderFrame(int index) const;

private:
    struct Star { float x, y, phase, brightness; };
    struct Layer { double e; double alpha; };

    void prepareKaleidoscope();
    cv::Mat kaleidoscopeFrame(double t) const;
    std::vector<cv::Mat> buildPyramid(const cv::Mat& base) const;
    void compositeLayers(cv::Mat& frame, const std::vector<cv::Mat>& pyramid, double t) const;
    void drawStars(cv::Mat& frame, double t) const;
    void applyWave(cv::Mat& bgr, double t) const;
    void applyColour(cv::Mat& bgr, double t, int index) const;

    GifSettings m_settings;
    double m_scale;
    cv::Size m_size;
    cv::Mat m_base;                 // premultiplied BGRA, output size
    cv::Rect m_opaqueRect;          // area of m_base that is fully opaque (empty if none)
    std::vector<cv::Mat> m_pyramid; // pyramid of m_base (when the kaleidoscope is static)
    bool m_animatedKaleido = false;
    cv::Mat m_kRadius, m_kAngle;    // polar lookup for the radial kaleidoscope
    cv::Mat m_vignette;             // CV_8UC3 multiplier, empty if off
    cv::Mat m_gridX, m_gridY;       // identity maps for the wave remap
    std::vector<Star> m_stars;
    double m_rotationTurns = 0.0;   // signed turns per loop
};
