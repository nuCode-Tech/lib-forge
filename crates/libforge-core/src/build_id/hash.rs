use std::collections::BTreeMap;

use serde_json::Value;
use sha2::{Digest, Sha256};

use super::{BuildInputValue, BuildInputs};

const HASH_VERSION: &str = "b1";

pub fn canonical_json(inputs: &BuildInputs) -> serde_json::Result<String> {
    let mut fields = inputs.fields();
    fields.sort_by(|left, right| left.name.cmp(right.name));

    let field_values: Vec<Value> = fields
        .into_iter()
        .map(|field| {
            let mut map = BTreeMap::new();
            map.insert("name".to_string(), Value::String(field.name.to_string()));
            map.insert("affects_abi".to_string(), Value::Bool(field.affects_abi));
            map.insert(
                "value".to_string(),
                match field.value {
                    BuildInputValue::Present(value) => Value::String(value),
                    BuildInputValue::Absent => Value::Null,
                },
            );
            Value::Object(map.into_iter().collect())
        })
        .collect();

    let mut root = BTreeMap::new();
    root.insert(
        "version".to_string(),
        Value::String(HASH_VERSION.to_string()),
    );
    root.insert("inputs".to_string(), Value::Array(field_values));

    serde_json::to_string(&Value::Object(root.into_iter().collect()))
}

pub fn hash_build_inputs(inputs: &BuildInputs) -> serde_json::Result<String> {
    let json = canonical_json(inputs)?;
    let mut hasher = Sha256::new();
    hasher.update(json.as_bytes());
    let digest = hasher.finalize();
    Ok(format!("{}-{}", HASH_VERSION, hex::encode(digest)))
}

pub fn canonical_json_without_target(inputs: &BuildInputs) -> serde_json::Result<String> {
    let mut fields = inputs.fields_without_target();
    fields.sort_by(|left, right| left.name.cmp(right.name));
    let field_values: Vec<Value> = fields
        .into_iter()
        .map(|field| {
            let mut map = BTreeMap::new();
            map.insert("name".to_string(), Value::String(field.name.to_string()));
            map.insert("affects_abi".to_string(), Value::Bool(field.affects_abi));
            map.insert(
                "value".to_string(),
                match field.value {
                    BuildInputValue::Present(value) => Value::String(value),
                    BuildInputValue::Absent => Value::Null,
                },
            );
            Value::Object(map.into_iter().collect())
        })
        .collect();

    let mut root = BTreeMap::new();
    root.insert(
        "version".to_string(),
        Value::String(HASH_VERSION.to_string()),
    );
    root.insert("inputs".to_string(), Value::Array(field_values));

    serde_json::to_string(&Value::Object(root.into_iter().collect()))
}

pub fn hash_release_inputs(inputs: &BuildInputs) -> serde_json::Result<String> {
    let json = canonical_json_without_target(inputs)?;
    let mut hasher = Sha256::new();
    hasher.update(json.as_bytes());
    let digest = hasher.finalize();
    Ok(format!("{}-{}", HASH_VERSION, hex::encode(digest)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build_id::{
        AbiInput, CargoLockfile, NormalizedCargoToml, NormalizedLibforgeConfig, NormalizedUdl,
        UniFfiInput,
    };

    fn sample_inputs() -> BuildInputs {
        BuildInputs {
            cargo_toml: AbiInput::new(NormalizedCargoToml(
                "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n".to_string(),
            )),
            cargo_lock: AbiInput::new(CargoLockfile(
                "version = 3\n[[package]]\nname = \"demo\"\nversion = \"0.1.0\"\n".to_string(),
            )),
            rust_target_triple: AbiInput::new("aarch64-apple-darwin".to_string()),
            uniffi: Some(AbiInput::new(UniFfiInput {
                udl: Some(NormalizedUdl(
                    "namespace demo; interface Demo { string ping(); };".to_string(),
                )),
            })),
            libforge_yaml: Some(AbiInput::new(NormalizedLibforgeConfig(
                "build:\n  targets:\n    - linux\nprecompiled_binaries:\n  url_prefix: https://github.com/stax/lib-forge/releases/download/precompiled_\n  public_key: demo-public-key\n".to_string(),
            ))),
        }
    }

    #[test]
    fn hash_vector_is_stable() {
        let inputs = sample_inputs();
        let hash = hash_build_inputs(&inputs).expect("hash should succeed");
        assert!(hash.starts_with("b1-"));
    }

    #[test]
    fn canonical_json_vector_is_stable() {
        let inputs = sample_inputs();
        let json = canonical_json(&inputs).expect("json should serialize");
        assert!(!json.contains("binding.metadata"));
        assert!(!json.contains("manifest.schema_version"));
    }

    #[test]
    fn hash_changes_on_abi_input() {
        let mut inputs = sample_inputs();
        if let Some(libforge_yaml) = &mut inputs.libforge_yaml {
            libforge_yaml.value.0 = "build:\n  targets:\n    - windows\n".to_string();
        }
        let hash = hash_build_inputs(&inputs).expect("hash should succeed");
        let original = hash_build_inputs(&sample_inputs()).expect("hash should succeed");
        assert_ne!(hash, original);
    }
}
