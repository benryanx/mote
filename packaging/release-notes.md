# Mote 0.2.0-rc.1 — Assistant release candidate

Second-release candidate, not a stable 2.0 release.

## New

- Prompt-first Assistant with separate connection settings.
- Omarchy default-agent bridge (Codex), Ollama and compatible HTTP API connections.
- Generated editable layers with preview, discard and one-step undo.
- Source-tab navigation and recovery into a new tab when the original changed.
- Tool-specific pointer icons and locked-layer feedback.

## Install or update

Save all artwork and close Mote first; pending previews are session-only.

```sh
git clone --branch v0.2.0-rc.1 https://github.com/benryanx/mote.git
cd mote
mise trust
mise exec -- bash scripts/install.sh
```

This is a source-build candidate. See README.md for dependencies and separate
Omarchy launcher installation. The wrapper does not update the native binary.

## Assistant and privacy

Generation is opt-in and sends prompt, dimensions and palette, not existing
artwork. API keys and connection settings stay in memory. Agent mode requires
an installed, signed-in Codex and supported CLI flags. Provider access, usage
limits and charges apply. See AI.md and SECURITY.md for bounds and trust surfaces.
AI animation and raster-image-model integration are not included.

## Validation status

All 52 Rust tests, formatting, Clippy, manifest, desktop-entry, QML and staged
install/reinstall/removal checks passed. Owner testing confirmed generation and
application. Clean-VM, complete shell lifecycle and broad provider testing remain
outstanding; no latency benchmark is claimed.

Marketplace approval is separate and pending. Keep backups of important artwork.
