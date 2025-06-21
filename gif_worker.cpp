// gif_worker.cpp
#include "gif_worker.h"
#include "gif.h"
#include <QDebug>
#include <opencv2/opencv.hpp>
#include <cmath>
#include <algorithm>
#include <random>

#ifndef M_PI
#define M_PI 3.14159265358979323846
#endif

GifWorker::GifWorker(const GifSettings& settings, const std::string& output_path)
    : m_settings(settings), m_output_path(output_path), m_isCancelled(false) {}

void GifWorker::emitProgress(int percentage, const std::string& message) {
    emit progressUpdated(percentage, QString::fromStdString(message));
}

void GifWorker::cancel() {
    m_isCancelled = true;
}

void GifWorker::process() {
    qDebug() << "Worker: Implementing 'Layered Collage' with new effects.";

    cv::Mat original_image_bgr = cv::imread(m_settings.image_path, cv::IMREAD_UNCHANGED);
    if (original_image_bgr.empty()) {
        emit finished(false, "Error: Could not load input image.");
        return;
    }

    cv::Mat original_image_rgba;
    if (original_image_bgr.channels() < 4) {
         cv::cvtColor(original_image_bgr, original_image_rgba, cv::COLOR_BGR2BGRA);
    } else {
        original_image_rgba = original_image_bgr.clone();
    }
    
    cv::resize(original_image_rgba, original_image_rgba, cv::Size(600, 600), 0, 0, cv::INTER_LANCZOS4);

    int width = original_image_rgba.cols;
    int height = original_image_rgba.rows;
    int frame_delay_cs = 8;

    GifWriter writer;
    if (!GifBegin(&writer, m_output_path.c_str(), width, height, frame_delay_cs, 8, false)) {
        emit finished(false, "Error: Failed to open GIF for writing.");
        return;
    }
    
    std::random_device rd;
    std::mt19937 gen(rd());
    std::uniform_int_distribution<> dist_x(0, width - 1);
    std::uniform_int_distribution<> dist_y(0, height - 1);
    
    double num_rotations = std::round(m_settings.rotation_speed / 2.0);
    double total_rotation_degrees = num_rotations * 360.0;
    
    if (m_settings.rotation_direction == "Counter-Clockwise") {
        total_rotation_degrees *= -1.0;
    } else if (m_settings.rotation_direction == "None") {
        total_rotation_degrees = 0.0;
    }
    
    double angle_per_frame = (m_settings.num_frames > 0) ? (total_rotation_degrees / m_settings.num_frames) : 0.0;

    for (int i = 0; i < m_settings.num_frames; ++i) {
        if (m_isCancelled) { qDebug() << "Worker: Cancellation requested."; break; }
        emitProgress((i + 1) * 100 / m_settings.num_frames, "Frame " + std::to_string(i + 1));
        
        cv::Mat frame = cv::Mat::zeros(height, width, CV_8UC4);
        double frame_progress = static_cast<double>(i) / m_settings.num_frames;

        if (m_settings.num_stars > 0 && m_settings.advanced_starfield_pattern != "None") {
            if (m_settings.advanced_starfield_pattern == "Random") {
                for (int s = 0; s < m_settings.num_stars; ++s) {
                    cv::circle(frame, cv::Point(dist_x(gen), dist_y(gen)), 1, cv::Scalar(255, 255, 255, 255), cv::FILLED);
                }
            } else if (m_settings.advanced_starfield_pattern == "Spiral") {
                for (int j = 0; j < m_settings.num_stars; ++j) {
                    double angle = (0.1 * j) + (i * 0.05);
                    int x = static_cast<int>(width / 2.0 + (2 * j) * std::cos(angle));
                    int y = static_cast<int>(height / 2.0 + (2 * j) * std::sin(angle));
                    if (x >= 0 && x < width && y >= 0 && y < height) {
                        cv::circle(frame, cv::Point(x, y), 1, cv::Scalar(255, 255, 255, 255), cv::FILLED);
                    }
                }
            }
        }
        
        double effective_max_scale = 1.0;
        double current_layer_scale = effective_max_scale;
        
        for (int layer = 0; layer < m_settings.max_layers; ++layer) {
            int scaled_width = static_cast<int>(width * current_layer_scale);
            int scaled_height = static_cast<int>(height * current_layer_scale);

            if (scaled_width < 2 || scaled_height < 2) break;

            cv::Mat resized_image;
            cv::resize(original_image_rgba, resized_image, cv::Size(scaled_width, scaled_height), 0, 0, cv::INTER_LANCZOS4);

            double angle_degrees = angle_per_frame * i;

            cv::Point2f center(scaled_width / 2.0F, scaled_height / 2.0F);
            cv::Mat rot_mat = cv::getRotationMatrix2D(center, angle_degrees, 1.0);
            cv::Mat rotated_image;
            cv::warpAffine(resized_image, rotated_image, rot_mat, resized_image.size(), cv::INTER_LINEAR, cv::BORDER_CONSTANT, cv::Scalar(0, 0, 0, 0));

            int paste_x = (width / 2) - (scaled_width / 2);
            int paste_y = (height / 2) - (scaled_height / 2);

            cv::Rect roi(paste_x, paste_y, rotated_image.cols, rotated_image.rows);
            cv::Rect frame_roi(0, 0, frame.cols, frame.rows);
            cv::Rect intersection = roi & frame_roi;

            if (intersection.empty()) continue;

            cv::Mat frame_sub_view = frame(intersection);
            cv::Mat rotated_sub_view = rotated_image(cv::Rect(intersection.x - roi.x, intersection.y - roi.y, intersection.width, intersection.height));

            for (int r = 0; r < intersection.height; ++r) {
                for (int c = 0; c < intersection.width; ++c) {
                    cv::Vec4b& frame_pixel = frame_sub_view.at<cv::Vec4b>(r, c);
                    cv::Vec4b& rotated_pixel = rotated_sub_view.at<cv::Vec4b>(r, c);

                    if (rotated_pixel[3] == 0) continue;

                    double alpha_rotated = rotated_pixel[3] / 255.0;
                    double alpha_frame = frame_pixel[3] / 255.0;
                    double new_alpha = alpha_rotated + alpha_frame * (1 - alpha_rotated);

                    if (new_alpha > 0) {
                        for (int k = 0; k < 3; ++k) {
                            frame_pixel[k] = static_cast<uchar>((rotated_pixel[k] * alpha_rotated + frame_pixel[k] * alpha_frame * (1 - alpha_rotated)) / new_alpha);
                        }
                        frame_pixel[3] = static_cast<uchar>(new_alpha * 255);
                    }
                }
            }
            current_layer_scale *= m_settings.scale_decay;
        }
        
        double global_scale = 1.0;
        if (m_settings.global_zoom_mode == "Linear") {
            global_scale = 1.0 + (m_settings.linear_zoom_speed * frame_progress);
        } else if (m_settings.global_zoom_mode == "Oscillating") {
            double sine_wave = sin(frame_progress * 2.0 * M_PI * m_settings.oscillating_zoom_frequency);
            double zoom_center = m_settings.oscillating_zoom_midpoint;
            global_scale = zoom_center + (m_settings.oscillating_zoom_amplitude * sine_wave);
        }

        if (global_scale != 1.0) {
            cv::Mat zoom_matrix = cv::getRotationMatrix2D(cv::Point2f(width / 2.0f, height / 2.0f), 0.0, global_scale);
            // --- THIS IS THE CHANGED LINE ---
            cv::warpAffine(frame, frame, zoom_matrix, frame.size(), cv::INTER_LINEAR, cv::BORDER_REFLECT_101);
        }

        if (m_settings.pixelation_level > 1) {
            cv::Mat small_img;
            cv::resize(frame, small_img, cv::Size(width / m_settings.pixelation_level, height / m_settings.pixelation_level), 0, 0, cv::INTER_NEAREST);
            cv::resize(small_img, frame, cv::Size(width, height), 0, 0, cv::INTER_NEAREST);
        }

        if (m_settings.wave_amplitude > 0.0 && m_settings.wave_frequency > 0.0 && m_settings.wave_direction != "None") {
            cv::Mat map_x(height, width, CV_32FC1), map_y(height, width, CV_32FC1), distorted_frame;
            for (int r = 0; r < height; ++r) {
                for (int c = 0; c < width; ++c) {
                    if (m_settings.wave_direction == "Horizontal") {
                        map_x.at<float>(r, c) = static_cast<float>(c + m_settings.wave_amplitude * std::sin(r * m_settings.wave_frequency + i * 0.1));
                        map_y.at<float>(r, c) = static_cast<float>(r);
                    } else if (m_settings.wave_direction == "Vertical") {
                        map_x.at<float>(r, c) = static_cast<float>(c);
                        map_y.at<float>(r, c) = static_cast<float>(r + m_settings.wave_amplitude * std::sin(c * m_settings.wave_frequency + i * 0.1));
                    } else {
                        map_x.at<float>(r, c) = static_cast<float>(c);
                        map_y.at<float>(r, c) = static_cast<float>(r);
                    }
                }
            }
            cv::remap(frame, distorted_frame, map_x, map_y, cv::INTER_LINEAR, cv::BORDER_REPLICATE);
            frame = distorted_frame;
        }

        if (m_settings.hue_speed > 0 && m_settings.hue_intensity > 0) {
            cv::Mat hsv_frame, temp_bgr;
            cv::cvtColor(frame, temp_bgr, cv::COLOR_BGRA2BGR);
            cv::cvtColor(temp_bgr, hsv_frame, cv::COLOR_BGR2HSV);

            double saturation_pulse = sin(frame_progress * 2.0 * M_PI * (m_settings.hue_speed / 4.0));
            double saturation_multiplier = 1.0 + (saturation_pulse * (m_settings.hue_intensity - 1.0));

            for (int r = 0; r < hsv_frame.rows; ++r) {
                for (int c = 0; c < hsv_frame.cols; ++c) {
                    auto& pixel = hsv_frame.at<cv::Vec3b>(r, c);
                    pixel[0] = static_cast<uchar>(std::fmod((pixel[0] + (i * m_settings.hue_speed)), 180.0));
                    pixel[1] = cv::saturate_cast<uchar>(pixel[1] * saturation_multiplier);
                }
            }
            cv::cvtColor(hsv_frame, temp_bgr, cv::COLOR_HSV2BGR);
            cv::cvtColor(temp_bgr, frame, cv::COLOR_BGR2BGRA);
        }

        if (m_settings.color_invert_frequency > 0 && (i % m_settings.color_invert_frequency == 0)) {
            cv::Mat bgr_frame;
            cv::cvtColor(frame, bgr_frame, cv::COLOR_BGRA2BGR);
            cv::bitwise_not(bgr_frame, bgr_frame);
            cv::cvtColor(bgr_frame, frame, cv::COLOR_BGR2BGRA);
        }

        if (m_settings.blur_radius > 0) {
            cv::GaussianBlur(frame, frame, cv::Size(0, 0), m_settings.blur_radius);
        }
        
        cv::cvtColor(frame, frame, cv::COLOR_BGRA2RGBA);
        if (!GifWriteFrame(&writer, frame.data, width, height, frame_delay_cs)) {
            emit finished(false, "Error: Failed to write frame to GIF.");
            GifEnd(&writer);
            return;
        }
    }

    GifEnd(&writer);
    if (m_isCancelled) {
        emit finished(false, "GIF generation cancelled.");
    } else {
        emit finished(true, QString::fromStdString(m_output_path));
    }
}