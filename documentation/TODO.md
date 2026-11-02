# TODO

Deferred items, open decisions and known issues. Planned milestone work lives in `PLAN.md`.

## High
- None.

## Medium
- File access: portal-only vs `--filesystem=home` (typed paths, remembering the last image).
  Decide in M5, finalise in M7.
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
- Translations (gettext) — English only for now.
- Import settings from v2's `~/.config/nicthegreatest/psychedelic-gif-creator.conf`.
- Animated input (GIF frames via the `gif` decoder; video via GStreamer from the runtime, not
  FFmpeg crates).
- aarch64 build of the bundle.
- AVIF/HEIC input (needs C libraries; would come from the runtime, not crates).

## Resolved
- Licence: GPL-3.0-or-later (matches Cutline, allows `imagequant`). `COPYING` added in M1.
- App ID `dev.nic.GifCreator` confirmed.
- JPEG 2000 (`.jp2`) input dropped: no pure-Rust decoder.

## Known issues
- Letterbox fit that scales by exactly 2 (golden config "letterboxed": 320×200 into 160×120)
  differs from OpenCV `INTER_AREA` by at most 1 per channel. Measured mean abs diff about 0.11,
  99th percentile 1. OpenCV takes a faster path for integer factors; the area kernel follows the
  general overlap formula and was left unchanged. Inside the M2 tolerance (mean ≤ 1, p99 ≤ 8).
