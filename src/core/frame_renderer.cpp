// frame_renderer.cpp
#include "frame_renderer.h"

#include <opencv2/imgproc.hpp>
#include <algorithm>
#include <cmath>
#include <random>

namespace {
constexpr double kPi = 3.14159265358979323846;
constexpr double kTwoPi = 2.0 * kPi;

void premultiply(cv::Mat& bgra) {
    for (int r = 0; r < bgra.rows; ++r) {
        auto* p = bgra.ptr<cv::Vec4b>(r);
        for (int c = 0; c < bgra.cols; ++c) {
            const int a = p[c][3];
            if (a == 255) continue;
            for (int k = 0; k < 3; ++k) p[c][k] = static_cast<uchar>((p[c][k] * a + 127) / 255);
        }
    }
}

// dst = src * alpha OVER dst, both premultiplied BGRA of equal size.
void blendOver(cv::Mat dst, const cv::Mat& src, int alpha8) {
    for (int r = 0; r < dst.rows; ++r) {
        auto* d = dst.ptr<cv::Vec4b>(r);
        const auto* s = src.ptr<cv::Vec4b>(r);
        for (int c = 0; c < dst.cols; ++c) {
            int sa = s[c][3];
            if (sa == 0) continue;
            if (alpha8 == 255) {
                if (sa == 255) { d[c] = s[c]; continue; }
                const int inv = 255 - sa;
                for (int k = 0; k < 4; ++k) d[c][k] = static_cast<uchar>(s[c][k] + (d[c][k] * inv + 127) / 255);
            } else {
                int sv[4];
                for (int k = 0; k < 4; ++k) sv[k] = (s[c][k] * alpha8 + 127) / 255;
                const int inv = 255 - sv[3];
                for (int k = 0; k < 4; ++k) d[c][k] = static_cast<uchar>(sv[k] + (d[c][k] * inv + 127) / 255);
            }
        }
    }
}

int pixelsFor(double value, double scale) { return std::max(1, static_cast<int>(std::lround(value * scale))); }
} // namespace

cv::Size FrameRenderer::outputSize(const GifSettings& s, double scale) {
    return {std::max(2, static_cast<int>(std::lround(s.output_width * scale))),
            std::max(2, static_cast<int>(std::lround(s.output_height * scale)))};
}

double FrameRenderer::effectiveRotationTurns(const GifSettings& s) {
    if (s.rotation_direction == RotationDirection::None || s.rotation_turns <= 0.0) return 0.0;
    if (s.zoom_mode == ZoomMode::Tunnel) return s.rotation_turns; // exact, see compositeLayers()
    return std::max(1.0, std::round(s.rotation_turns));
}

FrameRenderer::FrameRenderer(const GifSettings& settings, const cv::Mat& source, double scale)
    : m_settings(settings), m_scale(scale > 0 ? scale : 1.0) {
    CV_Assert(!source.empty() && source.type() == CV_8UC4);
    m_settings.num_frames = std::max(1, m_settings.num_frames);
    m_size = outputSize(m_settings, m_scale);
    const int W = m_size.width, H = m_size.height;

    // --- Fit the (premultiplied) source into the output frame ---------------
    cv::Mat src = source.clone();
    premultiply(src);
    const double sx = double(W) / src.cols, sy = double(H) / src.rows;
    auto resized = [&](double fx, double fy) {
        cv::Size sz(std::max(1, int(std::lround(src.cols * fx))), std::max(1, int(std::lround(src.rows * fy))));
        cv::Mat out;
        cv::resize(src, out, sz, 0, 0, (fx < 1.0 && fy < 1.0) ? cv::INTER_AREA : cv::INTER_CUBIC);
        return out;
    };
    cv::Rect content(0, 0, W, H);
    switch (m_settings.fit_mode) {
    case FitMode::Stretch:
        m_base = resized(sx, sy);
        break;
    case FitMode::Fill: {
        const double f = std::max(sx, sy);
        cv::Mat big = resized(f, f);
        const int ox = std::max(0, (big.cols - W) / 2), oy = std::max(0, (big.rows - H) / 2);
        m_base = cv::Mat(H, W, CV_8UC4, cv::Scalar::all(0));
        cv::Rect roi(ox, oy, std::min(W, big.cols), std::min(H, big.rows));
        big(roi).copyTo(m_base(cv::Rect(0, 0, roi.width, roi.height)));
        break;
    }
    case FitMode::Fit: {
        const double f = std::min(sx, sy);
        cv::Mat small = resized(f, f);
        m_base = cv::Mat(H, W, CV_8UC4, cv::Scalar::all(0));
        content = cv::Rect((W - small.cols) / 2, (H - small.rows) / 2, std::min(W, small.cols), std::min(H, small.rows));
        small(cv::Rect(0, 0, content.width, content.height)).copyTo(m_base(content));
        break;
    }
    }
    if (m_base.size() != m_size) cv::resize(m_base, m_base, m_size); // rounding safety

    // Opaque area is used to skip layers hidden behind a fully covering one.
    std::vector<cv::Mat> ch;
    cv::split(m_base(content), ch);
    double minA = 0;
    cv::minMaxLoc(ch[3], &minA);
    m_opaqueRect = (minA >= 255.0) ? content : cv::Rect();

    // --- Kaleidoscope -------------------------------------------------------
    prepareKaleidoscope();
    if (!m_animatedKaleido) m_pyramid = buildPyramid(m_base);

    // --- Vignette mask (constant) -------------------------------------------
    if (m_settings.vignette_strength > 0.0) {
        cv::Mat mask(H, W, CV_32F);
        const double cx = W / 2.0, cy = H / 2.0, maxD2 = cx * cx + cy * cy;
        for (int r = 0; r < H; ++r) {
            float* m = mask.ptr<float>(r);
            for (int c = 0; c < W; ++c) {
                const double d2 = ((c - cx) * (c - cx) + (r - cy) * (r - cy)) / maxD2;
                m[c] = static_cast<float>(std::clamp(1.0 - m_settings.vignette_strength * d2, 0.0, 1.0) * 255.0);
            }
        }
        cv::Mat m8;
        mask.convertTo(m8, CV_8U);
        cv::Mat three[3] = {m8, m8, m8};
        cv::merge(three, 3, m_vignette);
    }

    // --- Wave base grids ----------------------------------------------------
    if (m_settings.wave_direction != WaveDirection::None && m_settings.wave_amplitude > 0.0 &&
        m_settings.wave_frequency > 0.0) {
        m_gridX.create(H, W, CV_32F);
        m_gridY.create(H, W, CV_32F);
        for (int r = 0; r < H; ++r) {
            float* x = m_gridX.ptr<float>(r);
            float* y = m_gridY.ptr<float>(r);
            for (int c = 0; c < W; ++c) { x[c] = float(c); y[c] = float(r); }
        }
    }

    // --- Stars: generated once per render from the seed (deterministic) -----
    if (m_settings.star_pattern != StarPattern::None && m_settings.num_stars > 0) {
        std::mt19937 gen(static_cast<std::mt19937::result_type>(m_settings.star_seed));
        std::uniform_real_distribution<float> u(0.0f, 1.0f), b(0.5f, 1.0f);
        m_stars.reserve(m_settings.num_stars);
        for (int i = 0; i < m_settings.num_stars; ++i) {
            const float x = u(gen), y = u(gen), ph = u(gen), br = b(gen);
            m_stars.push_back({x, y, ph, br});
        }
    }

    m_rotationTurns = effectiveRotationTurns(m_settings);
    if (m_settings.rotation_direction == RotationDirection::CounterClockwise) m_rotationTurns = -m_rotationTurns;
}

void FrameRenderer::prepareKaleidoscope() {
    const auto mode = m_settings.kaleidoscope_mode;
    if (mode == KaleidoscopeMode::None) return;
    const int W = m_size.width, H = m_size.height;

    if (mode == KaleidoscopeMode::QuadrantMirror) {
        // Mirror the top-left quadrant into the other three (static).
        cv::Mat mx(H, W, CV_32F), my(H, W, CV_32F);
        for (int r = 0; r < H; ++r)
            for (int c = 0; c < W; ++c) {
                mx.at<float>(r, c) = float(std::min(c, W - 1 - c));
                my.at<float>(r, c) = float(std::min(r, H - 1 - r));
            }
        cv::Mat out;
        cv::remap(m_base, out, mx, my, cv::INTER_NEAREST, cv::BORDER_REFLECT_101);
        m_base = out;
        return;
    }

    const int n = mode == KaleidoscopeMode::Radial3 ? 3 : mode == KaleidoscopeMode::Radial4 ? 4 : 6;
    const double sector = kTwoPi / n;
    const double cx = (W - 1) / 2.0, cy = (H - 1) / 2.0;
    m_kRadius.create(H, W, CV_32F);
    m_kAngle.create(H, W, CV_32F);
    for (int r = 0; r < H; ++r) {
        float* rad = m_kRadius.ptr<float>(r);
        float* ang = m_kAngle.ptr<float>(r);
        for (int c = 0; c < W; ++c) {
            const double dx = c - cx, dy = r - cy;
            double a = std::atan2(dy, dx) + kPi / 2.0; // wedge points up
            a = std::fmod(a, sector);
            if (a < 0) a += sector;
            if (a > sector / 2.0) a = sector - a;      // mirror half -> no seams for any n
            rad[c] = float(std::sqrt(dx * dx + dy * dy));
            ang[c] = float(a - kPi / 2.0 - sector / 4.0);
        }
    }
    m_animatedKaleido = m_settings.kaleidoscope_turns != 0;
    if (!m_animatedKaleido) m_base = kaleidoscopeFrame(0.0);
}

cv::Mat FrameRenderer::kaleidoscopeFrame(double t) const {
    // Rotating the source under the mirrors; whole turns per loop => seamless.
    const double phase = kTwoPi * m_settings.kaleidoscope_turns * t;
    cv::Mat ang = m_kAngle + cv::Scalar(phase);
    cv::Mat mx, my;
    cv::polarToCart(m_kRadius, ang, mx, my);
    mx += cv::Scalar((m_size.width - 1) / 2.0);
    my += cv::Scalar((m_size.height - 1) / 2.0);
    cv::Mat out;
    cv::remap(m_base, out, mx, my, cv::INTER_LINEAR, cv::BORDER_REFLECT_101);
    return out;
}

std::vector<cv::Mat> FrameRenderer::buildPyramid(const cv::Mat& base) const {
    std::vector<cv::Mat> pyr{base};
    while (pyr.back().cols >= 32 && pyr.back().rows >= 32 && pyr.size() < 8) {
        cv::Mat next;
        cv::pyrDown(pyr.back(), next);
        pyr.push_back(next);
    }
    return pyr;
}

void FrameRenderer::compositeLayers(cv::Mat& frame, const std::vector<cv::Mat>& pyr, double t) const {
    const int L = std::max(1, m_settings.max_layers);
    const double d = std::clamp(m_settings.scale_decay, 0.05, 0.995);
    const cv::Point2d fc((m_size.width - 1) / 2.0, (m_size.height - 1) / 2.0);
    const bool tunnel = m_settings.zoom_mode == ZoomMode::Tunnel;

    // Each layer is described by its depth e (scale = d^e). In Tunnel mode the
    // layers flow outwards by `steps` layers per loop; because layer k at t=1
    // is exactly layer k-steps at t=0, the loop is seamless. Rotation is a
    // function of depth only (twist), which makes fractional turns seamless too.
    std::vector<Layer> layers;
    if (tunnel) {
        const int steps = std::max(1, m_settings.tunnel_steps);
        const double shift = steps * t;
        const double maxScale = 3.0; // stop adding outer layers past this size
        const double eMin = std::log(maxScale) / std::log(d); // negative
        for (int k = static_cast<int>(std::floor(eMin + shift)) - 1;; ++k) {
            const double e = k - shift;
            if (e >= L) break;
            if (e < eMin) continue;
            double a = 1.0;
            if (e > L - 1) a = L - e;                      // fade in at the centre
            if (e < eMin + 1.0) a = std::min(a, e - eMin); // fade out when huge
            layers.push_back({e, std::clamp(a, 0.0, 1.0)});
        }
    } else {
        for (int k = 0; k < L; ++k) layers.push_back({double(k), 1.0});
    }

    auto layerAngle = [&](double e) {
        if (tunnel) {
            const int steps = std::max(1, m_settings.tunnel_steps);
            return -360.0 * m_rotationTurns * e / steps;
        }
        return 360.0 * m_rotationTurns * t;
    };
    // getRotationMatrix2D: positive = counter-clockwise in image coordinates.
    // Maps pyramid level `level` into the frame: scale (exact per axis, since
    // pyrDown rounds sizes), rotate clockwise by layerAngle, centre on frame.
    auto layerMatrix = [&](const Layer& l, int level) {
        const cv::Mat& img = pyr[level];
        const double scale = std::pow(d, l.e);
        const double sx = scale * pyr[0].cols / img.cols, sy = scale * pyr[0].rows / img.rows;
        const double th = -layerAngle(l.e) * kPi / 180.0; // OpenCV: positive = counter-clockwise
        const double co = std::cos(th), si = std::sin(th);
        const double cx = (img.cols - 1) / 2.0, cy = (img.rows - 1) / 2.0;
        cv::Mat M = (cv::Mat_<double>(2, 3) << co * sx, si * sy, 0.0, -si * sx, co * sy, 0.0);
        M.at<double>(0, 2) = fc.x - (M.at<double>(0, 0) * cx + M.at<double>(0, 1) * cy);
        M.at<double>(1, 2) = fc.y - (M.at<double>(1, 0) * cx + M.at<double>(1, 1) * cy);
        return M;
    };

    // Find the smallest fully-opaque layer covering the whole frame: nothing
    // drawn before it can be visible, so start there.
    size_t first = 0;
    if (!m_opaqueRect.empty()) {
        for (size_t i = 0; i < layers.size(); ++i) {
            if (layers[i].alpha < 1.0) continue;
            cv::Mat M = layerMatrix(layers[i], 0), Minv;
            cv::invertAffineTransform(M, Minv);
            bool covers = true;
            const cv::Point2d corners[4] = {{0, 0}, {double(m_size.width), 0},
                                            {0, double(m_size.height)}, {double(m_size.width), double(m_size.height)}};
            for (const auto& p : corners) {
                const double x = Minv.at<double>(0, 0) * p.x + Minv.at<double>(0, 1) * p.y + Minv.at<double>(0, 2);
                const double y = Minv.at<double>(1, 0) * p.x + Minv.at<double>(1, 1) * p.y + Minv.at<double>(1, 2);
                if (x < m_opaqueRect.x || y < m_opaqueRect.y || x > m_opaqueRect.br().x - 1 || y > m_opaqueRect.br().y - 1) {
                    covers = false;
                    break;
                }
            }
            if (covers) first = i;
        }
    }

    const cv::Rect frameRect(0, 0, m_size.width, m_size.height);
    for (size_t i = first; i < layers.size(); ++i) {
        const Layer& l = layers[i];
        if (l.alpha <= 0.0) continue;
        const double scale = std::pow(d, l.e);
        int level = 0;
        while (level + 1 < int(pyr.size()) && scale * (1 << (level + 1)) <= 1.0) ++level;
        const cv::Mat& img = pyr[level];
        cv::Mat M = layerMatrix(l, level);

        // Bounding box of the transformed layer, clipped to the frame.
        std::vector<cv::Point2f> pts{{0, 0}, {float(img.cols), 0}, {0, float(img.rows)}, {float(img.cols), float(img.rows)}}, out;
        cv::transform(pts, out, M);
        cv::Rect roi = cv::boundingRect(out) & frameRect;
        if (roi.width < 1 || roi.height < 1) continue;
        M.at<double>(0, 2) -= roi.x;
        M.at<double>(1, 2) -= roi.y;
        cv::Mat warped;
        cv::warpAffine(img, warped, M, roi.size(), cv::INTER_LINEAR, cv::BORDER_CONSTANT, cv::Scalar::all(0));
        blendOver(frame(roi), warped, static_cast<int>(std::lround(l.alpha * 255.0)));
    }
}

void FrameRenderer::drawStars(cv::Mat& frame, double t) const {
    if (m_stars.empty()) return;
    const int W = m_size.width, H = m_size.height;
    const double unit = std::min(W, H) / 600.0; // star sizes are tuned for 600 px
    const int radius = std::max(1, static_cast<int>(std::lround(1.2 * unit)));
    cv::Mat stars(frame.size(), CV_8UC4, cv::Scalar::all(0));
    if (m_settings.star_pattern == StarPattern::Random) {
        for (const Star& s : m_stars) {
            // Two twinkles per loop (whole cycles => seamless).
            const double v = 255.0 * s.brightness * (0.55 + 0.45 * std::sin(kTwoPi * (2.0 * t + s.phase)));
            cv::circle(stars, cv::Point(int(s.x * W), int(s.y * H)), radius, cv::Scalar(v, v, v, 255), cv::FILLED, cv::LINE_AA);
        }
    } else { // Spiral: one revolution per loop
        const int n = int(m_stars.size());
        for (int j = 0; j < n; ++j) {
            const double angle = 0.1 * j + kTwoPi * t;
            const double r = 2.0 * j * unit;
            const int x = int(W / 2.0 + r * std::cos(angle)), y = int(H / 2.0 + r * std::sin(angle));
            if (x < 0 || x >= W || y < 0 || y >= H) continue;
            const double v = 255.0 * (0.6 + 0.4 * std::sin(kTwoPi * (t + m_stars[j].phase)));
            cv::circle(stars, cv::Point(x, y), radius, cv::Scalar(v, v, v, 255), cv::FILLED, cv::LINE_AA);
        }
    }
    cv::max(frame, stars, frame); // "lighten" so stars show on top of the tunnel
}

void FrameRenderer::applyWave(cv::Mat& bgr, double t) const {
    if (m_gridX.empty()) return;
    const double amp = m_settings.wave_amplitude * m_scale;
    const double freq = m_settings.wave_frequency / m_scale;
    const double phase = kTwoPi * m_settings.wave_cycles * t;
    const bool horizontal = m_settings.wave_direction == WaveDirection::Horizontal;
    cv::Mat mx = m_gridX, my = m_gridY;
    if (horizontal) {
        mx = m_gridX.clone();
        for (int r = 0; r < mx.rows; ++r) {
            const float off = float(amp * std::sin(r * freq + phase));
            float* p = mx.ptr<float>(r);
            for (int c = 0; c < mx.cols; ++c) p[c] += off;
        }
    } else {
        std::vector<float> off(my.cols);
        for (int c = 0; c < my.cols; ++c) off[c] = float(amp * std::sin(c * freq + phase));
        my = m_gridY.clone();
        for (int r = 0; r < my.rows; ++r) {
            float* p = my.ptr<float>(r);
            for (int c = 0; c < my.cols; ++c) p[c] += off[c];
        }
    }
    cv::Mat out;
    cv::remap(bgr, out, mx, my, cv::INTER_LINEAR, cv::BORDER_REPLICATE);
    bgr = out;
}

void FrameRenderer::applyColour(cv::Mat& bgr, double t, int index) const {
    const int hueShift = static_cast<int>(std::lround(180.0 * m_settings.hue_cycles * t)) % 180;
    const double satMul = 1.0 + m_settings.saturation_pulse * std::sin(kTwoPi * m_settings.saturation_pulses * t);
    if (hueShift != 0 || std::abs(satMul - 1.0) > 1e-9) {
        cv::Mat lut(1, 256, CV_8UC3);
        for (int v = 0; v < 256; ++v) {
            const int h = v < 180 ? (v + hueShift + 180) % 180 : v;
            lut.at<cv::Vec3b>(0, v) = cv::Vec3b(cv::saturate_cast<uchar>(h), cv::saturate_cast<uchar>(v * satMul), uchar(v));
        }
        cv::Mat hsv;
        cv::cvtColor(bgr, hsv, cv::COLOR_BGR2HSV);
        cv::LUT(hsv, lut, hsv);
        cv::cvtColor(hsv, bgr, cv::COLOR_HSV2BGR);
    }

    const int n = m_settings.num_frames, k = m_settings.invert_flashes;
    if (k > 0) {
        const int i = ((index % n) + n) % n;
        if ((static_cast<long long>(i) * k) % n < k) cv::bitwise_not(bgr, bgr); // k evenly spaced frames
    }
}

cv::Mat FrameRenderer::renderFrame(int index) const {
    const double t = double(index) / m_settings.num_frames;

    std::vector<cv::Mat> animatedPyr;
    if (m_animatedKaleido) animatedPyr = buildPyramid(kaleidoscopeFrame(t));
    const std::vector<cv::Mat>& pyr = m_animatedKaleido ? animatedPyr : m_pyramid;

    cv::Mat frame(m_size, CV_8UC4, cv::Scalar::all(0));
    compositeLayers(frame, pyr, t);
    drawStars(frame, t);

    cv::Mat bgr; // premultiplied => this is "composited over black"
    cv::cvtColor(frame, bgr, cv::COLOR_BGRA2BGR);
    if (!m_vignette.empty()) cv::multiply(bgr, m_vignette, bgr, 1.0 / 255.0);

    if (m_settings.zoom_mode == ZoomMode::Oscillating) {
        const double g = m_settings.zoom_midpoint + m_settings.zoom_amplitude * std::sin(kTwoPi * m_settings.zoom_cycles * t);
        if (std::abs(g - 1.0) > 1e-9 && g > 0.0) {
            const cv::Point2f c((m_size.width - 1) / 2.0f, (m_size.height - 1) / 2.0f);
            cv::Mat out;
            cv::warpAffine(bgr, out, cv::getRotationMatrix2D(c, 0.0, g), bgr.size(), cv::INTER_LINEAR, cv::BORDER_REFLECT_101);
            bgr = out;
        }
    }

    if (m_settings.pixelation_level > 1) {
        const int block = pixelsFor(m_settings.pixelation_level, m_scale);
        if (block > 1) {
            cv::Mat small;
            cv::resize(bgr, small, cv::Size(std::max(1, bgr.cols / block), std::max(1, bgr.rows / block)), 0, 0, cv::INTER_NEAREST);
            cv::resize(small, bgr, m_size, 0, 0, cv::INTER_NEAREST);
        }
    }

    applyWave(bgr, t);
    applyColour(bgr, t, index);

    if (m_settings.blur_radius > 0.0) {
        const double sigma = m_settings.blur_radius * m_scale;
        if (sigma > 0.05) cv::GaussianBlur(bgr, bgr, cv::Size(0, 0), sigma);
    }
    return bgr;
}
