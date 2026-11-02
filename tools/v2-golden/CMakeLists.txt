# Reference-frame harness. Compiles gif_creator_v2's core by absolute path into
# this build directory; v2's tree is not modified and nothing here is shipped.
#
#   cmake -S tools/v2-golden -B /tmp/v2-golden-build -DCMAKE_BUILD_TYPE=Release
#   cmake --build /tmp/v2-golden-build
#   /tmp/v2-golden-build/v2_golden crates/gifcore/tests/golden

cmake_minimum_required(VERSION 3.21)
project(v2_golden LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

set(V2 /home/nic/Software/gif_creator_v2)

find_package(Qt6 6.2 REQUIRED COMPONENTS Core Gui Concurrent)
find_package(OpenCV 4 REQUIRED COMPONENTS core imgproc imgcodecs)

qt_standard_project_setup()

add_executable(v2_golden
    main.cpp
    ${V2}/src/core/frame_renderer.cpp
    ${V2}/src/core/gif_worker.cpp
    ${V2}/src/core/image_loader.cpp
    ${V2}/src/core/settings_schema.cpp
)
target_include_directories(v2_golden PRIVATE
    ${V2}/src/core
    ${V2}/third_party/gif-h
)
target_link_libraries(v2_golden PRIVATE
    Qt6::Core
    Qt6::Gui
    Qt6::Concurrent
    opencv_core
    opencv_imgproc
    opencv_imgcodecs
)
if(CMAKE_CXX_COMPILER_ID MATCHES "GNU|Clang")
    target_compile_options(v2_golden PRIVATE -Wall -Wextra -Wpedantic)
endif()
