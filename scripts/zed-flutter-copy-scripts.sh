#!/usr/bin/env bash
# Copy zed-flutter helper scripts into the current Flutter project.
# Called by zed-flutter-init.sh; can also run standalone.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${1:-scripts}"

mkdir -p "$DEST"
for name in \
  zed-flutter-bootstrap.sh \
  zed-flutter-devtools.sh \
  zed-flutter-logs.sh \
  zed-flutter-hot.sh \
  zed-flutter-pick-device.sh
do
  if [[ -f "$REPO_ROOT/scripts/$name" ]]; then
    cp "$REPO_ROOT/scripts/$name" "$DEST/$name"
    chmod +x "$DEST/$name"
  fi
done
echo "Installed helper scripts → $DEST/"
