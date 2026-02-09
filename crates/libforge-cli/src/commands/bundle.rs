use std::fs;
use std::path::{Path, PathBuf};

use libforge_core::artifact::layout::{archive_layout, default_archive_kind};
use libforge_core::artifact::naming::{artifact_name, ArchiveKind};
use libforge_core::build_id::{hash_build_inputs, hash_release_inputs, AbiInput, BuildInputs};
use libforge_core::build_plan::BuiltArtifact;
use libforge_core::config;
use libforge_core::manifest::{
    ArtifactNaming, Artifacts, Bindings, Build, BuildIdentity, Manifest, Package, Platform,
    Platforms,
};
use libforge_core::platform::PlatformKey;
use libforge_pack::{PackExecutor, PackFormat, PackInput, PackRequest, TarGzPacker, ZipPacker};

use super::build::resolve_targets;

pub struct BundleArgs {
    pub manifest_dir: PathBuf,
    pub target: Option<String>,
    pub output_dir: PathBuf,
    pub profile: String,
}

pub struct BundleOutcome {
    pub build_id: String,
    pub manifest_path: PathBuf,
    pub archive_paths: Vec<PathBuf>,
}

pub fn run(args: BundleArgs) -> Result<BundleOutcome, String> {
    let manifest_dir = args.manifest_dir;
    let targets = resolve_targets(&manifest_dir, args.target)?;
    let toolchain_settings = config::toolchain_settings(&manifest_dir).map_err(|err| err.to_string())?;
    let (package_name, package_version) = package_metadata(&manifest_dir)?;

    let first_target = targets
        .first()
        .ok_or_else(|| "no build targets configured".to_string())?;
    let build_inputs = BuildInputs::from_manifest_dir(
        &manifest_dir,
        AbiInput::new(first_target.clone()),
        None,
    )
    .map_err(|err| format!("failed to read build inputs: {}", err))?;
    let build_id = hash_release_inputs(&build_inputs)
        .map_err(|err| format!("failed to hash release inputs: {}", err))?;

    fs::create_dir_all(&args.output_dir)
        .map_err(|err| format!("failed to create output dir: {}", err))?;

    let manifest_path = args.output_dir.join("libforge-manifest.json");
    let build_id_path = args.output_dir.join("build_id.txt");
    fs::write(&build_id_path, build_id.as_bytes())
        .map_err(|err| format!("failed to write build_id: {}", err))?;

    let host = rustc_host_triple().unwrap_or_else(|| "unknown".to_string());
    let toolchain = toolchain_settings
        .channel
        .unwrap_or_else(|| "default".to_string());

    let mut platform_entries = Vec::new();
    let mut archive_paths = Vec::new();

    let manifest = Manifest {
        schema_version: libforge_core::manifest::schema::SCHEMA_VERSION.to_string(),
        package: Package {
            name: package_name.clone(),
            version: package_version,
            description: None,
            license: None,
            authors: vec![],
            repository: None,
        },
        build: Build {
            id: build_id.clone(),
            identity: BuildIdentity {
                host,
                toolchain,
                profile: Some(args.profile.clone()),
                features: vec![],
            },
            timestamp: None,
            engine: None,
        },
        artifacts: Artifacts {
            naming: ArtifactNaming {
                template: "{package.name}-{build.id}-{platform}".to_string(),
                delimiter: "-".to_string(),
                include_platform: true,
                include_binding: false,
            },
        },
        bindings: Bindings {
            catalog: vec![],
            primary: None,
        },
        platforms: Platforms {
            default: first_target.clone(),
            targets: vec![],
        },
        signing: None,
    };
    let mut manifest = manifest;

    for target in &targets {
        let rust_targets = PlatformKey::from_rust_target(target);
        if rust_targets.len() != 1 {
            return Err(format!("unsupported target '{}'", target));
        }
        let platform = rust_targets[0];
        let per_target_inputs = BuildInputs::from_manifest_dir(
            &manifest_dir,
            AbiInput::new(target.clone()),
            None,
        )
        .map_err(|err| format!("failed to read build inputs: {}", err))?;
        let per_target_build_id = hash_build_inputs(&per_target_inputs)
            .map_err(|err| format!("failed to hash build inputs: {}", err))?;
        let archive_kind = default_archive_kind(&platform);
        let archive_name =
            artifact_name(&package_name, &build_id, &platform, archive_kind).map_err(|err| err.to_string())?;
        let library_path = manifest_dir
            .join("target")
            .join(target)
            .join(&args.profile)
            .join(libforge_core::artifact::layout::library_filename(&package_name, &platform));
        if !library_path.exists() {
            return Err(format!(
                "library not found at '{}'; run libforge build first",
                library_path.display()
            ));
        }
        let built_artifact = BuiltArtifact {
            platform,
            build_id: build_id.clone(),
            archive_kind,
            artifact_name: archive_name.clone(),
            output_dir: args.output_dir.to_string_lossy().into_owned(),
            library_path: library_path.to_string_lossy().into_owned(),
            include_dir: None,
            manifest_path: manifest_path.to_string_lossy().into_owned(),
            build_id_path: build_id_path.to_string_lossy().into_owned(),
        };
        let layout = archive_layout(&package_name, &platform);
        let pack_input = PackInput {
            artifact: built_artifact,
            layout,
        };
        let pack_request = PackRequest {
            format: match archive_kind {
                ArchiveKind::TarGz => PackFormat::TarGz,
                ArchiveKind::Zip => PackFormat::Zip,
            },
            inputs: vec![pack_input],
            output_dir: args.output_dir.to_string_lossy().into_owned(),
        };
        let archive_path = match archive_kind {
            ArchiveKind::TarGz => {
                let packer = TarGzPacker;
                packer
                    .pack(&pack_request)
                    .map_err(|err| err.to_string())?
            }
            ArchiveKind::Zip => {
                let packer = ZipPacker;
                packer
                    .pack(&pack_request)
                    .map_err(|err| err.to_string())?
            }
        }
        .output_paths
        .get(0)
        .ok_or_else(|| "missing archive output".to_string())?
        .clone();

        archive_paths.push(PathBuf::from(archive_path));
        platform_entries.push(Platform {
            name: target.clone(),
            build_id: per_target_build_id,
            triples: vec![target.clone()],
            bindings: vec![],
            artifacts: vec![archive_name],
            description: None,
        });
    }

    manifest.platforms.targets = platform_entries;
    let manifest_contents = libforge_core::manifest::serialize_manifest_pretty(&manifest)
        .map_err(|err| err.to_string())?;
    fs::write(&manifest_path, manifest_contents)
        .map_err(|err| format!("failed to write manifest: {}", err))?;

    Ok(BundleOutcome {
        build_id,
        manifest_path,
        archive_paths,
    })
}

pub fn package_metadata(manifest_dir: &Path) -> Result<(String, String), String> {
    #[derive(serde::Deserialize)]
    struct CargoToml {
        package: CargoPackage,
    }

    #[derive(serde::Deserialize)]
    struct CargoPackage {
        name: String,
        version: String,
    }

    let cargo_toml_path = manifest_dir.join("Cargo.toml");
    let contents = fs::read_to_string(&cargo_toml_path).map_err(|err| {
        format!(
            "failed to read Cargo.toml '{}': {}",
            cargo_toml_path.display(),
            err
        )
    })?;
    let parsed: CargoToml = toml::from_str(&contents)
        .map_err(|err| format!("failed to parse Cargo.toml: {}", err))?;
    Ok((parsed.package.name, parsed.package.version))
}

fn rustc_host_triple() -> Option<String> {
    let output = std::process::Command::new("rustc").arg("-vV").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if let Some(triple) = line.strip_prefix("host: ") {
            return Some(triple.trim().to_string());
        }
    }
    None
}
