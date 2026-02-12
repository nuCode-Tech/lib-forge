use std::path::Path;

use serde::Deserialize;

use crate::platform::is_supported_rust_target;

#[derive(Debug)]
pub enum ConfigError {
    Io(std::io::Error),
    Toml(toml::de::Error),
    Yaml(serde_yaml::Error),
    MissingToolchainFile,
    MissingToolchainField { field: &'static str, path: String },
    InvalidTarget { target: String },
    MissingPrecompiledField { field: &'static str },
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::Io(error) => write!(f, "failed to read config: {}", error),
            ConfigError::Toml(error) => write!(f, "failed to parse rust-toolchain.toml: {}", error),
            ConfigError::Yaml(error) => write!(f, "failed to parse config: {}", error),
            ConfigError::MissingToolchainFile => {
                write!(f, "rust-toolchain.toml not found in manifest dir or repo root")
            }
            ConfigError::MissingToolchainField { field, path } => write!(
                f,
                "rust-toolchain.toml '{}' missing required field '{}'",
                path, field
            ),
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
struct XforgeConfig {
    #[serde(default)]
    precompiled_binaries: Option<PrecompiledBinariesConfig>,
}

#[derive(Debug, Deserialize)]
struct RustToolchainConfig {
    toolchain: Option<RustToolchainSettings>,
}

#[derive(Debug, Default, Deserialize)]
struct RustToolchainSettings {
    channel: Option<String>,
    targets: Option<Vec<String>>,
    components: Option<Vec<String>>,
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
    pub components: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrecompiledSettings {
    pub repository: String,
    pub url_prefix: String,
    pub public_key: String,
}

pub fn build_targets(manifest_dir: &Path) -> Result<Vec<String>, ConfigError> {
    let settings = toolchain_settings(manifest_dir)?;
    Ok(settings.targets)
}

pub fn toolchain_settings(manifest_dir: &Path) -> Result<ToolchainSettings, ConfigError> {
    let (path, contents) = read_rust_toolchain(manifest_dir)?;
    let parsed: RustToolchainConfig = toml::from_str(&contents).map_err(ConfigError::Toml)?;
    let toolchain = parsed.toolchain.ok_or_else(|| ConfigError::MissingToolchainField {
        field: "toolchain",
        path: path.clone(),
    })?;
    let channel = toolchain.channel.filter(|value| !value.trim().is_empty());
    let channel = channel.ok_or_else(|| ConfigError::MissingToolchainField {
        field: "toolchain.channel",
        path: path.clone(),
    })?;
    let targets = toolchain
        .targets
        .filter(|value| !value.is_empty())
        .ok_or_else(|| ConfigError::MissingToolchainField {
            field: "toolchain.targets",
            path: path.clone(),
        })?;
    for target in &targets {
        if !is_supported_rust_target(target) {
            return Err(ConfigError::InvalidTarget {
                target: target.clone(),
            });
        }
    }
    let components = toolchain
        .components
        .filter(|value| !value.is_empty())
        .ok_or_else(|| ConfigError::MissingToolchainField {
            field: "toolchain.components",
            path: path.clone(),
        })?;
    Ok(ToolchainSettings {
        channel: Some(channel),
        targets,
        components,
    })
}

pub fn precompiled_settings(
    manifest_dir: &Path,
) -> Result<Option<PrecompiledSettings>, ConfigError> {
    let (_path, contents) = match read_optional_xforge_config(manifest_dir)? {
        Some(value) => value,
        None => return Ok(None),
    };
    let config: XforgeConfig = serde_yaml::from_str(&contents).map_err(ConfigError::Yaml)?;
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

fn read_optional_xforge_config(
    manifest_dir: &Path,
) -> Result<Option<(String, String)>, ConfigError> {
    let yaml_path = manifest_dir.join("xforge.yaml");
    if !yaml_path.exists() {
        return Ok(None);
    }

    let contents = std::fs::read_to_string(&yaml_path).map_err(ConfigError::Io)?;
    Ok(Some((
        yaml_path.to_str().unwrap_or("xforge.yaml").to_string(),
        contents,
    )))
}

fn read_rust_toolchain(manifest_dir: &Path) -> Result<(String, String), ConfigError> {
    let direct_path = manifest_dir.join("rust-toolchain.toml");
    if direct_path.exists() {
        let contents = std::fs::read_to_string(&direct_path).map_err(ConfigError::Io)?;
        return Ok((
            direct_path
                .to_str()
                .unwrap_or("rust-toolchain.toml")
                .to_string(),
            contents,
        ));
    }

    let repo_root = find_repo_root(manifest_dir);
    let root_path = repo_root.join("rust-toolchain.toml");
    if root_path.exists() {
        let contents = std::fs::read_to_string(&root_path).map_err(ConfigError::Io)?;
        return Ok((
            root_path
                .to_str()
                .unwrap_or("rust-toolchain.toml")
                .to_string(),
            contents,
        ));
    }

    Err(ConfigError::MissingToolchainFile)
}

fn find_repo_root(manifest_dir: &Path) -> std::path::PathBuf {
    let mut current = Some(manifest_dir);
    while let Some(dir) = current {
        if dir.join("Cargo.lock").exists() {
            return dir.to_path_buf();
        }
        current = dir.parent();
    }
    manifest_dir.to_path_buf()
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
        path.push(format!("xforge-core-{}-{}", name, stamp));
        std::fs::create_dir_all(&path).expect("create temp dir");
        path
    }

    #[test]
    fn missing_toolchain_is_rejected() {
        let dir = temp_dir("missing-config");
        let error = build_targets(&dir).expect_err("error");
        let message = error.to_string();
        assert!(message.contains("rust-toolchain.toml not found"));
    }

    #[test]
    fn reads_targets_from_rust_toolchain() {
        let dir = temp_dir("yaml-config");
        let path = dir.join("rust-toolchain.toml");
        std::fs::write(
            path,
            "[toolchain]\nchannel = \"stable\"\ntargets = [\"x86_64-unknown-linux-gnu\", \"aarch64-linux-android\"]\ncomponents = [\"rustfmt\", \"clippy\"]\n",
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
        let path = dir.join("rust-toolchain.toml");
        std::fs::write(
            path,
            "[toolchain]\nchannel = \"stable\"\ntargets = [\"linux\"]\ncomponents = [\"rustfmt\"]\n",
        )
        .expect("write config");
        let error = build_targets(&dir).expect_err("error");
        let message = error.to_string();
        assert!(message.contains("invalid build target"));
    }

    #[test]
    fn missing_toolchain_fields_are_rejected() {
        let dir = temp_dir("missing-fields");
        let path = dir.join("rust-toolchain.toml");
        std::fs::write(path, "[toolchain]\nchannel = \"stable\"\n").expect("write config");
        let error = toolchain_settings(&dir).expect_err("error");
        let message = error.to_string();
        assert!(message.contains("toolchain.targets"));
    }
}
