# Zed Flutter Extension Roadmap

## Milestone 7: Hot reload, script install, flavor detect (v0.4.2 — implemented)

- **`scripts/zed-flutter-hot.sh`** — hot reload / hot restart via VM Service (`ext.flutter.reassemble` / `ext.flutter.hotRestart`).
- **`scripts/zed-flutter-copy-scripts.sh`** — `zed-flutter-init.sh` installs all helper scripts into the project `scripts/` folder.
- **Debug locator** — launch configs include `vmServiceInfoFile: .zed/flutter/vmservice.json` for DAP URI capture.
- **Bootstrap `--detect-flavors`** — reads Android `productFlavors` from `build.gradle` / `build.gradle.kts`.
- **FVM-aware `--full` merge** — example tasks rewritten to `fvm flutter` when FVM is active.
- **Tasks** — integration test, VM-service hot reload/restart; keymap `Alt+Shift+R` / `Alt+Shift+T`.

---

## Milestone 6: DevTools, inspector, and log debugging (v0.4.1 — implemented)

- **`scripts/zed-flutter-devtools.sh`** — open DevTools connected to the running app; deep-link to `inspector`, `logging`, `network`, etc.; resolves VM Service URI from cache or `run.log`.
- **`scripts/zed-flutter-logs.sh`** — work around Zed’s missing terminal/console line filter: `watch`, `export`, `snapshot`, `levels`, `device-filter`, `cache-uri`.
- **Tasks + keymap** — DevTools/inspector/logging tasks; `Shift+D` / `Shift+I` / `Shift+L`; log export/watch bindings.
- **Docs** — README section on log debugging; `examples/zed-gitignore.example` for local log state.

---

## Milestone 5: Full Flutter workflow (v0.4 — implemented)

- **Enhanced debug locator** — maps `flutter run`, `test`, and `attach` tasks to Dart DAP configs with `profile` / `flutterMode`, `toolArgs` (flavor, dart-define, web options), program args after `--`, and `vmServiceUri` for attach.
- **Smarter platform inference** — uses `targetPlatform` from `flutter devices --machine`, not only device id heuristics.
- **FVM + Dart LSP** — when `.fvm/` exists, supplies `dart.flutterSdkPath` workspace configuration to the Dart language server.
- **Bootstrap `--full`** — merges `examples/zed-tasks.example.json` and `examples/zed-debug.example.json` into generated project files.
- **`scripts/zed-flutter-init.sh`** — one-shot project setup (keymap, device policy template, full bootstrap).
- **Bundled Flutter snippets** — common widget/test snippets via `snippets/dart.json`.

---

# Zed Flutter Extension Roadmap (v0.3)

This roadmap focuses on improving daily-driver Flutter workflows in Zed while staying within current extension API limits.

## Milestone 1: Project bootstrap for tasks/debug (implemented)

- Add a generator script that scaffolds `.zed/tasks.json` and `.zed/debug.json`.
- Support flavor-aware entries (for example `dev,staging,prod`).
- Support flavor target mapping (for example `dev:lib/main_dev.dart`).
- Auto-detect and optionally prefer FVM when `.fvm/` exists.

## Milestone 2: Device workflow improvements (implemented)

- Added richer persisted-device format (`.zed/flutter_devices.json`) with:
  - `default_device_id`
  - optional per-platform fallback order (`fallback_device_ids.{linux|mac|windows|all}`)
  - `last_seen` metadata from `flutter devices --machine`
- Kept compatibility with `.zed/flutter_device_id` (single-line legacy mode).

## Milestone 3: Monorepo + melos support (implemented)

- Added generator options for package path and melos commands:
  - `--package-path apps/mobile`
  - `--melos`
  - `--package-name <scope>`
- The bootstrap script now emits package-scoped tasks/debug presets for multi-package workspaces.

## Milestone 4: QA and CI hardening (implemented)

- Added [`scripts/ci-check.sh`](scripts/ci-check.sh): validates strict example JSON, `bash -n` on shell scripts, `cargo test`, wasm release build, and bootstrap smoke tests in a temp tree.
- Added [`.github/workflows/ci.yml`](.github/workflows/ci.yml) to run `ci-check.sh` on pushes and PRs to `main`.
- Added [`RELEASING.md`](RELEASING.md) with a pre-release checklist.
