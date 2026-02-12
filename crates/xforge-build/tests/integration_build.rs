use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

use xforge_build::{cargo::CargoExecutor, BuildExecutor};
use xforge_core::{
    artifact::naming::{artifact_name, ArchiveKind},
    build_id::{hash_build_inputs, release_hash, AbiInput, BuildInputs},
    build_plan::{BuildPlan, BuildProfile, BuildTargetPlan, BuiltArtifact},
    config,
    platform::PlatformKey,
    toolchain::Toolchain,
};

fn temp_dir(name: &str) -> PathBuf {
    let mut path = env::temp_dir();
    let stamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    path.push(format!("xforge-build-integration-{}-{}", name, stamp));
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
    panic!("host triple not found in rustc output");
}

fn init_sample_crate(manifest_dir: &Path, name: &str, target: &str) {
    let src_dir = manifest_dir.join("src");
    fs::create_dir_all(&src_dir).expect("create src dir");
    fs::write(
        manifest_dir.join("Cargo.toml"),
        format!(
            "[package]\nname = \"{}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
            name
        ),
    )
    .expect("write Cargo.toml");
    fs::write(
        src_dir.join("lib.rs"),
        "pub fn hello() -> &'static str { \"world\" }\n",
    )
    .expect("write lib.rs");
    let config = format!("build:\n  targets:\n    - {}\n", target);
    fs::write(manifest_dir.join("xforge.yaml"), config).expect("write xforge.yaml");
    fs::write(
        manifest_dir.join("Cargo.lock"),
        format!("[[package]]\nname = \"{}\"\nversion = \"0.1.0\"\n", name),
    )
    .expect("write Cargo.lock");
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
fn integration_build_flow_runs_host_build() {
    let dir = temp_dir("build-flow");
    let target = host_target_triple();
    let package_name = "integration-build";
    let crate_name = "integration_build";
    init_sample_crate(&dir, package_name, &target);

    let settings = config::toolchain_settings(&dir).expect("toolchain settings");
    assert!(settings.targets.contains(&target));

    let inputs = BuildInputs::from_manifest_dir(
        &dir,
        AbiInput::new(target.clone()),
        None,
    )
    .expect("collect build inputs");
    let build_id = hash_build_inputs(&inputs).expect("hash build inputs");
    let release_hash = release_hash(&build_id);
    assert_eq!(release_hash, build_id);

    let rust_targets = PlatformKey::from_rust_target(&target);
    assert_eq!(rust_targets.len(), 1);
    let platform = rust_targets[0];

    let release_dir = target_release_dir(&dir, &target);
    let artifact_name = artifact_name(package_name, &build_id, &platform, ArchiveKind::TarGz)
        .expect("artifact name");
    let built_artifact = BuiltArtifact {
        platform,
        build_id: build_id.clone(),
        archive_kind: ArchiveKind::TarGz,
        artifact_name: artifact_name.clone(),
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
            .join("xforge-manifest.json")
            .to_string_lossy()
            .into_owned(),
        build_id_path: dir.join("build-id.txt").to_string_lossy().into_owned(),
    };

    let plan = BuildPlan {
        package_name: package_name.to_string(),
        build_id: build_id.clone(),
        profile: BuildProfile {
            name: "release".to_string(),
            toolchain: Toolchain::default(),
            cargo_args: vec![],
            rustflags: vec![],
            env: vec![],
        },
        targets: vec![BuildTargetPlan {
            platform,
            rust_target_triple: target.clone(),
            working_dir: dir.to_string_lossy().into_owned(),
            cargo_manifest_path: dir.join("Cargo.toml").to_string_lossy().into_owned(),
            cargo_args: vec![],
            cargo_features: vec![],
            cross_image: None,
            env: vec![],
            artifact: built_artifact,
        }],
    };

    let executor = CargoExecutor::new();
    let artifacts = executor.execute(&plan).expect("cargo executor succeeded");
    assert_eq!(artifacts.len(), 1);
    assert_eq!(artifacts[0].build_id, build_id);
    assert_eq!(artifacts[0].artifact_name, artifact_name);
    assert_release_rlib_exists(&dir, &target, crate_name);
}
