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
    double vignette_strength = 0.0; // New setting for vignette
    double hue_speed = 3.6;
    double hue_intensity = 1.0;
    
    // Global Zoom Settings
    std::string global_zoom_mode = "Oscillating";
    double linear_zoom_speed = 0.5;
    double oscillating_zoom_amplitude = 0.1;
    double oscillating_zoom_frequency = 1.0;
    double oscillating_zoom_midpoint = 1.0;

    // Static method to get a GifSettings object with default values
    static GifSettings getDefaultSettings() {
        GifSettings defaults;
        // Core Settings
        defaults.image_path = "";
        defaults.num_frames = 60; // cycles: 60
        defaults.rotation_direction = "Clockwise"; // spin dir: clockwise
        defaults.rotation_speed = 3.6; // spin speed: 3.6

        // Layering / Tunnel Effect Settings
        defaults.max_layers = 12; // layers: 12
        defaults.scale_decay = 0.92; // warp: 0.92

        // Post-Processing Effects
        defaults.num_stars = 400; // stars: 400
        defaults.advanced_starfield_pattern = "Random"; // star pattern: Random
        defaults.pixelation_level = 0; // pixelation: 0
        defaults.color_invert_frequency = 0; // invert freq: 0
        defaults.wave_amplitude = 3.6; // wave amp: 3.6
        defaults.wave_frequency = 0.10; // wave freq: 0.10
        defaults.wave_direction = "Horizontal"; // wave dir: horizontal
        defaults.blur_radius = 0.0; // haze: 0.0
        defaults.vignette_strength = 1.0; // vignette: 1.0
        defaults.hue_speed = 11.7; // pulse speed: 11.7
        defaults.hue_intensity = 1.50; // pulse intensity: 1.50
        
        // Global Zoom Settings
        defaults.global_zoom_mode = "Oscillating"; // zoom mode: oscillating
        defaults.linear_zoom_speed = 0.5; // (no change to linear_zoom_speed as it's not in the new defaults)
        defaults.oscillating_zoom_amplitude = 0.40; // oscillating strength: 0.40
        defaults.oscillating_zoom_frequency = 2.15; // oscillating speed: 2.15
        defaults.oscillating_zoom_midpoint = 0.82; // zoom midpoint: 0.82

        return defaults;
    }
};

#endif // GIF_SETTINGS_H