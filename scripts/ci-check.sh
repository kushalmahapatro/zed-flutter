#!/usr/bin/env bash
# Local / CI: validate example JSON, shell scripts, Rust tests, wasm build, and bootstrap output.
# Run from repository root: ./scripts/ci-check.sh
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

die() { echo "ci-check: $*" >&2; exit 1; }

echo "==> Validating example JSON (strict json files only; keymap may be JSONC)"
for f in \
  "$REPO_ROOT/examples/zed-debug.example.json" \
  "$REPO_ROOT/examples/zed-tasks.example.json" \
  "$REPO_ROOT/examples/zed-flutter-devices.example.json"
do
  [[ -f "$f" ]] || continue
  python3 -m json.tool "$f" >/dev/null || die "invalid JSON: $f"
done

echo "==> bash -n helper scripts"
for s in "$REPO_ROOT"/scripts/*.sh; do
  [[ -f "$s" ]] || continue
  bash -n "$s" || die "bash -n failed: $s"
done

echo "==> cargo test"
cargo test

echo "==> cargo build (wasm32-wasip1)"
if command -v rustup >/dev/null 2>&1; then
  rustup target add wasm32-wasip1 2>/dev/null || true
fi
cargo build --release --target wasm32-wasip1

echo "==> Bootstrap script smoke (temp Flutter-like tree)"
SMOKE="$(mktemp -d)"
trap 'rm -rf "$SMOKE"' EXIT

MIN_PUBSPEC='name: ci_smoke
version: 0.0.0
environment:
  sdk: ">=3.0.0 <4.0.0"
'
write_pubspec() { printf '%s\n' "$MIN_PUBSPEC" > "$1"; }

# Single-package project
mkdir -p "$SMOKE/root"
write_pubspec "$SMOKE/root/pubspec.yaml"
(
  cd "$SMOKE/root"
  "$REPO_ROOT/scripts/zed-flutter-bootstrap.sh" --fvm off
  python3 -m json.tool .zed/tasks.json >/dev/null
  python3 -m json.tool .zed/debug.json >/dev/null

  "$REPO_ROOT/scripts/zed-flutter-bootstrap.sh" --fvm off --full
  python3 -c "import json; d=json.load(open('.zed/tasks.json')); assert 'flutter_doctor' in d"
  python3 -c "import json; d=json.load(open('.zed/debug.json')); assert any(c.get('device_id')=='chrome' for c in d)"
)

# Monorepo: root + apps/mobile, then melos-wrapped generation
mkdir -p "$SMOKE/mono/apps/mobile"
write_pubspec "$SMOKE/mono/pubspec.yaml"
write_pubspec "$SMOKE/mono/apps/mobile/pubspec.yaml"
(
  cd "$SMOKE/mono"
  "$REPO_ROOT/scripts/zed-flutter-bootstrap.sh" --fvm off \
    --flavors dev,prod --targets dev:lib/main_dev.dart,prod:lib/main_prod.dart \
    --package-path apps/mobile
  python3 -m json.tool .zed/tasks.json >/dev/null
  python3 -c "import json; d=json.load(open('.zed/tasks.json')); assert d['flutter_run_debug']['cwd'] == r'\$ZED_WORKTREE_ROOT/apps/mobile'"

  "$REPO_ROOT/scripts/zed-flutter-bootstrap.sh" --fvm off \
    --melos --package-path apps/mobile --package-name my_pkg
  python3 -m json.tool .zed/tasks.json >/dev/null
  python3 -c "import json; d=json.load(open('.zed/tasks.json')); c=d['melos_bootstrap']['command']; assert c.startswith('melos ')"
  python3 -c "import json; d=json.load(open('.zed/tasks.json')); assert 'melos exec' in d['flutter_run_debug']['command']"
)

echo "==> DevTools / log script smoke"
SMOKE_LOG="$(mktemp -d)"
mkdir -p "$SMOKE_LOG/proj/.zed/flutter"
printf '%s\n' 'name: ci_smoke
version: 0.0.0
environment:
  sdk: ">=3.0.0 <4.0.0"
' > "$SMOKE_LOG/proj/pubspec.yaml"
cat >"$SMOKE_LOG/proj/.zed/flutter/run.log" <<'EOF'
Launching lib/main.dart on Linux in debug mode...
A Dart VM Service on Linux is available at: http://127.0.0.1:4321/abc123=/
I/flutter (12345): App started
E/flutter (12345): Exception: test failure
EOF
(
  cd "$SMOKE_LOG/proj"
  mkdir -p scripts
  cp "$REPO_ROOT/scripts/zed-flutter-devtools.sh" scripts/
  cp "$REPO_ROOT/scripts/zed-flutter-logs.sh" scripts/
  chmod +x scripts/zed-flutter-devtools.sh scripts/zed-flutter-logs.sh
  bash scripts/zed-flutter-logs.sh cache-uri >/dev/null
  test -f .zed/flutter/vmservice.json
  bash scripts/zed-flutter-logs.sh export-levels error >/dev/null
  test -s .zed/flutter/filtered.log
  bash scripts/zed-flutter-logs.sh filter 'App started' | grep -q 'App started'
)

echo "ci-check: OK"
