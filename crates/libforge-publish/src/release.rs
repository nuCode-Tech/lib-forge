use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct ReleaseAsset {
    pub path: PathBuf,
    pub name: String,
    pub content_type: String,
}

#[derive(Clone, Debug)]
pub struct PublishRequest {
    pub repository: String,
    pub tag: String,
    pub name: String,
    pub body: String,
    pub build_id: String,
    pub manifest_path: PathBuf,
    pub assets: Vec<ReleaseAsset>,
}

#[derive(Clone, Debug)]
pub struct PublishOutcome {
    pub uploaded: Vec<String>,
    pub skipped: Vec<String>,
    pub release_url: Option<String>,
}

#[derive(Clone, Debug)]
pub enum PublishError {
    InvalidRequest(String),
    Io(String),
    Backend(String),
}

impl std::fmt::Display for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PublishError::InvalidRequest(message) => write!(f, "invalid request: {}", message),
            PublishError::Io(message) => write!(f, "io error: {}", message),
            PublishError::Backend(message) => write!(f, "backend error: {}", message),
        }
    }
}

impl std::error::Error for PublishError {}

pub trait Publisher {
    fn publish(&self, request: &PublishRequest) -> Result<PublishOutcome, PublishError>;
}

pub fn publish_release<P: Publisher>(
    publisher: &P,
    request: PublishRequest,
) -> Result<PublishOutcome, PublishError> {
    validate_request(&request)?;
    publisher.publish(&request)
}

fn validate_request(request: &PublishRequest) -> Result<(), PublishError> {
    if request.repository.trim().is_empty() {
        return Err(PublishError::InvalidRequest(
            "repository is required".to_string(),
        ));
    }
    if request.tag.trim().is_empty() {
        return Err(PublishError::InvalidRequest("tag is required".to_string()));
    }
    if request.build_id.trim().is_empty() {
        return Err(PublishError::InvalidRequest(
            "build_id is required".to_string(),
        ));
    }
    if !request.manifest_path.exists() {
        return Err(PublishError::InvalidRequest(format!(
            "manifest path '{}' does not exist",
            request.manifest_path.display()
        )));
    }
    for asset in &request.assets {
        if !asset.path.exists() {
            return Err(PublishError::InvalidRequest(format!(
                "asset '{}' does not exist",
                asset.path.display()
            )));
        }
        if requires_build_id_in_name(&asset.name) && !asset.name.contains(&request.build_id) {
            return Err(PublishError::InvalidRequest(format!(
                "asset '{}' does not include build_id '{}'",
                asset.name, request.build_id
            )));
        }
    }
    Ok(())
}

fn requires_build_id_in_name(name: &str) -> bool {
    if name == "libforge-manifest.json" {
        return false;
    }
    if name == "build_id.txt" {
        return false;
    }
    if name.ends_with(".sig") {
        return false;
    }
    true
}

pub fn asset_from_path(path: &Path) -> Result<ReleaseAsset, PublishError> {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| PublishError::InvalidRequest("asset filename missing".to_string()))?;
    Ok(ReleaseAsset {
        path: path.to_path_buf(),
        name: name.to_string(),
        content_type: content_type_for_path(path),
    })
}

fn content_type_for_path(path: &Path) -> String {
    let name = path.to_string_lossy();
    if name.ends_with(".zip") {
        "application/zip".to_string()
    } else if name.ends_with(".tar.gz") {
        "application/gzip".to_string()
    } else if name.ends_with(".json") {
        "application/json".to_string()
    } else if name.ends_with(".sig") {
        "application/octet-stream".to_string()
    } else {
        "application/octet-stream".to_string()
    }
}
