use serde_json;

use super::Manifest;

pub fn serialize_manifest(manifest: &Manifest) -> serde_json::Result<String> {
    serde_json::to_string(manifest)
}

pub fn serialize_manifest_pretty(manifest: &Manifest) -> serde_json::Result<String> {
    serde_json::to_string_pretty(manifest)
}

pub fn deserialize_manifest(input: &str) -> serde_json::Result<Manifest> {
    serde_json::from_str(input)
}
