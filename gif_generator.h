// gif_generator.h
#ifndef GIF_GENERATOR_H
#define GIF_GENERATOR_H

#include <string>
#include <functional> // Required for std::function

#include "gif_settings.h" // Include the settings struct definition

// Declare the core C++ GIF generation function.
// It takes a GifSettings object, output path, and an optional progress callback.
extern std::string createPsychedelicGif(const GifSettings& settings, const std::string& output_gif_path,
                                        std::function<void(int, const std::string&)> progress_callback = nullptr);

#endif // GIF_GENERATOR_H
