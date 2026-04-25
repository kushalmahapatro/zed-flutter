#!/usr/bin/env bash
# Generate flavor-aware .zed/tasks.json and .zed/debug.json for Flutter projects.
#
# Usage examples:
#   scripts/zed-flutter-bootstrap.sh
#   scripts/zed-flutter-bootstrap.sh --flavors dev,staging,prod
#   scripts/zed-flutter-bootstrap.sh --flavors dev,prod --targets dev:lib/main_dev.dart,prod:lib/main_prod.dart
#   scripts/zed-flutter-bootstrap.sh --fvm on
#   scripts/zed-flutter-bootstrap.sh --package-path apps/mobile --flavors dev,prod
#   scripts/zed-flutter-bootstrap.sh --melos --package-name mobile_app
#
# Notes:
# - Writes/overwrites .zed/tasks.json and .zed/debug.json.
# - If --fvm auto (default), it uses FVM when .fvm/ is present and fvm is on PATH.

set -euo pipefail

flavors_csv=""
targets_csv=""
fvm_mode="auto" # auto|on|off
package_path=""
melos_mode="off" # off|on
package_name=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --flavors)
      flavors_csv="${2:-}"
      shift 2
      ;;
    --targets)
      targets_csv="${2:-}"
      shift 2
      ;;
    --fvm)
      fvm_mode="${2:-auto}"
      shift 2
      ;;
    --package-path)
      package_path="${2:-}"
      shift 2
      ;;
    --melos)
      melos_mode="on"
      shift
      ;;
    --package-name)
      package_name="${2:-}"
      shift 2
      ;;
    -h|--help)
      sed -n '1,35p' "$0"
      exit 0
      ;;
    *)
      echo "Unknown arg: $1" >&2
      exit 1
      ;;
  esac
done

if [[ -n "$package_path" ]]; then
  if [[ ! -f "$package_path/pubspec.yaml" ]]; then
    echo "Package path '$package_path' does not contain pubspec.yaml." >&2
    exit 1
  fi
elif [[ ! -f "pubspec.yaml" ]]; then
  echo "Run this from your Flutter project root (pubspec.yaml not found)." >&2
  exit 1
fi

mkdir -p .zed

use_fvm=0
if [[ "$fvm_mode" == "on" ]]; then
  use_fvm=1
elif [[ "$fvm_mode" == "off" ]]; then
  use_fvm=0
else
  if [[ -d ".fvm" ]] && command -v fvm >/dev/null 2>&1; then
    use_fvm=1
  fi
fi

runner_prefix="flutter"
if [[ "$use_fvm" -eq 1 ]]; then
  runner_prefix="fvm flutter"
fi

python3 - "$flavors_csv" "$targets_csv" "$runner_prefix" "$package_path" "$melos_mode" "$package_name" <<'PY'
import json
import sys

flavors_csv = sys.argv[1].strip()
targets_csv = sys.argv[2].strip()
runner_prefix = sys.argv[3].strip()
package_path = sys.argv[4].strip()
melos_mode = sys.argv[5].strip()
package_name = sys.argv[6].strip()

flavors = [f.strip() for f in flavors_csv.split(",") if f.strip()] if flavors_csv else []
targets = {}
if targets_csv:
    for pair in [p.strip() for p in targets_csv.split(",") if p.strip()]:
        if ":" not in pair:
            continue
        k, v = pair.split(":", 1)
        k = k.strip()
        v = v.strip()
        if k and v:
            targets[k] = v

cwd = "$ZED_WORKTREE_ROOT" if not package_path else f"$ZED_WORKTREE_ROOT/{package_path}"
default_target = "lib/main.dart"
default_program = default_target

def quote_shell_arg(v: str) -> str:
    if not v:
        return "''"
    return "'" + v.replace("'", "'\"'\"'") + "'"

def make_flutter_command(args: str) -> str:
    if melos_mode != "on":
        return f"{runner_prefix} {args}".strip()
    if package_name:
        # Run package-scoped command through melos in monorepos.
        return f"melos exec --scope {quote_shell_arg(package_name)} -- {runner_prefix} {args}".strip()
    # Fallback: run in selected package path directly.
    return f"{runner_prefix} {args}".strip()

tasks = {
    "flutter_run_debug": {
        "label": f"{runner_prefix}: run --debug",
        "command": make_flutter_command("run --debug"),
        "cwd": cwd,
        "reveal": "always",
    },
    "flutter_run_profile": {
        "label": f"{runner_prefix}: run --profile",
        "command": make_flutter_command("run --profile"),
        "cwd": cwd,
        "reveal": "always",
    },
    "flutter_run_release": {
        "label": f"{runner_prefix}: run --release",
        "command": make_flutter_command("run --release"),
        "cwd": cwd,
        "reveal": "always",
    },
    "flutter_test": {
        "label": f"{runner_prefix}: test",
        "command": make_flutter_command("test"),
        "cwd": cwd,
        "reveal": "always",
    },
    "flutter_analyze": {
        "label": f"{runner_prefix}: analyze",
        "command": make_flutter_command("analyze"),
        "cwd": cwd,
        "reveal": "always",
    },
    "flutter_pub_get": {
        "label": f"{runner_prefix}: pub get",
        "command": make_flutter_command("pub get"),
        "cwd": cwd,
        "reveal": "never",
    },
}

if melos_mode == "on":
    tasks["melos_bootstrap"] = {
        "label": "Melos: bootstrap",
        "command": "melos bootstrap",
        "cwd": "$ZED_WORKTREE_ROOT",
        "reveal": "always",
    }
    tasks["melos_test_all"] = {
        "label": "Melos: test all packages",
        "command": "melos run test",
        "cwd": "$ZED_WORKTREE_ROOT",
        "reveal": "always",
    }

debug_configs = [
    {
        "adapter": "Dart",
        "label": f"{runner_prefix}: debug (default)",
        "type": "flutter",
        "request": "launch",
        "program": default_program,
        "cwd": cwd,
        "useFvm": runner_prefix.startswith("fvm "),
    }
]

for flavor in flavors:
    target = targets.get(flavor, default_target)
    task_key = f"flutter_run_debug_{flavor}"
    tasks[task_key] = {
        "label": f"{runner_prefix}: run --debug --flavor {flavor}",
        "command": make_flutter_command(f"run --debug --flavor {flavor} -t {target}"),
        "cwd": cwd,
        "reveal": "always",
    }
    debug_configs.append(
        {
            "adapter": "Dart",
            "label": f"{runner_prefix}: debug ({flavor})",
            "type": "flutter",
            "request": "launch",
            "program": target,
            "cwd": cwd,
            "useFvm": runner_prefix.startswith("fvm "),
            "args": [f"--flavor={flavor}"],
        }
    )

with open(".zed/tasks.json", "w", encoding="utf-8") as f:
    json.dump(tasks, f, indent=2)
    f.write("\n")

with open(".zed/debug.json", "w", encoding="utf-8") as f:
    json.dump(debug_configs, f, indent=2)
    f.write("\n")

print("Generated .zed/tasks.json and .zed/debug.json")
PY

echo "Runner: ${runner_prefix}"
echo "CWD: \$ZED_WORKTREE_ROOT${package_path:+/$package_path}"
if [[ -n "$flavors_csv" ]]; then
  echo "Flavors: ${flavors_csv}"
fi
if [[ "$melos_mode" == "on" ]]; then
  echo "Melos mode: on${package_name:+ (scope: $package_name)}"
fi
