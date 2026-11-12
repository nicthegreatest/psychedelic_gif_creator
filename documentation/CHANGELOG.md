# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Planning documents for the Rust + GTK 4 rewrite of Psychedelic GIF Creator: plan, feature
  inventory of v2, TODO, manual test plan and agent guide.
- App skeleton: an empty libadwaita window titled "Psychedelic GIF Creator".
- Flatpak packaging (`dev.nic.GifCreator`, GNOME 50 runtime) with desktop file, metainfo,
  settings schema and icon, validated by the Meson tests.
- Licensed under GPL-3.0-or-later.
- Main window: settings on the left (Input image, Output, Tunnel and motion, Colour), built
  from the settings schema with slider + spin, combo and switch rows, tooltips and v2's
  show/hide rules; live preview on the right, rendered off the main thread (debounced,
  coalesced, re-rendered after a resize, HiDPI aware) with Play/Pause, a frame slider and a
  "Live preview" switch.
- Open an image with the file dialog, a typed path or drag and drop; load errors appear in the
  preview and as a toast.
- "Match image aspect", "Reset all" (keeps the image) and a toast when spin is rounded to whole
  turns.
- Advanced Effects window (movable, non-modal): live preview of its working copy, Cosmic Chaos,
  Effect defaults, OK/Cancel; the main settings are locked while it is open.
- Generate GIF: save dialog suggesting `<image>_psychedelic.gif` in the last folder used,
  `.gif` appended, progress bar with Cancel, error dialog, "GIF generation complete." dialog
  with "Show in folder", and a "Stop it and quit?" question when closing during an export.

### Changed
- GIF frame delays alternate (8, 9, 8 cs at 12 fps) so the GIF plays at the chosen frame rate;
  v2 used one rounded delay (12.5 fps for 12). The fps tooltip says so.
