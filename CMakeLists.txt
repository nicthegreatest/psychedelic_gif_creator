cmake_minimum_required(VERSION 3.21)

project(PsychedelicGifCreator
    VERSION 0.2.0
    DESCRIPTION "Turn a still image into a looping psychedelic GIF"
    LANGUAGES CXX)

# Default to an optimised build when no build type is given.
if(NOT CMAKE_BUILD_TYPE AND NOT CMAKE_CONFIGURATION_TYPES)
    set(CMAKE_BUILD_TYPE Release CACHE STRING "Build type" FORCE)
endif()

if(CMAKE_SOURCE_DIR STREQUAL CMAKE_BINARY_DIR)
    message(FATAL_ERROR "In-source builds are not supported. Use: cmake -B build")
endif()

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

find_package(Qt6 6.2 REQUIRED COMPONENTS Widgets)
# Only the modules the app actually uses. Linking ${OpenCV_LIBS} (all modules)
# pulled in highgui/cvv, which on Fedora drag a second Qt into the process.
find_package(OpenCV 4 REQUIRED COMPONENTS core imgproc imgcodecs)

qt_standard_project_setup()

add_library(gif_h INTERFACE)
target_include_directories(gif_h SYSTEM INTERFACE ${CMAKE_CURRENT_SOURCE_DIR}/third_party/gif-h)

qt_add_executable(gif_creator_gui
    src/main.cpp
    src/mainwindow.cpp src/mainwindow.h
    src/advancedsettingsdialog.cpp src/advancedsettingsdialog.h
    src/gif_worker.cpp src/gif_worker.h
    src/gif_settings.h
)

target_link_libraries(gif_creator_gui PRIVATE
    Qt6::Widgets
    opencv_core opencv_imgproc opencv_imgcodecs
    gif_h)
target_include_directories(gif_creator_gui SYSTEM PRIVATE ${OpenCV_INCLUDE_DIRS})

if(CMAKE_CXX_COMPILER_ID MATCHES "GNU|Clang")
    target_compile_options(gif_creator_gui PRIVATE -Wall -Wextra -Wpedantic)
endif()
