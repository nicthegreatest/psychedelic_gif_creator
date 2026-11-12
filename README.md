# Psychedelic GIF Creator (Rust)

Turns a still image into a seamlessly looping "psychedelic tunnel" GIF: nested, spinning,
zooming copies of the image, with optional kaleidoscope, colour cycling, stars, ripples and
texture effects.

This is a Rust + GTK 4 / libadwaita rewrite of the Qt/OpenCV app in
`/home/nic/Software/gif_creator_v2`, packaged as a Flatpak so it runs on Bazzite or any distro
without installing Qt or OpenCV packages.

**Status: 1.0.0** (2026-11-12). See `documentation/CHANGELOG.md`.

- App ID: `dev.nic.GifCreator`
- Runtime: GNOME 50 (`org.gnome.Platform//50`), Rust from `org.freedesktop.Sdk.Extension.rust-stable`
- Layout: `crates/gifcore/` is the headless render and GIF engine (no GTK); `src/` is the GTK app.

## Install the bundle (any Linux distro with Flatpak)

`GifCreator-1.0.0.flatpak` is a single file you can send to anyone. Flatpak is not
Fedora-only: it runs on Debian, Ubuntu, Linux Mint, Arch, openSUSE, Bazzite, Steam Deck and
others.

1. Install Flatpak if it is missing (see <https://flathub.org/setup> for your distro), e.g.
   Debian/Ubuntu: `sudo apt install flatpak`, then log out and back in.
2. Add the Flathub remote once, so the GNOME 50 runtime can be downloaded:
   ```sh
   flatpak remote-add --user --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo
   ```
3. Install and run:
   ```sh
   flatpak install --user GifCreator-1.0.0.flatpak
   flatpak run dev.nic.GifCreator
   ```
   It also appears in the app menu. The first install downloads the GNOME 50 runtime
   (roughly 350–450 MB) unless another app already uses it; the app itself is under 1 MB.

To remove it: `flatpak uninstall --user dev.nic.GifCreator`.

## Build

The host has no `flatpak-builder`, so use the Flathub Builder app:

```sh
flatpak install --user flathub org.flatpak.Builder org.gnome.Sdk//50 \
    org.freedesktop.Sdk.Extension.rust-stable//25.08
cd /home/nic/Software/gif_creator_rs
flatpak run --command=flatpak-builder-wrapper org.flatpak.Builder \
    --user --install --force-clean --install-deps-from=flathub \
    --repo=flatpak-repo flatpak-build dev.nic.GifCreator.yml
flatpak run dev.nic.GifCreator
```

The build is offline: Rust crates come from `cargo-sources.json` (regenerate it after any
`Cargo.lock` change, see `documentation/AGENT_EXTRAS.md`).

## Make the bundle

After the build above (which exports to `flatpak-repo/`):

```sh
mkdir -p dist
flatpak build-bundle flatpak-repo dist/GifCreator-1.0.0.flatpak dev.nic.GifCreator \
    --runtime-repo=https://dl.flathub.org/repo/flathub.flatpakrepo
```

## Other platforms

macOS and Windows are possible in principle: Rust and GTK 4/libadwaita are cross-platform,
and the render and export engine (`gifcore`) has no Linux-specific code. Each would need its
own packaging (GTK libraries bundled into a Windows installer or a macOS `.app`) and testing,
so that is future work, not part of 1.0.0.

## Documentation

- `AGENTS.md`: rules and commands for coding agents
- `documentation/PLAN.md`: goals, architecture, crates, milestones, risks
- `documentation/FEATURE_INVENTORY.md`: every v2 feature and where it lives, for parity checks
- `documentation/TODO.md`, `documentation/TEST_PLAN.md`, `documentation/CHANGELOG.md`
