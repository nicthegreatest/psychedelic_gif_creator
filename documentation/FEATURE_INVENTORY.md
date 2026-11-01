# v2 feature inventory (parity checklist)

Everything Psychedelic GIF Creator v2 does, with where it lives. Paths are relative to
`/home/nic/Software/gif_creator_v2/` at commit `31b0c77`. Line numbers are approximate guides.
Tick **Rust** when the rewrite has it and a test or Nic's check covers it.

Legend for Rust column: `[ ]` not done, `[x]` done, `drop` intentionally dropped (with reason).

## 1. Input

| Rust | Feature | Where in v2 |
| --- | --- | --- |
| [ ] | Single still image path (`image_path`), typed/pasted (Enter) or Browse dialog | `src/ui/mainwindow.cpp` buildUi ~140–150, `browseForImage` ~225 |
| [ ] | File dialog filter: png jpg jpeg bmp webp tif tiff gif jp2 pnm ppm pgm + All files | `src/ui/mainwindow.cpp` ~57 (`kImageFilter`) |
| [ ] | Decode via OpenCV `IMREAD_UNCHANGED` (keeps 16-bit + alpha) | `src/core/image_loader.cpp` ~92 |
| [ ] | EXIF orientation applied for images without alpha (re-read with `ANYDEPTH|ANYCOLOR`) | `src/core/image_loader.cpp` ~93–96 |
| [ ] | Qt `QImage` fallback for formats OpenCV lacks | `src/core/image_loader.cpp` ~67, ~100 |
| drop? | JPEG 2000 (`.jp2`) via OpenCV — no pure-Rust decoder; decide in M2 | `image_loader.cpp` (OpenCV) |
| [ ] | GIF input = first frame only | README "Using it"; OpenCV imread |
| [ ] | Normalise to 8-bit BGRA: 16U ÷257; float 0..1 ×255 else min–max stretch; signed ints stretched; 1ch grey→BGRA; 2ch grey+alpha; 3ch→BGRA; 4ch as is; other channel counts error | `src/core/image_loader.cpp` `normaliseToBGRA8` ~11–65 |
| [ ] | Errors: empty path, not a readable file, unreadable/unsupported format (exact messages) | `src/core/image_loader.cpp` ~78–110 |
| [ ] | Decoded source cached for preview jobs (re-read only when path changes) | `src/ui/mainwindow.cpp` `SourceCache` ~36–52 |
| n/a | Video input: **none** in v2 | — |

## 2. Settings (31 + image path)

Defaults: `src/core/gif_settings.h` `GifSettings` ~45–93. Schema (key, label, tooltip, range,
step, decimals): `src/core/settings_schema.cpp` `buildFields` ~51–92 (line given per row).
Ints/choices are rounded and clamped; doubles clamped, non-finite → min (`intField`/`dblField`
~11–30). Choices are stored by label text.

| Rust | Key | Label | Type | Range / choices | Default | Schema line |
| --- | --- | --- | --- | --- | --- | --- |
| [ ] | output_width | Width (px) | int | 64–1600 | 600 | 53 |
| [ ] | output_height | Height (px) | int | 64–1600 | 600 | 54 |
| [ ] | fit_mode | Image fit | choice | Fill (crop) / Fit (letterbox) / Stretch | Fill | 55 |
| [ ] | num_frames | Frames per loop | int | 10–240 | 60 | 56 |
| [ ] | fps | Frame rate (fps) | int | 2–50 | 12 | 57 |
| [ ] | dither | Dithering | bool | | false | 58 |
| [ ] | max_layers | Tunnel layers | int | 1–24 | 12 | 61 |
| [ ] | scale_decay | Layer size ratio | double | 0.50–0.99, step 0.01 | 0.92 | 62 |
| [ ] | rotation_direction | Spin direction | choice | Clockwise / Counter-clockwise / None | Clockwise | 65 |
| [ ] | rotation_turns | Spin (turns per loop) | double | 0–10, step 0.1 | 2.0 | 66 |
| [ ] | zoom_mode | Zoom mode | choice | None / Oscillating / Tunnel (infinite zoom) | Oscillating | 67 |
| [ ] | zoom_amplitude | Zoom amount | double | 0–0.6, step 0.01 | 0.40 | 68 |
| [ ] | zoom_cycles | Zoom pulses per loop | int | 1–8 | 2 | 69 |
| [ ] | zoom_midpoint | Zoom centre | double | 0.5–1.5, step 0.01 | 0.82 | 70 |
| [ ] | tunnel_steps | Tunnel speed (layers per loop) | int | 1–6 | 1 | 71 |
| [ ] | hue_cycles | Colour cycles per loop | int | 0–12 | 4 | 74 |
| [ ] | saturation_pulse | Saturation pulse | double | 0–1, step 0.01 | 0.5 | 75 |
| [ ] | saturation_pulses | Saturation pulses per loop | int | 1–12 | 3 | 76 |
| [ ] | invert_flashes | Invert flashes per loop | int | 0–30 | 0 | 77 |
| [ ] | kaleidoscope_mode | Kaleidoscope | choice | None / Quadrant mirror / Radial 3-fold / Radial 4-fold / Radial 6-fold | None | 80 |
| [ ] | kaleidoscope_turns | Kaleidoscope turns per loop | int | 0–6 | 1 | 81 |
| [ ] | star_pattern | Star pattern | choice | None / Random / Spiral | Random | 82 |
| [ ] | num_stars | Stars | int | 0–1000 | 300 | 83 |
| [ ] | star_seed | Star layout seed | int | 0–99999 | 1 | 84 |
| [ ] | wave_direction | Wave direction | choice | None / Horizontal / Vertical | Horizontal | 85 |
| [ ] | wave_amplitude | Wave height (px) | double | 0–50, step 0.1 | 3.6 | 86 |
| [ ] | wave_frequency | Wave density | double | 0–1, step 0.01 | 0.10 | 87 |
| [ ] | wave_cycles | Wave cycles per loop | int | 0–10 | 1 | 88 |
| [ ] | pixelation_level | Pixelation (px) | int | 0–20 (0/1 off) | 0 | 89 |
| [ ] | blur_radius | Blur (px) | double | 0–10, step 0.1 | 0.0 | 90 |
| [ ] | vignette_strength | Vignette | double | 0–1, step 0.01 | 1.0 | 91 |
| [ ] | (all tooltips copied verbatim) | | | | | 53–91 |
| [ ] | Frame delay = clamp(round(100/fps), 2, 100) centiseconds | | | | | `gif_settings.h` `frameDelayCs` ~89 |

## 3. Render pipeline (`src/core/frame_renderer.cpp`)

Constructor precomputes; `renderFrame(index)` is const, thread-safe, deterministic,
t = index / num_frames, and index == num_frames must equal frame 0.

| Rust | Step / feature | Where |
| --- | --- | --- |
| [ ] | Output size = round(w·scale), round(h·scale), min 2 | `outputSize` ~49 |
| [ ] | Preview scale: every pixel parameter scaled (wave amp ×s, freq ÷s, blur ×s, pixel block ×s, star radius via min(W,H)/600) | ctor ~60, `pixelsFor` ~46, effects |
| [ ] | Premultiply alpha (rounded `(c·a+127)/255`) | `premultiply` ~13 |
| [ ] | Fit: Stretch / Fill (scale max, centre crop) / Fit (scale min, centre letterbox, transparent bars); INTER_AREA when shrinking both axes else INTER_CUBIC; safety resize | ctor ~67–100 |
| [ ] | Opaque-rect detection (min alpha of content == 255) | ctor ~102–107 |
| [ ] | Kaleidoscope Quadrant mirror (static, nearest remap, reflect-101) | `prepareKaleidoscope` ~157–174 |
| [ ] | Kaleidoscope Radial 3/4/6: polar lookup, wedge points up, mirrored half-sectors; static if turns = 0 else per-frame rotation by 2π·turns·t (bilinear, reflect-101) | `prepareKaleidoscope` ~176–196, `kaleidoscopeFrame` ~198 |
| [ ] | Image pyramid: pyrDown while ≥ 32 px, max 8 levels; rebuilt per frame only for animated kaleidoscope | `buildPyramid` ~211, `renderFrame` ~398 |
| [ ] | Layers, non-tunnel: k = 0..L-1, scale d^k, alpha 1, angle 360·turns·t | `compositeLayers` ~221–256 |
| [ ] | Layers, Tunnel: shift = steps·t, e = k − shift, outer cutoff at scale 3, fade-in at centre (e > L−1), fade-out when huge; rotation = twist by depth (−360·turns·e/steps), so fractional turns are seamless | `compositeLayers` ~231–256 |
| [ ] | `scale_decay` clamped 0.05–0.995 internally | `compositeLayers` ~223 |
| [ ] | Rotation: None or 0 → 0; Tunnel exact; otherwise max(1, round(turns)); CCW negates | `effectiveRotationTurns` ~54, ctor ~153 |
| [ ] | Per-layer affine: exact per-axis scale for pyramid level, rotate, centre at ((W−1)/2,(H−1)/2); pick level so scale·2^(level+1) ≤ 1 | `layerMatrix` ~260, loop ~297–316 |
| [ ] | Skip layers hidden behind the smallest fully opaque layer covering the frame | ~273–294 |
| [ ] | Warp into clipped ROI (bilinear, constant 0 border), blend premultiplied OVER with layer alpha | ~306–316, `blendOver` ~25 |
| [ ] | Stars Random: from mt19937(star_seed), uniform float x,y,phase in [0,1), brightness [0.5,1); v = 255·b·(0.55+0.45·sin(2π(2t+phase))) — 2 twinkles/loop | ctor ~142–151, `drawStars` ~325–330 |
| [ ] | Stars Spiral: angle 0.1·j + 2πt, r = 2·j·unit, v = 255·(0.6+0.4·sin(2π(t+phase))); one revolution/loop; off-screen skipped | `drawStars` ~331–341 |
| [ ] | Star radius max(1, round(1.2·min(W,H)/600)), filled anti-aliased circle, combined with per-channel max (lighten) | `drawStars` ~322–342 |
| [ ] | Composite over black (drop alpha of premultiplied) | `renderFrame` ~406–407 |
| [ ] | Vignette: mask = clamp(1 − strength·d²/maxd², 0, 1)·255 (8-bit), multiply/255 | ctor ~113–128, `renderFrame` ~408 |
| [ ] | Oscillating zoom of the whole frame: g = midpoint + amp·sin(2π·cycles·t), skipped when g≈1, bilinear, reflect-101 (mirrored edges when zoomed out) | `renderFrame` ~410–418 |
| [ ] | Pixelation: block = max(1, round(level·scale)); nearest down to W/block then nearest up | `renderFrame` ~420–427 |
| [ ] | Wave: horizontal = x offset per row amp·sin(row·freq+phase); vertical = y offset per column; phase 2π·cycles·t; bilinear, replicate border; off if None/amp 0/freq 0 | ctor ~130–140, `applyWave` ~345 |
| [ ] | Hue cycle: shift = round(180·cycles·t) mod 180 on OpenCV 8-bit HSV (H 0..179) | `applyColour` ~373–386 |
| [ ] | Saturation pulse: S × (1 + pulse·sin(2π·pulses·t)), saturating, via LUT | `applyColour` ~375–385 |
| [ ] | Invert flashes: frame i inverted when (i·k) mod n < k (k evenly spaced frames) | `applyColour` ~388–392 |
| [ ] | Gaussian blur sigma = radius·scale, skipped if ≤ 0.05, kernel size from sigma (OpenCV rule) | `renderFrame` ~432–435 |
| [ ] | Effect order: layers → stars → over black → vignette → osc. zoom → pixelate → wave → colour → invert → blur | `renderFrame` ~395–437 |

## 4. Export (`src/core/gif_worker.cpp`, `third_party/gif-h/gif.h`)

| Rust | Feature | Where |
| --- | --- | --- |
| [ ] | Settings clamped before export | `GifWorker` ctor ~40 |
| [ ] | GIF89a, 8-bit, single fixed delay for all frames, loops forever | `render` ~79, gif-h `GifBegin` |
| [ ] | Per-frame local palette: gif-h kd-tree median split, index 0 reserved transparent, palette zeroed first (thread-count invariance fix) | ~20–25, ~105; gif-h `GifMakePalette` ~381, `GifSplitPalette` ~216 |
| [ ] | Dithering off: threshold to nearest palette entry; on: Floyd–Steinberg (`GifDitherImage`), palette built "for dither" | ~138–141 |
| [ ] | Unchanged pixels vs previous output frame written as transparent (delta), first frame full | ~132–141 |
| [ ] | LZW image data | gif-h `GifWriteLzwImage`, ~142 |
| [ ] | Frames rendered + palettes built in parallel (window = 2 × threads), mapped/encoded strictly in order | `render` ~87–150 |
| [ ] | Thread count setting (0 = one per core) | `gif_worker.h` `setThreadCount` |
| [ ] | Output byte-identical for any thread count | `tests/render_tests.cpp` export block ~189–230 |
| [ ] | Progress (percent + "Frame i of n") | ~148 |
| [ ] | Cancel (atomic flag, checked each frame) → "GIF generation cancelled." | `cancel` ~45, ~119 |
| [ ] | Errors: cannot open output, frame render failure ("Rendering frame N failed: …"), write error ("disk full?"), finish error, image processing / unexpected / unknown error | ~51–58, ~79–83, ~127–147, ~152 |
| [ ] | Partial output deleted on cancel/failure, only if this run created it | `process` ~60 |

## 5. UI (`src/ui/`)

| Rust | Feature | Where |
| --- | --- | --- |
| [ ] | Window title "Psychedelic GIF Creator", default 1100×760, geometry remembered | `mainwindow.cpp` ~69, ~83 |
| [ ] | Settings panel (scrollable, min width 430) with groups: Input image; Output (width, height, fit, frames, fps, dither + "Match image aspect"); Tunnel and motion (layers, layer ratio, spin direction, spin turns, zoom mode); Colour (hue cycles) | `mainwindow.cpp` buildUi ~96–170 |
| [ ] | Number rows = slider + spin box (slider omitted if > 2000 steps), typed values apply on Enter/focus-out; choice = combo; bool = checkbox; tooltips | `settings_form.cpp` `SliderSpin` ~13, `addRow` ~67 |
| [ ] | Conditional visibility: kaleidoscope_turns only for radial; num_stars if stars; star_seed only for Random; wave params if wave; zoom amp/cycles/midpoint only Oscillating; tunnel_steps only Tunnel; rotation_turns unless None; saturation_pulses only if pulse > 0 | `settings_form.cpp` `updateVisibility` ~136 |
| [ ] | "Match image aspect": height = width × src.h/src.w (clamped), status message | `matchImageAspect` ~232 |
| [ ] | Status message when spin is rounded to whole turns (non-tunnel) | `onSettingChanged` ~204–212 |
| [ ] | "Reset all" (keeps the image path) | `resetAll` ~276 |
| [ ] | Preview: min 320×320, off-thread, debounced 150 ms, coalesced (one queued re-render), HiDPI scale, renders at min(1, box/output) scale (≥ 0.05), errors shown in place | `schedulePreview`/`startPreview`/`previewFinished` ~288–345 |
| [ ] | Resize: instant rescale of last image, re-render after 250 ms | `resizeEvent` ~361 |
| [ ] | "Live preview" checkbox (default on) | ~119–135 |
| [ ] | Frame slider 0..num_frames−1 (range follows num_frames) | ~117, ~205, ~281 |
| [ ] | Play/Pause: advances one frame per GIF delay, drops frames while rendering | ~77–80, `togglePlay` ~355 |
| [ ] | "Advanced effects…" dialog: non-modal (movable on GNOME), raised if already open; groups Zoom (amp, cycles, midpoint, tunnel steps), Colour (sat pulse, pulses, invert flashes), Kaleidoscope, Stars, Wave, Texture (pixelation, blur, vignette); hint text | `openAdvanced` ~246, `advancedsettingsdialog.cpp` ~18–70 |
| [ ] | Advanced dialog edits a working copy shown live in the preview; OK applies, Cancel restores; main form disabled while open | `openAdvanced` ~256–273 |
| [ ] | "Cosmic Chaos": randomise advanced effects (ranges: zoom amp 0.10–0.40, cycles 1–3, midpoint 0.85–1.10, tunnel steps 1–3; sat pulse 0.2–0.8, pulses 1–4; invert 25 % chance 1–4; kaleidoscope 40 % None else any radial/quadrant, turns 0–2; star pattern any, stars 100–500, seed 1–99999; wave any, amp 1–10, freq 0.03–0.20, cycles 1–3; pixelation 20 % chance 2–6; blur 30 % chance 0.3–1.5; vignette 0.3–1.0) | `randomise` ~91–126 |
| [ ] | "Effect defaults" (only the advanced keys) | `resetToDefaults` ~128 |
| [ ] | "Generate GIF": needs an image, validates it loads; save dialog with suggested `<basename>_psychedelic.gif` in last output folder; `.gif` appended; settings saved before export | `startExport` ~369–392 |
| [ ] | Busy state: controls locked, progress bar, Cancel button, status "Launching hyperspace…", "Cancelling…" | `setBusy` ~437, ~392 |
| [ ] | Finished: error dialog on failure; success dialog "GIF generation complete." with "Show in folder"; status "Saved …" | `exportFinished` ~412–435 |
| [ ] | Close during export asks "Stop it and quit?", cancels and waits | `closeEvent` ~460 |
| [ ] | Status bar hints: "Choose an image to begin." / "Ready." / "Loading …" | ~85, ~221 |
| [ ] | App icon (SVG + 256 PNG), desktop entry (Graphics;2DGraphics; keywords gif/animation/psychedelic/kaleidoscope) | `resources/` |
| restyle | Custom QSS stylesheet | `resources/style.qss` — replaced by libadwaita styling |

## 6. Persistence

| Rust | Feature | Where |
| --- | --- | --- |
| [ ] | All fields + image path saved under group `settings` (choices as label text), loaded with clamping, unknown/invalid values ignored | `settings_schema.cpp` `saveSettings`/`loadSettings` ~112–147 |
| [ ] | `ui/geometry`, `ui/lastOutputDir` | `mainwindow.cpp` ~83, ~382–387 |
| [ ] | Saved on export start and on close | `saveState` ~454 |
| Low | Import of the v2 config `~/.config/nicthegreatest/psychedelic-gif-creator.conf` | (TODO, Low) |

## 7. Tests and tools to port (`tests/`)

| Rust | Test / tool | Where |
| --- | --- | --- |
| [ ] | Loop seam (< 0.05 and < step/10), size, determinism: defaults; radial 6 + 2 turns; tunnel 0.7; oscillating 0.3 (rounded to 1); everything on letterboxed; tunnel transparent source | `render_tests.cpp` `checkLoop` ~51 and cases ~80–118 |
| [ ] | Stars visible on top of the tunnel | ~121–133 |
| [ ] | 16:9 aspect + preview scale 0.5 | ~135–143 |
| [ ] | Loader: 16-bit, grey, float normalise, missing file | ~145–170 |
| [ ] | Settings round trip + clamping (num_frames 100000 → 240) | ~172–187 |
| [ ] | Export: GIF89a header, progress 100, 1 vs 3 threads identical, cancel removes file, missing input fails | ~189–230 |
| [ ] | Bench CLI: `<image> <out.gif> [none|radial6] [threads] [WxH] [fit|fill] [dither]`, render-only 1-thread timing + export timing and size | `tests/bench.cpp` |
