# Precompiled Binaries (LibForge)

LibForge produces signed manifests, deterministic build hashes, and platform artifacts so consumer adapters can download verified binaries instead of compiling the Rust crate locally. The adapter flow mirrors the CLI publish step, which keeps `libforge-manifest.json`, signatures, and archives in sync with the GitHub release identified by `build_id`.

## How adapters resolve a binary

1. **Read `libforge.yaml`.** The adapter expects a `precompiled_binaries` block (see below). Missing this block means the adapter skips the precompiled route.
2. **Compute the `build_id`.** Every adapter uses the same hash as the CLI (Cargo.toml, Cargo.lock, libforge.yaml, `.udl` inputs). The Dart adapter ships with `crate_hash.dart` to replicate the CLI hashing logic.
3. **Download the manifest.** Adapters fetch `libforge-manifest.json` and its `.sig` from the configured release URL and verify the signature using the `public_key` from `libforge.yaml`.
4. **Match the platform.** The manifest lists `platforms.targets` entries; adapters match their host triple (e.g., `aarch64-apple-darwin`) to a platform with artifacts.
5. **Download the artifact.** The first artifact listed for the matched platform is downloaded along with its `.sig` and verified with the same `public_key`.
6. **Fallback.** Unless `precompiled_binaries.mode=always`, adapters fall back to building with Cargo when download/verification fails. Some consumer builders (like `libforge_dart`'s `PrecompiledBuilder`) detect whether Rust is available and only fall back when a toolchain exists.

## Manifest and artifact requirements

- `libforge-manifest.json` and `libforge-manifest.json.sig` must be part of the release.
- Every platform archive produced by `libforge bundle` must be uploaded with a matching `.sig` file.
- The release tag must equal the manifest `build.id` so adapters can derive URLs (`<url_prefix><build_id>/<file>`).
- The manifest schema is `schemas/manifest.schema.json` and enforces entries such as `build.id`, `platforms.targets[*].artifacts`, and the optional `signing` block that `libforge publish` populates.
- `libforge publish` refuses to upload assets whose names already exist in the release; it prints `uploaded`/`skipped` lines so you can verify what changed.

## Release checklist

### 1. Configure `libforge.yaml`

Add a `precompiled_binaries` block next to `Cargo.toml`.

```yaml
precompiled_binaries:
  repository: owner/repo
  public_key: "<public_key_hex>"
  url_prefix: "https://github.com/owner/repo/releases/download/"
  mode: auto
build:
  targets:
    - x86_64-unknown-linux-gnu
    - aarch64-apple-darwin
```

- `repository` is required and normalized to `owner/repo` (GitHub/GitHub-compatible hosts).
- `public_key` is the 32-byte hex string produced by `libforge keygen`.
- `url_prefix` overrides the GitHub download path when you host artifacts elsewhere.
- `mode` controls adapter fallbacks (`auto`, `always`/`download`, `never`/`build`/`off`).
- `build.targets` tells the CLI which Rust triples to build. See `docs/configuring-targets.md` for the full schema and valid triples.

### 2. Build each target

Run `libforge build --target <triple>` (or `cargo build --target`, `cross build`, etc.) for each platform listed in `build.targets`. `libforge build` prints `build_id` and the shared-library path for the target it just built. The next step assumes the artifacts exist under `target/<triple>/<profile>`.

### 3. Bundle artifacts and manifest

```bash
libforge bundle --output-dir dist --profile release
```

or, while the CLI is still under active development:

```bash
cargo run -p libforge-cli -- bundle --manifest-dir . --output-dir dist --profile release
```

This command writes `dist/libforge-manifest.json`, `dist/build_id.txt`, and one archive per target (tar.gz/zip depending on the platform). Inspect the manifest; it includes the `build.id`, `platforms.targets`, and empty binding list that shared adapters expect.

### 4. Publish and sign

1. Generate keys:

   ```bash
   cargo run -p libforge-cli -- keygen
   ```

   Copy the `public_key` into `libforge.yaml` and keep `private_key` secret.
2. Set environment variables:

   ```bash
   export LIBFORGE_PRIVATE_KEY="<private_key_hex>"
   export GITHUB_TOKEN="<token with repo scope>"
   ```
3. Run publish:

   ```bash
   cargo run -p libforge-cli -- publish --manifest dist/libforge-manifest.json --assets-dir dist --out-dir dist
   ```

`libforge publish` signs the manifest and every asset (creating `.sig` files), verifies the manifest signature, and uploads everything to a release tagged `build_id`. It reads `precompiled_binaries.repository` when `--repository` is omitted, so keep that block consistent with your GitHub repo. The CLI reuses existing releases and skips uploading assets that already exist.

`libforge sign`/`verify` are also available for non-manifest files when you need to manage signatures manually.

### Optional: validate the release locally

The Dart adapter ships with a validation CLI: `dart run libforge_dart validate-precompiled [--crate-dir] [--build-id] [--target]`. Running it from your workspace ensures the manifest and a platform artifact can be downloaded and verified with the public key before relying on the release in production.

## Sample GitHub Actions snippet

```yaml
on:
  push:
    branches: [ main ]
jobs:
  release:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          path: crate
      - uses: actions/checkout@v4
        with:
          repository: owner/lib-forge
          path: lib-forge
      - uses: dtolnay/rust-toolchain@stable
      - run: |
          cargo run --manifest-path ./lib-forge/crates/libforge-cli/Cargo.toml -- bundle \
            --manifest-dir ./crate \
            --output-dir ./crate/dist \
            --profile release
      - name: Publish release
        env:
          LIBFORGE_PRIVATE_KEY: ${{ secrets.LIBFORGE_PRIVATE_KEY }}
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        run: |
          cargo run --manifest-path ./lib-forge/crates/libforge-cli/Cargo.toml -- publish \
            --manifest ./crate/dist/libforge-manifest.json \
            --assets-dir ./crate/dist
```

This example builds on Ubuntu and reuses the `libforge-cli` workspace binary. Adjust the matrix (`macos-latest`, `windows-latest`) and pass `--android-ndk-version`/`--android-sdk-location` if you need Android archives.

## Android targets

Android targets require the NDK so `libforge bundle` can include `.so` files. On Ubuntu runners install the Android command-line tools, accept licenses, and install the NDK version you plan to ship. For example:

```bash
sudo apt-get update && sudo apt-get install -y wget unzip
wget https://dl.google.com/android/repository/commandlinetools-linux-108-9123335_latest.zip -O tools.zip
mkdir -p $HOME/android-sdk/cmdline-tools
unzip tools.zip -d $HOME/android-sdk/cmdline-tools
yes | $HOME/android-sdk/cmdline-tools/tools/bin/sdkmanager --sdk_root=$HOME/android-sdk "platform-tools" "ndk;24.0.8215888"
```

Then invoke `libforge bundle` with the SDK/NDK flags:

```bash
cargo run -p libforge-cli -- bundle \
  --manifest-dir . \
  --output-dir dist \
  --profile release \
  --android-sdk-location "$ANDROID_SDK_ROOT" \
  --android-ndk-version 24.0.8215888 \
  --android-min-sdk-version 23
```

`bundle` will still look for the built libraries under `target/<triple>/release`, so build them ahead of time (for example, with `cargo build --target=aarch64-linux-android`).

## Troubleshooting

- **Missing `precompiled_binaries`.** Adapters fall back to local builds; add the block to `libforge.yaml` to enable downloads.
- **Manifest or artifact signature fails.** Verify that the public key in `libforge.yaml` matches the private key used by `libforge publish`. You can test locally with `libforge verify` or `dart run libforge_dart validate-precompiled`.
- **Release missing files.** Ensure `dist` (or your `--output-dir`) contains both archives and their `.sig` siblings before running `libforge publish`. Each artifact must include the `build_id` in its name so the CLI can validate it.
