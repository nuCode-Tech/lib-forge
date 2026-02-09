# lib-forge

lib-forge is a Rust-based build, packaging, and distribution system for native libraries intended to be consumed by multiple programming languages. It is designed primarily for UniFFI-based Rust crates, but it is not limited to UniFFI. lib-forge solves the problem UniFFI explicitly leaves out: how native binaries are built, named, verified, published, and reused across languages and platforms.

Build native binaries once, publish them once, and reuse them everywhere. All intelligence lives in lib-forge itself. Language adapters are intentionally dumb.

---

## What lib-forge does

lib-forge provides:

- Deterministic, content-addressed native builds (ABI-stable identity)
- Cross-platform Rust compilation (linux, windows, Android, Apple, etc.)

## Configuring Targets

LibForge tooling honors a per-project `libforge.yaml` so consumers can choose which targets are built by default. See [docs/configuring-targets.md](docs/configuring-targets.md) for the expected schema, how the values map back to the canonical `PlatformKey` registry in `crates/libforge-core/src/platform/key.rs`, and examples.
- Standardized artifact naming and archive layouts
- A single manifest describing all binaries in a release
- GitHub Releases publishing
- Language adapters (Dart, Kotlin/Gradle, Swift, Python) that:
  - read the manifest
  - select the correct platform
  - download the required artifacts
  - place files

Adapters never compute hashes, infer compatibility, or invent logic. Releases are keyed by the LibForge `build_id`, which acts as the canonical release hash.

---

## What lib-forge explicitly does NOT do

- It does not regenerate bindings at install time.
- It does not let each language define its own ABI rules.
- It does not allow adapters to guess paths or names.
- It does not tie binaries to a single language ecosystem.

This is intentional.

---

## High-level architecture

lib-forge is a Cargo workspace composed of multiple focused crates:

- `libforge-core` – pure logic (ABI identity, targets, naming, manifest)
- `libforge-build` – invokes Cargo, Cross, and Zigbuild
- `libforge-pack` – creates archives, xcframeworks, AARs, etc.
- `libforge-publish` – publishes artifacts (GitHub Releases)
- `libforge-cli` – user-facing CLI wiring

Language adapters live outside the Rust workspace and consume published releases.

---

## Typical workflow

1. Author a Rust crate (usually with UniFFI)
2. Add `libforge.yaml`
3. Run:

   ```
   libforge build
   libforge bundle
   libforge publish
   ```

4. A GitHub Release is created containing:
   - native binaries
   - packaged artifacts (zip, tar, xcframework, aar)
   - `libforge-manifest.json`
5. Language adapters fetch binaries from that release.

---

## Supported platforms (initial)

Target triples for `libforge.yaml`:

- `armv7-linux-androideabi`
- `aarch64-linux-android`
- `x86_64-linux-android`
- `aarch64-apple-ios`
- `aarch64-apple-ios-sim`
- `x86_64-apple-ios`
- `aarch64-pc-windows-msvc`
- `x86_64-pc-windows-msvc`
- `aarch64-unknown-linux-gnu`
- `x86_64-unknown-linux-gnu`
- `aarch64-apple-darwin`
- `x86_64-apple-darwin`

---

## Supported languages (adapters)

- Dart / Flutter
- Kotlin / Gradle
- Swift (CocoaPods, SwiftPM)
- Python

Adapters are optional and replaceable. The manifest is the contract.

---

## Repository layout

```
lib-forge/
├── crates/          # Rust workspace
├── adapters/        # Language-specific consumers
├── schemas/         # Public JSON schemas
├── examples/        # Example projects
├── ci/              # CI workflows
└── docs/            # Architecture & internal docs
```

---

## When should you use lib-forge?

Use lib-forge if:

- You ship Rust native code to more than one language
- You want reproducible, verifiable native artifacts
- You want one GitHub release to serve all languages
- You are tired of per-language build pipelines drifting

---

## Documentation

- `docs/architecture.md` – internal architecture and design
- `docs/manifest.md` – manifest format and semantics
- `docs/adapters.md` – adapter responsibilities and constraints
- `docs/security.md`

---

## Status

lib-forge is under active development. The manifest schema and ABI identity rules are considered critical APIs and will be versioned conservatively.

---

## License

MIT
