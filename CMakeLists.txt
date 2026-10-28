# Minimum version of CMake required
cmake_minimum_required(VERSION 3.10)

# Project name
project(PsychedelicGifGenerator CXX)

# Set the C++ standard early to influence ABI detection
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED TRUE)
set(CMAKE_CXX_EXTENSIONS OFF) # Disable GNU extensions
# Force the NEW C++11 ABI explicitly.
add_compile_definitions(_GLIBCXX_USE_CXX11_ABI=1)

# Find OpenCV package
find_package(OpenCV REQUIRED)

# --- Find Qt5 Widgets and Concurrent modules ---
find_package(Qt5 COMPONENTS Widgets Concurrent REQUIRED)

# --- Enable AUTOMOC, AUTOUIC, AUTORCC ---
set(CMAKE_AUTOMOC ON)
set(CMAKE_AUTOUIC ON)
set(CMAKE_AUTORCC ON)

# --- Add include directory for our custom headers (.h files) ---
include_directories(${CMAKE_SOURCE_DIR})

# --- Fetch gif-h using FetchContent ---
include(FetchContent)
FetchContent_Declare(
    gif_h
    GIT_REPOSITORY https://github.com/ginsweater/gif-h.git
    GIT_TAG        master # Corrected tag for gif-h repository
)
FetchContent_MakeAvailable(gif_h)

# --- Add gif-h as an INTERFACE library ---
# This makes it cleanly linkable and ensures its include path is propagated.
add_library(GifH INTERFACE)
target_include_directories(GifH INTERFACE ${gif_h_SOURCE_DIR})


# Add the source files for the GUI application
add_executable(gif_creator_gui
    main.cpp
    mainwindow.cpp
    advancedsettingsdialog.cpp
    gif_worker.cpp
    # gif_generator.cpp # Commented out as its core logic has moved to gif_worker.cpp
)

# Link the GUI executable with libraries
# IMPORTANT: Link order can matter. Link OpenCV first, then Qt components.
target_link_libraries(gif_creator_gui PRIVATE
    ${OpenCV_LIBS}
    Qt5::Widgets
    Qt5::Concurrent # Keep if any QtConcurrent is used elsewhere, though gif_worker replaces its primary use
    Qt5::Core # Explicitly add Qt5::Core
    GifH # Link the gif-h interface library
)

# Define a preprocessor macro to pass the gif.h include path to source files.
# This is a temporary workaround for persistent 'not declared' errors, to be used
# with a conditional include in gif_worker.cpp.
# This MUST come AFTER add_executable.
target_compile_definitions(gif_creator_gui PRIVATE
    # Stringify the path and convert backslashes to forward slashes for include
    # The double quotes are crucial to pass it as a single string literal.
    GIF_H_INCLUDE_PATH="${gif_h_SOURCE_DIR}/gif.h"
)

# Optional: Set common compile options (e.g., for warnings)
set(CMAKE_CXX_STANDARD 17) # Explicitly set C++17 standard
set(CMAKE_CXX_STANDARD_REQUIRED TRUE)
set(CMAKE_CXX_EXTENSIONS OFF) # Disable GNU extensions (e.g., for better portability)
target_compile_options(gif_creator_gui PRIVATE -Wall -Wextra -Wpedantic) # Enable common warnings
