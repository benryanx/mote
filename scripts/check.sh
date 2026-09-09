#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."
bash -n scripts/install.sh scripts/uninstall.sh scripts/check.sh scripts/test-packaging.sh
mise exec -- cargo fmt --all --check
mise exec -- cargo test --locked
mise exec -- cargo clippy --all-targets --locked -- -D warnings
desktop-file-validate packaging/io.github.benryanx.mote.desktop
omarchy plugin validate .
# Quickshell provides the qs import prefix at runtime. Reproduce that import
# root for standalone qmllint, outside the plugin (which must not have symlinks).
qml_check_dir="$(mktemp -d /tmp/mote-qml-check.XXXXXX)"
trap 'unlink "$qml_check_dir/qs"; rmdir "$qml_check_dir"' EXIT
ln -s /usr/share/omarchy/shell "$qml_check_dir/qs"
if command -v qmllint >/dev/null 2>&1; then
  qmllint -I "$qml_check_dir" BarWidget.qml
elif [[ -x /usr/lib/qt6/bin/qmllint ]]; then
  /usr/lib/qt6/bin/qmllint -I "$qml_check_dir" BarWidget.qml
else
  echo 'Missing qmllint: install Qt declarative development tools before release.' >&2
  exit 1
fi
