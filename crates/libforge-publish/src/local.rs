use std::fs;
use std::path::{Path, PathBuf};

use crate::release::{PublishError, PublishOutcome, PublishRequest, Publisher};

pub struct LocalPublisher {
    out_dir: PathBuf,
}

impl LocalPublisher {
    pub fn new(out_dir: PathBuf) -> Result<Self, PublishError> {
        fs::create_dir_all(&out_dir).map_err(|err| {
            PublishError::Io(format!(
                "failed to create local publish dir '{}': {}",
                out_dir.display(),
                err
            ))
        })?;
        Ok(Self { out_dir })
    }
}

impl Publisher for LocalPublisher {
    fn publish(&self, request: &PublishRequest) -> Result<PublishOutcome, PublishError> {
        let release_dir = self.out_dir.join(&request.tag);
        fs::create_dir_all(&release_dir).map_err(|err| {
            PublishError::Io(format!(
                "failed to create release dir '{}': {}",
                release_dir.display(),
                err
            ))
        })?;

        let mut uploaded = Vec::new();
        let mut skipped = Vec::new();

        for asset in &request.assets {
            let dest = release_dir.join(&asset.name);
            if dest.exists() {
                skipped.push(asset.name.clone());
                continue;
            }
            fs::copy(&asset.path, &dest).map_err(|err| {
                PublishError::Io(format!(
                    "failed to copy '{}' to '{}': {}",
                    asset.path.display(),
                    dest.display(),
                    err
                ))
            })?;
            uploaded.push(asset.name.clone());
        }

        Ok(PublishOutcome {
            uploaded,
            skipped,
            release_url: Some(path_to_url(&release_dir)),
        })
    }
}

fn path_to_url(path: &Path) -> String {
    format!("file://{}", path.display())
}
