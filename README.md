# Psychedelic GIF Creator (Rust)

Turns a still image into a seamlessly looping "psychedelic tunnel" GIF: nested, spinning,
zooming copies of the image, with optional kaleidoscope, colour cycling, stars, ripples and
texture effects.

This is a Rust + GTK 4 / libadwaita rewrite of the Qt/OpenCV app in
`/home/nic/Software/gif_creator_v2`, packaged as a Flatpak so it runs on Bazzite or any distro
without installing Qt or OpenCV packages.

**Status: planning only. There is no code yet.** See `documentation/PLAN.md`.

- App ID: `dev.nic.GifCreator`
- Runtime: GNOME 50 (`org.gnome.Platform//50`), Rust from `org.freedesktop.Sdk.Extension.rust-stable`
- Layout (planned): `crates/gifcore/` is the headless render and GIF engine (no GTK);
  `src/` is the GTK app.

## Build (once M1 lands)

The host has no `flatpak-builder`, so use the Flathub Builder app:

```sh
flatpak install --user flathub org.flatpak.Builder org.gnome.Sdk//50 \
    org.freedesktop.Sdk.Extension.rust-stable//25.08
cd /home/nic/Software/gif_creator_rs
flatpak run --command=flatpak-builder-wrapper org.flatpak.Builder \
    --user --install --force-clean --install-deps-from=flathub \
    flatpak-build dev.nic.GifCreator.yml
flatpak run dev.nic.GifCreator
```

## Share with a friend (once M7 lands)

```sh
flatpak run --command=flatpak-builder-wrapper org.flatpak.Builder \
    --user --force-clean --repo=repo flatpak-build dev.nic.GifCreator.yml
flatpak build-bundle repo gif-creator.flatpak dev.nic.GifCreator \
    --runtime-repo=https://dl.flathub.org/repo/flathub.flatpakrepo
# friend: flatpak install --user gif-creator.flatpak
```

## Documentation

- `AGENTS.md`: rules and commands for coding agents
- `documentation/PLAN.md`: goals, architecture, crates, milestones, risks
- `documentation/FEATURE_INVENTORY.md`: every v2 feature and where it lives, for parity checks
- `documentation/TODO.md`, `documentation/TEST_PLAN.md`, `documentation/CHANGELOG.md`
