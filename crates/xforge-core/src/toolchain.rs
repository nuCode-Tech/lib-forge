use std::path::Path;

use crate::config::{toolchain_settings, ConfigError};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Toolchain {
    pub channel: Option<String>,
    pub targets: Vec<String>,
    pub components: Vec<String>,
}

impl Toolchain {
    pub fn from_manifest_dir(manifest_dir: &Path) -> Result<Self, ConfigError> {
        let settings = toolchain_settings(manifest_dir)?;
        Ok(Self {
            channel: settings.channel,
            targets: settings.targets,
            components: settings.components,
        })
    }
}
