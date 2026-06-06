#!/usr/bin/env bash
# Open Flutter DevTools for the running app (widget inspector, logging, network, etc.).
#
# Resolves the VM Service URI from (in order):
#   1. First CLI argument (full ws:// or http:// URI)
#   2. .zed/flutter/vmservice.json  (written by zed-flutter-logs.sh or debug sessions)
#   3. .zed/flutter/vmservice_uri   (legacy single-line file)
#   4. .zed/flutter/run.log         (parsed from recent `flutter run` output)
#
# Usage:
#   scripts/zed-flutter-devtools.sh [screen]
#   scripts/zed-flutter-devtools.sh inspector
#   scripts/zed-flutter-devtools.sh logging
#   scripts/zed-flutter-devtools.sh ws://127.0.0.1:12345/abc=/ inspector
#
# Screens: home, inspector, logging, debugger, network, memory, performance, cpu-profiler

set -euo pipefail

SCREEN="home"
VM_URI=""
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

use_fvm=0
if [[ -d ".fvm" ]] && command -v fvm >/dev/null 2>&1; then
  use_fvm=1
fi

dart_cmd() {
  if [[ "$use_fvm" -eq 1 ]]; then
    fvm dart "$@"
  else
    dart "$@"
  fi
}

open_url() {
  local url="$1"
  if command -v xdg-open >/dev/null 2>&1; then
    xdg-open "$url" >/dev/null 2>&1 || true
  elif command -v open >/dev/null 2>&1; then
    open "$url" || true
  else
    echo "$url"
  fi
}

looks_like_uri() {
  [[ "$1" == ws://* || "$1" == http://* || "$1" == https://* ]]
}

known_screens="home inspector logging debugger network memory performance cpu-profiler"

if [[ $# -ge 1 ]]; then
  if looks_like_uri "$1"; then
    VM_URI="$1"
    shift
    SCREEN="${1:-home}"
  elif [[ " $known_screens " == *" $1 "* ]]; then
    SCREEN="$1"
  else
    echo "Unknown screen or URI: $1" >&2
    echo "Screens: $known_screens" >&2
    exit 1
  fi
fi

read_json_uri() {
  local file="$1"
  [[ -f "$file" ]] || return 1
  python3 - "$file" <<'PY'
import json, sys
from pathlib import Path
p = Path(sys.argv[1])
raw = p.read_text(encoding="utf-8").strip()
if not raw:
    sys.exit(1)
try:
    data = json.loads(raw)
    uri = data.get("uri") or data.get("vmServiceUri")
    if isinstance(uri, str) and uri:
        print(uri)
        sys.exit(0)
except json.JSONDecodeError:
    pass
if raw.startswith("ws://") or raw.startswith("http://"):
    print(raw)
PY
}

parse_run_log_uri() {
  local log="${1:-.zed/flutter/run.log}"
  [[ -f "$log" ]] || return 1
  python3 - "$log" <<'PY'
import re, sys
from pathlib import Path
text = Path(sys.argv[1]).read_text(encoding="utf-8", errors="replace")
# Prefer explicit VM Service line from flutter run
patterns = [
    r"A Dart VM Service on .* is available at:\s*(\S+)",
    r"An Observatory debugger and profiler on .* is available at:\s*(\S+)",
    r'"vmServiceUri"\s*:\s*"([^"]+)"',
    r"(ws://127\.0\.0\.1:\d+/[^\s\"']+)",
    r"(http://127\.0\.0\.1:\d+/[^\s\"']+=)",
]
for pat in patterns:
    m = re.findall(pat, text)
    if m:
        print(m[-1].rstrip(".,)"))
        sys.exit(0)
sys.exit(1)
PY
}

cache_vmservice_json() {
  local uri="$1"
  mkdir -p .zed/flutter
  python3 - "$uri" <<'PY'
import json, sys
from pathlib import Path
uri = sys.argv[1]
Path(".zed/flutter").mkdir(parents=True, exist_ok=True)
Path(".zed/flutter/vmservice.json").write_text(
    json.dumps({"uri": uri, "vmServiceUri": uri}, indent=2) + "\n",
    encoding="utf-8",
)
Path(".zed/flutter/vmservice_uri").write_text(uri + "\n", encoding="utf-8")
PY
}

resolve_vm_uri() {
  if [[ -n "$VM_URI" ]]; then
    echo "$VM_URI"
    return 0
  fi
  local candidate=""
  candidate="$(read_json_uri ".zed/flutter/vmservice.json" 2>/dev/null || true)"
  if [[ -n "$candidate" ]]; then
    echo "$candidate"
    return 0
  fi
  if [[ -f .zed/flutter/vmservice_uri ]]; then
    candidate="$(tr -d '\n' < .zed/flutter/vmservice_uri)"
    if [[ -n "$candidate" ]]; then
      echo "$candidate"
      return 0
    fi
  fi
  candidate="$(parse_run_log_uri ".zed/flutter/run.log" 2>/dev/null || true)"
  if [[ -n "$candidate" ]]; then
    cache_vmservice_json "$candidate"
    echo "$candidate"
    return 0
  fi
  return 1
}

if ! command -v dart >/dev/null 2>&1 && [[ "$use_fvm" -eq 0 ]]; then
  echo "dart not found on PATH. Install Flutter/Dart or use FVM (.fvm/)." >&2
  exit 1
fi

VM_URI="$(resolve_vm_uri)" || {
  cat >&2 <<EOF
Could not find a VM Service URI.

Start a Flutter debug session or run:
  flutter run --debug 2>&1 | tee .zed/flutter/run.log

Then run this script again, or pass the URI explicitly:
  bash scripts/zed-flutter-devtools.sh ws://127.0.0.1:PORT/token=/ inspector

Tip: run task "Flutter: cache VM service from run log" after flutter run.
EOF
  exit 1
}

cache_vmservice_json "$VM_URI"

PORT="${ZED_FLUTTER_DEVTOOLS_PORT:-9100}"

# Prefer dart devtools CLI for home screen (starts server + browser).
if [[ "$SCREEN" == "home" ]] && dart_cmd devtools --help 2>/dev/null | grep -q vm-service-uri; then
  echo "Opening DevTools for $VM_URI"
  exec dart_cmd devtools --vm-service-uri="$VM_URI" --port "$PORT"
fi

encoded="$(python3 -c 'import sys, urllib.parse; print(urllib.parse.quote(sys.argv[1], safe=""))' "$VM_URI")"
if [[ "$SCREEN" == "home" ]]; then
  url="http://127.0.0.1:${PORT}/?uri=${encoded}"
else
  url="http://127.0.0.1:${PORT}/${SCREEN}?uri=${encoded}"
fi

echo "DevTools URL ($SCREEN):"
echo "  $url"
echo
echo "If the server is not running yet, start it in another terminal:"
echo "  dart devtools --port ${PORT}"
echo "  # or task: Flutter: DevTools server (port ${PORT})"

open_url "$url"
