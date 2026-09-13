# Mote second-release candidate

Version: **0.2.0-rc.1**; tag: **v0.2.0-rc.1**.
Repository: https://github.com/benryanx/mote
Plugin ID: `io.github.benryanx.mote`.

The Assistant is optional. The native editor and launcher remain separate.
See AI.md and SECURITY.md for network/process capabilities and limitations.

## Release gates

Run `bash scripts/check.sh` and `bash scripts/test-packaging.sh`.
The first covers Rust tests, formatting, Clippy, desktop entry, manifest and QML.
The second builds and tests staged install/reinstall/removal without changing the
live application. Preserve artwork and settings in the staged tests.

On 2026-09-14 all 52 Rust tests, formatting, Clippy, desktop-entry, manifest and
QML checks passed on Omarchy 4.0.3-1. Staged install/reinstall/removal also passed,
preserving test artwork and settings. Loopback HTTP fixture tests required an
unsandboxed rerun; the sandbox denied socket binding, not an application failure.

Manual owner testing confirmed API/agent generation, previews and application.
No latency benchmark, broad agent/provider matrix, clean-VM installation or full
shell enable/disable/restart lifecycle test is claimed. Open tabs and pending
previews are not restored across restarts.

## Publication sequence

1. Integrate upstream main, commit all reviewed release files, and run checks.
2. Push the exact reviewed commit to main and tag the release candidate.
3. Prepare a draft GitHub prerelease using packaging/release-notes.md.
4. Update marketplace submission #5900 to disclose Assistant capabilities and
   request new validation/security-baseline checks for that exact full SHA.
5. Wait for maintainer review; do not apply approval labels or claim approval.
   Keep main unchanged while the immutable snapshot is under review.

The prior marketplace review was blocked because its validated 0.1.0 SHA no
longer matched main after README updates. Revalidation must cover the final
candidate, not the earlier snapshot.

## Distribution

Source build with Cargo.lock; no prebuilt portability claims. The marketplace
wrapper does not install or update the native executable. Request manual-setup
classification. Installation and removal preserve user artwork and settings.
