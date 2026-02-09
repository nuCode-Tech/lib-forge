use crate::artifact::ArchiveKind;
use crate::platform::PlatformKey;
use crate::toolchain::Toolchain;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuildPlan {
    pub package_name: String,
    pub build_id: String,
    pub profile: BuildProfile,
    pub targets: Vec<BuildTargetPlan>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuildProfile {
    pub name: String,
    pub toolchain: Toolchain,
    pub cargo_args: Vec<String>,
    pub rustflags: Vec<String>,
    pub env: Vec<BuildEnvVar>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuildTargetPlan {
    pub platform: PlatformKey,
    pub rust_target_triple: String,
    pub working_dir: String,
    pub cargo_manifest_path: String,
    pub cargo_args: Vec<String>,
    pub cargo_features: Vec<String>,
    pub cross_image: Option<String>,
    pub env: Vec<BuildEnvVar>,
    pub artifact: BuiltArtifact,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuiltArtifact {
    pub platform: PlatformKey,
    pub build_id: String,
    pub archive_kind: ArchiveKind,
    pub artifact_name: String,
    pub output_dir: String,
    pub library_path: String,
    pub include_dir: Option<String>,
    pub manifest_path: String,
    pub build_id_path: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuildEnvVar {
    pub key: String,
    pub value: String,
}
