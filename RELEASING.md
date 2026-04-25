# Releasing the Zed Flutter extension

Use this checklist before tagging or publishing a version.

## Pre-release

1. **Changelog / roadmap**
   - Update [`ROADMAP.md`](ROADMAP.md) if milestones change.
2. **Versions**
   - Set the same version in:
     - [`extension.toml`](extension.toml) `version = "…"`
     - [`Cargo.toml`](Cargo.toml) `version = "…"`
3. **Build / CI**
   - Run `./scripts/ci-check.sh` (strict JSON for known examples, `bash -n`, `cargo test`, wasm build, bootstrap smoke). Or let GitHub Actions on the PR go green.
4. **Dev extension**
   - `cargo build --release --target wasm32-wasip1`
   - In Zed: **Install dev extension** and smoke-test: debug from a Flutter task, device resolution, a generated bootstrap if you use it.

## Publishing to the Zed extension registry

Follow [Developing extensions → Publishing](https://zed.dev/docs/extensions/developing-extensions.html#publishing-your-extension): open a PR on [zed-industries/extensions](https://github.com/zed-industries/extensions) with a submodule update and a matching `extensions.toml` version bump. Ensure the repository root has a valid open-source [license](https://zed.dev/docs/extensions/developing-extensions.html#license-requirement) (this repo is MIT in `Cargo.toml`).

## Tagging (optional)

Tag the same commit you submitted to the registry, for example: `v0.3.3`.
