# Precompiled Binaries (LibForge)

LibForge produces signed manifests and platform artifacts so adapters (Dart, etc.) can download a verified binary instead of rebuilding the Rust crate from source. The adapter flow mirrors what `libforge-cli` publishes, which keeps the manifest, signatures, and archives in sync with your GitHub release.

## How the adapter picks a binary

1. **Load `libforge.yaml`.** The consumer expects a `precompiled_binaries` block beside `Cargo.toml`. If that section is missing, the adapter skips precompiled binaries altogether. The available `mode` values are `auto` (default), `always` (fail if anything is missing), and `never` (force a local build).
2. **Compute the `build_id`.** LibForge derives the release hash (`b1-<digest>`) from `Cargo.toml`, `Cargo.lock`, any `libforge.yaml`, and optional `.udl` files. This deterministic hash ties the manifest and artifacts to the exact source and configuration that produced them. When a crate lives inside a workspace, LibForge reads the nearest `Cargo.lock` by walking up from `--manifest-dir` to the workspace root.
3. **Download the manifest.** The adapter fetches `libforge-manifest.json` and `libforge-manifest.json.sig` from the configured `url_prefix` (default: `https://github.com/<owner>/<repo>/releases/download/`). It verifies the signature with the `public_key` stored in `libforge.yaml`.
4. **Pick an artifact.** The manifest lists `platforms` with `name`, `triples`, and `artifacts`. The adapter matches the host target triple (e.g., `aarch64-apple-darwin`) to one of the platforms, then downloads the first artifact listed for that platform plus its `.sig`.
5. **Verify the artifact.** Every archive is verified against the same `public_key`. If the check fails (corrupted download, outdated release, wrong key), the cached files are discarded.
6. **Fall back to a local build (unless `mode=always`).** If the adapter cannot download or verify a binary and the developer has `rustup` on `PATH`, it falls back to building with Cargo. If Rust is unavailable, the adapter raises an error so the issue can be surfaced as a CI failure instead of silently shipping unsigned code.

## Release manifest and artifact requirements

- `libforge-manifest.json` plus `libforge-manifest.json.sig` must live in the GitHub release.
- Each platform archive emitted by `libforge-cli bundle` must be uploaded alongside its `.sig`.
- Manifest entries look like this:

```json
{
  "build": { "id": "b1-abc123…" },
  "platforms": {
    "linux-x86_64": {
      "name": "linux-x86_64",
      "triples": ["x86_64-unknown-linux-gnu"],
      "artifacts": ["libforge-linux-x86_64.tar.gz"]
    }
  }
}
```

- The release tag and manifest `build.id` are the same `build_id`, so adapter downloads can be deterministic (release URL = `<url_prefix><build_id>/<file>`).
- `libforge-cli bundle` prints the `build_id`, manifest path, and each archive it produces so you can see what will be uploaded.

## Preparing your crate for precompiled binaries

### 1. Configure `libforge.yaml`

Place a YAML file beside `Cargo.toml` with at least the `precompiled_binaries` block:

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

- `repository` is required and must look like `owner/repo` (GitHub normalized owners work too).
- `public_key` is the 32-byte hex string produced by `libforge keygen`; the adapter uses it to verify every signature.
- `url_prefix` overrides the default GitHub download path when you host artifacts somewhere else.
- `mode` controls how aggressively the adapter falls back to a local build.
- `build.targets` lets `libforge-cli bundle` and `build` know which Rust targets you care about.

### 2. Generate a key pair

```bash
cargo run -p libforge-cli -- keygen
```

Run this from the LibForge repository root (where the workspace `Cargo.toml` lives) so Cargo knows `libforge-cli` belongs to the workspace. If you must run it from another directory, include the CLI manifest explicitly (e.g., `cargo run --manifest-path /path/to/lib-forge/crates/libforge-cli/Cargo.toml -- keygen`) instead of depending on the current working directory.

Note for crate authors: `libforge-cli` is not published to crates.io. If you are following this guide from another repository, keep a checkout of LibForge available (submodule, sibling checkout, etc.) and point `--manifest-path` at that checkout when running the CLI.

Copy `public_key=…` into `libforge.yaml` and store `private_key=…` in a secret (GitHub Actions, CI, or your local environment).

### 3. Bundle everything

```bash
cargo run -p libforge-cli -- bundle --manifest-dir . --output-dir dist
```

This command builds every configured target, archives the shared libraries, and writes `dist/libforge-manifest.json` plus platform archives (e.g., `libforge-darwin-aarch64.tar.gz`). `bundle` also prints the `build_id` you will use in the release name and download URLs.

### 4. Publish to GitHub

```bash
export LIBFORGE_PRIVATE_KEY="<private_key_hex>"
export GITHUB_TOKEN="<token_with_repo_scope>"

cargo run -p libforge-cli -- publish \
  --manifest dist/libforge-manifest.json \
  --assets-dir dist
```

- The CLI signs the manifest and every asset (adding `.sig` files) before hitting GitHub.
- `publish` creates (or reuses) a release tagged with the `build_id`.
- If you skip `--repository`, the CLI reads the `repository` value from `libforge.yaml`.

## GitHub Actions workflow

We ship a reusable workflow at `.github/workflows/release.yml`. It exposes `workflow_dispatch` inputs for `manifest_dir`, `output_dir`, `repository`, and `profile` and does the following:

1. Checkout your repo (`actions/checkout@v4`).
2. Install Rust via `dtolnay/rust-toolchain@stable`.
3. Run `libforge bundle` with the provided inputs so every target gets built and packaged.
4. Run `libforge publish` with `LIBFORGE_PRIVATE_KEY` and `GITHUB_TOKEN` supplied from secrets, uploading the manifest, archives, and signatures to the release identified by the `build_id`.

Required secrets:
- `LIBFORGE_PRIVATE_KEY` (hex string from `libforge keygen`).
- `GITHUB_TOKEN` (default token has `contents: write` permission; no extra scope is necessary unless you push to different repos).

If you prefer to publish on each push to `main` (or another branch), use the same steps but change the workflow trigger and add a matrix if you want to run on macOS or Windows agents. For example:

```yaml
on:
  push:
    branches: [ main ]

jobs:
  release:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
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

This example builds on every OS in the matrix so you get platform-specific archives signed and uploaded in a single workflow run. Android targets can be added by passing `--android-sdk-location`, `--android-ndk-version`, and `--android-min-sdk-version` to `bundle` on the Ubuntu runner.

### Android targets

Rust Android targets require the Android SDK/NDK so `libforge-cli bundle` can produce `.so` files for each ABI. On Ubuntu runners you must install the Android SDK command-line tools (or use an action like `android-actions/setup-android@v2` or `reactivecircus/android-emulator-runner@v3`), accept the licenses, and install the specific NDK version you plan to ship.

Set environment variables that point to the SDK/NDK before calling `bundle`, for example:

```yaml
- name: Install Android SDK/NDK
  run: |
    sudo apt-get update
    sudo apt-get install -y wget unzip
    wget https://dl.google.com/android/repository/commandlinetools-linux-108-9123335_latest.zip -O tools.zip
    mkdir -p $HOME/android-sdk/cmdline-tools
    unzip tools.zip -d $HOME/android-sdk/cmdline-tools
    yes | $HOME/android-sdk/cmdline-tools/tools/bin/sdkmanager --sdk_root=$HOME/android-sdk "platform-tools" "ndk;24.0.8215888"
- name: Bundle artifacts (Android)
  env:
    ANDROID_SDK_ROOT: ${{ env.HOME }}/android-sdk
  run: |
    cargo run -p libforge-cli -- bundle \
      --manifest-dir . \
      --output-dir ./dist \
      --profile release \
      --android-sdk-location "$ANDROID_SDK_ROOT" \
      --android-ndk-version 24.0.8215888 \
      --android-min-sdk-version 23
```

Make sure the NDK version you install matches the value you pass to `--android-ndk-version`. If you build for multiple Android ABIs, `bundle` will emit one archive per triple (e.g., `aarch64-linux-android`, `x86_64-linux-android`) plus signatures for each artifact.

## Troubleshooting

- **Missing `precompiled_binaries`** — Adapters fall back to local build; add the section to `libforge.yaml`.
- **Signature verification fails** — Ensure the `public_key` in `libforge.yaml` matches the private key used by `publish`.
- **Release missing files** — Verify that `dist` contains both archives and their `.sig` siblings before running `publish`.
