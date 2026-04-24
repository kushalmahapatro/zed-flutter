#!/usr/bin/env bash
# Interactive helper: writes the chosen Flutter device id to .zed/flutter_device_id
# so the Zed Flutter extension can reuse it when you debug via a task without `-d`.
#
# Usage (from your Flutter project root):
#   bash scripts/zed-flutter-pick-device.sh
# Or point Zed task cwd at $ZED_WORKTREE_ROOT and run the matching example task.

set -euo pipefail

if ! command -v flutter >/dev/null 2>&1; then
  echo "flutter not found on PATH." >&2
  exit 1
fi

mkdir -p .zed

echo "Fetching devices (flutter devices --machine)..."
json="$(flutter devices --machine 2>/dev/null || true)"
if [[ -z "${json}" || "${json}" == "[]" ]]; then
  echo "No devices reported. Plug in a device or start an emulator, then retry." >&2
  exit 1
fi

mapfile -t lines < <(
  python3 -c '
import json, sys
raw = sys.stdin.read()
try:
    data = json.loads(raw)
except json.JSONDecodeError:
    sys.exit(1)
for row in data:
    i = row.get("id")
    n = row.get("name", "")
    sup = row.get("isSupported", True)
    if not i:
        continue
    flag = "" if sup else " (unsupported)"
    print(f"{i}\t{n}{flag}")
' <<<"${json}"
)

if [[ ${#lines[@]} -eq 0 ]]; then
  echo "Could not parse device list." >&2
  exit 1
fi

echo
echo "Pick a device (number), or q to quit:"
select choice in "${lines[@]}"; do
  if [[ -z "${REPLY}" ]]; then
    continue
  fi
  if [[ "${REPLY}" == "q" || "${REPLY}" == "Q" ]]; then
    echo "Cancelled."
    exit 0
  fi
  if [[ -n "${choice}" ]]; then
    id="${choice%%$'\t'*}"
    printf '%s' "${id}" > .zed/flutter_device_id
    echo "Saved default device id to .zed/flutter_device_id → ${id}"
    echo "Tip: add .zed/flutter_device_id to .gitignore if you do not want to share it."
    exit 0
  fi
  echo "Invalid selection."
done
