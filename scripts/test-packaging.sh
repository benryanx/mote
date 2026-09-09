#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."
# Never repoint HOME or touch the live desktop installation.
cargo_executable="$(mise which cargo)"
export PATH="$(dirname -- "$cargo_executable"):$PATH"
test_root="$(mktemp -d /tmp/mote-packaging.XXXXXX)"
export MOTE_BIN_DIR="$test_root/bin"
export XDG_DATA_HOME="$test_root/data"
mkdir -p "$test_root/artwork" "$test_root/state/mote" "$test_root/config/mote"
touch "$test_root/artwork/keep.mote" "$test_root/state/mote/recovery.mote" "$test_root/config/mote/settings"
bash scripts/install.sh
cmp target/release/mote "$MOTE_BIN_DIR/mote"
test -f "$XDG_DATA_HOME/applications/io.github.benryanx.mote.desktop"
test -f "$XDG_DATA_HOME/icons/hicolor/scalable/apps/io.github.benryanx.mote.svg"
test -f "$XDG_DATA_HOME/mime/packages/io.github.benryanx.mote.xml"
desktop-file-validate "$XDG_DATA_HOME/applications/io.github.benryanx.mote.desktop"
bash scripts/install.sh
bash scripts/uninstall.sh
test ! -e "$MOTE_BIN_DIR/mote"
test ! -e "$XDG_DATA_HOME/applications/io.github.benryanx.mote.desktop"
test ! -e "$XDG_DATA_HOME/icons/hicolor/scalable/apps/io.github.benryanx.mote.svg"
test ! -e "$XDG_DATA_HOME/mime/packages/io.github.benryanx.mote.xml"
test -f "$test_root/artwork/keep.mote"
test -f "$test_root/state/mote/recovery.mote"
test -f "$test_root/config/mote/settings"
echo "Staged install, reinstall and removal passed. Test files retained at $test_root"
