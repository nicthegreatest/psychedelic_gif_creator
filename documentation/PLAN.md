# Plan: Rust rewrite of Psychedelic GIF Creator

Source of truth for behaviour: `/home/nic/Software/gif_creator_v2` (v0.2.0, C++17, Qt 6 Widgets,
OpenCV core/imgproc/imgcodecs, vendored gif-h). Every feature is listed with its source location
in `FEATURE_INVENTORY.md`.

## 1. Goals

1. **Parity with v2.** Same 31 settings with the same keys, ranges and defaults; same effects
   in the same order; same seamless loops; output identical at any thread count. Frames should
   look the same as v2 (checked by golden tests within a small tolerance, see M2).
2. **Tiny install, no distro packages.** One Flatpak on the GNOME runtime. No Qt, no OpenCV.
   Few, pure-Rust crates.
3. **Runs anywhere Flatpak runs**, Bazzite included, shareable as a single `.flatpak` file
   (and Flathub-ready later).
4. **Headless core.** All render and export logic in a library with no GTK, so tests and a
   benchmark run with plain `cargo test`.

Non-goals for parity: video input (v2 has none), new effects, translations. Presets are a small
extra in M6.

## 2. What v2 does (summary)

Loads one still image (PNG, JPEG, WebP, TIFF, BMP, GIF first frame, PNM, JP2 via OpenCV, with Qt
as fallback; 16-bit, float, grey and alpha normalised to 8-bit BGRA; EXIF orientation applied).
Fits it to the output size (Fill / Fit / Stretch). Builds a tunnel of up to 24 nested,
rotating copies (Oscillating zoom or infinite Tunnel zoom), then per frame: stars, vignette,
oscillating zoom, pixelation, wave, hue/saturation cycling, invert flashes, blur. Exports an
infinitely looping GIF89a with a per-frame 256-colour palette (gif-h median-cut, optional
Floyd–Steinberg dithering, unchanged pixels written as transparent). Frames render in parallel;
encoding is in order. UI: schema-driven settings form, live debounced preview with play and
scrub, an Advanced Effects dialog with live preview, "Cosmic Chaos" randomiser, reset,
"Match image aspect", progress and cancel, settings remembered in QSettings.
**No video input** — so no GStreamer or FFmpeg is needed.

## 3. Architecture

```
gif_creator_rs/
  Cargo.toml                 workspace; root package = the GTK app (binary `gif-creator`)
  crates/gifcore/            headless library, no GTK
    src/settings.rs          GifSettings, enums + labels, defaults (one place)
    src/schema.rs            field table: key, label, tooltip, kind, min/max/step/decimals,
                             get/set with clamping; to/from "key=value" text (presets, tests)
    src/image.rs             Rgba8 / Bgr8 buffers (Vec<u8> + w/h), no external image types in API
    src/loader.rs            file -> straight RGBA8 (image crate), EXIF orientation, normalisation
    src/kernels/             OpenCV-equivalent ops: resize (area/cubic/nearest), pyr_down,
                             warp_affine (bilinear), remap (nearest/bilinear, borders),
                             gaussian_blur, bgr<->hsv (OpenCV 8-bit semantics), aa_circle
    src/rng.rs               mt19937 + libstdc++ uniform_real_distribution<float> (star parity)
    src/render.rs            FrameRenderer: precompute in new(), render_frame(i) is &self + pure
    src/effects/             kaleidoscope, stars, wave, colour, texture (pixelate/blur/vignette)
    src/quantize.rs          port of gif-h palette (kd-tree median split), threshold + dither
    src/export.rs            parallel render + palette, ordered delta-map + LZW via `gif` crate,
                             progress callback, cancel flag, partial-file cleanup
    examples/bench.rs        same CLI as v2's gif_bench
    tests/                   loop, determinism, thread-count, loader, schema, golden
    tests/golden/            small reference PNG/GIF/RGBA dumps from v2
  src/                       GTK app: main, application, window, settings_rows (built from the
                             schema), preview, advanced (dialog), export_ui, config.rs.in
  data/                      .desktop, metainfo, gschema, icons (reuse v2 SVG/PNG)
  build-aux/cargo.sh         as in Cutline
  tools/v2-golden/           C++ harness compiled against v2 sources out of tree (not shipped)
  meson.build, dev.nic.GifCreator.yml, cargo-sources.json
```

Key design points:

- **Determinism.** `FrameRenderer::render_frame(&self, index) -> Bgr8` has no interior
  mutability. Export uses rayon over a sliding window of frame indices; each job renders the
  frame and builds its palette (zeroed, as v2 does after its bug fix), results are consumed
  strictly in order for the sequential part (delta transparency against the previous output
  frame, LZW). A dedicated `rayon::ThreadPool` per export lets tests pin 1 vs N threads and
  compare bytes. No float sums across threads; all randomness from fixed seeds (stars use
  `star_seed`; Cosmic Chaos uses a UI-only RNG that only sets settings).
- **Seamless loops** are a property of the maths (whole cycles per loop; tunnel layer k at t=1
  equals layer k-steps at t=0); a test renders frame N and checks it equals frame 0.
- **Preview** uses the same renderer at a scale < 1, with every pixel parameter scaled (wave
  amplitude/frequency, blur sigma, pixel block, star radius), exactly as v2.
- **Pixel format.** Core works in BGRA/BGR internally to mirror v2 so ports stay line-for-line
  comparable; converts to RGBA only for the encoder and the GTK texture.
- **Settings persistence** in GSettings (`dev.nic.GifCreator.gschema.xml`) with the v2 key
  names; the core schema table stays the single source of ranges/defaults and a unit test
  checks the gschema defaults match it. Presets (M6) use the core's `key=value` text format,
  so no serde.
- **File access** through portals (`gtk::FileDialog`, drag and drop, `gtk::FileLauncher` for
  "Show in folder"), no `--filesystem` permission. If the typed-path entry or remembered last
  image proves awkward through the document portal, fall back to `--filesystem=home` like
  Cutline (decision in TODO).
- **UI** built in Rust code from the schema (an `AdwPreferencesGroup` of rows per group, a
  slider + spin row for numbers, `AdwComboRow` for choices, `AdwSwitchRow` for bools). No
  Blueprint, which avoids the blueprint-compiler subproject.

## 4. Crates

| Crate | Use | Why | Licence | Approx. size in release binary |
| --- | --- | --- | --- | --- |
| `image` 0.25 (`default-features = false`, features `png jpeg gif webp tiff bmp pnm`) | decode input | pure Rust, 16-bit/float/alpha, EXIF orientation (`apply_orientation`) | MIT/Apache-2.0 | ~1.5–2.5 MB |
| `gif` 0.14 | GIF89a container + LZW (`weezl`) | tiny, pure Rust, accepts our own palette + indices, so the gif-h quantiser port gives v2-identical output | MIT/Apache-2.0 | ~0.1 MB |
| `rayon` 1.x | parallel frames | work stealing, scoped pools; ordering kept by us | MIT/Apache-2.0 | ~0.2 MB |
| `gtk4` 0.11, `libadwaita` 0.9, `glib` 0.22 | UI (app crate only) | same versions as Cutline; the libraries themselves come from the runtime | MIT | ~1–2 MB of bindings |

Considered and not chosen by default:

- **Quantiser:** port gif-h's ~300-line public-domain median-cut + dither instead of a crate.
  Needed for byte-level parity and determinism, and it is zero dependencies.
  `color_quant` 2.0 (NeuQuant, MIT, ~30 KB) is the fallback if the port proves hard.
  `imagequant` 4.4 (libimagequant) is better quality but **GPL-3.0-or-later or commercial**,
  usable only if this app is GPL-3.0-or-later; candidate for an optional "High quality" mode
  later (TODO, Medium).
- **Resize/warp:** hand-written kernels to match OpenCV (`INTER_AREA`, bicubic a = -0.75,
  fixed-point bilinear with `INTER_BITS = 5`, `pyrDown` 5-tap, `BORDER_REFLECT_101`).
  `fast_image_resize` 6.x is the fallback only if resize shows up in profiles.
- **No** `rand` (own mt19937, ~40 lines), **no** `serde` (key=value text), **no** `gettext`
  (English only for now), **no** `gstreamer`/`ffmpeg` (no video input), **no** `anyhow`.

Release profile: `opt-level = 3`, `lto = "fat"`, `codegen-units = 1`, `panic = "unwind"`
(so a render panic becomes an error message), `strip = true`. Speed beats a few hundred KB.

## 5. Milestones

Each milestone ends with: Flatpak builds and installs, `cargo test` passes, clippy clean
(`-D warnings`), `cargo fmt` clean, CHANGELOG updated if user-visible, commit(s) on `main`.
The order differs from "UI before export" on purpose: the export engine is headless, so it is
finished and tested before the UI just wires it up.

### M1 — Skeleton and Flatpak
Scope: Cargo workspace (app + empty `gifcore`), `meson.build`, `build-aux/cargo.sh`,
`dev.nic.GifCreator.yml` (copy Cutline's, minus pulseaudio, filesystem and GStreamer bits),
`cargo-sources.json`, desktop file, metainfo, empty gschema, icons from v2,
`config.rs.in`, an empty `AdwApplicationWindow` titled "Psychedelic GIF Creator".
Acceptance: Flatpak builds offline and installs; `flatpak run dev.nic.GifCreator` shows the
empty window (Nic checks); desktop/metainfo/schema validation passes in the meson tests;
installed app size reported.

### M2 — Core render engine + golden harness
Scope: `settings`, `schema` (all 31 fields, labels, tooltips, ranges, clamping), `loader`,
fit modes, premultiply/blend, `kernels` needed for the tunnel (resize, pyr_down, warp_affine,
remap), tunnel compositing for Oscillating and Tunnel modes with depth twist, opaque-layer
skip, rotation rounding (`effective_rotation_turns`), vignette, oscillating zoom, preview
scale. `tools/v2-golden/`: a CMake project that compiles v2's `src/core/*.cpp` by absolute
path into its own build dir (v2 untouched) and writes reference frames (PNG, small, e.g.
160×120 at frames 0, n/4, n/2) plus raw RGBA dumps and v2 GIFs for a fixed list of configs.
Acceptance: ports of v2's render tests pass (loop seam < 0.05 mean abs diff and well below
frame-to-frame motion; deterministic; 16:9 aspect; preview scale 0.5; loader 16-bit/grey/
float/alpha/missing file; schema round trip + clamping). Golden: configs "defaults",
"tunnel 0.7 spin", "oscillating 0.3 spin (rounded)", "letterboxed", "transparent source"
match v2 with mean abs diff ≤ 1.0 and 99th percentile ≤ 8 per channel. Golden data
≤ 2 MB in git.

### M3 — Effects parity
Scope: kaleidoscope (quadrant mirror, radial 3/4/6, static and animated turns), stars
(mt19937 + `uniform_real_distribution<float>` port, Random and Spiral, AA circles, lighten),
wave (horizontal/vertical, replicate border), pixelation (nearest down/up), HSV hue and
saturation with OpenCV 8-bit HSV semantics (H in 0..180) and LUT, invert flashes, Gaussian
blur (OpenCV kernel size rule for 8-bit). Effect order exactly as v2 `renderFrame`.
Acceptance: golden per effect plus "everything on, letterboxed" and "radial 6, 2 turns"
within the M2 tolerance; star positions for seed 1 equal v2's (exact, from a dump); all
loops seamless.

### M4 — GIF export engine (headless)
Scope: gif-h palette port (kd-tree median split, transparent index 0, dither-aware build),
threshold and Floyd–Steinberg mapping with delta transparency, `gif` crate writer (GIF89a,
infinite loop, delay = `frame_delay_cs`), rayon pipeline with window = 2 × threads,
progress callback, cancel flag, error strings as v2, partial file removed on cancel/failure
(never a file it did not create), `examples/bench.rs` with v2's arguments.
Acceptance: given v2's RGBA dumps, our encoder writes a GIF **byte-identical** to v2's
(dither off and on); 1-thread vs 3-thread exports byte-identical; cancel removes the file;
missing input fails cleanly; bench at 600×600, 60 frames within 2× of v2's `gif_bench` time
on Nic's PC (numbers recorded in the report).

### M5 — GTK UI
Scope: main window (`AdwApplicationWindow` + `AdwToolbarView`, split view: settings on the
left, preview on the right), input row (file dialog, typed path, drag and drop), Output /
Tunnel and motion / Colour groups generated from the schema, conditional row visibility as
v2, "Match image aspect", "Reset all", spin-rounding toast, live preview (debounced 150 ms,
off-thread via `gio::spawn_blocking` or a rayon job, coalesced, source cache, re-render on
resize at 250 ms, HiDPI aware), Play/Pause at the GIF frame rate (drops frames when busy),
frame scrubber, "Live preview" toggle, Advanced Effects window (non-modal, live preview of a
working copy, Cosmic Chaos with v2's ranges, Effect defaults, OK/Cancel, main form locked
while open), Generate GIF with save dialog (suggested name `<input>_psychedelic.gif`,
remembered folder, `.gif` appended), progress bar + Cancel, success dialog with
"Show in folder", close-during-export confirmation.
Acceptance: builds, tests and clippy pass; every UI item in FEATURE_INVENTORY §5 is wired;
TEST_PLAN rows added for Nic.

### M6 — Settings persistence, presets, polish
Scope: GSettings for all fields + window size + last input/output folders + last image;
gschema default ↔ core default test; presets: save/load/delete named presets (key=value
files in the app data dir) and import/export a preset file; keyboard shortcuts
(Ctrl+O open, Ctrl+E/Ctrl+Return generate, Escape cancel, Ctrl+Q quit, shortcuts dialog);
about dialog; error toasts in place of the status bar.
Acceptance: settings survive restart (Nic checks); out-of-range stored values are clamped
(unit test); presets round-trip (unit test); CHANGELOG updated.

### M7 — Packaging, bundle and manual test pass
Scope: final finish-args review (portals only, or `--filesystem=home` if decided), metainfo
with screenshots placeholder and release notes, `flatpak build-bundle` script/README steps
with `--runtime-repo` pointing at Flathub, size report, a run on a clean user (or Bazzite VM)
to confirm the runtime auto-installs, Nic's TEST_PLAN pass, version 1.0.0 entry in CHANGELOG.
Acceptance: `gif-creator.flatpak` installs on a machine without the SDK; size numbers in the
report; TEST_PLAN all PASS or listed in TODO.

## 6. Risks

| Risk | Impact | Mitigation |
| --- | --- | --- |
| **Matching OpenCV pixel-for-pixel** (fixed-point bilinear, `INTER_AREA`, `pyrDown`, 8-bit HSV tables, `GaussianBlur` kernel rules, `LINE_AA` circles) | Golden tests fail; subtle look changes | Tolerance-based golden tests, not exact; port the integer/fixed-point paths from OpenCV's documented algorithms; compare one stage at a time with harness dumps. Exactness required only for integer-only steps and the encoder. |
| **Palette quantisation quality / parity** | Banding, or GIF bytes differ from v2 | Port gif-h exactly (public domain) and test byte-identity from identical RGBA input. `color_quant` as fallback; `imagequant` (GPL) as an optional HQ mode later. |
| **Performance vs OpenCV** (SIMD, tuned warps) | Slower preview/export | Frames already parallel; ROI-limited warps and pyramid levels as v2; fixed-point integer inner loops that auto-vectorise; precompute polar maps; bench in M4 with a 2× budget; `fast_image_resize` only if needed. |
| **Image format coverage** | v2 opens JP2 (OpenCV) and anything Qt plugins read; `image` has no JPEG 2000, and AVIF/HEIC need C libs | Accept PNG/JPEG/WebP/TIFF/BMP/GIF/PNM (covers v2's README list); JP2 listed as dropped in FEATURE_INVENTORY; optional later fallback via the runtime's gdk-pixbuf/glycin in the UI layer only. |
| **Float input normalisation** | 32-bit float TIFFs look different | Port `normaliseToBGRA8` rules exactly (0..1 → ×255, else min–max stretch); loader test. |
| **Flatpak file access** | Typed paths and remembered image may not work under portals | Portal-first; decision point in M5/M7 with `--filesystem=home` fallback. |
| **mt19937/float distribution parity** | Different star layout from v2 for the same seed | Port libstdc++ `generate_canonical<float,24>` behaviour; exact test against a v2 dump. |
| **Toolchain friction** (host rustup too old, shadowing cargo-clippy) | Wasted runs | Always run cargo inside the GNOME 50 SDK with a separate `CARGO_HOME` (AGENTS.md). |
| **Video input** | — | Checked: v2 reads only still images (GIF = first frame). No GStreamer needed. If animated input is wanted later, use the `gif` decoder or GStreamer from the runtime, not FFmpeg crates. |

## 7. Download size for a friend (estimate)

| Item | Size | When it is downloaded |
| --- | --- | --- |
| App (`dev.nic.GifCreator`) | ~4–7 MB installed, ~2–3 MB as a `.flatpak` bundle (Rust binary with GTK bindings and image decoders, plus icons/data) | Always |
| GNOME 50 runtime (`org.gnome.Platform//50`) | 1.1 GB installed (measured on Nic's PC); roughly 350–450 MB download | Only if no other Flatpak on that machine uses GNOME 50; shared by every GNOME-runtime app |
| GL driver extension (`org.freedesktop.Platform.GL.default//25.08`) | ~100–150 MB download | Only if no app on the 25.08 base is installed yet |

So: **~3 MB** if the friend already has a GNOME 50 Flatpak app (common on Bazzite GNOME), up to
**~0.5 GB one-time** on a machine with no GNOME runtime (e.g. a fresh Bazzite KDE). Bazzite has
Flathub set up already, so `flatpak install --user gif-creator.flatpak` pulls the runtime
automatically via `--runtime-repo`. For comparison, v2 needs Qt 6 and OpenCV from dnf and only
works on Fedora-like hosts. Publishing on Flathub later would make updates automatic.
