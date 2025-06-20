// gif_generator.cpp
#include "gif_generator.h" // Include the declaration for createPsychedelicGif
#include "gif_settings.h"  // Include the definition of GifSettings

#include <iostream>
#include <string>
#include <vector>
#include <cmath>
#include <random>
#include <algorithm> // For std::min, std::max
#include <filesystem> // Required for std::filesystem::exists
#include <cstdio>     // Required for std::remove and std::system

// Include OpenCV headers
#include <opencv2/opencv.hpp>
#include <opencv2/imgcodecs.hpp>
#include <opencv2/highgui.hpp>
#include <opencv2/imgproc.hpp>

// --- Core GIF Generation Logic Implementation ---
std::string createPsychedelicGif(const GifSettings& settings, const std::string& output_gif_path,
                                 std::function<void(int, const std::string&)> progress_callback) {
    try {
        // Check if image file exists
        if (!std::filesystem::exists(settings.image_path)) {
            return "Error: Image file not found at '" + settings.image_path + "'";
        }

        // Load image using OpenCV
        cv::Mat img = cv::imread(settings.image_path, cv::IMREAD_UNCHANGED);
        if (img.empty()) {
            return "Error: Could not open or find the image at '" + settings.image_path + "'";
        }

        // Ensure image has an alpha channel for consistent blending
        if (img.channels() < 4) {
             cv::cvtColor(img, img, cv::COLOR_BGR2BGRA);
        }
        cv::resize(img, img, cv::Size(600, 600), 0, 0, cv::INTER_LANCZOS4);

        int width = img.cols;
        int height = img.rows;
        std::vector<cv::Mat> frames;
        std::vector<std::string> temp_png_paths;

        std::random_device rd;
        std::mt19937 gen(rd());
        std::uniform_int_distribution<> dist_x(0, width - 1);
        std::uniform_int_distribution<> dist_y(0, height - 1);
        std::uniform_real_distribution<> dist_alpha(0.5, 1.0);

        std::filesystem::path output_fs_path(output_gif_path);
        std::filesystem::path temp_dir;
        if (output_fs_path.has_parent_path()) {
            temp_dir = output_fs_path.parent_path();
        } else {
            temp_dir = std::filesystem::current_path();
        }
        std::string base_filename = output_fs_path.stem().string();

        for (int i = 0; i < settings.num_frames; ++i) {
            if (progress_callback) {
                int progress = static_cast<int>((static_cast<double>(i) / settings.num_frames) * 100);
                progress_callback(progress, "Generating frame " + std::to_string(i + 1) + "/" + std::to_string(settings.num_frames) + "...");
            }

            cv::Mat frame = cv::Mat::zeros(height, width, CV_8UC4);

            // --- Draw Starfield ---
            if (settings.advanced_starfield_pattern == "Random") {
                for (int s = 0; s < settings.num_stars; ++s) {
                    int x = dist_x(gen);
                    int y = dist_y(gen);
                    int alpha = static_cast<int>(255 * dist_alpha(gen));
                    cv::Scalar star_color(0, 255, 0, alpha);
                    cv::circle(frame, cv::Point(x, y), 2, star_color, cv::FILLED);
                }
            } else if (settings.advanced_starfield_pattern == "Spiral") {
                for (int j = 0; j < settings.num_stars; ++j) {
                    double angle = 0.1 * j;
                    int x = static_cast<int>(width / 2 + (5 * angle) * std::cos(angle));
                    int y = static_cast<int>(height / 2 + (5 * angle) * std::sin(angle));
                    if (x >= 0 && x < width && y >= 0 && y < height) {
                        int alpha = static_cast<int>(255 * dist_alpha(gen));
                        cv::Scalar star_color(0, 255, 0, alpha);
                        cv::circle(frame, cv::Point(x, y), 2, star_color, cv::FILLED);
                    }
                }
            }

            // --- Apply Global Zoom Effect ---
            // Calculate current zoom factor for this frame
            double current_zoom_factor = 1.0 + (settings.global_zoom_speed * i);
            if (current_zoom_factor < 0.1) current_zoom_factor = 0.1; // Prevent too small

            cv::Mat zoomed_img;
            // Original image is `img`, layers are based on `current_image`
            // Apply zoom to the *entire* canvas before applying layers if desired as a background
            // For now, let's zoom the original source image that gets layered
            
            // To apply zoom to the final frame content after all layers are composited,
            // we'd need to create a temporary frame, draw everything onto it, THEN zoom that.
            // For simplicity, let's incorporate it into the layer scaling
            // (or apply it as a final transform to the `frame` before hue/blur)
            
            // Let's apply zoom as a final step to the composite frame before hue/blur
            // This means we might need an initial copy of the base image that gets transformed.

            // To avoid complex intermediate frame logic, we'll apply zoom to the *layers* themselves,
            // by adjusting their scale based on the global zoom factor for this frame.
            // This requires modifying the `current_scale` within the layer loop.
            // However, a 'global zoom' often implies a transformation of the *entire* scene.
            // For current structure, let's treat it as an overall scale modifier for the layers.

            // To make it a *global* zoom that affects the final frame, we can calculate
            // the source ROI from the original image for the current frame's zoom.
            // This is more complex, as it implies cropping/resampling the original
            // image differently for each frame based on zoom.
            // For now, let's adjust how the layers are composited slightly, or apply as post-processing.
            // A simpler interpretation for 'global_zoom_speed': It affects the effective max_scale.

            double effective_max_scale = settings.max_scale * (1.0 + (settings.global_zoom_speed * i));
            if (effective_max_scale <= 0.1) effective_max_scale = 0.1; // Prevent going too small

            double current_layer_scale = effective_max_scale;
            int x_center = width / 2;
            int y_center = height / 2;
            cv::Mat current_image = img.clone(); // Start with the original image for layers

            for (int layer = 0; layer < settings.max_layers; ++layer) {
                int scaled_width = static_cast<int>(width * current_layer_scale);
                int scaled_height = static_cast<int>(height * current_layer_scale);

                if (scaled_width < 10 || scaled_height < 10) {
                    break;
                }

                cv::Mat resized_image;
                cv::resize(current_image, resized_image, cv::Size(scaled_width, scaled_height), 0, 0, cv::INTER_LANCZOS4);

                double angle_degrees = 0;
                if (settings.rotation_direction == "Clockwise") {
                    angle_degrees = std::fmod((static_cast<double>(i) * settings.rotation_speed), 360.0);
                } else if (settings.rotation_direction == "Counter-Clockwise") {
                    angle_degrees = std::fmod((-static_cast<double>(i) * settings.rotation_speed), 360.0);
                }

                cv::Point2f center(scaled_width / 2.0F, scaled_height / 2.0F);
                cv::Mat rot_mat = cv::getRotationMatrix2D(center, angle_degrees, 1.0);

                cv::Mat rotated_image;
                cv::warpAffine(resized_image, rotated_image, rot_mat, resized_image.size(), cv::INTER_LINEAR, cv::BORDER_CONSTANT, cv::Scalar(0, 0, 0, 0));

                int paste_x = x_center - scaled_width / 2;
                int paste_y = y_center - scaled_height / 2;

                cv::Rect roi(paste_x, paste_y, rotated_image.cols, rotated_image.rows);
                cv::Rect frame_roi(0, 0, frame.cols, frame.rows);
                cv::Rect intersection = roi & frame_roi;

                if (intersection.empty()) continue;

                cv::Mat sub_frame = frame(intersection);
                cv::Mat sub_rotated_image = rotated_image(cv::Rect(intersection.x - roi.x, intersection.y - roi.y, intersection.width, intersection.height));

                for (int r = 0; r < intersection.height; ++r) {
                    for (int c = 0; c < intersection.width; ++c) {
                        cv::Vec4b& frame_pixel = sub_frame.at<cv::Vec4b>(r, c);
                        cv::Vec4b& rotated_pixel = sub_rotated_image.at<cv::Vec4b>(r, c);

                        double alpha_rotated = rotated_pixel[3] / 255.0;
                        double alpha_frame = frame_pixel[3] / 255.0;

                        double new_alpha = alpha_rotated + alpha_frame * (1 - alpha_rotated);

                        if (new_alpha > 0) {
                            for (int k = 0; k < 3; ++k) {
                                frame_pixel[k] = static_cast<uchar>((rotated_pixel[k] * alpha_rotated + frame_pixel[k] * alpha_frame * (1 - alpha_rotated)) / new_alpha);
                            }
                            frame_pixel[3] = static_cast<uchar>(new_alpha * 255);
                        } else {
                            frame_pixel = cv::Vec4b(0, 0, 0, 0);
                        }
                    }
                }

                current_layer_scale *= settings.scale_decay;
                current_image = resized_image;
            }
            
            // --- Pixelation/Mosaic Effect ---
            if (settings.pixelation_level > 0) {
                // Ensure pixelation level is at least 1
                int pixel_size = std::max(1, settings.pixelation_level);
                cv::Mat small_img;
                cv::resize(frame, small_img, cv::Size(width / pixel_size, height / pixel_size), 0, 0, cv::INTER_NEAREST);
                cv::resize(small_img, frame, cv::Size(width, height), 0, 0, cv::INTER_NEAREST);
            }

            // --- Wave Distortion Effect ---
            if (settings.wave_amplitude > 0.0 && settings.wave_frequency > 0.0 && settings.wave_direction != "None") {
                cv::Mat map_x(height, width, CV_32FC1);
                cv::Mat map_y(height, width, CV_32FC1);

                for (int r = 0; r < height; ++r) {
                    for (int c = 0; c < width; ++c) {
                        if (settings.wave_direction == "Horizontal") {
                            map_x.at<float>(r, c) = static_cast<float>(c + settings.wave_amplitude * std::sin(r * settings.wave_frequency + i * 0.1));
                            map_y.at<float>(r, c) = static_cast<float>(r);
                        } else if (settings.wave_direction == "Vertical") {
                            map_x.at<float>(r, c) = static_cast<float>(c);
                            map_y.at<float>(r, c) = static_cast<float>(r + settings.wave_amplitude * std::sin(c * settings.wave_frequency + i * 0.1));
                        }
                    }
                }
                cv::remap(frame, frame, map_x, map_y, cv::INTER_LINEAR, cv::BORDER_REPLICATE);
            }

            // --- Hue Shifting (Convert to HSV, shift hue, convert back) ---
            cv::Mat hsv_frame;
            cv::cvtColor(frame, hsv_frame, cv::COLOR_BGRA2BGR);
            cv::cvtColor(hsv_frame, hsv_frame, cv::COLOR_BGR2HSV);

            for (int r = 0; r < hsv_frame.rows; ++r) {
                for (int c = 0; c < hsv_frame.cols; ++c) {
                    hsv_frame.at<cv::Vec3b>(r, c)[0] = static_cast<uchar>(
                        std::fmod((hsv_frame.at<cv::Vec3b>(r, c)[0] + (i * settings.hue_speed)), 180.0)
                    );
                }
            }
            cv::cvtColor(hsv_frame, hsv_frame, cv::COLOR_HSV2BGR);
            cv::cvtColor(hsv_frame, frame, cv::COLOR_BGR2BGRA);

            // --- Color Inversion Effect ---
            if (settings.color_invert_frequency > 0 && (i % settings.color_invert_frequency == 0)) {
                cv::bitwise_not(frame, frame); // Invert all pixel bits
                // If you only want to invert RGB, not alpha:
                // for (int r = 0; r < frame.rows; ++r) {
                //     for (int c = 0; c < frame.cols; ++c) {
                //         cv::Vec4b& pixel = frame.at<cv::Vec4b>(r, c);
                //         pixel[0] = 255 - pixel[0]; // B
                //         pixel[1] = 255 - pixel[1]; // G
                //         pixel[2] = 255 - pixel[2]; // R
                //     }
                // }
            }

            // --- Gaussian Blur ---
            if (settings.blur_radius > 0) {
                cv::GaussianBlur(frame, frame, cv::Size(0, 0), settings.blur_radius);
            }

            frames.push_back(frame);
        }

        if (progress_callback) {
            progress_callback(100, "Compiling GIF...");
        }

        std::string png_sequence_pattern = temp_dir.string() + "/" + base_filename + "_frame_*.png";
        std::string command = "magick -delay 8 -loop 0 " + png_sequence_pattern + " " + output_gif_path;

        for (size_t i = 0; i < frames.size(); ++i) {
            char temp_filename[256];
            snprintf(temp_filename, sizeof(temp_filename), "%s/%s_frame_%04d.png",
                     temp_dir.string().c_str(), base_filename.c_str(), static_cast<int>(i)); // Cast i to int
            std::string current_png_path = temp_filename;
            cv::imwrite(current_png_path, frames[i]);
            temp_png_paths.push_back(current_png_path);
        }

        int system_result = std::system(command.c_str());

        for (const std::string& path : temp_png_paths) {
            std::filesystem::remove(path);
        }

        if (system_result == 0) {
            return "GIF saved successfully to: " + output_gif_path;
        } else {
            return "Error: ImageMagick GIF compilation failed (code " + std::to_string(system_result) + "). Command: " + command;
        }

    } catch (const cv::Exception& e) {
        return "OpenCV Error: " + std::string(e.what());
    } catch (const std::exception& e) {
        return "Standard Error: " + std::string(e.what());
    } catch (...) {
        return "An unknown error occurred.";
    }
}
