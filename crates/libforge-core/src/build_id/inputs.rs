//! ABI-affecting inputs that define a build identity.
//!
//! Excludes timestamps, absolute paths, environment variables, and CI metadata.

use crate::bindings::BindingMetadataSet;

/// ABI-affecting inputs that define a build identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuildInputs {
    /// ABI-affecting: normalized Cargo manifest content.
    /// This captures dependency graph, feature flags, and package metadata.
    pub cargo_toml: AbiInput<NormalizedCargoToml>,
    /// ABI-affecting: Cargo lockfile content.
    /// This locks down resolved dependency versions and checksums.
    pub cargo_lock: AbiInput<CargoLockfile>,
    /// ABI-affecting: Rust target triple.
    /// This changes ABI, linkage, and platform-specific codegen.
    pub rust_target_triple: AbiInput<String>,
    /// ABI-affecting: UniFFI UDL.
    /// This defines the FFI surface for bindings.
    pub uniffi: Option<AbiInput<UniFfiInput>>,
    /// ABI-affecting: libforge.yaml config.
    /// This captures build target selection and precompiled binary metadata.
    pub libforge_yaml: Option<AbiInput<NormalizedLibforgeConfig>>,
    /// ABI-affecting: binding metadata that affects ABI compatibility.
    pub binding_metadata: AbiInput<BindingMetadataSet>,
    /// ABI-affecting: libforge manifest schema version.
    /// This ensures schema evolution invalidates incompatible build identities.
    pub manifest_schema_version: AbiInput<String>,
}

impl BuildInputs {
    pub fn from_manifest_dir(
        manifest_dir: &std::path::Path,
        rust_target_triple: AbiInput<String>,
        uniffi: Option<AbiInput<UniFfiInput>>,
        binding_metadata: AbiInput<BindingMetadataSet>,
        manifest_schema_version: AbiInput<String>,
    ) -> std::io::Result<Self> {
        let cargo_toml_path = manifest_dir.join("Cargo.toml");
        let cargo_lock_path = manifest_dir.join("Cargo.lock");
        let libforge_yaml_path = manifest_dir.join("libforge.yaml");
        let cargo_toml = std::fs::read_to_string(cargo_toml_path)?;
        let cargo_lock = std::fs::read_to_string(cargo_lock_path)?;
        let libforge_yaml = read_optional_file(&libforge_yaml_path)?
            .map(|contents| AbiInput::new(NormalizedLibforgeConfig(contents)));
        Ok(Self {
            cargo_toml: AbiInput::new(NormalizedCargoToml(cargo_toml)),
            cargo_lock: AbiInput::new(CargoLockfile(cargo_lock)),
            rust_target_triple,
            uniffi,
            libforge_yaml,
            binding_metadata,
            manifest_schema_version,
        })
    }

    /// Enumerate every ABI-affecting field with explicit presence.
    pub fn fields(&self) -> Vec<BuildInputField> {
        vec![
            BuildInputField::abi(
                "cargo.toml",
                BuildInputValue::Present(self.cargo_toml.value.0.clone()),
            ),
            BuildInputField::abi(
                "cargo.lock",
                BuildInputValue::Present(self.cargo_lock.value.0.clone()),
            ),
            BuildInputField::abi(
                "rust.target_triple",
                BuildInputValue::Present(self.rust_target_triple.value.clone()),
            ),
            BuildInputField::abi(
                "uniffi.udl",
                self.uniffi
                    .as_ref()
                    .and_then(|value| value.value.udl.as_ref())
                    .map(|value| BuildInputValue::Present(value.0.clone()))
                    .unwrap_or(BuildInputValue::Absent),
            ),
            BuildInputField::abi(
                "libforge.yaml",
                self.libforge_yaml
                    .as_ref()
                    .map(|value| BuildInputValue::Present(value.value.0.clone()))
                    .unwrap_or(BuildInputValue::Absent),
            ),
            BuildInputField::abi(
                "binding.metadata",
                BuildInputValue::Present(self.binding_metadata.value.canonical_string()),
            ),
            BuildInputField::abi(
                "manifest.schema_version",
                BuildInputValue::Present(self.manifest_schema_version.value.clone()),
            ),
        ]
    }
}

/// Explicit ABI-affecting wrapper for a build input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AbiInput<T> {
    pub value: T,
}

impl<T> AbiInput<T> {
    pub const AFFECTS_ABI: bool = true;

    pub fn new(value: T) -> Self {
        Self { value }
    }
}

/// UniFFI ABI inputs: optional UDL.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UniFfiInput {
    pub udl: Option<NormalizedUdl>,
}

/// Normalized Cargo.toml contents.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NormalizedCargoToml(pub String);

/// Cargo.lock content captured verbatim.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CargoLockfile(pub String);

/// Normalized UDL source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NormalizedUdl(pub String);

/// Normalized libforge.yaml contents.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NormalizedLibforgeConfig(pub String);

/// Explicit enumeration of ABI-affecting inputs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuildInputField {
    pub name: &'static str,
    pub value: BuildInputValue,
    pub affects_abi: bool,
}

impl BuildInputField {
    pub fn abi(name: &'static str, value: BuildInputValue) -> Self {
        Self {
            name,
            value,
            affects_abi: true,
        }
    }
}

/// Explicit presence marker for ABI-affecting fields.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BuildInputValue {
    Present(String),
    Absent,
}

fn read_optional_file(path: &std::path::Path) -> std::io::Result<Option<String>> {
    match std::fs::read_to_string(path) {
        Ok(contents) => Ok(Some(contents)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}
