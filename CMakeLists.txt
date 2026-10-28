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

find_package(Qt6 6.2 REQUIRED COMPONENTS Core Gui Concurrent Widgets)
# Only the modules the app actually uses. Linking ${OpenCV_LIBS} (all modules)
# pulled in highgui/cvv, which on Fedora drag a second Qt into the process.
find_package(OpenCV 4 REQUIRED COMPONENTS core imgproc imgcodecs)

qt_standard_project_setup()

add_library(gif_h INTERFACE)
target_include_directories(gif_h SYSTEM INTERFACE ${CMAKE_CURRENT_SOURCE_DIR}/third_party/gif-h)

set(GIF_WARNINGS)
if(CMAKE_CXX_COMPILER_ID MATCHES "GNU|Clang")
    set(GIF_WARNINGS -Wall -Wextra -Wpedantic)
endif()

# --- Core: settings, image loading, the shared renderer and the exporter ----
# No widgets here, so it is used by the GUI, the tests and the benchmark.
add_library(gifcore STATIC
    src/core/gif_settings.h
    src/core/settings_schema.cpp src/core/settings_schema.h
    src/core/image_loader.cpp src/core/image_loader.h
    src/core/frame_renderer.cpp src/core/frame_renderer.h
    src/core/gif_worker.cpp src/core/gif_worker.h
)
target_include_directories(gifcore PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}/src/core)
target_include_directories(gifcore SYSTEM PUBLIC ${OpenCV_INCLUDE_DIRS})
target_link_libraries(gifcore
    PUBLIC Qt6::Core Qt6::Gui opencv_core opencv_imgproc opencv_imgcodecs
    PRIVATE Qt6::Concurrent gif_h)
target_compile_options(gifcore PRIVATE ${GIF_WARNINGS})

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

target_compile_options(gif_creator_gui PRIVATE ${GIF_WARNINGS})

# --- Tests and benchmark ----------------------------------------------------
include(CTest)
if(BUILD_TESTING)
    qt_add_executable(gif_render_tests tests/render_tests.cpp)
    target_link_libraries(gif_render_tests PRIVATE gifcore)
    target_compile_options(gif_render_tests PRIVATE ${GIF_WARNINGS})
    set_target_properties(gif_render_tests PROPERTIES WIN32_EXECUTABLE OFF MACOSX_BUNDLE OFF)
    add_test(NAME render_tests COMMAND gif_render_tests)
    set_tests_properties(render_tests PROPERTIES ENVIRONMENT "QT_QPA_PLATFORM=offscreen" TIMEOUT 300)

    qt_add_executable(gif_bench tests/bench.cpp)
    target_link_libraries(gif_bench PRIVATE gifcore)
    target_compile_options(gif_bench PRIVATE ${GIF_WARNINGS})
endif()

# --- Install ----------------------------------------------------------------
# Configure with -DCMAKE_INSTALL_PREFIX=$HOME/.local so the .desktop Exec path
# points at the installed binary, then: cmake --install build
include(GNUInstallDirs)
install(TARGETS gif_creator_gui RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR})
configure_file(resources/psychedelic-gif-creator.desktop.in
               ${CMAKE_CURRENT_BINARY_DIR}/psychedelic-gif-creator.desktop @ONLY)
install(FILES ${CMAKE_CURRENT_BINARY_DIR}/psychedelic-gif-creator.desktop
        DESTINATION ${CMAKE_INSTALL_DATADIR}/applications)
install(FILES resources/psychedelic-gif-creator.svg
        DESTINATION ${CMAKE_INSTALL_DATADIR}/icons/hicolor/scalable/apps)
