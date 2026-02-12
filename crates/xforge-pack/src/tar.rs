use std::fs;
use std::path::PathBuf;

use crate::common::{build_archive_entries, replace_extension, write_tar_gz};
use crate::{PackError, PackExecutor, PackFormat, PackRequest, PackResult};

pub struct TarGzPacker;

impl PackExecutor for TarGzPacker {
    fn pack(&self, request: &PackRequest) -> Result<PackResult, PackError> {
        if request.format != PackFormat::TarGz {
            return Err(PackError::InvalidRequest {
                message: "tar.gz packer only supports PackFormat::TarGz".to_string(),
            });
        }
        if request.inputs.len() != 1 {
            return Err(PackError::InvalidRequest {
                message: "tar.gz packer expects a single input".to_string(),
            });
        }
        let input = &request.inputs[0];
        let entries = build_archive_entries(input)?;
        let mut output_dir = PathBuf::from(&request.output_dir);
        fs::create_dir_all(&output_dir).map_err(|err| PackError::Io {
            message: err.to_string(),
        })?;
        let output_name = replace_extension(&input.artifact.artifact_name, "tar.gz");
        output_dir.push(output_name);
        write_tar_gz(&output_dir, &entries)?;
        Ok(PackResult {
            format: PackFormat::TarGz,
            output_paths: vec![output_dir.to_string_lossy().into_owned()],
        })
    }
}
