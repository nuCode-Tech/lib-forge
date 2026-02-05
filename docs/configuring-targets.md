# Configuring Target Platforms

LibForge Core ships with canonical Rust target triples in `crates/libforge-core/src/platform/key.rs`. The `PlatformKey::as_str()` values are those triples, and they are the values you list in your per-project `libforge.yaml`. Adapters and CLI commands consult this configuration file to determine which targets to build when no flags are provided.

## Declare defaults in `libforge.yaml`

Create a `libforge.yaml` at the root of your repository to declare the platform targets and other defaults that matter to your project:

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

When LibForge tooling boots, it reads `build.targets` and uses the listed target triples to:

- populate the manifest’s `platforms.targets` list,
- determine which adapters/artifacts to invoke,
- choose the default `target` when the user omits one,
- resolve toolchain target triples from the same list (single source of truth).

Each entry must match a supported Rust target triple (for example, `x86_64-unknown-linux-gnu`, `aarch64-apple-darwin`, `aarch64-apple-ios-sim`, `x86_64-pc-windows-msvc`). The `PlatformKey` enum in `crates/libforge-core/src/platform/key.rs` is the authoritative list of supported strings, so use that file as your source of truth when adding or removing keys.

`build.toolchain.channel` is optional and pins the Rust toolchain channel used by build executors. It is read from the same `libforge.yaml` so no separate toolchain file is required.

If a project omits `libforge.yaml`, tooling falls back to the canonical registry defined by `PlatformKey`, meaning every supported key remains available until you explicitly prune the list. Extending the registry requires touching `crates/libforge-core/src/platform`.
