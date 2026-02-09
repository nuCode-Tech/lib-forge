use std::fs;
use std::path::PathBuf;
use std::process::Command;

use crate::common::{derive_package_name, entries_from_dir};
use crate::{PackError, PackExecutor, PackFormat, PackRequest, PackResult};

pub struct XcframeworkPacker;

impl PackExecutor for XcframeworkPacker {
    fn pack(&self, request: &PackRequest) -> Result<PackResult, PackError> {
        if request.format != PackFormat::XCFramework {
            return Err(PackError::InvalidRequest {
                message: "xcframework packer only supports PackFormat::XCFramework".to_string(),
            });
        }
        if request.inputs.is_empty() {
            return Err(PackError::InvalidRequest {
                message: "xcframework packer expects at least one input".to_string(),
            });
        }
        let first = &request.inputs[0];
        let output_name = format!("{}.xcframework", derive_package_name(&first.artifact));
        let mut output_dir = PathBuf::from(&request.output_dir);
        fs::create_dir_all(&output_dir).map_err(|err| PackError::Io {
            message: err.to_string(),
        })?;
        output_dir.push(output_name);
        if output_dir.exists() {
            fs::remove_dir_all(&output_dir).map_err(|err| PackError::Io {
                message: err.to_string(),
            })?;
        }
        let mut command = Command::new("xcodebuild");
        command.arg("-create-xcframework");
        for input in &request.inputs {
            command.arg("-library").arg(&input.artifact.library_path);
            if let Some(headers) = &input.artifact.include_dir {
                command.arg("-headers").arg(headers);
            }
        }
        command.arg("-output").arg(&output_dir);
        let output = command.output().map_err(|err| PackError::Io {
            message: err.to_string(),
        })?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(PackError::Io {
                message: format!("xcodebuild failed: {}", stderr.trim()),
            });
        }
        write_metadata(&output_dir, &first.layout, &first.artifact)?;
        let entries = entries_from_dir(&output_dir)?;
        if entries.is_empty() {
            return Err(PackError::InvalidRequest {
                message: "xcframework output is empty".to_string(),
            });
        }
        Ok(PackResult {
            format: PackFormat::XCFramework,
            output_paths: vec![output_dir.to_string_lossy().into_owned()],
        })
    }
}

fn write_metadata(
    root: &PathBuf,
    layout: &libforge_core::artifact::layout::ArchiveLayout,
    artifact: &libforge_core::build_plan::BuiltArtifact,
) -> Result<(), PackError> {
    let manifest_path = root.join(&layout.manifest_path);
    if let Some(parent) = manifest_path.parent() {
        fs::create_dir_all(parent).map_err(|err| PackError::Io {
            message: err.to_string(),
        })?;
    }
    fs::copy(&artifact.manifest_path, &manifest_path).map_err(|err| PackError::Io {
        message: err.to_string(),
    })?;
    let build_id_path = root.join(&layout.build_id_path);
    if let Some(parent) = build_id_path.parent() {
        fs::create_dir_all(parent).map_err(|err| PackError::Io {
            message: err.to_string(),
        })?;
    }
    fs::copy(&artifact.build_id_path, &build_id_path).map_err(|err| PackError::Io {
        message: err.to_string(),
    })?;
    Ok(())
}
