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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bindings::{
        BindingMetadata, BindingMetadataSet, DartBinding, KotlinBinding, PythonBinding,
        SwiftBinding,
    };
    use crate::build_id::{
        AbiInput, CargoLockfile, NormalizedCargoToml, NormalizedLibforgeConfig, NormalizedUdl,
        UniFfiInput,
    };

    const GOLDEN_HASH_V1: &str =
        "b1-27990a950e05e88ae9e3b83c40f4af8a9fa0a83a07489b808d83ab4b0082f558";
    const GOLDEN_CANONICAL_JSON_V1: &str = r#"{"inputs":[{"affects_abi":true,"name":"binding.metadata","value":"dart:sdk_constraint=3.0;ffi_abi=1|kotlin:min_sdk=21;jvm_target=1.8;ndk_abis=arm64-v8a,x86_64|python:abi_tag=cp311;platform_tag=manylinux_2_28|swift:toolchain=5.9;deployment_target=13.0"},{"affects_abi":true,"name":"cargo.lock","value":"version = 3\n[[package]]\nname = \"demo\"\nversion = \"0.1.0\"\n"},{"affects_abi":true,"name":"cargo.toml","value":"[package]\nname = \"demo\"\nversion = \"0.1.0\"\n"},{"affects_abi":true,"name":"libforge.yaml","value":"build:\n  targets:\n    - linux\nprecompiled_binaries:\n  url_prefix: https://github.com/stax/lib-forge/releases/download/precompiled_\n  public_key: demo-public-key\n"},{"affects_abi":true,"name":"manifest.schema_version","value":"libforge.manifest.v1"},{"affects_abi":true,"name":"rust.target_triple","value":"aarch64-apple-darwin"},{"affects_abi":true,"name":"uniffi.udl","value":"namespace demo; interface Demo { string ping(); };"}],"version":"b1"}"#;

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
            binding_metadata: AbiInput::new(BindingMetadataSet {
                bindings: vec![
                    BindingMetadata::Dart(DartBinding {
                        sdk_constraint: "3.0".to_string(),
                        ffi_abi: "1".to_string(),
                    }),
                    BindingMetadata::Kotlin(KotlinBinding {
                        min_sdk: 21,
                        jvm_target: "1.8".to_string(),
                        ndk_abis: vec!["arm64-v8a".to_string(), "x86_64".to_string()],
                    }),
                    BindingMetadata::Python(PythonBinding {
                        abi_tag: "cp311".to_string(),
                        platform_tag: "manylinux_2_28".to_string(),
                    }),
                    BindingMetadata::Swift(SwiftBinding {
                        toolchain: "5.9".to_string(),
                        deployment_target: "13.0".to_string(),
                    }),
                ],
            }),
            manifest_schema_version: AbiInput::new("libforge.manifest.v1".to_string()),
        }
    }

    #[test]
    fn hash_vector_is_stable() {
        let inputs = sample_inputs();
        let hash = hash_build_inputs(&inputs).expect("hash should succeed");
        assert_eq!(hash, GOLDEN_HASH_V1);
    }

    #[test]
    fn canonical_json_vector_is_stable() {
        let inputs = sample_inputs();
        let json = canonical_json(&inputs).expect("json should serialize");
        assert_eq!(json, GOLDEN_CANONICAL_JSON_V1);
    }

    #[test]
    fn hash_changes_on_abi_input() {
        let mut inputs = sample_inputs();
        if let BindingMetadata::Swift(binding) = &mut inputs.binding_metadata.value.bindings[3] {
            binding.toolchain = "5.10".to_string();
        }
        let hash = hash_build_inputs(&inputs).expect("hash should succeed");
        assert_ne!(hash, GOLDEN_HASH_V1);
    }
}
