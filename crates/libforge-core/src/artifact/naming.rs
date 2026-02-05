use crate::platform::PlatformKey;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArchiveKind {
    TarGz,
    Zip,
}

impl ArchiveKind {
    pub fn extension(self) -> &'static str {
        match self {
            ArchiveKind::TarGz => "tar.gz",
            ArchiveKind::Zip => "zip",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChecksumKind {
    Sha256,
}

impl ChecksumKind {
    pub fn extension(self) -> &'static str {
        match self {
            ChecksumKind::Sha256 => "sha256",
        }
    }
}

pub fn artifact_name(
    lib_name: &str,
    build_id: &str,
    platform_key: &PlatformKey,
    archive: ArchiveKind,
) -> Result<String, ArtifactNameError> {
    validate_component("package", lib_name)?;
    validate_component("build_id", build_id)?;
    validate_build_id(build_id)?;
    Ok(format!(
        "{}-{}-{}.{}",
        lib_name,
        build_id,
        platform_key,
        archive.extension()
    ))
}

pub fn checksum_name(artifact_name: &str, checksum: ChecksumKind) -> String {
    format!("{}.{}", artifact_name, checksum.extension())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ArtifactNameError {
    InvalidComponent { field: &'static str, value: String },
    InvalidBuildId { value: String },
}

impl std::fmt::Display for ArtifactNameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArtifactNameError::InvalidComponent { field, value } => {
                write!(f, "invalid {} value '{}'", field, value)
            }
            ArtifactNameError::InvalidBuildId { value } => {
                write!(f, "build_id '{}' must include a version prefix", value)
            }
        }
    }
}

impl std::error::Error for ArtifactNameError {}

fn validate_component(field: &'static str, value: &str) -> Result<(), ArtifactNameError> {
    if value.is_empty() || !is_canonical_component(value) {
        return Err(ArtifactNameError::InvalidComponent {
            field,
            value: value.to_string(),
        });
    }
    Ok(())
}

fn validate_build_id(value: &str) -> Result<(), ArtifactNameError> {
    if is_versioned_build_id(value) {
        return Ok(());
    }
    Err(ArtifactNameError::InvalidBuildId {
        value: value.to_string(),
    })
}

fn is_canonical_component(value: &str) -> bool {
    value
        .chars()
        .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
}

fn is_versioned_build_id(value: &str) -> bool {
    if !value.starts_with('b') {
        return false;
    }
    let rest = &value[1..];
    let mut chars = rest.chars();
    let mut digit_count = 0;
    let mut seen_dash = false;
    for ch in chars.by_ref() {
        if ch.is_ascii_digit() {
            digit_count += 1;
            continue;
        }
        if ch == '-' {
            seen_dash = true;
            break;
        }
        return false;
    }
    if !seen_dash || digit_count == 0 {
        return false;
    }
    let remainder = chars.as_str();
    !remainder.is_empty() && is_canonical_component(remainder)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::PlatformKey;

    #[test]
    fn artifact_name_is_deterministic() {
        let key = PlatformKey::LinuxX86_64;
        let name = artifact_name("libname", "b1-abc123", &key, ArchiveKind::TarGz).expect("name");
        assert_eq!(name, "libname-b1-abc123-x86_64-unknown-linux-gnu.tar.gz");
    }

    #[test]
    fn checksum_name_appends_extension() {
        let checksum = checksum_name(
            "libname-build-1-x86_64-unknown-linux-gnu.tar.gz",
            ChecksumKind::Sha256,
        );
        assert_eq!(
            checksum,
            "libname-build-1-x86_64-unknown-linux-gnu.tar.gz.sha256"
        );
    }

    #[test]
    fn invalid_component_rejected() {
        let key = PlatformKey::LinuxX86_64;
        let result = artifact_name("LibName", "b1-abc123", &key, ArchiveKind::TarGz);
        assert!(matches!(
            result,
            Err(ArtifactNameError::InvalidComponent { .. })
        ));
    }

    #[test]
    fn invalid_build_id_rejected() {
        let key = PlatformKey::LinuxX86_64;
        let result = artifact_name("libname", "build-1", &key, ArchiveKind::TarGz);
        assert!(matches!(
            result,
            Err(ArtifactNameError::InvalidBuildId { .. })
        ));
    }
}
