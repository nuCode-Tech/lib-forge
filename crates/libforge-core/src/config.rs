use std::path::Path;

use serde::Deserialize;

use crate::platform::{all_rust_targets, is_supported_rust_target};

#[derive(Debug)]
pub enum ConfigError {
    Io(std::io::Error),
    Yaml(serde_yaml::Error),
    MissingTargets { path: String },
    InvalidTarget { target: String },
    MissingPrecompiledField { field: &'static str },
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::Io(error) => write!(f, "failed to read config: {}", error),
            ConfigError::Yaml(error) => write!(f, "failed to parse config: {}", error),
            ConfigError::MissingTargets { path } => {
                write!(f, "config '{}' must declare build.targets", path)
            }
            ConfigError::InvalidTarget { target } => {
                write!(f, "invalid build target '{}'", target)
            }
            ConfigError::MissingPrecompiledField { field } => {
                write!(f, "precompiled_binaries missing required field '{}'", field)
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
    #[serde(default)]
    precompiled_binaries: Option<PrecompiledBinariesConfig>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BuildConfig {
    #[serde(default)]
    targets: Vec<String>,
    #[serde(default)]
    toolchain: ToolchainConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ToolchainConfig {
    #[serde(default)]
    channel: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
struct PrecompiledBinariesConfig {
    repository: Option<String>,
    url_prefix: Option<String>,
    public_key: Option<String>,
}

#[derive(Debug, Default)]
pub struct ToolchainSettings {
    pub channel: Option<String>,
    pub targets: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrecompiledSettings {
    pub repository: String,
    pub url_prefix: String,
    pub public_key: String,
}

pub fn build_targets(manifest_dir: &Path) -> Result<Vec<String>, ConfigError> {
    let (path, contents) = match read_optional_config(manifest_dir)? {
        Some(value) => value,
        None => {
            return Ok(all_rust_targets()
                .into_iter()
                .map(|value| value.to_string())
                .collect())
        }
    };

    let config: LibforgeConfig = serde_yaml::from_str(&contents).map_err(ConfigError::Yaml)?;
    if config.build.targets.is_empty() {
        return Err(ConfigError::MissingTargets { path });
    }

    let mut targets = Vec::with_capacity(config.build.targets.len());
    for target in config.build.targets {
        if !is_supported_rust_target(&target) {
            return Err(ConfigError::InvalidTarget { target });
        }
        targets.push(target);
    }

    Ok(targets)
}

pub fn toolchain_settings(manifest_dir: &Path) -> Result<ToolchainSettings, ConfigError> {
    let (_path, contents) = match read_optional_config(manifest_dir)? {
        Some(value) => value,
        None => {
            return Ok(ToolchainSettings {
                channel: None,
                targets: all_rust_targets()
                    .into_iter()
                    .map(|value| value.to_string())
                    .collect(),
            })
        }
    };

    let config: LibforgeConfig = serde_yaml::from_str(&contents).map_err(ConfigError::Yaml)?;
    let targets = build_targets(manifest_dir)?;

    Ok(ToolchainSettings {
        channel: config.build.toolchain.channel,
        targets,
    })
}

pub fn precompiled_settings(
    manifest_dir: &Path,
) -> Result<Option<PrecompiledSettings>, ConfigError> {
    let (_path, contents) = match read_optional_config(manifest_dir)? {
        Some(value) => value,
        None => return Ok(None),
    };
    let config: LibforgeConfig = serde_yaml::from_str(&contents).map_err(ConfigError::Yaml)?;
    let precompiled = match config.precompiled_binaries {
        Some(value) => value,
        None => return Ok(None),
    };
    let repository = precompiled
        .repository
        .ok_or(ConfigError::MissingPrecompiledField {
            field: "repository",
        })?;
    let public_key = precompiled
        .public_key
        .ok_or(ConfigError::MissingPrecompiledField {
            field: "public_key",
        })?;
    let url_prefix = precompiled.url_prefix.unwrap_or_else(|| {
        format!(
            "https://github.com/{}/releases/download/",
            repository
        )
    });
    Ok(Some(PrecompiledSettings {
        repository,
        url_prefix,
        public_key,
    }))
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
        assert!(targets.contains(&"x86_64-unknown-linux-gnu".to_string()));
        assert!(targets.contains(&"aarch64-linux-android".to_string()));
    }

    #[test]
    fn reads_targets_from_yaml() {
        let dir = temp_dir("yaml-config");
        let path = dir.join("libforge.yaml");
        std::fs::write(
            path,
            "build:\n  targets:\n    - x86_64-unknown-linux-gnu\n    - aarch64-linux-android\n",
        )
        .expect("write config");
        let targets = build_targets(&dir).expect("targets");
        assert_eq!(targets.len(), 2);
        assert_eq!(targets[0], "x86_64-unknown-linux-gnu");
        assert_eq!(targets[1], "aarch64-linux-android");
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
