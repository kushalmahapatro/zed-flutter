# AGENTS.md

Guidance for AI agents and cloud development environments working on this repository.

## What this repo is

A **Rust → Wasm Zed extension** (`zed_flutter`) that adds a Flutter debug locator for Zed. It is not a standalone application. There is no `package.json`, Docker stack, or long-running server.

## Cursor Cloud specific instructions

### Toolchain requirements

- **Rust (latest stable)** — `zed_extension_api` 0.7.0 requires a recent Cargo (edition 2024). Rust 1.83 or older will fail with `feature edition2024 is required`. Use `rustup default stable` and ensure `rustc --version` is current before building.
- **Wasm target** — `rustup target add wasm32-wasip1`
- **Python 3** and **bash** — used by `scripts/ci-check.sh` and helper scripts

### Optional (not in cloud VM by default)

- **Zed editor** — required for end-to-end manual testing (Extensions → Install Dev Extension → this repo directory). Not available in typical cloud VMs.
- **Flutter SDK** — required for real `flutter run` / device resolution at runtime in Zed; not needed for `./scripts/ci-check.sh`.
- **Dart Zed extension** — required alongside this extension when debugging Flutter in Zed.

### Primary validation command

From the repo root:

```bash
./scripts/ci-check.sh
```

This runs: example JSON validation, `bash -n` on scripts, `cargo test`, `cargo build --release --target wasm32-wasip1`, and bootstrap smoke tests.

Individual steps:

| Task | Command |
|------|---------|
| Unit tests | `cargo test` |
| Wasm build | `cargo build --release --target wasm32-wasip1` |
| Artifact | `target/wasm32-wasip1/release/zed_flutter.wasm` |

There is no separate linter (clippy/format) wired in CI; `ci-check.sh` is the canonical check.

### Bootstrap helper (project integration smoke test)

To generate `.zed/tasks.json` and `.zed/debug.json` for a Flutter-like tree:

```bash
bash scripts/zed-flutter-bootstrap.sh --fvm off --flavors dev,prod \
  --targets dev:lib/main_dev.dart,prod:lib/main_prod.dart
```

Run from a directory containing `pubspec.yaml`.

### Gotchas

- If `cargo test` fails on `zed_extension_api` with `edition2024`, upgrade Rust: `rustup update stable && rustup default stable`.
- `ci-check.sh` already runs `rustup target add wasm32-wasip1` when `rustup` is present; the VM update script sets the toolchain default separately.
- No git pre-commit hooks are configured in this repo.
