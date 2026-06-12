# Upstream PR: zed-extensions/dart — Flutter DAP field passthrough

## Problem

The [Dart Zed extension](https://github.com/zed-extensions/dart) `get_dap_binary` rebuilds a minimal Flutter DAP config and **drops** fields that the [Flutter debug adapter](https://github.com/flutter/flutter/blob/master/packages/flutter_tools/lib/src/debug_adapters/README.md) supports:

| Field from `.zed/debug.json` / Flutter locator | Current Dart extension | Effect |
|------------------------------------------------|------------------------|--------|
| `toolArgs` (`--flavor`, `--dart-define`, …) | **Ignored** | Flavor builds break |
| `profile` / `flutterMode` | Hardcoded `"debug"` | Profile/release debug wrong |
| `vmServiceInfoFile` | **Ignored** | DevTools / hot reload scripts cannot auto-connect |
| `vmServiceUri` (attach) | Passed | OK |
| `device_id` | Passed but defaults to **`chrome`** | Wrong device when omitted |
| `platform` | Passed but defaults to **`web`** | Wrong platform when omitted |

The [zed-flutter](https://github.com/kushalmahapatro/zed-flutter) companion extension emits correct JSON; the Dart extension must forward it.

## Proposed fix

This directory contains a tested builder:

- `src/lib.rs` — `build_flutter_dap_configuration()` + unit tests
- `proposed_get_dap_binary.rs` — drop-in sketch for `src/dart.rs`

### Behavior changes

1. **Passthrough** `toolArgs`, `flutterMode` (from `flutterMode` or `profile`), `vmServiceInfoFile`, `vmServiceUri`, `sendLogsToClient`, `customTool`, etc.
2. **Do not default** `deviceId` / `platform` when absent — let Flutter infer from `flutter devices`.
3. **Resolve** relative `vmServiceInfoFile` against launch `cwd` (supports package-scoped `cwd`).

## How to open the PR

```bash
git clone https://github.com/zed-extensions/dart.git
cd dart

# Copy proposal module
cp /path/to/zed-flutter/upstream/zed-extensions-dart/src/lib.rs src/dap_config.rs

# Add to src/dart.rs:
#   mod dap_config;
# Replace get_dap_binary body with proposed_get_dap_binary.rs logic
# (use dap_config::build_flutter_dap_configuration)

# Add to Cargo.toml [dependencies] if needed:
#   serde_json = "1.0"  # already via zed_extension_api

cargo test
# Install as dev extension in Zed and verify:
# - flutter run --profile task → profile mode
# - flavor toolArgs → correct --flavor on flutter run
# - vmServiceInfoFile written after launch

git checkout -b passthrough-flutter-dap-fields
git commit -am "fix: pass toolArgs, flutterMode, vmServiceInfoFile to Flutter DAP"
git push origin passthrough-flutter-dap-fields
```

Open PR against https://github.com/zed-extensions/dart with:

**Title:** `fix: pass Flutter DAP launch/attach fields (toolArgs, flutterMode, vmServiceInfoFile)`

**Body:**

```markdown
## Summary

Forwards Flutter debug adapter configuration from Zed debug tasks instead of hardcoding `flutterMode: debug` and defaulting to `chrome`/`web`.

## Motivation

Companion extensions (e.g. zed-flutter) and project `.zed/debug.json` files already emit `toolArgs`, `profile`, and `vmServiceInfoFile`. The current adapter strips them, breaking flavors, profile runs, and VM service URI capture for DevTools.

## Changes

- Add `dap_config::build_flutter_dap_configuration`
- Pass `toolArgs`, `flutterMode`, `vmServiceInfoFile`, optional `deviceId`/`platform`
- Remove incorrect defaults for device/platform when not specified

## Testing

- [ ] `cargo test` in extension repo
- [ ] Dev extension: debug `flutter run --profile` task
- [ ] Dev extension: flavor via `toolArgs` in debug.json
- [ ] Attach + `vmServiceInfoFile` path under `.zed/flutter/`

/cc @kushalmahapatro — zed-flutter locator emits these fields
```

## Verify proposal tests (in zed-flutter repo)

```bash
cd upstream/zed-extensions-dart && cargo test
```

## Related

- Flutter DAP args: https://github.com/flutter/flutter/blob/master/packages/flutter_tools/lib/src/debug_adapters/flutter_adapter_args.dart
- zed-flutter locator: `src/flutter.rs` (`toolArgs`, `profile`, `vmServiceInfoFile`)
