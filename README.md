# Psychedelic GIF Creator

Turns a still image into a seamlessly looping "psychedelic tunnel" GIF:
nested, spinning, zooming copies of the image, with optional kaleidoscope,
colour cycling, stars, ripples and texture effects. Qt 6 desktop app, C++17.

## Build and install on Fedora 44

```sh
sudo dnf install qt6-qtbase-devel opencv-devel cmake gcc-c++
# optional, only needed to open WebP/SVG files OpenCV cannot read itself:
sudo dnf install qt6-qtimageformats qt6-qtsvg

cmake -S . -B build -DCMAKE_INSTALL_PREFIX="$HOME/.local"
cmake --build build -j"$(nproc)"
ctest --test-dir build --output-on-failure
cmake --install build

# refresh the menu and icon caches (harmless if already current)
update-desktop-database ~/.local/share/applications
gtk-update-icon-cache -f -t ~/.local/share/icons/hicolor
```

Run it from the app menu ("Psychedelic GIF Creator") or with
`~/.local/bin/gif_creator_gui`.

Set `CMAKE_INSTALL_PREFIX` when configuring (not only at `cmake --install`
time): the `.desktop` file's `Exec=` line is generated from it.

Uninstall: `xargs rm -v < build/install_manifest.txt`.

Builds are Release by default and must be out-of-source (`-B build`).
Pass `-DBUILD_TESTING=OFF` to skip the test and benchmark programs.

## Using it

1. Choose an image (Browse, or type/paste a path and press Enter). PNG, JPEG,
   WebP, TIFF, BMP and GIF (first frame) work, including 16-bit, greyscale
   and transparent images.
2. Tweak the main controls; the preview updates live (untick **Live preview**
   on slow machines). **Play** animates the preview, the slider scrubs it.
3. **Advanced effects…** opens the extra effects. Changes show in the preview
   straight away; **OK** keeps them, **Cancel** puts things back.
   **Cosmic Chaos** randomises just those effects within sensible ranges.
4. **Generate GIF**, pick where to save. **Cancel** stops an export and removes
   the half-written file.

Settings, the last folders used and the window size are remembered in
`~/.config/nicthegreatest/psychedelic-gif-creator.conf`. **Reset all** returns
to the defaults.

## Layout

```
src/core/        renderer, settings schema, image loader, GIF export worker (no widgets)
src/ui/          main window, advanced dialog, schema-driven form widgets
src/main.cpp
resources/       style.qss, icon (SVG + PNG), .desktop template
tests/           render_tests (ctest) and gif_bench
third_party/gif-h  vendored encoder, pinned commit + licence in its README
docs/            the original design notes
```

## What changed from the original

**Build and packaging**
- Qt 6 only; links just OpenCV core/imgproc/imgcodecs (no more pulling in all
  of OpenCV). gif-h is vendored at a pinned commit instead of being
  downloaded from `master` at configure time.
- Release by default, in-source builds refused, `-Wall -Wextra -Wpedantic`
  clean, CMake cruft removed.
- `cmake --install` installs the binary, a `.desktop` entry and icons.

**Reliability**
- The export worker catches errors and reports them instead of crashing,
  checks write errors, never leaks, and deletes a partial GIF on cancel or
  failure (but never an existing file it did not create).
- Closing the window during an export asks first, then cancels cleanly.
- Removed undefined behaviour (`reinterpret_cast` of Mat data), debug prints,
  and string-compared status codes.
- Images that previously broke the pipeline (16-bit, greyscale, alpha,
  float, palette) are converted properly; a missing or unreadable file gives
  a clear message.

**Rendering**
- One renderer is shared by preview and export, so the preview is exactly
  what you get. The preview renders off the UI thread (debounced, from a
  cached decoded image), so the controls never freeze.
- Every effect loops seamlessly: spin, zoom, hue, saturation, invert flashes,
  ripples, kaleidoscope and stars are all whole cycles per loop. Fractional
  spin is exact in Tunnel mode; in other modes it is rounded to whole turns.
- The starfield is seeded (repeatable) and actually visible.
- Output size, fit (Fill / Fit / Stretch), frame count and fps are
  configurable; "Match image aspect" sets the height from the image.
- Optional Floyd-Steinberg dithering.
- Frames are rendered and their palettes built in parallel; the output is
  byte-identical whatever the thread count. Default export: ~1.8 s vs ~5.9 s
  before on the same machine and image.
- "Clockwise" now really spins clockwise (the old code had it backwards).

**UI**
- All settings, ranges, labels and tooltips come from one schema
  (`src/core/settings_schema.cpp`), which also drives defaults, clamping and
  saving. Controls that do nothing for the current mode are hidden.
- Stylesheet moved to `resources/style.qss` with Linux-available fonts.

**Settings that were renamed or reinterpreted**
- "Linear" zoom became **Tunnel** (endless fly-through).
- Hue intensity became **Colour cycles per loop**.
- Default star count is 300.

## Tests

`ctest --test-dir build` runs `gif_render_tests` headlessly: loop seams for
several effect combinations (last frame flows into the first), stars visible,
aspect handling, image loading edge cases, settings save/load/clamping,
export, cancel, failure, and thread-count determinism.

`build/gif_bench <image> <out.gif> [none|radial6] [threads] [WxH] [fit|fill] [dither]`
times rendering and a full export.
