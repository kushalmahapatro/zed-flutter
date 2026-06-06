#!/usr/bin/env bash
# One-shot Flutter + Zed setup for a project.
#
# Copies starter keymap, optional device policy template, and generates
# .zed/tasks.json + .zed/debug.json (with --full example presets).
#
# Usage (from your Flutter project root):
#   bash /path/to/zed-flutter/scripts/zed-flutter-init.sh
#   bash /path/to/zed-flutter/scripts/zed-flutter-init.sh --flavors dev,prod --targets dev:lib/main_dev.dart,prod:lib/main_prod.dart
#
# Forwards extra args to zed-flutter-bootstrap.sh (e.g. --fvm on, --melos, --package-path).

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

if [[ ! -f "pubspec.yaml" ]]; then
  echo "Run from your Flutter project root (pubspec.yaml not found)." >&2
  exit 1
fi

mkdir -p .zed

bash "$REPO_ROOT/scripts/zed-flutter-copy-scripts.sh" scripts

if [[ ! -f .zed/keymap.json ]]; then
  cp "$REPO_ROOT/examples/zed-keymap.example.json" .zed/keymap.json
  echo "Copied .zed/keymap.json"
fi

if [[ ! -f .zed/flutter_devices.json ]]; then
  cp "$REPO_ROOT/examples/zed-flutter-devices.example.json" .zed/flutter_devices.json
  echo "Copied .zed/flutter_devices.json (edit default_device_id for your machine)"
fi

"$REPO_ROOT/scripts/zed-flutter-bootstrap.sh" --full --detect-flavors "$@"

if [[ ! -f .gitignore ]] || ! grep -q '.zed/flutter/run.log' .gitignore 2>/dev/null; then
  echo "Tip: append examples/zed-gitignore.example entries to .gitignore for local log/device files"
fi

echo
echo "Zed Flutter init complete."
echo "  - Install the Dart extension in Zed"
echo "  - Install this repo as a dev extension (Extensions → Install Dev Extension)"
echo "  - Run tasks via task: spawn; debug via debugger: start"
echo "  - DevTools: Shift+D (connected), Shift+I (inspector), Shift+L (logging)"
echo "  - Hot reload/restart: Alt+Shift+R / Alt+Shift+T (after app is running)"
echo "  - Logs: Shift+R (tee), Shift+W (watch), Shift+E (export errors)"
