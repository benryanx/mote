### Repository URL

https://github.com/benryanx/mote

### Category

Productivity

### Tags

launcher, media, quickshell

### Suggest a missing tag

_No response_

### Maintainer notes

Mote is a native Rust pixel-art and animation editor. This plugin supplies its
Omarchy bar launcher; the editor runs in a separate native window, not inside
the shell. Please classify it as manual-setup: users must build and install the
native executable using the documented script before using the bar button.
Adding/updating the plugin does not build or update the executable.

The application includes layers, frames, document tabs, customizable shortcuts,
palettes, gradients and image/animation exports. Version 0.2.0-rc.1 adds an optional
Assistant: user-approved outbound HTTP (Ollama/compatible API) or an installed
Codex CLI subprocess using the existing login. Only prompt, canvas dimensions
and palette are sent; credentials are session-only. Generated JSON is bounded,
validated, previewed and applied as undoable layers. See AI.md and SECURITY.md
for timeout, cancellation, endpoint restrictions and agent trust boundaries.
No inbound listener, telemetry or background service is installed.
Build dependencies require downloads; file dialogs use the
desktop portal. Installation is per-user without sudo or desktop config edits.
Application and plugin removal are documented separately and preserve artwork.

### Submission checklist

- [ ] The repository is public and contains installation and removal instructions.
- [ ] I have documented the plugin license and any external dependencies.
- [ ] I confirm that I own or have permission to submit this plugin and its preview assets.
- [ ] The plugin does not overwrite user configuration without explicit consent.
- [ ] I understand that approval is for listing and is not a security review.
