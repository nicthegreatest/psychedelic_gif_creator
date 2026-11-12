# TODO

Deferred items, open decisions and known issues. Planned milestone work lives in `PLAN.md`.

## High
- None.

## Medium
- Optional "High quality" palette mode with `imagequant` (only if GPL-3.0-or-later).
- Headless CLI (`gif-creator --settings preset.txt in.png out.gif`) built on `gifcore`.
- Publish to Flathub (needs metainfo screenshots, a public git repo and release tags).

## Low
- Fit safety resize (`cv::resize` `INTER_LINEAR` when rounding leaves the fitted canvas off
  the output size). No side length in 1..=2000 triggers it, so it is not checked against OpenCV.
  The implementation uses the same coordinates as the general bilinear formula, not OpenCV's
  fixed-point resize.
- `tools/v2-golden/` writes raw RGBA frames only. PNG previews and v2 GIFs fit the PLAN
  description but, with the raw frames, were about 3.5 MB, over the 2 MB cap.
- Single-thread render is about 5× slower than v2 (600×600, 60 frames: 3.2 s vs 0.63 s, both
  with OpenCV SIMD on v2's side). Full export with 12 threads is still faster than v2 (1.10 s vs
  1.20 s). The M5 preview renders at preview scale and reuses the built renderer while only the
  frame changes (scrub, Play); profile if Nic finds slider edits sluggish.
- Translations (gettext) — English only for now.
- Import settings from v2's `~/.config/nicthegreatest/psychedelic-gif-creator.conf`.
- Animated input (GIF frames via the `gif` decoder; video via GStreamer from the runtime, not
  FFmpeg crates).
- aarch64 build of the bundle.
- AVIF/HEIC input (needs C libraries; would come from the runtime, not crates).

## Resolved
- M6 scope trimmed by Nic: only window size/maximized, last folders and About. Not planned:
  persisting effect settings or the last image, presets, extra keyboard shortcuts, remembering
  the console toggle.
- File access (M5): the Flatpak gets `--filesystem=home` (as Cutline) and keeps the typed path
  box. Files outside home (e.g. `/run/media`) still go through the portal.
- Frame delays (M5): keep `DelayMode::Accumulated` as the default; fps tooltip and
  `tests/schema.rs` reworded.
- v2's "Cannot read image" check before export lives in the M5 export UI.
- Licence: GPL-3.0-or-later (matches Cutline, allows `imagequant`). `COPYING` added in M1.
- App ID `dev.nic.GifCreator` confirmed.
- JPEG 2000 (`.jp2`) input dropped: no pure-Rust decoder.

## Known issues
- M4 encoder (palette, mapping, LZW, container) is a direct gif-h port and is byte-identical to
  v2's gif-h on v2's golden frames (checked by hash, dither off and on; no GIF bytes stored).
  It uses no new crates: the `gif` crate in PLAN §4 and `rayon` were not needed (std scoped
  threads). Whole exported files still differ from v2 by the small M2/M3 frame differences
  and, by default, the per-frame delays.
- Letterbox fit that scales by exactly 2 (golden config "letterboxed": 320×200 into 160×120)
  differs from OpenCV `INTER_AREA` by at most 1 per channel. Measured mean abs diff about 0.11,
  99th percentile 1. OpenCV takes a faster path for integer factors; the area kernel follows the
  general overlap formula and was left unchanged. Inside the M2 tolerance (mean ≤ 1, p99 ≤ 8).
- On the 64×64 M3 goldens, a handful of pixels differ by 1 in the blue channel even when the
  effect under test is idle at that frame (5 pixels on the plain oscillating frame, spread by
  pixelate and blur). One further HSV-to-BGR pixel differs by 1 where the AVX2 `v_trunc` result
  and the scalar float formula land on opposite sides of an integer. Mean abs diff stays under
  0.003 and the 99th percentile is 0. Radial 3-fold and 4-fold have no golden of their own; the
  directory was within 18 KB of the 2 MB cap, and they share the radial 6 map code.
