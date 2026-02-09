# uniffi-xforge example

This example includes:
- A minimal Rust crate with a UniFFI UDL file.
- A Dart package that uses the `libforge_dart` adapter to download and verify precompiled artifacts.

## Rust crate

- `uniffi-xforge.udl` defines the FFI surface.
- `build.rs` generates scaffolding.

## Dart package

Location: `examples/uniffi-xforge/dart`

Run:

```bash
dart run bin/example.dart
```

Optional arguments:

```bash
dart run bin/example.dart /absolute/path/to/examples/uniffi-xforge
```

## Configure precompiled binaries

Edit `examples/uniffi-xforge/libforge.yaml` and set:
- `precompiled_binaries.repository`
- `precompiled_binaries.url_prefix` (optional)
- `precompiled_binaries.public_key`

The adapter caches downloads under `.dart_tool/libforge/` in the crate directory.
