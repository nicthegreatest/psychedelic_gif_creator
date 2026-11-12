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
| [ ] | Busy lock | During an export the input, settings, Advanced, Reset and Generate are disabled; Cancel and the progress bar ("Frame n of N (p%)") show; the header says "Launching hyperspace…", then "Cancelling…" after Cancel | M5 | NIC |
| [ ] | Export errors | Generate with no image says "Please choose an input image first."; with a deleted or non-image file says "Cannot read image" with the loader text | M5 | NIC |
| [ ] | Done dialog | "GIF generation complete." shows the path; "Show in folder" opens the folder in Files (Flatpak: through the portal) | M5 | NIC |
| [ ] | Console | No toasts for normal changes. Tick "Show console": the log slides open under the preview; changing a setting adds `HH:MM:SS  Label: old → new`; Oscillating spin 2.5 adds a line saying it is rounded to 3 whole turns (none in Tunnel mode); Match aspect, Reset all, Advanced edits (marked "(preview)") and OK/Cancel, and export start/save/cancel are logged; it auto-scrolls; Clear empties it; unticking hides it | M5 | NIC |
| [ ] | Flatpak file access | In the Flatpak: Browse, drag and drop and a typed path anywhere in your home folder (e.g. `~/Pictures/x.png`) load the image; saving to a home folder works, including a name typed without `.gif` (saved as `<name>.gif`) | M5 | NIC |
| [ ] | Preview speed | Dragging a slider on a 600×600 default config feels responsive; Play at 12 fps does not stutter badly (frames are dropped, not queued) | M5 | NIC |
| [ ] | HiDPI | At 200 % scaling the preview is sharp | M5 | NIC |
| [ ] | Same look as v2 | Same image + same settings in v2 and the Rust app give GIFs that look the same side by side | M5 | NIC |
| [ ] | Default preset vs v2 | Headless frames for the default motion and each M3 effect match v2 within the golden tolerance. A side-by-side look of the preview and the exported GIF is still the "Same look as v2" check | M5 | NIC |
| [ ] | GIF timing | A 12 fps export plays at 12 fps in a browser and loops with no pause at the seam (delays repeat 8, 9, 8 cs) | M5 | NIC |
| [ ] | Export speed | `cargo run -p gifcore --release --example bench -- <photo> /tmp/rs.gif none 0 600x600` is no slower than 2× v2's `gif_bench` with the same arguments (M4: 1.10 s vs 1.20 s on Nic's PC) | M4 | NIC |
| [ ] | Window remembered | Resize, quit, reopen: same size. Maximize, quit, reopen: opens maximized; unmaximizing returns to the earlier size | M6 | NIC |
| [ ] | Folders remembered | Open an image and save a GIF in two different folders, quit, reopen: Browse starts in the image folder, Generate GIF's dialog in the save folder. Delete or rename the save folder, reopen: the dialog opens normally (no error) | M6 | NIC |
| [ ] | Fresh start | Quit, delete `~/.var/app/dev.nic.GifCreator/config/glib-2.0/settings/keyfile` (or put junk in it), reopen: 1100×760 window, no errors; effect settings are always the defaults and the console is off | M6 | NIC |
| [ ] | About | Header menu → About Psychedelic GIF Creator shows the icon, version 0.1.0, GPL-3.0-or-later and developer nicthegreatest | M6 | NIC |
| [ ] | Bundle install | On another machine or user (e.g. Debian/Ubuntu with Flatpak and the Flathub remote): `flatpak install --user GifCreator-1.0.0.flatpak` pulls the GNOME 50 runtime if missing; the app starts from the app menu with its icon; About shows 1.0.0 | M7 | NIC |
| [ ] | Bundle on this PC | `flatpak install --user --reinstall dist/GifCreator-1.0.0.flatpak` then `flatpak info --user dev.nic.GifCreator` shows Version 1.0.0 (checked by the agent on 2026-11-12) | M7 | PASS |
