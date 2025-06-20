// gif_settings.h
#ifndef GIF_SETTINGS_H
#define GIF_SETTINGS_H

#include <string>
#include <vector>

// This struct holds all the configurable settings for GIF generation.
struct GifSettings {
    std::string image_path = "";
    int num_frames = 20; // Initializer default (can be different from getDefaultSettings)
    double max_scale = 0.9;
    double scale_decay = 0.8;
    int rotation_speed = 18;
    double hue_speed = 12.8;
    int max_layers = 10;
    double blur_radius = 1.0;
    int num_stars = 100;
    std::string rotation_direction = "Clockwise";
    std::string advanced_fractal_type = "Sierpinski";
    std::string advanced_starfield_pattern = "Random";

    // --- New Psychedelic Effect Parameters ---
    double global_zoom_speed = 0.0;
    int pixelation_level = 0;
    int color_invert_frequency = 0;
    double wave_amplitude = 0.0;
    double wave_frequency = 0.0;
    std::string wave_direction = "None";

    // Static method to get a GifSettings object with your desired app-wide default values
    static GifSettings getDefaultSettings() {
        GifSettings defaults;
        defaults.image_path = ""; // Always start with no image selected
        defaults.num_frames = 60;
        defaults.max_scale = 0.9;
        defaults.scale_decay = 0.8;
        defaults.rotation_speed = 6;
        defaults.hue_speed = 3.6;
        defaults.rotation_direction = "Clockwise";

        defaults.max_layers = 5;
        defaults.blur_radius = 0.2;
        defaults.num_stars = 110;
        defaults.global_zoom_speed = 0.100;
        defaults.pixelation_level = 0;
        defaults.color_invert_frequency = 18;
        defaults.wave_amplitude = 1.1;
        defaults.wave_frequency = 0.33;
        defaults.wave_direction = "None";
        defaults.advanced_fractal_type = "Sierpinski";
        defaults.advanced_starfield_pattern = "Random";
        return defaults;
    }
};

#endif // GIF_SETTINGS_H