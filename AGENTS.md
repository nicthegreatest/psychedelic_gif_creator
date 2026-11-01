# AGENTS.md

Psychedelic GIF Creator: Rust rewrite of /home/nic/Software/gif_creator_v2 (C++/Qt/OpenCV).
Stack: Rust 2021, GTK4 0.11 + libadwaita 0.9 (`adw`), pure-Rust image code (image, gif, rayon).
Layout: crates/gifcore/ = headless core (no GTK, all tests here); src/ = GTK app.

## Rules
1. Do only the task in the prompt. Touch only the files it names. Read other files only if the
   task names them or you need a type signature (use grep, not rg; rg is not installed).
2. Never modify /home/nic/Software/gif_creator_v2. Never commit or push; Nic reviews and commits.
3. No Flatpak, Meson, manifest or cargo-sources work. No new dependencies.
4. Determinism: a frame is a pure function of (settings, image, frame index). No shared mutable
   state, no float reductions across threads, no unseeded RNG. Animations loop seamlessly.
5. Tests: never hand-calculate expected values. Use obvious inputs (identity, all-zero, uniform
   colour, 2x2 to 1x1) or check properties (size, value range, monotonic). If an exact number is
   needed, compute it with a short script and paste the output.
6. Write stubs with todo!() first, then fill in bodies. Prefer iterators/zip over index loops.
7. If a fix fails twice, stop and report the exact error text.

## Commands (run inside the GNOME 50 SDK; host cargo is too old)
SDK='flatpak run --share=network --filesystem=home --command=bash org.gnome.Sdk//50 -c'
ENV='source /usr/lib/sdk/rust-stable/enable.sh && export CARGO_HOME=$HOME/.cache/cargo-gnome-sdk && cd /home/nic/Software/gif_creator_rs'
$SDK "$ENV && cargo test -p gifcore"
$SDK "$ENV && cargo clippy -p gifcore --all-targets -- -D warnings"
$SDK "$ENV && cargo fmt --all"

## Report
Files changed, test and clippy results, anything skipped or uncertain.
