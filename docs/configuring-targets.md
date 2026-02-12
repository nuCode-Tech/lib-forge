# Configuring Target Platforms

XForge reads `xforge.yaml` next to `Cargo.toml` to decide which targets to build, how to name artifacts, and how adapters should find released binaries. The canonical registry of supported triples lives inside `crates/xforge-core/src/platform/key.rs`; every triple you list must match one of the `PlatformKey::as_str()` values defined there.

## Declare defaults in `xforge.yaml`

Create a `xforge.yaml` with a `build.targets` list so `xforge build` and `xforge bundle` know what to build/package. The CLI will iterate through this list, hash each target's inputs, and include the triples in the manifest.

```yaml
build:
  targets:
    - x86_64-unknown-linux-gnu
    - aarch64-linux-android
    - aarch64-apple-darwin
    - x86_64-pc-windows-msvc
  toolchain:
    channel: stable
```

- `build.targets` is required when `xforge.yaml` exists; the CLI rejects invalid or unsupported target triples (see `PlatformKey` for the authoritative list).
- `xforge build` picks the first entry as its default target unless you override it with `--target`, so keep the list ordered by your primary consumer.
- `xforge bundle` packages every listed target by reading the already-built libraries under `target/<triple>/<profile>`; run `xforge build` (or `cargo build`/`cross build`) for each triple before bundling.

## Toolchain settings

`build.toolchain.channel` is optional and makes the CLI spin up that Rust toolchain channel when invoking Cargo/Cross/Zigbuild. Omitting it leaves the channel unspecified so Cargo uses whatever `rustup default` already provides. `xforge build` also reads `build.toolchain.targets` when preparing the `CARGO_TARGET_DIR`, but the CLI enforces the same `build.targets` list you declared.

## Precompiled binaries block

Adapters and language-specific builders read the `precompiled_binaries` block to know where to download signed artifacts and which public key should verify them.

```yaml
precompiled_binaries:
  repository: owner/repo
  public_key: "<public_key_hex>"
  url_prefix: "https://github.com/owner/repo/releases/download/"
  mode: auto
```

- `repository` is required and is normalized to `owner/repo` (GitHub or GitHub-compatible hosts).
- `public_key` must be the 32-byte hex string produced by `xforge keygen` and is used both when signing a manifest in `xforge publish` and when adapters verify it.
- `url_prefix` overrides the default GitHub download URL when you host artifacts elsewhere.
- `mode` controls what happens when precompiled binaries cannot be found: `auto` prefers downloads but falls back to building locally, `always` treats missing/invalid binaries as an error, and `never` forces a local build. Additional aliases (`download`→`always`, `build`/`off`/`disabled`→`never`) are accepted.
- The CLI also consults this block to infer the repository when you omit `--repository` from `xforge publish`.

See `docs/release.md` for the full release flow (bundle, sign, publish) that relies on this configuration.

## Defaults when `xforge.yaml` is missing or incomplete

If you do not check in a `xforge.yaml`, tooling gracefully falls back to the full set of supported triples (`PlatformKey::registry`). This lets you try `xforge bundle` or `xforge build` before stabilizing your `xforge.yaml`. However, as soon as you add the file, you must define `build.targets`, otherwise the CLI refuses to run.
