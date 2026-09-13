# Security and reviewer notes

Mote 0.2 adds optional outbound generation to the native editor. The QML plugin
remains a launcher only. No server, automatic model download, telemetry or
background system service is installed.

## New capabilities

- User-approved HTTP requests send prompt, dimensions, palette and drawing-format
  instructions. Existing artwork, document paths and layer names are not included.
- API credentials stay in process memory and are not stored in projects or settings.
  HTTPS is required except loopback HTTP. URLs with credentials, query strings or
  fragments are rejected; redirects and automatic proxy discovery are disabled.
- HTTP connection/total timeouts are 10/120 seconds; response bodies are limited
  to 2 MB. Cancel discards HTTP results but cannot guarantee remote cancellation.
- Agent mode runs installed `omarchy-default-agent` and, for Codex, `codex exec`
  via argument arrays, with the prompt on stdin. It reuses the installed login.
  User config/rules are ignored; read-only sandboxing, disabled shell tools/apps/
  web search and ephemeral sessions are requested in a private temporary folder.
  This is not an OS sandbox around the CLI itself: installed executables, managed
  policies and provider behavior remain trust boundaries. Read-only does not
  mean filesystem-inaccessible. No claim of complete agent isolation is made.
- Agent output and duration are bounded. Cancellation kills only the process
  group Mote launches. Remote processing or billing may still continue.
- Returned artwork is strict bounded JSON, not executed code. Coordinates,
  palette indices, layer counts and pixel work are validated before preview.
  Apply requires the unchanged source document and supports undo. A separate
  recovery action opens the full preview in a new tab without overwriting work.

## Installation scope

Source builds download pinned Cargo dependencies. The per-user installer writes
the Mote executable, desktop entry, icon and MIME registration. It uses no sudo
and does not change desktop configuration. The optional plugin must be added
separately. Uninstall preserves artwork, preferences, palettes and recovery files.

## Reporting and limits

Use the repository's private vulnerability reporting if available; otherwise
open a minimal issue requesting a private contact, without credentials, private
artwork or exploit details. Automated checks are not a security certification.
Clean-VM and broad provider compatibility testing remain outstanding.
