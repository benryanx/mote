#!/usr/bin/env bash
set -euo pipefail
data_dir="${XDG_DATA_HOME:-${HOME}/.local/share}"
bin_dir="${MOTE_BIN_DIR:-${HOME}/.local/bin}"
rm -f -- "$bin_dir/mote" \
  "$data_dir/applications/io.github.benryanx.mote.desktop" \
  "$data_dir/icons/hicolor/scalable/apps/io.github.benryanx.mote.svg" \
  "$data_dir/mime/packages/io.github.benryanx.mote.xml"
update-desktop-database "$data_dir/applications"
update-mime-database "$data_dir/mime"
echo 'Mote removed. Artwork and recovery files have been preserved.'
