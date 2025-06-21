# Psychedelic GIF Creator
![gif_create](https://github.com/user-attachments/assets/fb969e92-eec9-459a-8ee5-4c67440816cb)
![wowzers](https://github.com/nicthegreatest/psychedelic_gif_creator/blob/v0.1/output2.gif)


Turn images in to trippy psychedelic GIFs

## Features

* **Image Input**: Use your own images (PNG, JPG, BMP, GIF) as the base for transformations.
* **Core Settings**: Control fundamental aspects like GIF duration, warp, spin, and color pulse.
* **Advanced Effects**: Dive deeper with options for layers, blur, starfields, global zoom, pixelation, color inversion, and wave distortions.
* **Randomization**: Explore unpredictable visual styles with a "Cosmic Chaos" option.
* **Background Processing**: Generate GIFs without freezing the application.
* **Live Preview**: Preview changes before you render

  
## Building from Source

This project uses CMake and Qt5. OpenCV is required for image processing.

### Prerequisites

* C++17 compatible compiler (e.g., GCC, Clang, MSVC)
* CMake (version 3.10 or higher) 
* Qt5 (Widgets and Concurrent modules) 
* OpenCV library

### Steps

1.  **Clone the repository:**
    ```bash
    git clone [https://github.com/nicthegreatest/psychedelic_gif_creator.git](https://github.com/nicthegreatest/psychedelic_gif_creator.git)
    cd psychedelic_gif_creator
    ```
2.  **Create a build directory and configure CMake:**
    ```bash
    mkdir build
    cd build
    cmake ..
    ```
3.  **Build the application:**
    ```bash
    make
    # Or on Windows with Visual Studio: cmake --build . --config Release
    ```
4.  **Run the application:**
    ```bash
    ./gif_creator_gui
    ```
