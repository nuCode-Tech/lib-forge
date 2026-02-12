use std::path::{Path, PathBuf};

use xforge_build::cargo::CargoExecutor;
use xforge_build::cross::CrossExecutor;
use xforge_build::zigbuild::ZigbuildExecutor;
use xforge_build::BuildExecutor;
use xforge_core::artifact::layout::library_filename;
use xforge_core::build_id::{hash_release_inputs, AbiInput, BuildInputs};
use xforge_core::build_plan::{BuildPlan, BuildProfile, BuildTargetPlan, BuiltArtifact};
use xforge_core::config;
use xforge_core::platform::PlatformKey;
use xforge_core::toolchain::Toolchain;

use crate::commands::bundle::package_metadata;

pub struct BuildArgs {
    pub manifest_dir: PathBuf,
    pub target: Option<String>,
    pub profile: String,
    pub executor: BuildExecutorKind,
    pub cross_image: Option<String>,
}

#[derive(Clone, Debug)]
pub enum BuildExecutorKind {
    Cargo,
    Cross,
    Zigbuild,
}

pub struct BuildOutcome {
    pub build_id: String,
    pub library_path: PathBuf,
}

pub fn run(args: BuildArgs) -> Result<BuildOutcome, String> {
    let manifest_dir = args.manifest_dir;
    let targets = resolve_targets(&manifest_dir, args.target)?;
    let toolchain_settings =
        config::toolchain_settings(&manifest_dir).map_err(|err| err.to_string())?;

    let (package_name, _package_version) = package_metadata(&manifest_dir)?;
    let first_target = targets
        .first()
        .ok_or_else(|| "no build targets configured".to_string())?;
    let build_inputs =
        BuildInputs::from_manifest_dir(&manifest_dir, AbiInput::new(first_target.clone()), None)
            .map_err(|err| format!("failed to read build inputs: {}", err))?;
    let build_id = hash_release_inputs(&build_inputs)
        .map_err(|err| format!("failed to hash release inputs: {}", err))?;

    let profile = BuildProfile {
        name: args.profile.clone(),
        toolchain: Toolchain {
            channel: toolchain_settings.channel.clone(),
            targets: toolchain_settings.targets.clone(),
        },
        cargo_args: vec![],
        rustflags: vec![],
        env: vec![],
    };

    let mut target_plans = Vec::new();
    let target_root = resolve_target_root(&manifest_dir);
    for target in &targets {
        let rust_targets = PlatformKey::from_rust_target(target);
        if rust_targets.len() != 1 {
            return Err(format!("unsupported target '{}'", target));
        }
        let platform = rust_targets[0];
        let target_dir = target_root.join("target").join(target).join(&args.profile);
        let library_name = library_filename(&package_name, &platform);
        let library_path = target_dir.join(&library_name);
        let artifact_name = format!(
            "{}-{}-{}.{}",
            package_name,
            build_id,
            platform,
            xforge_core::artifact::naming::ArchiveKind::TarGz.extension()
        );
        let built_artifact = BuiltArtifact {
            platform,
            build_id: build_id.clone(),
            archive_kind: xforge_core::artifact::naming::ArchiveKind::TarGz,
            artifact_name,
            output_dir: target_dir.to_string_lossy().into_owned(),
            library_path: library_path.to_string_lossy().into_owned(),
            include_dir: None,
            manifest_path: manifest_dir
                .join("xforge-manifest.json")
                .to_string_lossy()
                .into_owned(),
            build_id_path: manifest_dir
                .join("build_id.txt")
                .to_string_lossy()
                .into_owned(),
        };
        let target_dir_arg = target_root.join("target").to_string_lossy().into_owned();
        target_plans.push(BuildTargetPlan {
            platform,
            rust_target_triple: target.clone(),
            working_dir: manifest_dir.to_string_lossy().into_owned(),
            cargo_manifest_path: "Cargo.toml".to_string(),
            cargo_args: vec!["--target-dir".to_string(), target_dir_arg.clone()],
            cargo_features: vec![],
            cross_image: args.cross_image.clone(),
            env: vec![xforge_core::build_plan::BuildEnvVar {
                key: "CARGO_TARGET_DIR".to_string(),
                value: target_dir_arg,
            }],
            artifact: built_artifact,
        });
    }

    let plan = BuildPlan {
        package_name,
        build_id: build_id.clone(),
        profile,
        targets: target_plans,
    };

    match args.executor {
        BuildExecutorKind::Cargo => {
            let executor = CargoExecutor::new();
            executor.execute(&plan).map_err(|err| err.to_string())?;
        }
        BuildExecutorKind::Cross => {
            let executor = CrossExecutor::new();
            executor.execute(&plan).map_err(|err| err.to_string())?;
        }
        BuildExecutorKind::Zigbuild => {
            let executor = ZigbuildExecutor::new();
            executor.execute(&plan).map_err(|err| err.to_string())?;
        }
    }

    let first_library = plan
        .targets
        .first()
        .map(|target| PathBuf::from(&target.artifact.library_path))
        .unwrap_or_else(|| manifest_dir.join("target"));
    Ok(BuildOutcome {
        build_id,
        library_path: first_library,
    })
}

pub(crate) fn resolve_targets(
    manifest_dir: &Path,
    target: Option<String>,
) -> Result<Vec<String>, String> {
    if let Some(target) = target {
        return Ok(vec![target]);
    }
    let targets = config::build_targets(manifest_dir).map_err(|err| err.to_string())?;
    if targets.is_empty() {
        return Err("no build targets configured".to_string());
    }
    Ok(targets)
}

fn resolve_target_root(manifest_dir: &Path) -> PathBuf {
    let mut current = Some(manifest_dir);
    while let Some(dir) = current {
        if dir.join("Cargo.lock").exists() {
            return dir.to_path_buf();
        }
        current = dir.parent();
    }
    manifest_dir.to_path_buf()
}
