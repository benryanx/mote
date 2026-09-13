# Mote

A little canvas. Endless possibilities.

Development branch: **0.2.0-alpha.1**, adding optional AI generation. The public
`v0.1.0` release remains unchanged and does not contain this feature.

![Mote editing a layered pixel-art flower, with tools, palette, navigator and animation frames](preview.png)

Mote is a fresh Rust pixel art and animation editor for Omarchy. The interface
puts the canvas first: tools and palette on the left, navigator on the right,
and layers beside frames below. Every panel has a draggable dock tab and resizable
dividers; drag tabs onto docking targets or out into floating panels. The layout
is saved between launches. View → Reset workspace layout restores the default.
It uses eframe/egui with native Wayland/X11 windows and
OpenGL rendering. The optional Quickshell bar widget launches the same binary.

This is an early working alpha, not Aseprite feature parity. It was developed
independently of OmaPixel. See [ROADMAP.md](ROADMAP.md) for the remaining scope.

## Run

From the repository directory:

```sh
mise exec -- cargo run --locked
# Open artwork directly:
mise exec -- cargo run --locked -- path/to/art.mote
```

Build dependencies: Rust 1.98, a C compiler, pkg-config, Wayland/X11 development
libraries. Runtime: libGL/EGL, libxkbcommon, Wayland or X11, and a working desktop
file chooser portal (xdg-desktop-portal plus a backend). Omarchy provides the
desktop runtime. Cargo downloads the dependencies pinned in Cargo.lock.

## Implemented

- Experimental Assistant: prompt-to-art using the Omarchy default agent (Codex),
  Ollama or an OpenAI-compatible Chat Completions endpoint. Preview/discard/apply on new layers with one undo
  step. See [AI.md](AI.md) for setup, privacy, supported limits and testing status.
- Native file choosers and their save/open/export work run in the background,
  keeping the window responsive while browsing folders. Editing pauses until
  the operation finishes; canceled or failed saves keep unsaved work intact.
- New/Open creates document tabs without replacing current work. Each tab keeps
  its artwork, undo/redo, frame/layer selection, palette, colors, zoom and pan.
  Save, Save As, export and resize act on the active tab only. Close a tab with
  its × button or Ctrl+W; Ctrl+Tab / Ctrl+Shift+Tab cycles documents. Closing a
  modified tab prompts to save; quitting checks every modified document.
  Open supports selecting multiple files. Reopening an already-open `.mote`
  project selects its tab. Document tabs last for the current session; restart
  restores the workspace layout, not open document sessions.
- Gradient tool (D or Shift+G): drag from foreground to background color using
  linear or radial gradients. Choose smooth RGBA interpolation or Bayer 2×2,
  4×4 or 8×8 two-color dithering. Applies to the active cel inside the selection,
  or the whole canvas when unselected. Locked layers are respected. A drag is
  one undo step; brush size and symmetry do not affect gradient fills.
- Palette ramps: select foreground/background, choose 2–32 colors in the palette
  panel and Add ramp to palette. Ramps include endpoints and interpolate alpha;
  save them through the existing custom-palette library.
- Pencil, eraser, contiguous flood fill, eyedropper, lines, rectangles and ellipses.
- Brush sizes, horizontal/vertical symmetry, filled shapes, foreground/background colors.
- Color opacity from 0–100%, synchronized with the color picker and sampled alpha.
- File → Canvas Size expands/crops all layers and frames, with a nine-position
  anchor and transparent padding. File → Sprite Size scales all artwork using
  nearest-neighbor sampling, with aspect-ratio locking and 50–400% presets.
  For example, use 200% to scale 50×50 artwork to 100×100 before exporting.
  Both operations are undoable and include hidden/locked layers. Shrinking the
  canvas crops pixels; shrinking the sprite samples fewer pixels. The existing
  2048-pixel side and total stored-pixel limits apply.
- Rectangular selection, internal copy/paste, delete and horizontal/vertical flip.
- RGBA layers, visibility, opacity, locks, rename and stacking order.
- Duplicate/delete animation frames, individual frame timing, playback and previous-frame onion skin.
- Right-click frames to duplicate, insert blank frames, move earlier/later, delete
  or set timing. Frame actions support undo/redo.
- Nearest-neighbor zoom, pan, pixel grid, tile preview and navigator.
- Editable project palette, atomic `.mote` saves, PNG import and configurable image/animation export.
- Preset palettes: Sweetie 16, ENDESGA 32, Mote 24, Google-inspired 8, retro
  handheld 4 and grayscale 16. These are color counts, not image bit depths.
- Create a palette with New palette, add foreground colors, name it and Save to
  my palettes. Custom palettes are stored atomically in
  `$XDG_DATA_HOME/mote/palettes.json` (default `~/.local/share/mote/palettes.json`).
  Use a new name to save a variant. Right-click swatches to select the background
  color and access replace/remove commands. Existing artwork stays unchanged
  when switching palettes; project palette edits support undo.
- Bounded undo/redo history and unsaved-change prompts for open/new/close.
- Recovery snapshots every 30 seconds for each modified document, including
  inactive tabs, with a separate process/document ID in each filename under
  `$XDG_STATE_HOME/mote/` (default `~/.local/state/mote/`). Open the desired
  `recovery-*.mote` file through File → Open. Snapshots remain after exit; they
  are a recovery aid, not a versioned backup system.
- Automatic Omarchy light/dark theme refresh every two seconds. Reads
  `$XDG_STATE_HOME/omarchy/current/theme/colors.toml`, falling back to the older
  `$XDG_CONFIG_HOME/omarchy/current/theme/colors.toml` location and then Mote dark.
  Theme colors affect the interface, never the artwork palette.

## Controls

All command/tool keyboard assignments are editable in **Edit → Keyboard
Shortcuts** (default Ctrl+Alt+Shift+K; F1 also opens the searchable guide).
Click a binding and press the desired key with Ctrl, Alt and/or Shift. Each
command has up to two bindings. Clear removes both; conflicts are rejected with
the existing command identified. Restore defaults resets the map. Import/Export
shares validated JSON shortcut profiles. Assignments apply immediately and are
saved with application preferences on exit. Menus and tool labels show current
bindings. Text fields keep their normal editing behavior. The defaults below are
starting points; compositor-owned shortcuts may be intercepted by the desktop.
Mouse-wheel/pan gestures and temporary Alt eyedropper behavior are not remappable.

| Action | Control |
| --- | --- |
| New / duplicate layer | Shift+N / Ctrl+Shift+J |
| New blank frame | Alt+Shift+N |
| Export As | Ctrl+Alt+Shift+S |
| Keyboard Shortcuts | Ctrl+Alt+Shift+K or F1 |
| Gradient | D or Shift+G |
| Close / next / previous document | Ctrl+W / Ctrl+Tab / Ctrl+Shift+Tab |
| Pencil / eraser / fill / picker | B / E / G / I |
| Line / rectangle / ellipse / selection | L / U / O / M |
| Sample color / swap colors | Alt+click / X |
| Background color | Right mouse button |
| Zoom / pan | Scroll / middle drag |
| Undo / redo | Ctrl+Z / Ctrl+Shift+Z or Ctrl+Y |
| Duplicate frame / step frames | Alt+N / Left and Right arrows |
| Copy / paste | Ctrl+C / Ctrl+V |
| Deselect / clear | Ctrl+D or Escape / Delete |
| Play animation | Space |
| New / open / save / save as | Ctrl+N / Ctrl+O / Ctrl+S / Ctrl+Shift+S |
| Help | F1 |

## Export As

**File → Export As** exports only the active document and remembers its options.
The quick PNG, sheet and GIF menu entries open the same dialog with those modes
selected. Saving a `.mote` project remains separate from exporting a flattened copy.

- Formats: PNG, JPEG/JPG, lossless WebP, BMP, TGA and GIF.
- Output: one current frame, a grid sprite sheet, a numbered image sequence,
  or GIF animation.
- All/current/range frame selection, forward/reverse/ping-pong ordering, and GIF loop control.
- Nearest-neighbor scaling from 1× to 16× without resizing the project.
- Visible composite or active layer only (including a hidden active layer).
- Transparent-border trim uses shared bounds across frames to avoid animation jitter.
- Sprite-sheet columns and padding in output pixels.
- JPEG quality and a selectable opaque background for JPEG/BMP; optional flattening for other formats.
- Output dimensions and validation before exporting. Maximum 16,384 pixels per
  side and 64 million total output pixels; reduce scale/range for larger exports.

PNG/WebP/TGA retain alpha. GIF has limited colors, binary transparency and 10 ms
timing precision; WebP export is static/lossless, not animated WebP. JPEG is lossy.
Sequence export creates a new `mote-frames-*` subfolder in the chosen parent,
containing `frame-0001.ext`, subsequent frames and `frames.json` timing metadata.
The complete sequence is staged before success is reported; existing sequences
are not overwritten. Single-file outputs use atomic replacement. Choose a matching
filename extension in the save dialog. This does not implement video export,
packed-atlas metadata, non-square pixel ratios or social-network-specific fixes.

Copy/paste is internal to Mote and affects the active cel. Paste starts at the
selection's top-left, or the canvas origin if no selection exists. Fill currently
requires deselection. Onion skin shows the preceding frame. GIF reduces color and
alpha precision according to that format. Project limits: 2048 pixels per side,
128 layers, 1024 frames, 16 million stored pixels total and 160 MB project input.

## Install the native application

Install the initial alpha from its versioned source:

```sh
git clone --branch v0.1.0 https://github.com/benryanx/mote.git
cd mote
mise trust
mise exec -- bash scripts/install.sh
```

The installer compiles a release binary and installs it in `~/.local/bin`, with
a desktop entry, SVG icon and project MIME type under `$XDG_DATA_HOME`. It needs
no root access and does not modify desktop configuration. Ensure `~/.local/bin`
is in the desktop session PATH. Launch **Mote** from the application menu.

Build setup also requires `desktop-file-utils` and `shared-mime-info` for the
desktop and MIME registration commands. The build downloads pinned Cargo
dependencies; mise may download the Rust toolchain. Review the scripts first.
The installer replaces an existing user-installed Mote binary and its launcher
assets; save work and close Mote before installing an update.

To remove the application (artwork and recovery files are preserved):

```sh
bash scripts/uninstall.sh
```

## Optional Omarchy plugin

The repository root contains the Quattro manifest and bar launcher. **Install
the native binary first** using the steps above. Adding only the QML plugin
does not compile or install Mote.

After installing the native application:

```sh
omarchy plugin add https://github.com/benryanx/mote.git --enable
```

This is a **manual-setup plugin**, not a one-click binary installer. The native
editor and shell wrapper have separate update/removal steps. After obtaining
a newer source release, rerun `mise exec -- bash scripts/install.sh` to update
the editor. For a git-managed wrapper,
`omarchy plugin update io.github.benryanx.mote` updates the wrapper only; it does
not rebuild Rust. Install the native application from a reviewed release tag;
do not assume the wrapper's branch head is the same version as the executable.

To test locally after installing the binary, copy `manifest.json` and
`BarWidget.qml` into `~/.config/omarchy/plugins/io.github.benryanx.mote/`, then:

```sh
omarchy plugin validate ~/.config/omarchy/plugins/io.github.benryanx.mote
omarchy plugin enable io.github.benryanx.mote
```

Click the bar icon to launch a window. Remove the wrapper with
`omarchy plugin remove io.github.benryanx.mote`. The wrapper executes `mote` from
PATH, inherits shell theme styling and runs no background service. The editor
uses no listener, browser or webview. Optional AI generation sends an outbound
request only after explicit user consent and Generate. It reads theme files, accesses
user-chosen artwork and writes recovery snapshots. File dialogs use the desktop
portal. No telemetry or remote code execution is implemented.

Contract references: [Omarchy plugin development](https://plugins.omarchy.org/develop.html)
and [marketplace submission requirements](https://github.com/omacom/omarchy-plugin-marketplace/blob/main/SUBMISSION.md).

Release preparation and remaining checks: [RELEASE.md](RELEASE.md).

## Verify

```sh
mise exec -- cargo fmt --all --check
mise exec -- cargo test --locked
mise exec -- cargo clippy --all-targets --locked -- -D warnings
omarchy plugin validate .
```

MIT licensed. Mote's name is a working project name; trademark and marketplace
ID availability have not been cleared for publication.

## Design references and palette credits

The draggable workspace and frame workflow draw on
[Aseprite workspace](https://www.aseprite.org/docs/workspace/),
[workspace layout](https://www.aseprite.org/docs/workspace-layout/),
[timeline](https://www.aseprite.org/docs/timeline/) and
[color bar](https://www.aseprite.org/docs/color-bar/) guidance.
This is an independent implementation with Omarchy theme colors.

Gradient research: Aseprite's [context bar](https://www.aseprite.org/docs/context-bar/)
documents linear/radial gradients and Bayer 2×2, 4×4 and 8×8 dithering.
Its [palette tutorial](https://www.aseprite.org/docs/tutorial/color-bar-tutorial/)
also describes palette gradients. Mote implements these core concepts; it does
not yet implement Aseprite's gradient fill tolerance, connected-region options,
indexed-color behavior or simultaneous views of multiple documents.

The searchable shortcut editor and export options were informed by Aseprite's
[keyboard shortcut guide](https://www.aseprite.org/docs/keyboard-shortcuts/) and
[exporting guide](https://www.aseprite.org/docs/exporting/).

[Sweetie 16](https://lospec.com/palette-list/sweetie-16) is by GrafxKid;
[ENDESGA 32](https://lospec.com/palette-list/endesga-32) is by ENDESGA.
The Google-inspired set combines familiar brand hues and neutral colors; it is
not an official Google Pixel device palette or a Material You wallpaper palette.
Retro handheld and grayscale presets are descriptive color sets.
