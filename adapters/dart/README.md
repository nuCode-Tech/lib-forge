# xforge_dart

`xforge_dart` is the Dart adapter for XForge precompiled binaries. It exports a `PrecompiledBuilder` for `code_assets`/`hooks` workflows, the helpers that compute the same `build_id` as the CLI, and a small CLI that verifies a published release.

## Precompiled builder

Use `PrecompiledBuilder` to prefer signed artifacts and fall back to a local build only when the release is missing or invalid. The builder:

- Loads `xforge.yaml` (the same file the CLI reads) to find `precompiled_binaries.repository`, `public_key`, `url_prefix`, and `mode`.
- Computes the deterministic `build_id` with `crate_hash.dart`, so it will download the exact manifest name that `xforge bundle` generated.
- Downloads `xforge-manifest.json` plus the chosen platform archive, verifies both with the ED25519 `public_key`, caches them under `.dart_tool/xforge`, and extracts the shared library into the Dart app's code assets.
- Adds the extracted library as a `CodeAsset` for the current package, routing it through the `assetName` you supplied, and respects the `linkMode` preference from `code_assets`.
- Calls your provided `fallback` builder when mode is `never`, when verification fails, or when Rust is available and the builder decides to fall back.

```dart
const builder = PrecompiledBuilder(
  assetName: 'native_lib',
  cratePath: 'native',
  buildModeName: 'release',
  fallback: myLocalBuild,
);
```

`fallback` is a function with the same signature as a `hooks` builder: you are responsible for invoking Cargo/Cross/Zigbuild, collecting the resulting shared library, and registering it via `output.assets.code.add(...)`.

The builder honors `precompiled_binaries.mode`: `auto` prefers downloaded binaries but falls back when necessary, `always` treats any download/verification failure as an error, and `never` always runs the fallback. The default `UserOptions` value is `auto` when Rust is on `PATH` and `always` otherwise.

## CLI: validate-precompiled

`xforge_dart` ships with a CLI (`dart run xforge_dart validate-precompiled`) that mirrors the runtime behavior. It:

1. Reads `xforge.yaml` to fetch the same `public_key`/`url_prefix`.
2. Computes `build_id` (with `crate_hash.dart`) unless `--build-id` is supplied.
3. Detects the host triple unless `--target` is supplied.
4. Downloads and verifies the manifest plus the single artifact for that target.

Usage:

```bash
dart run xforge_dart validate-precompiled \
  --crate-dir path/to/crate \
  --build-id b1-abc123 \
  --target aarch64-apple-darwin
```

The CLI returns `0` on success, `1` if verification fails, and `2` on argument/configuration errors. Set `XFORGE_DART_PRECOMPILED_VERBOSE=1` to emit debug logs.

## CLI: keygen

Generate a new Ed25519 keypair in the same format as `xforge keygen`:

```bash
dart run xforge_dart keygen
```

This prints:

- `public_key=<32-byte hex>`
- `private_key=<64-byte hex (seed + public)>`

You can also run the standalone executable form:

```bash
dart run xforge_dart:keygen
```

## Configuration

`PrecompiledBuilder` and the CLI respect the `precompiled_binaries` block you declare in `xforge.yaml`:

```yaml
precompiled_binaries:
  repository: owner/repo
  public_key: "${public_key_hex}"
  url_prefix: "https://github.com/owner/repo/releases/download/"
  mode: auto
build:
  targets:
    - x86_64-unknown-linux-gnu
```

- `repository` and `public_key` are required.
- `mode` accepts `auto`, `always`/`download`, and `never`/`build`/`off`. When `mode=always`, the builder throws instead of falling back; when `mode=never`, it always runs your `fallback`.
- `url_prefix` lets you point to an alternative host than GitHub.

## Caching and logging

Downloaded manifests, artifacts, and extracted libraries live under `.dart_tool/xforge`. Signatures are verified with `ed25519_edwards`, and HTTP downloads are retried with exponential backoff (`httpGetWithRetry`). Verbose logging can be enabled with `XFORGE_DART_PRECOMPILED_VERBOSE=1`.
