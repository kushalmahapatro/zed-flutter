#!/usr/bin/env bash
# Flutter log helpers — work around Zed's lack of terminal/console line filtering.
#
# Zed can search terminal scrollback (Ctrl+Shift+F) but cannot hide non-matching
# lines. These helpers tee flutter output to a file, filter with grep, and write
# results you can open in the editor (full search, multibuffer, etc.).
#
# Usage:
#   zed-flutter-logs.sh watch [regex]       # live tail of run.log with filter
#   zed-flutter-logs.sh live [regex]        # alias for watch
#   zed-flutter-logs.sh filter <regex>      # print matching lines once
#   zed-flutter-logs.sh export <regex>      # write matches → .zed/flutter/filtered.log (open in Zed)
#   zed-flutter-logs.sh snapshot [regex]    # timestamped export under .zed/flutter/snapshots/
#   zed-flutter-logs.sh levels [error|warn|info|debug]
#   zed-flutter-logs.sh device [id]         # flutter logs stream
#   zed-flutter-logs.sh device-filter <re>  # flutter logs | grep
#   zed-flutter-logs.sh cache-uri           # parse run.log → vmservice.json
#
# Log file: .zed/flutter/run.log (from "Flutter: run (tee to log)" task)

set -euo pipefail

LOG_FILE="${ZED_FLUTTER_LOG_FILE:-.zed/flutter/run.log}"
FILTERED_FILE="${ZED_FLUTTER_FILTERED_FILE:-.zed/flutter/filtered.log}"
DEVICE_FILE=".zed/flutter_device_id"
DEVICES_JSON=".zed/flutter_devices.json"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

use_fvm=0
if [[ -d ".fvm" ]] && command -v fvm >/dev/null 2>&1; then
  use_fvm=1
fi

flutter_cmd() {
  if [[ "$use_fvm" -eq 1 ]]; then
    fvm flutter "$@"
  else
    flutter "$@"
  fi
}

default_device() {
  if [[ -f "$DEVICES_JSON" ]]; then
    python3 - "$DEVICES_JSON" <<'PY'
import json, sys
from pathlib import Path
try:
    d = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
    print(d.get("default_device_id", "") or "")
except Exception:
    pass
PY
    return
  fi
  if [[ -f "$DEVICE_FILE" ]]; then
    head -n 1 "$DEVICE_FILE" | tr -d '\n'
  fi
}

level_pattern() {
  case "${1:-error}" in
    error) echo '(E/flutter|ERROR|Exception|Error:|FATAL|assert|AssertionError)' ;;
    warn)  echo '(W/flutter|WARNING|Warn)' ;;
    info)  echo '(I/flutter|INFO)' ;;
    debug) echo '(D/flutter|DEBUG|flutter:)' ;;
    *)     echo "$1" ;;
  esac
}

require_log() {
  [[ -f "$LOG_FILE" ]] || {
    echo "No log at $LOG_FILE." >&2
    echo "Run task: Flutter: run (tee to log) or flutter run 2>&1 | tee $LOG_FILE" >&2
    exit 1
  }
}

export_matches() {
  local pattern="$1"
  local dest="$2"
  mkdir -p "$(dirname "$dest")"
  require_log
  if grep -E "$pattern" "$LOG_FILE" >"$dest"; then
    local count
    count="$(wc -l <"$dest" | tr -d ' ')"
    echo "Wrote $count lines → $dest"
    echo "Open in Zed for full editor search (Ctrl+F), multibuffer, and diff."
  else
    : >"$dest"
    echo "No matches for /$pattern/ — wrote empty $dest"
  fi
}

cache_uri_from_log() {
  require_log
  python3 - "$LOG_FILE" <<'PY'
import re, json, sys
from pathlib import Path
text = Path(sys.argv[1]).read_text(encoding="utf-8", errors="replace")
patterns = [
    r"A Dart VM Service on .* is available at:\s*(\S+)",
    r"An Observatory debugger and profiler on .* is available at:\s*(\S+)",
    r"(ws://127\.0\.0\.1:\d+/[^\s\"']+)",
    r"(http://127\.0\.0\.1:\d+/[^\s\"']+=)",
]
for pat in patterns:
    m = re.findall(pat, text)
    if m:
        uri = m[-1].rstrip(".,)")
        root = Path(".zed/flutter")
        root.mkdir(parents=True, exist_ok=True)
        payload = {"uri": uri, "vmServiceUri": uri}
        (root / "vmservice.json").write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
        (root / "vmservice_uri").write_text(uri + "\n", encoding="utf-8")
        print("Cached VM Service URI → .zed/flutter/vmservice.json")
        print(uri)
        sys.exit(0)
print("No VM Service URI found in log.", file=sys.stderr)
sys.exit(1)
PY
}

cmd="${1:-watch}"
shift || true

case "$cmd" in
  watch|live)
    pattern="${1:-.*}"
    mkdir -p "$(dirname "$LOG_FILE")"
    touch "$LOG_FILE"
    echo "Live filter on $LOG_FILE — pattern: /$pattern/"
    echo "Zed terminal cannot hide non-matching lines; this shows only matches."
    echo "For DevTools structured logs: task → Flutter: DevTools (logging)"
    echo "Ctrl+C to stop."
    tail -n 0 -F "$LOG_FILE" 2>/dev/null | grep --line-buffered -E "$pattern" || \
      tail -f "$LOG_FILE" 2>/dev/null | grep --line-buffered -E "$pattern"
    ;;
  filter)
    pattern="${1:?usage: zed-flutter-logs.sh filter <regex>}"
    require_log
    grep -E "$pattern" "$LOG_FILE" || true
    ;;
  export)
    pattern="${1:?usage: zed-flutter-logs.sh export <regex>}"
    export_matches "$pattern" "$FILTERED_FILE"
    ;;
  snapshot)
    pattern="${1:-.*}"
    stamp="$(date +%Y%m%d-%H%M%S)"
    dest=".zed/flutter/snapshots/filter-${stamp}.log"
    export_matches "$pattern" "$dest"
    ;;
  levels)
    pattern="$(level_pattern "${1:-error}")"
    require_log
    echo "Level filter: ${1:-error} → /$pattern/"
    grep -E "$pattern" "$LOG_FILE" || true
    ;;
  export-levels)
    pattern="$(level_pattern "${1:-error}")"
    export_matches "$pattern" "$FILTERED_FILE"
    ;;
  device)
    dev="${1:-$(default_device)}"
    if ! command -v flutter >/dev/null 2>&1 && [[ "$use_fvm" -eq 0 ]]; then
      echo "flutter not on PATH" >&2
      exit 1
    fi
    if [[ -n "$dev" ]]; then
      echo "Device logs: $dev (Ctrl+C to stop). Pipe: ... | grep PATTERN"
      exec flutter_cmd logs -d "$dev"
    fi
    echo "Default device logs (Ctrl+C to stop)"
    exec flutter_cmd logs
    ;;
  device-filter)
    pattern="${1:?usage: zed-flutter-logs.sh device-filter <regex>}"
    dev="${2:-$(default_device)}"
    if ! command -v flutter >/dev/null 2>&1 && [[ "$use_fvm" -eq 0 ]]; then
      echo "flutter not on PATH" >&2
      exit 1
    fi
    echo "Device logs filtered: /$pattern/ (Ctrl+C to stop)"
    if [[ -n "$dev" ]]; then
      flutter_cmd logs -d "$dev" 2>&1 | grep --line-buffered -E "$pattern"
    else
      flutter_cmd logs 2>&1 | grep --line-buffered -E "$pattern"
    fi
    ;;
  cache-uri)
    cache_uri_from_log
    ;;
  -h|--help)
    sed -n '1,24p' "$0"
    ;;
  *)
    echo "Unknown command: $cmd" >&2
    exit 1
    ;;
esac
