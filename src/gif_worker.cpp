// gif_worker.cpp
#include "gif_worker.h"
#include "gif.h"
#include <QFile>
#include <opencv2/imgproc.hpp>
#include <opencv2/imgcodecs.hpp>
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

void GifWorker::applyKaleidoscope(cv::Mat& image, int frame_idx) {
    if (m_settings.kaleidoscope_mode == "None") {
        return;
    }

    int w = image.cols;
    int h = image.rows;
    cv::Mat temp_image = cv::Mat::zeros(h, w, CV_8UC4); // Temporary image for transformations

    if (m_settings.kaleidoscope_mode == "Quadrant Mirror") {
        // Quadrant Mirror remains largely static relative to the input image,
        // as radial modes will show the dynamic effect more clearly.

        // Top-left quadrant (original)
        cv::Rect roi_tl(0, 0, w / 2, h / 2);
        image(roi_tl).copyTo(temp_image(roi_tl));

        // Top-right (mirror top-left horizontally)
        cv::Mat flipped_lr;
        cv::flip(image(roi_tl), flipped_lr, 1); // Flip horizontally
        cv::Rect roi_tr(w / 2, 0, w / 2, h / 2);
        flipped_lr.copyTo(temp_image(roi_tr));

        // Bottom-left (mirror top-left vertically)
        cv::Mat flipped_ud;
        cv::flip(image(roi_tl), flipped_ud, 0); // Flip vertically
        cv::Rect roi_bl(0, h / 2, w / 2, h / 2);
        flipped_ud.copyTo(temp_image(roi_bl));

        // Bottom-right (mirror top-left horizontally and vertically, or mirror top-right vertically)
        cv::flip(flipped_lr, flipped_ud, 0); // Flip top-right vertically
        cv::Rect roi_br(w / 2, h / 2, w / 2, h / 2);
        flipped_ud.copyTo(temp_image(roi_br));

        image = temp_image.clone();

    } else if (m_settings.kaleidoscope_mode == "Radial 3-Fold" ||
               m_settings.kaleidoscope_mode == "Radial 4-Fold" ||
               m_settings.kaleidoscope_mode == "Radial 6-Fold") {

        int num_sections = 0;
        if (m_settings.kaleidoscope_mode == "Radial 3-Fold") num_sections = 3;
        else if (m_settings.kaleidoscope_mode == "Radial 4-Fold") num_sections = 4;
        else if (m_settings.kaleidoscope_mode == "Radial 6-Fold") num_sections = 6;

        if (num_sections == 0) return;

        cv::Mat output_image = cv::Mat::zeros(h, w, CV_8UC4);
        double sector_angle_rad = (2.0 * M_PI) / num_sections;
        cv::Point2f center(w / 2.0F, h / 2.0F);

        // Calculate dynamic phase offset
        // kaleidoscope_phase_speed now defines total "rotations" of the kaleidoscope pattern
        // over the entire GIF duration (m_settings.num_frames).
        double total_phase_shift_rad = m_settings.kaleidoscope_phase_speed * (2.0 * M_PI); // Total 2*PI radians per 'speed' unit
        double phase_shift_per_frame_rad = total_phase_shift_rad / m_settings.num_frames;
        double current_frame_phase_offset_rad = phase_shift_per_frame_rad * frame_idx;


        for (int r = 0; r < h; ++r) {
            for (int c = 0; c < w; ++c) {
                cv::Point2f p_orig(c - center.x, r - center.y);
                double angle = std::atan2(p_orig.y, p_orig.x);
                double magnitude = std::sqrt(p_orig.x * p_orig.x + p_orig.y * p_orig.y);

                // Apply dynamic phase offset to the angle before mapping to sector
                angle -= current_frame_phase_offset_rad;

                // Normalize angle to 0 to 2*PI
                // Use fmod for proper wrapping, and add 2*PI to ensure positive
                angle = std::fmod(angle, 2.0 * M_PI);
                if (angle < 0) angle += 2.0 * M_PI;

                int sector_idx = static_cast<int>(angle / sector_angle_rad);
                double mapped_angle = angle - (sector_idx * sector_angle_rad);

                if (num_sections > 0 && (sector_idx % 2 != 0)) { // Apply mirror for odd sectors
                     mapped_angle = sector_angle_rad - mapped_angle;
                }
                
                double new_x = magnitude * std::cos(mapped_angle);
                double new_y = magnitude * std::sin(mapped_angle);

                // Map back to Cartesian coordinates relative to center
                int src_c = static_cast<int>(new_x + center.x);
                int src_r = static_cast<int>(new_y + center.y);

                if (src_c >= 0 && src_c < w && src_r >= 0 && src_r < h) {
                    output_image.at<cv::Vec4b>(r, c) = image.at<cv::Vec4b>(src_r, src_c);
                }
            }
        }
        image = output_image.clone();
    }
}

void GifWorker::process() {
    QString message;
    Result result = Result::Failed;
    try {
        result = render(message);
    } catch (const cv::Exception& e) {
        message = QString("Image processing error: %1").arg(QString::fromStdString(e.what()));
    } catch (const std::exception& e) {
        message = QString("Unexpected error: %1").arg(QString::fromStdString(e.what()));
    } catch (...) {
        message = "Unknown error during GIF generation.";
    }

    if (result != Result::Success && m_fileCreated) {
        // Never leave a half-written GIF behind.
        QFile::remove(QString::fromStdString(m_output_path));
    } else {
        message = QString::fromStdString(m_output_path);
    }
    emit finished(result, message);
}

GifWorker::Result GifWorker::render(QString& message) {

    cv::Mat original_image_bgr = cv::imread(m_settings.image_path, cv::IMREAD_UNCHANGED);
    if (original_image_bgr.empty()) {
        message = "Could not load input image.";
        return Result::Failed;
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
        message = "Failed to open GIF for writing.";
        return Result::Failed;
    }
    m_fileCreated = true;

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
        if (m_isCancelled) break;
        emitProgress((i + 1) * 100 / m_settings.num_frames, "Frame " + std::to_string(i + 1));
        
        cv::Mat current_base_image = original_image_rgba.clone();
        applyKaleidoscope(current_base_image, i);

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
            cv::resize(current_base_image, resized_image, cv::Size(scaled_width, scaled_height), 0, 0, cv::INTER_LANCZOS4);

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
        
        // Apply Vignette effect
        if (m_settings.vignette_strength > 0.0) {
            cv::Mat vignette_mask(height, width, CV_32FC1);
            cv::Point2f center(width / 2.0F, height / 2.0F);
            double max_dist = std::sqrt(center.x * center.x + center.y * center.y);

            for (int r = 0; r < height; ++r) {
                for (int c = 0; c < width; ++c) {
                    double dist = cv::norm(cv::Point2f(c, r) - center);
                    double normalized_dist = dist / max_dist;
                    // Apply a power function to the normalized distance for a smoother falloff
                    double vignette_value = 1.0 - m_settings.vignette_strength * std::pow(normalized_dist, 2.0);
                    vignette_mask.at<float>(r, c) = static_cast<float>(std::max(0.0, std::min(1.0, vignette_value)));
                }
            }

            for (int r = 0; r < height; ++r) {
                for (int c = 0; c < width; ++c) {
                    cv::Vec4b& pixel = frame.at<cv::Vec4b>(r, c);
                    float mask_val = vignette_mask.at<float>(r, c);
                    for (int k = 0; k < 3; ++k) { // Apply to B, G, R channels
                        pixel[k] = static_cast<uchar>(pixel[k] * mask_val);
                    }
                }
            }
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
            cv::Mat zoom_matrix = cv::getRotationMatrix2D(cv::Point2f(width / 2.0F, height / 2.0F), 0.0, global_scale);
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
            GifEnd(&writer);
            message = "Failed to write frame to GIF.";
            return Result::Failed;
        }
    }

    if (!GifEnd(&writer)) {
        message = "Failed to finalise GIF file.";
        return Result::Failed;
    }
    if (m_isCancelled) {
        message = "GIF generation cancelled.";
        return Result::Cancelled;
    }
    return Result::Success;
}