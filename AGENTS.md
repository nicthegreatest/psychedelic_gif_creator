# AGENTS.md

Single entry point for LLM coding agents working on Psychedelic GIF Creator (Rust). Do not add
tool-specific files such as `CLAUDE.md` or `GEMINI.md`; put agent guidance here.

## Project

- Rust rewrite of `/home/nic/Software/gif_creator_v2` (C++17, Qt 6, OpenCV, gif-h). Goal: feature
  parity, minimal dependencies, distributed as a Flatpak.
- Stack: Rust (edition 2021), GTK 4 + libadwaita (`gtk4` 0.11, `libadwaita` 0.9), Meson + Cargo,
  user Flatpak `dev.nic.GifCreator` on the GNOME 50 runtime. Pure-Rust image code (`image`,
  `gif`, `rayon`). No OpenCV, no FFmpeg, no GStreamer.
- Layout: `crates/gifcore/` = headless render + GIF engine (no GTK, all tests run here);
  `src/` = GTK app; `tools/v2-golden/` = C++ harness that dumps reference frames from v2
  (never shipped).
- Target machine: Fedora 44, Wayland, Ryzen 5 5600, Radeon RX 6700 XT. Friends run Bazzite.
- Read `documentation/PLAN.md` before any work. Do only the milestone you are given, but keep
  designs compatible with later ones. `documentation/FEATURE_INVENTORY.md` is the parity checklist.
- **Never modify `/home/nic/Software/gif_creator_v2`.** Read it, and build the golden harness
  against it out of tree, nothing more.

## Rules

1. **Changelog (hard rule).** Every user-visible change, UI included, gets an entry under
   `[Unreleased]` in `documentation/CHANGELOG.md` in the same change, without being asked.
   Keep a Changelog headings (Added, Changed, Removed, Fixed), each at most once per section.
2. **Commits.** Commit to `main` with conventional commit messages (`feat(render): …`,
   `fix(export): …`, `test: …`, `docs: …`, `chore: …`). One or a few commits per milestone is
   fine. **Never push.**
3. **Determinism (hard rule).** Every animation loops seamlessly (frame N == frame 0), and the
   exported GIF is byte-identical for any thread count. A frame is a pure function of
   (settings, source image, frame index). No shared mutable state, no float reductions across
   threads, no RNG without a fixed seed.
4. **Docs.** Topic docs live in `documentation/` as `UPPER_SNAKE.md`. Deferred work goes in
   `documentation/TODO.md` (High / Medium / Low). Before a major doc rewrite, back up the old
   files to `backups/<topic>-YYYY-MM-DD/` (gitignored).
5. **Token-efficient runs.** Per milestone run only: the Flatpak build, `cargo test`, clippy.
   No GUI or pointer automation, no AT-SPI, no screenshots. Nic tests by hand using
   `documentation/TEST_PLAN.md`; add rows there for anything you could not check.
6. **One failed approach, then stop.** If an approach fails (a build error you cannot fix
   cleanly, a crate that does not fit, a parity test you cannot get within tolerance), stop and
   report what you tried and why it failed. Do not try a second approach unasked.
7. **Stay light.** No new dependencies beyond the crates listed in `PLAN.md` without asking.
   Prefer a small hand-written function to a crate.

## Build, test, lint

```sh
# Build + install the Flatpak (also runs cargo test via meson)
cd /home/nic/Software/gif_creator_rs
flatpak run --command=flatpak-builder-wrapper org.flatpak.Builder \
    --user --install --force-clean --install-deps-from=flathub \
    flatpak-build dev.nic.GifCreator.yml
```

The host rustup toolchain is too old for the GTK crates, and `~/.cargo/bin` holds an old
`cargo-clippy` that shadows the SDK one. Run tests and clippy inside the GNOME 50 SDK with a
separate `CARGO_HOME`:

```sh
SDK='flatpak run --share=network --filesystem=home --command=bash org.gnome.Sdk//50 -c'
ENV='source /usr/lib/sdk/rust-stable/enable.sh && export CARGO_HOME=$HOME/.cache/cargo-gnome-sdk && cd /home/nic/Software/gif_creator_rs'
$SDK "$ENV && cargo test --workspace --locked"
$SDK "$ENV && cargo clippy --workspace --all-targets --locked -- -D warnings"
$SDK "$ENV && cargo fmt --all --check"
```

- Core-only tests are fast: `cargo test -p gifcore --release` (render tests are slow in debug).
- After changing `Cargo.lock`, regenerate `cargo-sources.json` with
  [flatpak-cargo-generator.py](https://github.com/flatpak/flatpak-builder-tools/tree/master/cargo):
  `python3 flatpak-cargo-generator.py Cargo.lock -o cargo-sources.json`.

## Report back

Commits (hash + message), what changed and where, build/test/clippy results, the changelog
entry, and anything you skipped, guessed or did not verify.
