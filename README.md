# Flutter (Zed extension)

Companion extension for [Flutter](https://flutter.dev/) in [Zed](https://zed.dev/). It does **not** replace the official [Dart](https://github.com/zed-extensions/dart) extension: install **Dart** first for the Dart analyzer/LSP on `.dart` files and for the **Dart** debug adapter (`flutter debug_adapter` / `dart debug_adapter`). That adapter is what implements breakpoints, stepping, variables, and isolate/thread presentation in Zed’s debugger UI.

This **Flutter** extension adds:

- **`flutter` debug locator** — When you start a debug session from a **task** such as `flutter run`, `fvm flutter run`, `flutter test …`, or `fvm flutter test …`, Zed can offer a matching scenario that targets the **Dart** adapter with `"type": "flutter"`. Device flags on the task (`-d` / `--device-id`) and entrypoint (`-t` / `--target`) are carried into the launch config when present.
- **Example launch presets** — See [`examples/zed-debug.example.json`](examples/zed-debug.example.json). Copy entries into your project’s [`.zed/debug.json`](https://zed.dev/docs/debugger.html#getting-started) (or use the global `debug.json` from the command palette: **zed: open debug tasks**).

Zed does not yet support declaring another extension as a hard package dependency in `extension.toml`. Treat **Dart + Flutter** as the supported pair: without Dart, there is no analyzer and no `Dart` DAP for this locator to drive.

## Install locally (dev extension)

1. Install [Rust via rustup](https://rustup.rs/) and the Wasm target: `rustup target add wasm32-wasip1`.
2. From this repo: `cargo build --release --target wasm32-wasip1`.
3. In Zed: **Extensions → Install Dev Extension** (or command **zed: install dev extension**) and choose this directory.

## Publish to the Zed registry

Follow [Developing Extensions → Publishing](https://zed.dev/docs/extensions/developing-extensions.html#publishing-your-extension): open a PR on [zed-industries/extensions](https://github.com/zed-industries/extensions) adding this repo as a submodule and an entry in the top-level `extensions.toml`.

## Debugging behavior

Run **debugger: start** (or the debug panel **+**). Pick the **Dart** adapter, then a Flutter task or a configuration from `.zed/debug.json`. The Dart extension launches Google’s Dart/Flutter DAP; Zed’s debugger then supports the usual DAP features supported by that adapter (including breakpoints and isolate-related views where the client exposes them).
