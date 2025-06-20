# Minimum version of CMake required
cmake_minimum_required(VERSION 3.10)

# Project name
project(PsychedelicGifGenerator CXX) # CXX indicates it's a C++ project

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

# Add the source files for the GUI application
add_executable(gif_creator_gui
    main.cpp
    mainwindow.cpp
    advancedsettingsdialog.cpp # <--- ADDED: New source file
    gif_generator.cpp
)

# Link the GUI executable with OpenCV libraries
target_link_libraries(gif_creator_gui PRIVATE ${OpenCV_LIBS})

# Link the GUI executable with Qt5 Widgets and Concurrent modules
target_link_libraries(gif_creator_gui PRIVATE Qt5::Widgets Qt5::Concurrent)

# Optional: Set C++ standard (e.g., C++17)
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED True)
set(CMAKE_CXX_EXTENSIONS OFF) # Don't use GNU extensions

# Optional: Set common compile options (e.g., for warnings)
target_compile_options(gif_creator_gui PRIVATE -Wall -Wextra -Wpedantic)

# Optional: Keep the command-line gif_generator for independent testing
# add_executable(gif_generator gif_generator.cpp)
# target_link_libraries(gif_generator PRIVATE ${OpenCV_LIBS})
