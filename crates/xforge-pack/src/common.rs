use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use xforge_core::build_plan::BuiltArtifact;
use walkdir::WalkDir;

use crate::{PackError, PackInput};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArchiveEntry {
    pub archive_path: String,
    pub source: EntrySource,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EntrySource {
    File(PathBuf),
}

pub fn build_archive_entries(input: &PackInput) -> Result<Vec<ArchiveEntry>, PackError> {
    let layout = &input.layout;
    let artifact = &input.artifact;
    if artifact.include_dir.is_some() != layout.include_path.is_some() {
        return Err(PackError::InvalidRequest {
            message: "include directory and layout include_path must match".to_string(),
        });
    }
    let mut entries = Vec::new();
    entries.push(file_entry(&artifact.manifest_path, &layout.manifest_path)?);
    entries.push(file_entry(&artifact.build_id_path, &layout.build_id_path)?);
    entries.push(file_entry(&artifact.library_path, &layout.library_path)?);
    if let (Some(include_dir), Some(include_path)) =
        (artifact.include_dir.as_ref(), layout.include_path.as_ref())
    {
        let include_entries = include_dir_entries(include_dir, include_path)?;
        entries.extend(include_entries);
    }
    entries.sort_by(|left, right| left.archive_path.cmp(&right.archive_path));
    Ok(entries)
}

pub fn entries_from_dir(root: &Path) -> Result<Vec<ArchiveEntry>, PackError> {
    let mut entries = Vec::new();
    for entry in WalkDir::new(root).follow_links(false) {
        let entry = entry.map_err(|err| PackError::Io {
            message: err.to_string(),
        })?;
        if entry.file_type().is_dir() {
            continue;
        }
        let relative = entry
            .path()
            .strip_prefix(root)
            .map_err(|err| PackError::Io {
                message: err.to_string(),
            })?;
        let archive_path = path_to_archive_path(relative);
        entries.push(ArchiveEntry {
            archive_path,
            source: EntrySource::File(entry.path().to_path_buf()),
        });
    }
    entries.sort_by(|left, right| left.archive_path.cmp(&right.archive_path));
    Ok(entries)
}

pub fn write_zip(path: &Path, entries: &[ArchiveEntry]) -> Result<(), PackError> {
    let file = fs::File::create(path).map_err(|err| PackError::Io {
        message: err.to_string(),
    })?;
    let mut writer = zip::ZipWriter::new(file);
    let timestamp = zip::DateTime::from_date_and_time(1980, 1, 1, 0, 0, 0).map_err(|_| {
        PackError::InvalidRequest {
            message: "invalid zip timestamp".to_string(),
        }
    })?;
    let options = zip::write::FileOptions::<()>::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .last_modified_time(timestamp)
        .unix_permissions(0o644);
    for entry in entries {
        writer
            .start_file(entry.archive_path.as_str(), options)
            .map_err(|err| PackError::Io {
                message: err.to_string(),
            })?;
        match &entry.source {
            EntrySource::File(path) => {
                let mut input = fs::File::open(path).map_err(|err| PackError::Io {
                    message: err.to_string(),
                })?;
                io::copy(&mut input, &mut writer).map_err(|err| PackError::Io {
                    message: err.to_string(),
                })?;
            }
        }
    }
    writer.finish().map_err(|err| PackError::Io {
        message: err.to_string(),
    })?;
    Ok(())
}

pub fn write_tar_gz(path: &Path, entries: &[ArchiveEntry]) -> Result<(), PackError> {
    let file = fs::File::create(path).map_err(|err| PackError::Io {
        message: err.to_string(),
    })?;
    let encoder = flate2::GzBuilder::new()
        .mtime(0)
        .write(file, flate2::Compression::default());
    let mut builder = tar::Builder::new(encoder);
    for entry in entries {
        let mut header = tar::Header::new_gnu();
        match &entry.source {
            EntrySource::File(path) => {
                let metadata = fs::metadata(path).map_err(|err| PackError::Io {
                    message: err.to_string(),
                })?;
                header.set_size(metadata.len());
                header.set_mode(0o644);
                header.set_uid(0);
                header.set_gid(0);
                header.set_mtime(0);
                header
                    .set_path(&entry.archive_path)
                    .map_err(|err| PackError::Io {
                        message: err.to_string(),
                    })?;
                header.set_cksum();
                let mut input = fs::File::open(path).map_err(|err| PackError::Io {
                    message: err.to_string(),
                })?;
                builder
                    .append_data(&mut header, &entry.archive_path, &mut input)
                    .map_err(|err| PackError::Io {
                        message: err.to_string(),
                    })?;
            }
        }
    }
    builder.finish().map_err(|err| PackError::Io {
        message: err.to_string(),
    })?;
    builder
        .into_inner()
        .map_err(|err| PackError::Io {
            message: err.to_string(),
        })?
        .finish()
        .map_err(|err| PackError::Io {
            message: err.to_string(),
        })?;
    Ok(())
}

pub fn derive_package_name(artifact: &BuiltArtifact) -> String {
    let needle = format!("-{}-", artifact.build_id);
    if let Some(idx) = artifact.artifact_name.find(&needle) {
        return artifact.artifact_name[..idx].to_string();
    }
    strip_known_extension(&artifact.artifact_name)
}

pub fn replace_extension(name: &str, new_extension: &str) -> String {
    if let Some(stripped) = name.strip_suffix(".tar.gz") {
        return format!("{}.{}", stripped, new_extension);
    }
    if let Some(stripped) = name.strip_suffix(".zip") {
        return format!("{}.{}", stripped, new_extension);
    }
    format!("{}.{}", name, new_extension)
}

fn file_entry(source: &str, archive_path: &str) -> Result<ArchiveEntry, PackError> {
    let path = PathBuf::from(source);
    if !path.is_file() {
        return Err(PackError::InvalidRequest {
            message: format!("missing file '{}'", source),
        });
    }
    Ok(ArchiveEntry {
        archive_path: archive_path.to_string(),
        source: EntrySource::File(path),
    })
}

fn include_dir_entries(
    include_dir: &str,
    include_path: &str,
) -> Result<Vec<ArchiveEntry>, PackError> {
    let mut entries = Vec::new();
    let root = Path::new(include_dir);
    if !root.is_dir() {
        return Err(PackError::InvalidRequest {
            message: format!("missing include dir '{}'", include_dir),
        });
    }
    for entry in WalkDir::new(root).follow_links(false) {
        let entry = entry.map_err(|err| PackError::Io {
            message: err.to_string(),
        })?;
        if entry.file_type().is_dir() {
            continue;
        }
        let relative = entry
            .path()
            .strip_prefix(root)
            .map_err(|err| PackError::Io {
                message: err.to_string(),
            })?;
        let relative_path = path_to_archive_path(relative);
        let archive_path = join_archive_path(include_path, &relative_path);
        entries.push(ArchiveEntry {
            archive_path,
            source: EntrySource::File(entry.path().to_path_buf()),
        });
    }
    entries.sort_by(|left, right| left.archive_path.cmp(&right.archive_path));
    Ok(entries)
}

fn path_to_archive_path(path: &Path) -> String {
    let mut components = Vec::new();
    for component in path.components() {
        components.push(component.as_os_str().to_string_lossy().into_owned());
    }
    components.join("/")
}

fn join_archive_path(prefix: &str, suffix: &str) -> String {
    if prefix.ends_with('/') {
        format!("{}{}", prefix, suffix)
    } else {
        format!("{}/{}", prefix, suffix)
    }
}

fn strip_known_extension(name: &str) -> String {
    if let Some(stripped) = name.strip_suffix(".tar.gz") {
        return stripped.to_string();
    }
    if let Some(stripped) = name.strip_suffix(".zip") {
        return stripped.to_string();
    }
    name.to_string()
}
