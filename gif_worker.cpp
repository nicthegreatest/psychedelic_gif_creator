// gif_worker.cpp
#include "gif_worker.h"
#include <QDebug>
#include <QDir>
#include <QFileInfo>
#include <QColor>

// Include necessary OpenCV headers here
#include <opencv2/opencv.hpp>
#include <cmath>
#include <algorithm>
#include <vector>
#include <string>

// Include the gif.h header from FetchContent.
// The CMake setup ensures this path is available via -I/path/to/gif-h-src
#include "gif.h" // This is correct, as gif.h is now found on the include path.


#ifndef M_PI
#define M_PI 3.14159265358979323846
#endif

GifWorker::GifWorker(const GifSettings& settings, const std::string& output_path)
    : m_settings(settings), m_output_path(output_path)
{
}

void GifWorker::emitProgress(int percentage, const std::string& message) {
    emit progressUpdated(percentage, QString::fromStdString(message));
}

void GifWorker::process() {
    qDebug() << "Worker: Entering process() (GIF generation).";

    if (m_settings.image_path.empty()) {
        emit finished("Error: No input image path provided.");
        return;
    }

    cv::Mat original_image = cv::imread(m_settings.image_path);
    if (original_image.empty()) {
        emit finished(QString("Error: Could not load input image from %1").arg(QString::fromStdString(m_settings.image_path)));
        return;
    }

    if (m_settings.num_frames <= 0) {
        emit finished("Error: Number of frames must be greater than 0.");
        return;
    }

    // Ensure image is in a format compatible with gif.h (RGBA)
    // gif.h expects 8-bit RGBA (R, G, B, A) channel order.
    cv::Mat processed_image_rgba;
    if (original_image.channels() == 3) {
        cv::cvtColor(original_image, processed_image_rgba, cv::COLOR_BGR2RGBA);
    } else if (original_image.channels() == 4) {
        // If it's 4 channels, ensure it's RGBA and not BGRA
        // OpenCV's imread typically loads BGRA for 4-channel PNGs
        cv::cvtColor(original_image, processed_image_rgba, cv::COLOR_BGRA2RGBA);
    } else {
        emit finished("Error: Input image must have 3 or 4 channels (BGR or BGRA).");
        return;
    }

    // --- Setup gif.h GifWriter ---
    int gif_width = processed_image_rgba.cols;
    int gif_height = processed_image_rgba.rows;
    // Delay in 100ths of a second. 10 = 0.1s delay, 10 frames per second.
    // This value is passed per frame in GifWriteFrame, but GifBegin needs a dummy one.
    int frame_delay_cs = 10; // Default: 100ms per frame

    GifWriter writer;
    // GifBegin takes filename (const char*), width, height, delay, bitDepth, dither
    // For bitDepth and dither, we can use defaults (8 and false).
    if (!GifBegin(&writer, m_output_path.c_str(), gif_width, gif_height, frame_delay_cs, 8, false)) {
        emit finished(QString("Error: Failed to open GIF file for writing: %1").arg(QString::fromStdString(m_output_path)));
        return;
    }

    // gif-h doesn't have a direct "setRepeat" function like some other libraries.
    // Looping is usually set in the GifBegin function's internal header writing,
    // or through a Netscape 2.0 extension which GifBegin already handles for infinite loop (delay != 0).
    // So, no direct equivalent to gif_encoder.setRepeat(0); needed here.

    double total_rotation_degrees = static_cast<double>(m_settings.rotation_speed) * 360.0;
    if (m_settings.rotation_direction == "Counter-Clockwise") {
        total_rotation_degrees *= -1.0;
    }
    double angle_per_frame = total_rotation_degrees / m_settings.num_frames;

    // --- Frame Generation Loop ---
    for (int i = 0; i < m_settings.num_frames; ++i) {
        emitProgress(static_cast<int>((static_cast<double>(i + 1) / m_settings.num_frames) * 100),
                     "Generating frame " + std::to_string(i + 1) + " of " + std::to_string(m_settings.num_frames));

        // Start each frame with a fresh copy of the *original image* converted to RGBA
        // This ensures the base image always has the correct format for subsequent processing
        cv::Mat frame = processed_image_rgba.clone();


        // --- Apply Global Zoom Effect ---
        double current_zoom_factor = 1.0;
        if (m_settings.global_zoom_mode == "Linear") {
            current_zoom_factor = 1.0 + (m_settings.global_zoom_speed * i);
        } else if (m_settings.global_zoom_mode == "Oscillating") {
            double phase = 2.0 * M_PI * m_settings.oscillating_zoom_frequency * (static_cast<double>(i) / m_settings.num_frames);
            current_zoom_factor = 1.0 + m_settings.oscillating_zoom_amplitude * std::sin(phase);
            current_zoom_factor = std::max(0.1, std::min(5.0, current_zoom_factor));
            qDebug() << "Worker: Frame " << i << ": Zoom factor =" << current_zoom_factor;
        }

        if (current_zoom_factor != 1.0) {
            cv::resize(frame, frame, cv::Size(), current_zoom_factor, current_zoom_factor, cv::INTER_LINEAR);
            // Need to crop or pad after zoom to keep original dimensions (gif_width, gif_height)
            cv::Rect roi((frame.cols - gif_width) / 2, (frame.rows - gif_height) / 2,
                          gif_width, gif_height);

            if (roi.width > frame.cols || roi.height > frame.rows || roi.x < 0 || roi.y < 0) {
                 // Image became smaller than target size, center it on a black canvas of original_image size
                 cv::Mat centered_frame = cv::Mat::zeros(gif_height, gif_width, frame.type()); // Use frame.type() for channel count
                 int x_offset = (gif_width - frame.cols) / 2;
                 int y_offset = (gif_height - frame.rows) / 2;
                 cv::Rect paste_roi(std::max(0, x_offset), std::max(0, y_offset),
                                    std::min(frame.cols, gif_width - std::max(0, x_offset)),
                                    std::min(frame.rows, gif_height - std::max(0, y_offset)));

                 if(paste_roi.width > 0 && paste_roi.height > 0) {
                     cv::Mat sub_frame = frame(cv::Rect(0, 0, paste_roi.width, paste_roi.height));
                     sub_frame.copyTo(centered_frame(paste_roi));
                 }
                 frame = centered_frame;
            } else {
                 frame = frame(roi); // Crop the center part if zoomed in
            }
        }


        // --- Apply Rotation ---
        double current_rotation_angle = angle_per_frame * i;
        cv::Point2f center((float)frame.cols / 2, (float)frame.rows / 2);
        cv::Mat rot_mat = cv::getRotationMatrix2D(center, current_rotation_angle, 1.0);
        cv::warpAffine(frame, frame, rot_mat, frame.size());

        // --- Apply Hue Shift (HSV needs BGR, then convert back to RGBA) ---
        if (m_settings.hue_speed != 0) {
            cv::Mat bgr_frame;
            cv::cvtColor(frame, bgr_frame, cv::COLOR_RGBA2BGR); // Convert to BGR for HSV
            cv::Mat hsv_frame;
            cv::cvtColor(bgr_frame, hsv_frame, cv::COLOR_BGR2HSV);

            for (int r = 0; r < hsv_frame.rows; ++r) {
                for (int c = 0; c < hsv_frame.cols; ++c) {
                    cv::Vec3b hsv_pixel = hsv_frame.at<cv::Vec3b>(r, c);
                    hsv_pixel[0] = (hsv_pixel[0] + static_cast<uchar>(m_settings.hue_speed)) % 180;
                    hsv_frame.at<cv::Vec3b>(r, c) = hsv_pixel;
                }
            }
            cv::cvtColor(hsv_frame, bgr_frame, cv::COLOR_HSV2BGR); // Convert back to BGR
            cv::cvtColor(bgr_frame, frame, cv::COLOR_BGR2RGBA); // Convert back to RGBA for output
        }

        // --- Apply Other Psychedelic Effects ---
        // Pixelation:
        if (m_settings.pixelation_level > 0) {
            int block_size = m_settings.pixelation_level;
            cv::Mat small_img;
            cv::resize(frame, small_img, cv::Size(frame.cols / block_size, frame.rows / block_size), 0, 0, cv::INTER_LINEAR);
            cv::resize(small_img, frame, frame.size(), 0, 0, cv::INTER_NEAREST);
        }

        // Color Invert Frequency:
        if (m_settings.color_invert_frequency > 0 && (i + 1) % m_settings.color_invert_frequency == 0) {
            cv::bitwise_not(frame, frame);
        }

        // Wave Distortion:
        if (m_settings.wave_amplitude > 0 && m_settings.wave_frequency > 0 && m_settings.wave_direction != "None") {
            cv::Mat map_x(frame.size(), CV_32FC1);
            cv::Mat map_y(frame.size(), CV_32FC1);

            for (int y = 0; y < frame.rows; ++y) {
                for (int x = 0; x < frame.cols; ++x) {
                    float u_x = static_cast<float>(x);
                    float u_y = static_cast<float>(y);

                    if (m_settings.wave_direction == "Horizontal") {
                        u_x = x + m_settings.wave_amplitude * std::sin(y * m_settings.wave_frequency);
                    } else if (m_settings.wave_direction == "Vertical") {
                        u_y = y + m_settings.wave_amplitude * std::sin(x * m_settings.wave_frequency);
                    }
                    map_x.at<float>(y, x) = u_x;
                    map_y.at<float>(y, x) = u_y;
                }
            }
            cv::remap(frame, frame, map_x, map_y, cv::INTER_LINEAR, cv::BORDER_REFLECT_101);
        }

        // --- Add frame to GIF encoder ---
        // Ensure the frame has the correct final dimensions and format (RGBA)
        if (frame.cols != gif_width || frame.rows != gif_height) {
            cv::resize(frame, frame, cv::Size(gif_width, gif_height));
        }
        if (frame.channels() != 4) {
            // As a fallback, try to convert from BGR if possible.
            if (frame.channels() == 3) {
                cv::cvtColor(frame, frame, cv::COLOR_BGR2RGBA);
            } else {
                emit finished(QString("Error: Frame has incorrect channel count (%1) for GIF output. Must be 4 (RGBA).").arg(frame.channels()));
                GifEnd(&writer); // Ensure the file is closed on error
                return;
            }
        }

        // GifWriteFrame expects a pointer to uint8_t data (RGBA interleaved)
        // If your OpenCV matrix is `CV_8UC4`, `frame.data` is exactly what it needs.
        if (!GifWriteFrame(&writer, (uint8_t*)frame.data, gif_width, gif_height, frame_delay_cs)) {
            emit finished(QString("Error: Failed to write frame %1 to GIF file.").arg(i));
            GifEnd(&writer); // Ensure the file is closed on error
            return;
        }
    }

    // --- Finalize GIF Encoding ---
    GifEnd(&writer); // Close the GIF file

    qDebug() << "Worker: Exiting process() successfully.";
    emit finished(QString("GIF saved successfully to: %1").arg(QString::fromStdString(m_output_path)));
}