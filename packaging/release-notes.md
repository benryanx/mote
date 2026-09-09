# Mote 0.1.0 — first public alpha

A native Rust pixel-art and animation editor for Omarchy, with an optional bar
launcher. Includes document tabs, layers and frames, undo/redo, customizable
shortcuts, palette presets and editing, dithered gradients, canvas/sprite resizing,
and image, GIF, sprite-sheet and sequence exports.

## Install

This is a **source-build release**, not a prebuilt binary distribution.
Install the native app before adding the Omarchy bar plugin:

```sh
git clone --branch v0.1.0 https://github.com/benryanx/mote.git
cd mote
mise trust
mise exec -- bash scripts/install.sh
omarchy plugin add https://github.com/benryanx/mote.git --enable
```

See README.md for build/runtime dependencies, separate app/plugin updates,
removal and feature limitations. Builds download Rust/Cargo dependencies.
The editor has no network API or telemetry, and AI generation is not included.

## Validation and limitations

- 32 Rust tests, formatting, Clippy, manifest, desktop-entry and QML checks pass.
- Staged installation, reinstallation and removal pass without touching the
  live installation; test artwork and settings remain intact.
- Full clean-VM installation and shell lifecycle tests are still pending.
- This is an early alpha, not Aseprite feature parity. Save backup copies of
  important artwork. Open document tabs are not restored across application restarts.
- Marketplace listing and approval are separate from this GitHub release.

The preview is owner-supplied artwork captured in the editor. MIT licensed;
see README.md for palette credits and CHANGELOG.md for release features.
