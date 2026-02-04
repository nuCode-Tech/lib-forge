use std::collections::HashSet;

use crate::platform::{PlatformFamily, PlatformKey, PlatformOs};

pub const MANIFEST_FILE_NAME: &str = "manifest.json";
pub const CHECKSUMS_FILE_NAME: &str = "checksums.txt";
pub const BUILD_ID_FILE_NAME: &str = "build_id.txt";
pub const METADATA_DIR_NAME: &str = "metadata";
pub const LIB_DIR_NAME: &str = "lib";
pub const INCLUDE_DIR_NAME: &str = "include";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArchiveLayout {
    pub layout: LayoutVariant,
    pub manifest_path: String,
    pub checksums_path: String,
    pub build_id_path: String,
    pub library_path: String,
    pub include_path: Option<String>,
}

pub fn archive_layout(lib_name: &str, platform_key: &PlatformKey) -> ArchiveLayout {
    let layout = layout_variant(platform_key);
    ArchiveLayout {
        layout,
        manifest_path: metadata_path(MANIFEST_FILE_NAME),
        checksums_path: metadata_path(CHECKSUMS_FILE_NAME),
        build_id_path: metadata_path(BUILD_ID_FILE_NAME),
        library_path: format!(
            "{}/{}",
            LIB_DIR_NAME,
            library_filename(lib_name, platform_key)
        ),
        include_path: None,
    }
}

pub fn library_filename(lib_name: &str, platform_key: &PlatformKey) -> String {
    match platform_key.os() {
        PlatformOs::Linux | PlatformOs::Android => format!("lib{}.so", lib_name),
        PlatformOs::Windows => format!("{}.dll", lib_name),
        PlatformOs::Macos | PlatformOs::Ios => format!("lib{}.dylib", lib_name),
    }
}

pub fn default_archive_kind(platform_key: &PlatformKey) -> super::naming::ArchiveKind {
    match platform_key.os() {
        PlatformOs::Ios | PlatformOs::Macos | PlatformOs::Windows => {
            super::naming::ArchiveKind::Zip
        }
        PlatformOs::Linux | PlatformOs::Android => super::naming::ArchiveKind::TarGz,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayoutVariant {
    Desktop,
    Apple,
    Android,
}

pub fn layout_variant(platform_key: &PlatformKey) -> LayoutVariant {
    match platform_key.family() {
        PlatformFamily::Apple => LayoutVariant::Apple,
        PlatformFamily::Android => LayoutVariant::Android,
        PlatformFamily::Linux | PlatformFamily::Windows | PlatformFamily::Desktop => {
            LayoutVariant::Desktop
        }
    }
}

pub fn required_entries(layout: &ArchiveLayout) -> Vec<String> {
    let mut entries = vec![
        layout.manifest_path.clone(),
        layout.checksums_path.clone(),
        layout.build_id_path.clone(),
        layout.library_path.clone(),
    ];
    if let Some(include_path) = &layout.include_path {
        entries.push(include_path.clone());
    }
    entries
}

pub fn validate_archive_entries<I>(
    layout: &ArchiveLayout,
    entries: I,
) -> Result<(), LayoutValidationError>
where
    I: IntoIterator,
    I::Item: AsRef<str>,
{
    let present: HashSet<String> = entries
        .into_iter()
        .map(|entry| entry.as_ref().to_string())
        .collect();
    for required in required_entries(layout) {
        if !present.contains(&required) {
            return Err(LayoutValidationError::MissingEntry(required));
        }
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LayoutValidationError {
    MissingEntry(String),
}

impl std::fmt::Display for LayoutValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LayoutValidationError::MissingEntry(path) => {
                write!(f, "archive missing required entry '{}'", path)
            }
        }
    }
}

impl std::error::Error for LayoutValidationError {}

fn metadata_path(file_name: &str) -> String {
    format!("{}/{}", METADATA_DIR_NAME, file_name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::PlatformKey;

    #[test]
    fn linux_layout_uses_so() {
        let key = PlatformKey::LinuxX86_64;
        let layout = archive_layout("demo", &key);
        assert_eq!(layout.library_path, "lib/libdemo.so");
        assert_eq!(layout.manifest_path, "metadata/manifest.json");
        assert_eq!(layout.checksums_path, "metadata/checksums.txt");
        assert_eq!(layout.build_id_path, "metadata/build_id.txt");
    }

    #[test]
    fn ios_defaults_to_zip() {
        let key = PlatformKey::IosArm64;
        let kind = default_archive_kind(&key);
        assert_eq!(kind, crate::artifact::naming::ArchiveKind::Zip);
    }

    #[test]
    fn layout_validation_requires_entries() {
        let key = PlatformKey::LinuxX86_64;
        let layout = archive_layout("demo", &key);
        let entries = vec![
            "metadata/manifest.json",
            "metadata/checksums.txt",
            "metadata/build_id.txt",
            "lib/libdemo.so",
        ];
        assert!(validate_archive_entries(&layout, entries).is_ok());
    }
}
