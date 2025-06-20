// gif_settings.h
#ifndef GIF_SETTINGS_H
#define GIF_SETTINGS_H

#include <string>
#include <vector>

// This struct holds all the configurable settings for GIF generation.
struct GifSettings {
    std::string image_path = "";
    int num_frames = 20;
    double max_scale = 0.9;
    double scale_decay = 0.8;
    int rotation_speed = 18;
    double hue_speed = 12.8;
    int max_layers = 10;
    double blur_radius = 1.0;
    int num_stars = 100;
    std::string rotation_direction = "Clockwise"; // Default to Clockwise
    std::string advanced_fractal_type = "Sierpinski";
    std::string advanced_starfield_pattern = "Random";

    // --- New Psychedelic Effect Parameters ---
    double global_zoom_speed = 0.0;     // Speed of global zoom effect (e.g., 0.01 for 1% zoom per frame)
    int pixelation_level = 0;          // Level of pixelation (0 for none, higher for more blocky)
    int color_invert_frequency = 0;    // Invert colors every N frames (0 for never)
    double wave_amplitude = 0.0;       // Amplitude of wave distortion
    double wave_frequency = 0.0;       // Frequency of wave distortion
    std::string wave_direction = "None"; // Direction of wave distortion ("None", "Horizontal", "Vertical")
    // Add any other advanced settings here with default values
};

#endif // GIF_SETTINGS_H
