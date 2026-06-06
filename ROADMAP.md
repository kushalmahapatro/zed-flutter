# Zed Flutter Extension Roadmap

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
