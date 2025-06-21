// gif_settings.h
#ifndef GIF_SETTINGS_H
#define GIF_SETTINGS_H

#include <string>
#include <vector>

// This struct holds all the configurable settings for GIF generation.
struct GifSettings {
    // Core Settings
    std::string image_path = "";
    int num_frames = 60;
    std::string rotation_direction = "Clockwise";
    double rotation_speed = 3.6; 
    
    // Layering / Tunnel Effect Settings
    int max_layers = 10;
    double scale_decay = 0.85;

    // Post-Processing Effects
    int num_stars = 0;
    std::string advanced_starfield_pattern = "None";
    int pixelation_level = 0;
    int color_invert_frequency = 0; 
    double wave_amplitude = 0.0;
    double wave_frequency = 0.0;
    std::string wave_direction = "None";
    double blur_radius = 0.0;
    double hue_speed = 3.6;
    double hue_intensity = 1.0;
    
    // Global Zoom Settings
    std::string global_zoom_mode = "Oscillating";
    double linear_zoom_speed = 0.5;
    double oscillating_zoom_amplitude = 0.1;
    double oscillating_zoom_frequency = 1.0;
    double oscillating_zoom_midpoint = 1.0; // New setting for zoom center

    // Static method to get a GifSettings object with default values
    static GifSettings getDefaultSettings() {
        GifSettings defaults;
        defaults.image_path = "";
        defaults.num_frames = 60;
        defaults.rotation_direction = "Clockwise";
        defaults.rotation_speed = 3.6;

        defaults.max_layers = 10;
        defaults.scale_decay = 0.85;

        defaults.num_stars = 0;
        defaults.advanced_starfield_pattern = "None";
        defaults.pixelation_level = 0;
        defaults.color_invert_frequency = 0;
        defaults.wave_amplitude = 0.0;
        defaults.wave_frequency = 0.0;
        defaults.wave_direction = "None";
        defaults.blur_radius = 0.0;
        defaults.hue_speed = 3.6;
        defaults.hue_intensity = 1.0;
        
        defaults.global_zoom_mode = "Oscillating";
        defaults.linear_zoom_speed = 0.5;
        defaults.oscillating_zoom_amplitude = 0.1;
        defaults.oscillating_zoom_frequency = 1.0;
        defaults.oscillating_zoom_midpoint = 1.0; // Default for new setting

        return defaults;
    }
};

#endif // GIF_SETTINGS_H