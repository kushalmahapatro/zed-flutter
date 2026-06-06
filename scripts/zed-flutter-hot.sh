#!/usr/bin/env bash
# Hot reload / hot restart for a running Flutter app via the VM Service API.
#
# Requires a cached VM Service URI (.zed/flutter/vmservice.json or run.log).
# Use while debugging or after "Flutter: run (tee to log)" + cache-uri.
#
# Usage:
#   scripts/zed-flutter-hot.sh reload
#   scripts/zed-flutter-hot.sh restart
#   scripts/zed-flutter-hot.sh reload ws://127.0.0.1:12345/abc=/

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ACTION="${1:-reload}"
VM_URI="${2:-}"

if [[ "$ACTION" == "reload" || "$ACTION" == "restart" ]]; then
  if [[ $# -ge 2 ]] && [[ "$2" == ws://* || "$2" == http://* ]]; then
    VM_URI="$2"
  fi
else
  if [[ "$ACTION" == ws://* || "$ACTION" == http://* ]]; then
    VM_URI="$ACTION"
    ACTION="${2:-reload}"
  else
    echo "Usage: $0 [reload|restart] [vm-service-uri]" >&2
    exit 1
  fi
fi

resolve_uri() {
  if [[ -n "$VM_URI" ]]; then
    echo "$VM_URI"
    return 0
  fi
  if [[ -f .zed/flutter/vmservice.json ]]; then
    python3 - <<'PY'
import json
from pathlib import Path
p = Path(".zed/flutter/vmservice.json")
d = json.loads(p.read_text(encoding="utf-8"))
print(d.get("uri") or d.get("vmServiceUri") or "")
PY
    return
  fi
  if [[ -f .zed/flutter/vmservice_uri ]]; then
    tr -d '\n' < .zed/flutter/vmservice_uri
    return
  fi
  bash "$SCRIPT_DIR/zed-flutter-logs.sh" cache-uri 2>/dev/null | tail -1 || true
}

VM_URI="$(resolve_uri | tail -1)"
if [[ -z "$VM_URI" ]]; then
  echo "No VM Service URI. Start the app, then run: bash scripts/zed-flutter-logs.sh cache-uri" >&2
  exit 1
fi

python3 - "$VM_URI" "$ACTION" <<'PY'
import json
import sys
import urllib.error
import urllib.parse
import urllib.request

uri = sys.argv[1].strip()
action = sys.argv[2].strip()
extension = "ext.flutter.reassemble" if action == "reload" else "ext.flutter.hotRestart"

# Normalize to HTTP VM service base (…/token=/)
base = uri.replace("ws://", "http://").split("/ws")[0].rstrip("/")
if not base.endswith("="):
    if base.endswith("/"):
        base = base[:-1]
    if not base.endswith("="):
        base = base + "="
http_base = base + "/"

def get_json(url: str):
    req = urllib.request.Request(url, headers={"Accept": "application/json"})
    with urllib.request.urlopen(req, timeout=8) as resp:
        return json.loads(resp.read().decode("utf-8"))

def post_empty(url: str):
    req = urllib.request.Request(
        url,
        data=b"{}",
        method="POST",
        headers={"Content-Type": "application/json", "Accept": "application/json"},
    )
    with urllib.request.urlopen(req, timeout=15) as resp:
        return resp.read()

# List isolates and pick Flutter main isolate
isolates = get_json(http_base + "json/list")
if not isolates:
    raise SystemExit("No isolates reported by VM Service")
main = None
for iso in isolates:
    name = (iso.get("name") or "").lower()
    if iso.get("isMainIsolate") or name in ("main", "main.dart"):
        main = iso
        break
if main is None:
    main = isolates[0]
isolate_id = main["id"]
quoted = urllib.parse.quote(isolate_id, safe="")
url = f"{http_base}{extension}?isolateId={quoted}"
try:
    post_empty(url)
except urllib.error.HTTPError as e:
    body = e.read().decode("utf-8", errors="replace")
    raise SystemExit(f"VM Service {extension} failed ({e.code}): {body}") from e

label = "Hot reload" if action == "reload" else "Hot restart"
print(f"{label} sent via {extension} (isolate {isolate_id})")
PY
