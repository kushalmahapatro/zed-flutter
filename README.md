# Flutter (Zed extension)

Companion extension for [Flutter](https://flutter.dev/) in [Zed](https://zed.dev/). It does **not** replace the official [Dart](https://github.com/zed-extensions/dart) extension: install **Dart** first for the Dart analyzer/LSP on `.dart` files and for the **Dart** debug adapter (`flutter debug_adapter` / `dart debug_adapter`). That adapter is what implements breakpoints, stepping, variables, and isolate/thread presentation in Zed's debugger UI.

This **Flutter** extension adds:

- **`flutter` debug locator** — When you start a debug session from a **task** such as `flutter run`, `fvm flutter run`, `flutter test …`, or `fvm flutter test …`, Zed can offer a matching scenario that targets the **Dart** adapter with `"type": "flutter"`. Device flags on the task (`-d` / `--device-id`) and entrypoint (`-t` / `--target`) are carried into the launch config when present. If the task omits `-d`, the locator picks a device from `flutter devices --machine` (run in the task `cwd` so FVM resolves the right SDK), preferring the id stored in **`.zed/flutter_device_id`** (first line) when that id is still available; otherwise it uses the first supported device in the machine list.
- **Example launch presets** — See [`examples/zed-debug.example.json`](examples/zed-debug.example.json). Copy entries into your project's [`.zed/debug.json`](https://zed.dev/docs/debugger.html#getting-started) (or use the global `debug.json` from the command palette: **zed: open debug tasks**).
- **Device selection** — Debug.json presets for common targets, plus automatic device resolution from `.zed/flutter_device_id` and `flutter devices --machine` when tasks omit `-d`.
- **Hot reload** — Example keybindings and tasks (see examples); prefer the Dart debug adapter’s hot reload while a session is active where available.
- **DevTools** — Example tasks to open DevTools in the browser or as a local server, plus device listing tasks.
- **Common Flutter tasks** — Examples for `flutter pub get`, `flutter clean`, `flutter doctor`, etc.
- **Keymap examples** — Starter bindings in [`examples/zed-keymap.example.json`](examples/zed-keymap.example.json).

Zed does not yet support declaring another extension as a hard package dependency in `extension.toml`. Treat **Dart + Flutter** as the supported pair: without Dart, there is no analyzer and no `Dart` DAP for this locator to drive.

## Installation

### Install locally (dev extension)

1. Install [Rust via rustup](https://rustup.rs/) and the Wasm target: `rustup target add wasm32-wasip1`.
2. From this repo: `cargo build --release --target wasm32-wasip1`.
3. In Zed: **Extensions → Install Dev Extension** (or command **zed: install dev extension**) and choose this directory.

### Publish to the Zed registry

Follow [Developing Extensions → Publishing](https://zed.dev/docs/extensions/developing-extensions.html#publishing-your-extension): open a PR on [zed-industries/extensions](https://github.com/zed-industries/extensions) adding this repo as a submodule and an entry in the top-level `extensions.toml`.

## Setup

### 1. Install the Dart extension first

Before using this extension, make sure you have the **[Dart extension](https://github.com/zed-extensions/dart)** installed in Zed.

### 2. Copy example configurations

Copy the example files from this repo to your Flutter project's `.zed/` directory:

```bash
cp examples/zed-debug.example.json .zed/debug.json
cp examples/zed-tasks.example.json .zed/tasks.json
cp examples/zed-keymap.example.json .zed/keymap.json
# optional richer device policy:
cp examples/zed-flutter-devices.example.json .zed/flutter_devices.json
```

### 2b. (Recommended) Generate flavor-aware Zed files

Use the bootstrap script to generate `.zed/tasks.json` and `.zed/debug.json` for your flavors:

```bash
bash scripts/zed-flutter-bootstrap.sh --flavors dev,staging,prod --targets dev:lib/main_dev.dart,staging:lib/main_staging.dart,prod:lib/main_prod.dart
```

The script auto-detects FVM when `.fvm/` exists (or force with `--fvm on` / `--fvm off`).

For monorepos, you can target a package and optional melos scope:

```bash
bash scripts/zed-flutter-bootstrap.sh --melos --package-path apps/mobile --package-name mobile --flavors dev,prod --targets dev:lib/main_dev.dart,prod:lib/main_prod.dart
```

- `--package-path` sets task/debug `cwd` under `$ZED_WORKTREE_ROOT`.
- `--melos` adds melos tasks and emits package-scoped commands.
- `--package-name` scopes melos exec commands (when omitted, commands run directly in package `cwd`).

### 3. Start a debug session

Run **debugger: start** (or the debug panel **+**). Pick the **Dart** adapter, then a Flutter task or a configuration from `.zed/debug.json`. The Dart extension launches Google's Dart/Flutter DAP; Zed's debugger then supports the usual DAP features supported by that adapter (including breakpoints and isolate-related views where the client exposes them).

### Flutter CLI in Zed: tasks, not a second command bus

Zed’s Wasm extension API does not let this extension register an open-ended list of **command palette** actions for every `flutter` / `dart` subcommand. The supported way to wire **all** of Flutter’s CLIs into the editor is **tasks**: define them in your project [`.zed/tasks.json`](https://zed.dev/docs/tasks) (or copy [`examples/zed-tasks.example.json`](examples/zed-tasks.example.json) and adjust). You then run them via **command palette** → **task: spawn** / **run task** (or bind keys) and they execute in the integrated terminal with full shell behavior.

What this **Flutter** extension *does* in Rust/Wasm: the **`flutter` debug locator** and any subprocess it needs to resolve a device (for example `flutter devices --machine`, `fvm flutter …`, plus small shell helpers) — those are declared in [`extension.toml`](extension.toml) under `[[capabilities]]` with `kind = "process:exec"` so Zed can allow them. Everything else you want (`flutter create`, `build`, `pub`, `analyze`, DevTools, and so on) is ordinary **task** definitions pointing at the same `flutter` / `dart` / `fvm` binaries you already have on your `PATH`.

## Features

### Device selection

**Automatic default (task has no `-d`):**

- Preferred config file: `.zed/flutter_devices.json`
- Legacy fallback (still supported): `.zed/flutter_device_id` (first line only)

The locator validates candidates against `flutter devices --machine` and picks the first available in this order:

1. `default_device_id` from `.zed/flutter_devices.json`
2. `fallback_device_ids.<os>` list (where `<os>` is `linux`, `mac`, or `windows`)
3. `fallback_device_ids.all` list
4. `.zed/flutter_device_id` legacy value
5. first supported device from `flutter devices --machine`

After resolving, it updates `.zed/flutter_devices.json` with the selected `default_device_id` and `last_seen` metadata from the machine device output.

**Interactive picker:** Copy [`scripts/zed-flutter-pick-device.sh`](scripts/zed-flutter-pick-device.sh) into your repo (for example under `scripts/`) and run the example task **Flutter: set default device for Zed debug** (requires `python3` for parsing JSON). It writes both `.zed/flutter_device_id` and `.zed/flutter_devices.json`.

Add `.zed/flutter_device_id` and `.zed/flutter_devices.json` to `.gitignore` if you do not want to commit local defaults.

**Explicit device:** Use `-d` / `--device-id` on the task, or keep using named presets in `.zed/debug.json`:

| Preset | Device | Description |
|--------|--------|-------------|
| Flutter: run on iPhone | `iphone` | Launch on iPhone simulator |
| Flutter: run on Android | `android` | Launch on Android emulator/device |
| Flutter: run on Linux | `linux` | Launch on Linux desktop |
| Flutter: run on Windows | `windows` | Launch on Windows desktop |
| Flutter: run on macOS | `macos` | Launch on macOS desktop |
| Flutter: run on Chrome | `chrome` | Launch on Chrome web |
| Flutter: run on Edge (web) | `edge` | Launch on Edge web |
| Flutter: run in release mode | N/A | Launch in release profile |
| Flutter: run in profile mode | N/A | Launch in profile (performance) mode |

### Hot Reload

There are two ways to trigger hot reload:

**Option 1: Keymap (recommended)**

Press `Ctrl+S` (Linux/Windows) or `Cmd+S` (macOS) while editing Flutter files during a debug session. This is configured in `.zed/keymap.json`.

**Option 2: Task**

Run the **Flutter: Hot Reload** task from the command palette: **zed: run task** → **Flutter: Hot Reload**.

### DevTools and widget tree

Zed extensions cannot host a native widget tree panel yet; use **Flutter DevTools** in the browser for the widget inspector, performance, logging, and network tools.

Example tasks (see [`examples/zed-tasks.example.json`](examples/zed-tasks.example.json)):

- **Flutter: Open DevTools in browser** — runs `dart devtools` (opens a browser UI; connect to the VM service URL from your running app or from the debug session when prompted).
- **Flutter: Open DevTools (local server)** — runs the DevTools server on port 9100.
- **Flutter: list devices** / **machine JSON** — quick views of `flutter devices` output.

You can bind a key in `.zed/keymap.json` to run a task or paste a terminal command (see [`examples/zed-keymap.example.json`](examples/zed-keymap.example.json)).

### Common Flutter tasks (examples)

The reference list lives in [`examples/zed-tasks.example.json`](examples/zed-tasks.example.json). It includes entries for `flutter pub`, `create`, `analyze`, `test`, `build` (APK, app bundle, iOS, web, Linux), `dart format` / `dart fix`, DevTools, `gen-l10n`, and a few **FVM** examples. All use `"cwd": "$ZED_WORKTREE_ROOT"` so they run in the project root. Add or duplicate tasks for anything else (for example `flutter drive`, custom flavors, or a `create my_app` run in a parent directory).

Use the example task **Flutter: bootstrap .zed tasks/debug** to generate project-specific flavored configs quickly.

## Roadmap

See [`ROADMAP.md`](ROADMAP.md) for the v0.3 roadmap (milestones 1–4). See also [`RELEASING.md`](RELEASING.md).

**CI locally:** from the repo root run `./scripts/ci-check.sh` (Rust tests, wasm build, JSON checks, bootstrap smoke tests).

## Debugging behavior

When you start a debug session, the Dart extension launches Google's Dart/Flutter DAP. Zed's debugger then supports the usual DAP features supported by that adapter, including breakpoints, stepping, variables, and isolate/thread presentation.

### Troubleshooting

**Breakpoints not working?**

1. Make sure the **Dart extension** is installed.
2. Make sure your breakpoints are **red** (bound), not gray (unbound).
3. Make sure the app has compiled before setting breakpoints.
4. Check the **Debug Console** tab for any errors.

**Can't see debug configurations?**

1. Make sure you copied `zed-debug.example.json` to `.zed/debug.json`.
2. Make sure the `"adapter"` is `"Dart"` (capital D), not `"dart"`.

## Directory Structure of a Zed Extension

A Zed extension is a Git repository that contains an `extension.toml`. This file must contain some basic information about the extension:

```
my-extension/
  extension.toml
  Cargo.toml
  src/
    lib.rs
  examples/
    zed-debug.example.json
    zed-tasks.example.json
    zed-keymap.example.json
```

## WebAssembly

Procedural parts of extensions are written in Rust and compiled to WebAssembly. To develop an extension that includes custom code, include a `Cargo.toml` like this:

```
[package]
name = "my-extension"
version = "0.0.1"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
zed_extension_api = "0.1.0"
```

Use the latest version of the `zed_extension_api` available on crates.io. Make sure it's still compatible with Zed versions you want to support.

In the `src/lib.rs` file in your Rust crate you will need to define a struct for your extension and implement the `Extension` trait, as well as use the `register_extension!` macro to register your extension:

```
use zed_extension_api as zed;

struct MyExtension {
    // ... state
}

impl zed::Extension for MyExtension {
    // ...
}

zed::register_extension!(MyExtension);
```

`stdout`/ `stderr` is forwarded directly to the Zed process. In order to see `println!`/ `dbg!` output from your extension, you can start Zed in your terminal with a `--foreground` flag.

## Extension License Requirements

As of October 1st, 2025, extension repositories must include a license. The following licenses are accepted:

- Apache 2.0
- BSD 2-Clause
- BSD 3-Clause
- CC BY 4.0
- GNU GPLv3
- GNU LGPLv3
- MIT
- Unlicense
- zlib

This allows us to distribute the resulting binary produced from your extension to our users. Without a valid license, the pull request to add or update your extension in the following steps will fail CI.

Your license file should be at the root of your extension repository. Any filename that has `LICENCE` or `LICENSE` as a prefix (case insensitive) will be inspected to ensure it matches one of the accepted licenses.

## Notes on widget tree and panels

The Zed extension API does not yet expose a first-class **widget tree** side panel for Flutter. Until it does, use **DevTools → Widget Inspector** while your app is running (or attached) for the full tree and property editing, and the **Debug** variables / console in Zed for breakpoint-time inspection.