#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."
cargo build --release --locked
bin_dir="${MOTE_BIN_DIR:-${HOME}/.local/bin}"
install -Dm755 target/release/mote "$bin_dir/mote"
data_dir="${XDG_DATA_HOME:-${HOME}/.local/share}"
install -Dm644 packaging/io.github.benryanx.mote.desktop "$data_dir/applications/io.github.benryanx.mote.desktop"
install -Dm644 packaging/io.github.benryanx.mote.svg "$data_dir/icons/hicolor/scalable/apps/io.github.benryanx.mote.svg"
install -Dm644 packaging/io.github.benryanx.mote.xml "$data_dir/mime/packages/io.github.benryanx.mote.xml"
update-desktop-database "$data_dir/applications"
update-mime-database "$data_dir/mime"
