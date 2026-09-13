# Changelog

## 0.2.0-alpha.1 — development (unreleased)

- Omarchy default-agent connection (Codex), reusing login with restricted ephemeral
  CLI runs, preview validation and owned-process cancellation.
- Optional Assistant window with Ollama and OpenAI-compatible JSON chat adapters.
- Prompt, canvas dimensions and palette context; no existing artwork upload.
- Bounded drawing-data validation and pixel-exact new-layer rendering.
- Preview/discard/apply with one-step undo and stale/wrong-tab protection.
- Background requests, time/response limits and discard-on-cancel behavior.
- Session-only credentials; HTTPS except loopback; no automatic retries or redirects.

## 0.1.0 — initial public alpha (2026-09-09)

- Native Rust pixel-art editor with Omarchy theme integration and dockable panels.
- Multiple document tabs, layers, frames, onion skin and animation playback.
- Pencil, eraser, fill, picker, shapes, selection and dithered gradients.
- Palette presets, custom palettes, color opacity and palette ramps.
- Canvas resizing and nearest-neighbor sprite resizing.
- Customizable keyboard shortcuts and undo/redo.
- PNG, JPEG, WebP, BMP, TGA and GIF export, sprite sheets and image sequences.
- Background file dialogs to keep the editor responsive.
- Freehand strokes track outside the canvas and resume on re-entry.
- Optional Omarchy bar launcher, desktop entry, icon and project MIME association.

AI-assisted generation is planned for a later version and is not included.
This is an alpha, not Aseprite feature parity. See ROADMAP.md and README.md
for limitations, including document-session persistence.
