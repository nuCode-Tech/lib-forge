use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use libforge_core::{
    artifact::naming::{artifact_name, checksum_name, ArchiveKind, ChecksumKind},
    bindings::{BindingMetadata, BindingMetadataSet, DartBinding},
    build_id::{hash_build_inputs, AbiInput, BuildInputs},
    config,
    manifest::schema::SCHEMA_VERSION,
    platform::PlatformKey,
};

fn temp_dir(name: &str) -> PathBuf {
    let mut path = env::temp_dir();
    let stamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    path.push(format!("libforge-core-integration-{}-{}", name, stamp));
    fs::create_dir_all(&path).expect("create temp dir");
    path
}

fn write_cargo_files(manifest_dir: &Path, package_name: &str) {
    let cargo_toml = format!(
        "[package]\nname = \"{}\"\nversion = \"0.1.0\"\n",
        package_name
    );
    let cargo_lock = format!(
        "[[package]]\nname = \"{}\"\nversion = \"0.1.0\"\n",
        package_name
    );
    fs::write(manifest_dir.join("Cargo.toml"), cargo_toml).expect("write Cargo.toml");
    fs::write(manifest_dir.join("Cargo.lock"), cargo_lock).expect("write Cargo.lock");
}

#[test]
fn integration_flow_from_config_to_artifact_identity() {
    let dir = temp_dir("full-flow");
    let config_contents = r#"build:
  targets:
    - x86_64-unknown-linux-gnu
    - aarch64-apple-darwin
  toolchain:
    channel: nightly
"#;
    fs::write(dir.join("libforge.yaml"), config_contents).expect("write config");

    const LIB_NAME: &str = "integration-demo";
    write_cargo_files(&dir, LIB_NAME);

    let binding_metadata = BindingMetadataSet {
        bindings: vec![BindingMetadata::Dart(DartBinding {
            sdk_constraint: ">=3.0".to_string(),
            ffi_abi: "1".to_string(),
        })],
    };

    let targets = config::build_targets(&dir).expect("build targets");
    let expected = vec![
        "x86_64-unknown-linux-gnu".to_string(),
        "aarch64-apple-darwin".to_string(),
    ];
    assert_eq!(targets, expected);

    let settings = config::toolchain_settings(&dir).expect("toolchain settings");
    assert_eq!(settings.channel.as_deref(), Some("nightly"));
    assert_eq!(settings.targets, expected);

    for target in expected {
        let keys = PlatformKey::from_rust_target(&target);
        assert_eq!(keys.len(), 1);
        let platform = keys[0];
        let inputs = BuildInputs::from_manifest_dir(
            &dir,
            AbiInput::new(target.clone()),
            None,
            AbiInput::new(binding_metadata.clone()),
            AbiInput::new(SCHEMA_VERSION.to_string()),
        )
        .expect("build inputs");
        let build_id = hash_build_inputs(&inputs).expect("hash build inputs");
        assert!(build_id.starts_with("b1-"));
        let artifact =
            artifact_name(LIB_NAME, &build_id, &platform, ArchiveKind::TarGz).expect("artifact");
        assert!(artifact.starts_with(LIB_NAME));
        assert!(artifact.contains(&build_id));
        assert!(artifact.contains(platform.as_str()));
        assert!(artifact.ends_with(".tar.gz"));
        let checksum = checksum_name(&artifact, ChecksumKind::Sha256);
        assert!(checksum.ends_with(".sha256"));
    }
}
