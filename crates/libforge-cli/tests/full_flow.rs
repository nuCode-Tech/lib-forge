use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

use libforge_build::{cargo::CargoExecutor, BuildExecutor};
use libforge_core::{
    artifact::naming::{artifact_name, ArchiveKind},
    bindings::BindingMetadataSet,
    build_id::{hash_build_inputs, AbiInput, BuildInputs},
    build_plan::{BuildPlan, BuildProfile, BuildTargetPlan, BuiltArtifact},
    manifest::schema::SCHEMA_VERSION,
    platform::PlatformKey,
    toolchain::Toolchain,
};

fn temp_dir(name: &str) -> PathBuf {
    let mut path = env::temp_dir();
    let stamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    path.push(format!("libforge-cli-full-flow-{}-{}", name, stamp));
    fs::create_dir_all(&path).expect("create temp dir");
    path
}

fn host_target_triple() -> String {
    let output = Command::new("rustc")
        .arg("-vV")
        .output()
        .expect("failed to run rustc -vV");
    assert!(output.status.success(), "rustc -vV must succeed");
    let stdout = String::from_utf8(output.stdout).expect("rustc output utf8");
    for line in stdout.lines() {
        if let Some(triple) = line.strip_prefix("host: ") {
            return triple.trim().to_string();
        }
    }
    panic!("host triple not found");
}

fn init_sample_crate(manifest_dir: &Path, name: &str, target: &str) {
    let src = manifest_dir.join("src");
    fs::create_dir_all(&src).expect("create src dir");
    fs::write(
        manifest_dir.join("Cargo.toml"),
        format!(
            "[package]\nname = \"{}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
            name
        ),
    )
    .expect("write Cargo.toml");
    fs::write(
        src.join("lib.rs"),
        "pub fn greet() -> &'static str { \"hello\" }\n",
    )
    .expect("write lib.rs");
    fs::write(
        manifest_dir.join("Cargo.lock"),
        format!("[[package]]\nname = \"{}\"\nversion = \"0.1.0\"\n", name),
    )
    .expect("write Cargo.lock");
    fs::write(
        manifest_dir.join("libforge.yaml"),
        format!("build:\n  targets:\n    - {}\n", target),
    )
    .expect("write libforge.yaml");
}

fn target_release_dir(manifest_dir: &Path, target: &str) -> PathBuf {
    manifest_dir.join("target").join(target).join("release")
}

fn assert_release_rlib_exists(manifest_dir: &Path, target: &str, crate_name: &str) {
    let deps_dir = target_release_dir(manifest_dir, target).join("deps");
    let entries = fs::read_dir(&deps_dir).expect("read release deps");
    let prefix = format!("lib{}", crate_name);
    let mut found = false;
    for entry in entries {
        let path = entry.expect("read dir entry").path();
        let filename = path.file_name().and_then(|value| value.to_str());
        if let Some(name) = filename {
            if name.starts_with(&prefix) && name.ends_with(".rlib") {
                found = true;
                break;
            }
        }
    }
    assert!(found, "missing rlib in {}", deps_dir.display());
}

#[test]
fn full_flow_executes_core_plan_and_build_executor() {
    let dir = temp_dir("combined-flow");
    let target = host_target_triple();
    let package_name = "full-flow-cli";
    let crate_name = "full_flow_cli";
    init_sample_crate(&dir, package_name, &target);

    let toolchain = Toolchain::from_manifest_dir(&dir).expect("toolchain settings");

    let binding_metadata = BindingMetadataSet { bindings: vec![] };

    let inputs = BuildInputs::from_manifest_dir(
        &dir,
        AbiInput::new(target.clone()),
        None,
        AbiInput::new(binding_metadata.clone()),
        AbiInput::new(SCHEMA_VERSION.to_string()),
    )
    .expect("build inputs");
    let build_id = hash_build_inputs(&inputs).expect("hash build inputs");

    let keys = PlatformKey::from_rust_target(&target);
    assert_eq!(keys.len(), 1);
    let platform = keys[0];
    let artifact =
        artifact_name(package_name, &build_id, &platform, ArchiveKind::TarGz).expect("name");

    let release_dir = target_release_dir(&dir, &target);
    let profile = BuildProfile {
        name: "release".to_string(),
        toolchain: toolchain.clone(),
        cargo_args: vec![],
        rustflags: vec![],
        env: vec![],
    };

    let plan = BuildPlan {
        package_name: package_name.to_string(),
        build_id: build_id.clone(),
        profile,
        targets: vec![BuildTargetPlan {
            platform,
            rust_target_triple: target.clone(),
            working_dir: dir.to_string_lossy().into_owned(),
            cargo_manifest_path: dir.join("Cargo.toml").to_string_lossy().into_owned(),
            cargo_args: vec![],
            cargo_features: vec![],
            cross_image: None,
            env: vec![],
            artifact: BuiltArtifact {
                platform,
                build_id: build_id.clone(),
                archive_kind: ArchiveKind::TarGz,
                artifact_name: artifact.clone(),
                output_dir: release_dir.to_string_lossy().into_owned(),
                library_path: dir
                    .join("target")
                    .join(&target)
                    .join("release")
                    .join(format!("lib{}.rlib", crate_name))
                    .to_string_lossy()
                    .into_owned(),
                include_dir: None,
                manifest_path: dir
                    .join("libforge-manifest.json")
                    .to_string_lossy()
                    .into_owned(),
                checksums_path: dir.join("checksums.txt").to_string_lossy().into_owned(),
                build_id_path: dir.join("build-id.txt").to_string_lossy().into_owned(),
            },
        }],
    };

    let executor = CargoExecutor::new();
    let artifacts = executor.execute(&plan).expect("build executor ran");

    assert_eq!(artifacts.len(), 1);
    assert_eq!(artifacts[0].build_id, build_id);
    assert_eq!(artifacts[0].artifact_name, artifact);
    assert_release_rlib_exists(&dir, &target, crate_name);
}
