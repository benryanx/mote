# First public release preparation

Public repository: `https://github.com/benryanx/mote` (owner-approved).
Version: `0.1.0`; proposed tag: `v0.1.0`, marked as a GitHub prerelease.
Permanent plugin ID: `io.github.benryanx.mote`.

The editor stays Rust. The shell integration is a small QML bar launcher.
No AI feature, model download, cloud account or background service is included.

## Prepared locally

- Root manifest, QML entry point, MIT license and install/removal documentation.
- Owner-supplied screenshot copied unchanged to `preview.png`.
- Changelog and marketplace submission draft in `packaging/marketplace-submission.md`.
- Repeatable local checks in `scripts/check.sh`.

## Required before submission

- [x] Review publication file list and scan common credential patterns (none
      found); owner confirmed screenshot/artwork publication rights. This is
      not a comprehensive security or dependency-license audit.
- [x] No identifier match in the current full registry (including retired IDs)
      or previous submission search on 2026-09-09; final allocation is subject
      to marketplace validation.
- [x] Run `bash scripts/check.sh`: 32 Rust tests, formatting, Clippy, desktop
      entry, manifest and QML validation passed locally on Omarchy 4.0.3-1.
- [x] Staged install/reinstall/removal passed using `scripts/test-packaging.sh`;
      test artwork, recovery and settings files remained. This does not replace
      the clean desktop tests below.
- [ ] Test install, launch, file association, update and uninstall in a clean
      Omarchy test account or VM; verify artwork and settings survive removal.
- [ ] Test bar click, disable/re-enable, shell restart and wrapper removal.
- [ ] Verify window layout and file chooser on the supported desktop session.
- [ ] Approve and create the public repository, review committed files, and push.
- [x] Replace README's unpublished status with actual clone/install instructions.
- [ ] Tag the reviewed commit and publish the initial prerelease with CHANGELOG notes.
- [ ] Review the exact submission body with the owner; confirm all five checklist
      statements before checking them and explicitly approving submission.
- [ ] Submit one issue and follow validation/security-baseline feedback.

No live desktop configuration is changed by the local check script. Integration
tests above must use a disposable environment, not remove the owner's installation.
Do not imply these pending tests have passed.

## Distribution and updates

Initial release preparation uses source builds with Cargo.lock. Dependencies and
Rust are downloaded during setup; ordinary editing has no network API or telemetry.
The marketplace wrapper does not install or update the executable automatically.
Request the marketplace's `manual-setup` classification and disclose the native
installation prerequisite. Never silently compile/download code on a bar click.

If prebuilt binaries are added later, build on an explicitly supported baseline,
test runtime library compatibility, and distribute checksums with versioned assets.
Do not claim arbitrary Linux portability for a binary built on this workstation.

## Preview

`preview.png` is the supplied 2560×1440 screenshot, copied without modification.
The owner confirmed rights to both the screenshot and the pictured artwork.
The marketplace handles preview optimization. No generated or retouched asset
has been substituted.

## References

- https://plugins.omarchy.org/publish.html
- https://plugins.omarchy.org/develop.html
- https://github.com/omacom/omarchy-plugin-marketplace/blob/main/SUBMISSION.md
