use std::fs;
use std::path::PathBuf;

use crate::common::{build_archive_entries, replace_extension, write_zip};
use crate::{PackError, PackExecutor, PackFormat, PackRequest, PackResult};

pub struct ZipPacker;

impl PackExecutor for ZipPacker {
    fn pack(&self, request: &PackRequest) -> Result<PackResult, PackError> {
        if request.format != PackFormat::Zip {
            return Err(PackError::InvalidRequest {
                message: "zip packer only supports PackFormat::Zip".to_string(),
            });
        }
        if request.inputs.len() != 1 {
            return Err(PackError::InvalidRequest {
                message: "zip packer expects a single input".to_string(),
            });
        }
        let input = &request.inputs[0];
        let entries = build_archive_entries(input)?;
        let mut output_dir = PathBuf::from(&request.output_dir);
        fs::create_dir_all(&output_dir).map_err(|err| PackError::Io {
            message: err.to_string(),
        })?;
        let output_name = replace_extension(&input.artifact.artifact_name, "zip");
        output_dir.push(output_name);
        write_zip(&output_dir, &entries)?;
        Ok(PackResult {
            format: PackFormat::Zip,
            output_paths: vec![output_dir.to_string_lossy().into_owned()],
        })
    }
}
