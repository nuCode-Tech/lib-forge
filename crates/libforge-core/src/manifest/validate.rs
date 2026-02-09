use std::collections::{HashMap, HashSet};

use super::Manifest;
use crate::platform::PlatformKey;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManifestError {
    InvalidPlatformKey {
        platform: String,
    },
    InvalidDefaultPlatform {
        platform: String,
    },
    UnknownBindingPlatform {
        binding: String,
        platform: String,
    },
    BindingVersionMissing {
        binding: String,
    },
    DuplicateArtifactIdentifier {
        identifier: String,
    },
    ArtifactMissingPlatform {
        binding: String,
        artifact: String,
    },
    ArtifactPlatformMismatch {
        binding: String,
        artifact: String,
        platform: String,
    },
    AbiFieldMissing {
        field: String,
    },
    EmptyArtifactIdentifier {
        platform: String,
    },
    MissingPlatformBuildId {
        platform: String,
    },
}

impl std::fmt::Display for ManifestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ManifestError::InvalidPlatformKey { platform } => {
                write!(f, "platform '{}' is not a valid platform key", platform)
            }
            ManifestError::InvalidDefaultPlatform { platform } => write!(
                f,
                "platforms.default '{}' must match a platforms.targets[].name value",
                platform
            ),
            ManifestError::UnknownBindingPlatform { binding, platform } => write!(
                f,
                "binding '{}' references unknown platform '{}'",
                binding, platform
            ),
            ManifestError::BindingVersionMissing { binding } => {
                write!(f, "binding '{}' must declare a language version", binding)
            }
            ManifestError::DuplicateArtifactIdentifier { identifier } => write!(
                f,
                "artifact identifier '{}' must be unique across platforms",
                identifier
            ),
            ManifestError::ArtifactMissingPlatform { binding, artifact } => write!(
                f,
                "binding '{}' references artifact '{}' that is not declared by any platform",
                binding, artifact
            ),
            ManifestError::ArtifactPlatformMismatch {
                binding,
                artifact,
                platform,
            } => write!(
                f,
                "binding '{}' references artifact '{}' which belongs to platform '{}'",
                binding, artifact, platform
            ),
            ManifestError::AbiFieldMissing { field } => {
                write!(f, "ABI-affecting field '{}' must be declared", field)
            }
            ManifestError::EmptyArtifactIdentifier { platform } => write!(
                f,
                "platform '{}' contains an empty artifact identifier",
                platform
            ),
            ManifestError::MissingPlatformBuildId { platform } => {
                write!(f, "platform '{}' missing build_id", platform)
            }
        }
    }
}

impl std::error::Error for ManifestError {}

pub fn validate(manifest: &Manifest) -> Result<(), ManifestError> {
    for platform in &manifest.platforms.targets {
        if platform.name.parse::<PlatformKey>().is_err() {
            return Err(ManifestError::InvalidPlatformKey {
                platform: platform.name.clone(),
            });
        }
        if platform.build_id.trim().is_empty() {
            return Err(ManifestError::MissingPlatformBuildId {
                platform: platform.name.clone(),
            });
        }
    }

    if manifest.platforms.default.parse::<PlatformKey>().is_err() {
        return Err(ManifestError::InvalidPlatformKey {
            platform: manifest.platforms.default.clone(),
        });
    }

    let platform_names: HashSet<&str> = manifest
        .platforms
        .targets
        .iter()
        .map(|platform| platform.name.as_str())
        .collect();

    if !platform_names.contains(manifest.platforms.default.as_str()) {
        return Err(ManifestError::InvalidDefaultPlatform {
            platform: manifest.platforms.default.clone(),
        });
    }

    if manifest
        .build
        .identity
        .profile
        .as_ref()
        .map(|value| value.trim().is_empty())
        .unwrap_or(true)
    {
        return Err(ManifestError::AbiFieldMissing {
            field: "build.identity.profile".to_string(),
        });
    }

    for platform in &manifest.platforms.targets {
        if platform.triples.is_empty() {
            return Err(ManifestError::AbiFieldMissing {
                field: format!("platforms.targets[{}].triples", platform.name),
            });
        }
    }

    let mut artifact_platforms: HashMap<String, String> = HashMap::new();
    for platform in &manifest.platforms.targets {
        for artifact in &platform.artifacts {
            if artifact.trim().is_empty() {
                return Err(ManifestError::EmptyArtifactIdentifier {
                    platform: platform.name.clone(),
                });
            }

            if artifact_platforms
                .insert(artifact.clone(), platform.name.clone())
                .is_some()
            {
                return Err(ManifestError::DuplicateArtifactIdentifier {
                    identifier: artifact.clone(),
                });
            }
        }
    }

    for binding in &manifest.bindings.catalog {
        if binding.version.trim().is_empty() {
            return Err(ManifestError::BindingVersionMissing {
                binding: binding.name.clone(),
            });
        }

        for platform in &binding.platforms {
            if !platform_names.contains(platform.as_str()) {
                return Err(ManifestError::UnknownBindingPlatform {
                    binding: binding.name.clone(),
                    platform: platform.clone(),
                });
            }
        }

        for artifact in &binding.artifacts {
            let platform = match artifact_platforms.get(artifact) {
                Some(platform) => platform,
                None => {
                    return Err(ManifestError::ArtifactMissingPlatform {
                        binding: binding.name.clone(),
                        artifact: artifact.clone(),
                    })
                }
            };

            if !binding.platforms.is_empty()
                && !binding.platforms.iter().any(|value| value == platform)
            {
                return Err(ManifestError::ArtifactPlatformMismatch {
                    binding: binding.name.clone(),
                    artifact: artifact.clone(),
                    platform: platform.clone(),
                });
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{
        ArtifactNaming, Artifacts, BindingDescriptor, Bindings, Build, BuildIdentity, Manifest,
        Package, Platform, Platforms,
    };

    fn sample_manifest() -> Manifest {
        Manifest {
            schema_version: "libforge.manifest.v1".to_string(),
            signing: None,
            package: Package {
                name: "libforge-sample".to_string(),
                version: "0.1.0".to_string(),
                description: None,
                license: None,
                authors: vec![],
                repository: None,
            },
            build: Build {
                id: "build-1".to_string(),
                identity: BuildIdentity {
                    host: "linux".to_string(),
                    toolchain: "rustc 1.78.0".to_string(),
                    profile: Some("release".to_string()),
                    features: vec!["feature-a".to_string()],
                },
                timestamp: None,
                engine: None,
            },
            artifacts: Artifacts {
                naming: ArtifactNaming {
                    template: "{package.name}-{package.version}-{platform}".to_string(),
                    delimiter: "-".to_string(),
                    include_platform: true,
                    include_binding: true,
                },
            },
            bindings: Bindings {
                primary: None,
                catalog: vec![BindingDescriptor {
                    name: "dart".to_string(),
                    version: "3.0.0".to_string(),
                    platforms: vec!["x86_64-unknown-linux-gnu".to_string()],
                    artifacts: vec!["bundle".to_string()],
                }],
            },
            platforms: Platforms {
                default: "x86_64-unknown-linux-gnu".to_string(),
                targets: vec![Platform {
                    name: "x86_64-unknown-linux-gnu".to_string(),
                    build_id: "b1-demo".to_string(),
                    triples: vec!["x86_64-unknown-linux-gnu".to_string()],
                    bindings: vec!["dart".to_string()],
                    artifacts: vec!["bundle".to_string()],
                    description: None,
                }],
            },
        }
    }

    #[test]
    fn invalid_default_platform_fails() {
        let mut manifest = sample_manifest();
        manifest.platforms.default = "ios-arm64".to_string();

        let result = validate(&manifest);
        assert!(matches!(
            result,
            Err(ManifestError::InvalidDefaultPlatform { .. })
        ));
    }

    #[test]
    fn invalid_platform_key_fails() {
        let mut manifest = sample_manifest();
        manifest.platforms.targets[0].name = "linux".to_string();

        let result = validate(&manifest);
        assert!(matches!(
            result,
            Err(ManifestError::InvalidPlatformKey { .. })
        ));
    }

    #[test]
    fn binding_version_missing_fails() {
        let mut manifest = sample_manifest();
        manifest.bindings.catalog[0].version = " ".to_string();

        let result = validate(&manifest);
        assert!(matches!(
            result,
            Err(ManifestError::BindingVersionMissing { .. })
        ));
    }

    #[test]
    fn duplicate_artifact_identifier_fails() {
        let mut manifest = sample_manifest();
        manifest.platforms.targets.push(Platform {
            name: "aarch64-linux-android".to_string(),
            build_id: "b1-demo-android".to_string(),
            triples: vec!["aarch64-linux-android".to_string()],
            bindings: vec!["dart".to_string()],
            artifacts: vec!["bundle".to_string()],
            description: None,
        });

        let result = validate(&manifest);
        assert!(matches!(
            result,
            Err(ManifestError::DuplicateArtifactIdentifier { .. })
        ));
    }

    #[test]
    fn artifact_missing_platform_fails() {
        let mut manifest = sample_manifest();
        manifest.bindings.catalog[0].artifacts = vec!["missing".to_string()];

        let result = validate(&manifest);
        assert!(matches!(
            result,
            Err(ManifestError::ArtifactMissingPlatform { .. })
        ));
    }

    #[test]
    fn artifact_platform_mismatch_fails() {
        let mut manifest = sample_manifest();
        manifest.platforms.targets.push(Platform {
            name: "aarch64-linux-android".to_string(),
            triples: vec!["aarch64-linux-android".to_string()],
            bindings: vec!["dart".to_string()],
            artifacts: vec![],
            description: None,
            build_id: "b1-demo-android".to_string(),
        });
        manifest.bindings.catalog[0].platforms = vec!["aarch64-linux-android".to_string()];

        let result = validate(&manifest);
        assert!(matches!(
            result,
            Err(ManifestError::ArtifactPlatformMismatch { .. })
        ));
    }

    #[test]
    fn abi_field_missing_fails() {
        let mut manifest = sample_manifest();
        manifest.build.identity.profile = None;

        let result = validate(&manifest);
        assert!(matches!(result, Err(ManifestError::AbiFieldMissing { .. })));
    }
}
