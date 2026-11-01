# TODO

Deferred items, open decisions and known issues. Planned milestone work lives in `PLAN.md`.

## High
- Generate golden reference data from v2 with `tools/v2-golden/` (M2). Needs v2's build deps
  on the host (Qt 6, OpenCV devel), which are already installed since v2 builds.

## Medium
- File access: portal-only vs `--filesystem=home` (typed paths, remembering the last image).
  Decide in M5, finalise in M7.
- Optional "High quality" palette mode with `imagequant` (only if GPL-3.0-or-later).
- Headless CLI (`gif-creator --settings preset.txt in.png out.gif`) built on `gifcore`.
- Publish to Flathub (needs metainfo screenshots, a public git repo and release tags).

## Low
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
- None yet.
