pub mod hash;
pub mod inputs;

pub use hash::{
    canonical_json, canonical_json_without_target, hash_build_inputs, hash_release_inputs,
};
pub use inputs::{
    AbiInput, BuildInputField, BuildInputValue, BuildInputs, CargoLockfile, NormalizedCargoToml,
    NormalizedLibforgeConfig, NormalizedUdl, UniFfiInput,
};

/// Release hash used for precompiled artifact lookup.
/// This is intentionally identical to the build_id.
pub fn release_hash(build_id: &str) -> String {
    build_id.to_string()
}
