# x-forge

`x-forge` is a Rust crate that automates deterministic native builds, packaging, signing, and publishing so a single GitHub release can serve every language consumer. Build targets, channels, and components come from `rust-toolchain.toml`, while `xforge.yaml` only configures `precompiled_binaries`.

## Documentation

- `docs/overview.md` — full guide covering the release loop, CLI surface area, workspace layout, adapters, and schemas.
- `docs/configuring-targets.md` — schema-driven reference for declaring build targets and adapter settings.
- `docs/release.md` — release checklist, signing notes, and automation snippets.

## Next steps

Start with `docs/overview.md` before running the CLI or inspecting adapters so you know how the workspace pieces fit together.
  