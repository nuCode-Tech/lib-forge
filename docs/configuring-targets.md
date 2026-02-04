# Configuring Target Platforms

LibForge Core ships with canonical platform keys in `crates/libforge-core/src/platform/key.rs`. Each `PlatformKey` exposes a string representation (`PlatformKey::as_str()`), and those strings are the values you list in your per-project `libforge.yaml`. Adapters and CLI commands consult this configuration file to determine which targets to build when no flags are provided.

## Declare defaults in `libforge.yaml`

Create a `libforge.yaml` at the root of your repository to declare the platform targets and other defaults that matter to your project:

```yaml
build:
  targets:
    - linux-x86_64
    - android-arm64
    - macos-arm64
    - windows-x86_64-msvc
```

When LibForge tooling boots, it reads `build.targets` and uses the listed platform keys to:

- populate the manifest’s `platforms.targets` list,
- determine which adapters/artifacts to invoke,
- choose the default `target` when the user omits one.

Each entry must match a supported platform key (for example, `linux-x86_64`, `macos-arm64`, `ios-simulator`, `windows-x86_64-msvc`). The `PlatformKey` enum in `crates/libforge-core/src/platform/key.rs` is the authoritative list of supported strings, so use that file as your source of truth when adding or removing keys.

If a project omits `libforge.yaml`, tooling falls back to the canonical registry defined by `PlatformKey`, meaning every supported key remains available until you explicitly prune the list. Extending the registry requires touching `crates/libforge-core/src/platform`.
