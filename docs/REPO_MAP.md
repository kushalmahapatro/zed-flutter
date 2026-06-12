# Repository map (kushalmahapatro)

You maintain **two related repos** for full Flutter support in Zed:

| Repo | Role | URL |
|------|------|-----|
| **zed-flutter** | Flutter *companion* extension — debug locator, tasks bootstrap, DevTools/log scripts, snippets | https://github.com/kushalmahapatro/zed-flutter |
| **dart** (fork of zed-extensions/dart) | Official *Dart* extension — LSP + **debug adapter** that must forward DAP fields | Fork: https://github.com/kushalmahapatro/dart → PR to https://github.com/zed-extensions/dart |

## Install in Zed (today)

1. **Dart extension** — from Zed marketplace (or your patched dev extension after the upstream PR).
2. **Flutter extension** — **Extensions → Install Dev Extension** → clone of `kushalmahapatro/zed-flutter`.
3. In each Flutter project: `bash scripts/zed-flutter-init.sh`.

## Why two repos?

Zed splits responsibilities:

- **Dart extension** runs `dart language-server` and `flutter debug_adapter`.
- **Flutter extension** (this repo) cannot replace Dart; it adds the `flutter` debug locator, device resolution, and project tooling.

Your locator already emits `toolArgs`, `profile`, `vmServiceInfoFile`. The Dart extension must **pass them through** — see [`upstream/zed-extensions-dart/UPSTREAM_PR.md`](../upstream/zed-extensions-dart/UPSTREAM_PR.md).

## Apply the Dart patch from this repo

A ready-made patch is in [`upstream/zed-extensions-dart-dap.patch`](../upstream/zed-extensions-dart-dap.patch).

```bash
# 1. Fork zed-extensions/dart on GitHub → kushalmahapatro/dart (if you have not already)

git clone https://github.com/kushalmahapatro/dart.git
cd dart
git checkout -b passthrough-flutter-dap-fields
git am /path/to/zed-flutter/upstream/zed-extensions-dart-dap.patch
cargo test
git push -u origin passthrough-flutter-dap-fields
```

Open PR: **kushalmahapatro/dart** → **zed-extensions/dart** (`main`).

Until that PR merges, use Flutter extension scripts for DevTools/hot reload/log filtering.
