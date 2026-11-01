# TODO

Deferred items, open decisions and known issues. Planned milestone work lives in `PLAN.md`.

## High
- Decide the licence before the first code commit. v2 has no LICENSE file. GPL-3.0-or-later
  matches Cutline and would allow `imagequant`; MIT/Apache-2.0 would match the crates.
- Confirm the app ID `dev.nic.GifCreator` before sharing a bundle (changing it later loses
  users' settings and makes a "new" app).
- Generate golden reference data from v2 with `tools/v2-golden/` (M2). Needs v2's build deps
  on the host (Qt 6, OpenCV devel), which are already installed since v2 builds.
- Decide JPEG 2000 (`.jp2`) support: drop (no pure-Rust decoder) or UI-side fallback through
  the runtime's image loaders.

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

## Known issues
- None yet (no code).
