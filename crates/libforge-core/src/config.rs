use std::path::Path;

use serde::Deserialize;

use crate::platform::{all_platform_keys, PlatformKey, PlatformKeyError};

#[derive(Debug)]
pub enum ConfigError {
    Io(std::io::Error),
    Yaml(serde_yaml::Error),
    MissingTargets {
        path: String,
    },
    InvalidTarget {
        target: String,
        source: PlatformKeyError,
    },
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::Io(error) => write!(f, "failed to read config: {}", error),
            ConfigError::Yaml(error) => write!(f, "failed to parse config: {}", error),
            ConfigError::MissingTargets { path } => {
                write!(f, "config '{}' must declare build.targets", path)
            }
            ConfigError::InvalidTarget { target, source } => {
                write!(f, "invalid build target '{}': {}", target, source)
            }
        }
    }
}

impl std::error::Error for ConfigError {}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LibforgeConfig {
    #[serde(default)]
    build: BuildConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BuildConfig {
    #[serde(default)]
    targets: Vec<String>,
}

pub fn build_targets(manifest_dir: &Path) -> Result<Vec<PlatformKey>, ConfigError> {
    let (path, contents) = match read_optional_config(manifest_dir)? {
        Some(value) => value,
        None => return Ok(all_platform_keys()),
    };

    let config: LibforgeConfig = serde_yaml::from_str(&contents).map_err(ConfigError::Yaml)?;
    if config.build.targets.is_empty() {
        return Err(ConfigError::MissingTargets { path });
    }

    let mut targets = Vec::with_capacity(config.build.targets.len());
    for target in config.build.targets {
        let parsed =
            target
                .parse::<PlatformKey>()
                .map_err(|source| ConfigError::InvalidTarget {
                    target: target.clone(),
                    source,
                })?;
        targets.push(parsed);
    }

    Ok(targets)
}

fn read_optional_config(manifest_dir: &Path) -> Result<Option<(String, String)>, ConfigError> {
    let yaml_path = manifest_dir.join("libforge.yaml");
    if !yaml_path.exists() {
        return Ok(None);
    }

    let contents = std::fs::read_to_string(&yaml_path).map_err(ConfigError::Io)?;
    Ok(Some((
        yaml_path.to_str().unwrap_or("libforge.yaml").to_string(),
        contents,
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let mut path = std::env::temp_dir();
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        path.push(format!("libforge-core-{}-{}", name, stamp));
        std::fs::create_dir_all(&path).expect("create temp dir");
        path
    }

    #[test]
    fn defaults_to_all_platform_keys_when_missing() {
        let dir = temp_dir("missing-config");
        let targets = build_targets(&dir).expect("targets");
        assert!(!targets.is_empty());
        assert!(targets.contains(&PlatformKey::LinuxX86_64));
        assert!(targets.contains(&PlatformKey::AndroidArm64));
    }

    #[test]
    fn reads_targets_from_yaml() {
        let dir = temp_dir("yaml-config");
        let path = dir.join("libforge.yaml");
        std::fs::write(
            path,
            "build:\n  targets:\n    - linux-x86_64\n    - android-arm64\n",
        )
        .expect("write config");
        let targets = build_targets(&dir).expect("targets");
        assert_eq!(targets.len(), 2);
        assert_eq!(targets[0], PlatformKey::LinuxX86_64);
        assert_eq!(targets[1], PlatformKey::AndroidArm64);
    }

    #[test]
    fn invalid_target_is_rejected() {
        let dir = temp_dir("invalid-target");
        let path = dir.join("libforge.yaml");
        std::fs::write(path, "build:\n  targets:\n    - linux\n").expect("write config");
        let error = build_targets(&dir).expect_err("error");
        let message = error.to_string();
        assert!(message.contains("invalid build target"));
    }
}
