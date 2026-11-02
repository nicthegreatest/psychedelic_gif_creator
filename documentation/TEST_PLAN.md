# Manual test plan

Short checks Nic does by hand after a milestone. Agents add rows for anything they could not
verify with `cargo test`; they do not drive the GUI. Mark **PASS**, **FAIL** (with a note) or
**NIC** (still to check). Use any photo plus one transparent PNG and one 16-bit PNG.

| | Check | Expected | Milestone | Result |
| --- | --- | --- | --- | --- |
| [ ] | Empty window | `flatpak run dev.nic.GifCreator` shows an empty window titled Psychedelic GIF Creator | M1 | NIC |
| [ ] | Launch | `flatpak run dev.nic.GifCreator` opens "Psychedelic GIF Creator" from the terminal and the app menu, with icon | M1 | NIC |
| [ ] | Open image | File dialog, drag and drop and a typed path all load the image; preview appears | M5 | NIC |
| [ ] | Odd inputs | Transparent PNG, 16-bit PNG, greyscale, rotated phone JPEG (EXIF) all look right; a text file shows a clear error | M5 | NIC |
| [ ] | Live preview | Moving a slider updates the preview within a moment; unticking Live preview stops updates | M5 | NIC |
| [ ] | Play / scrub | Play animates at roughly the GIF speed; the slider scrubs; the loop has no visible jump | M5 | NIC |
| [ ] | Row visibility | Changing zoom mode, kaleidoscope, stars, wave, spin direction, saturation pulse shows/hides the matching rows as in v2 | M5 | NIC |
| [ ] | Advanced dialog | Opens beside the main window and can be moved; changes preview live; Cancel restores; OK keeps; Cosmic Chaos randomises; Effect defaults resets only those | M5 | NIC |
| [ ] | Match aspect / Reset | Match image aspect sets the height from the image; Reset all restores defaults and keeps the image | M5 | NIC |
| [ ] | Export | Generate GIF suggests `<name>_psychedelic.gif`; progress runs; the GIF loops seamlessly in a browser and looks like the preview | M5 | NIC |
| [ ] | Cancel export | Cancel stops quickly and no half-written file is left | M5 | NIC |
| [ ] | Close during export | Asks before quitting; quitting cancels cleanly | M5 | NIC |
| [ ] | Same look as v2 | Same image + same settings in v2 and the Rust app give GIFs that look the same side by side | M5 | NIC |
| [ ] | Default preset vs v2 | Kaleidoscope, stars, wave, pixelate, hue, saturation, invert and blur are still no-ops, so a default preset will not match v2 until those effects land | M3 | NIC |
| [ ] | Settings persist | Change settings, quit, reopen: same settings, window size and folders | M6 | NIC |
| [ ] | Presets | Save, load, delete, export and import a preset | M6 | NIC |
| [ ] | Bundle install | `flatpak install --user gif-creator.flatpak` on another machine/user pulls the runtime if missing and the app runs | M7 | NIC |
