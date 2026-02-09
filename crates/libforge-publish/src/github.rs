use std::collections::HashSet;
use std::fs;

use reqwest::blocking::Client;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

use crate::release::{PublishError, PublishOutcome, PublishRequest, Publisher, ReleaseAsset};

pub struct GitHubPublisher {
    client: Client,
    token: String,
}

impl GitHubPublisher {
    pub fn new(token: String) -> Result<Self, PublishError> {
        let client = Client::builder()
            .user_agent("libforge-publish")
            .build()
            .map_err(|err| PublishError::Backend(format!("failed to build client: {}", err)))?;
        Ok(Self { client, token })
    }
}

impl Publisher for GitHubPublisher {
    fn publish(&self, request: &PublishRequest) -> Result<PublishOutcome, PublishError> {
        let repo = &request.repository;
        let release = get_or_create_release(&self.client, &self.token, repo, request)?;
        let existing = existing_asset_names(&release);

        let mut uploaded = Vec::new();
        let mut skipped = Vec::new();
        for asset in &request.assets {
            if existing.contains(&asset.name) {
                skipped.push(asset.name.clone());
                continue;
            }
            upload_asset(&self.client, &self.token, &release.upload_url, asset)?;
            uploaded.push(asset.name.clone());
        }

        Ok(PublishOutcome {
            uploaded,
            skipped,
            release_url: release.html_url,
        })
    }
}

#[derive(Clone, Debug, Deserialize)]
struct ReleaseResponse {
    upload_url: String,
    html_url: Option<String>,
    assets: Option<Vec<ReleaseAssetResponse>>,
}

#[derive(Clone, Debug, Deserialize)]
struct ReleaseAssetResponse {
    name: String,
}

#[derive(Debug, Serialize)]
struct CreateReleaseRequest {
    tag_name: String,
    name: String,
    body: String,
    draft: bool,
    prerelease: bool,
}

fn get_or_create_release(
    client: &Client,
    token: &str,
    repo: &str,
    request: &PublishRequest,
) -> Result<ReleaseResponse, PublishError> {
    let url = format!("https://api.github.com/repos/{}/releases/tags/{}", repo, request.tag);
    let response = client
        .get(&url)
        .bearer_auth(token)
        .send()
        .map_err(|err| PublishError::Backend(format!("github release lookup failed: {}", err)))?;
    if response.status() == StatusCode::NOT_FOUND {
        return create_release(client, token, repo, request);
    }
    if !response.status().is_success() {
        return Err(PublishError::Backend(format!(
            "github release lookup failed: {}",
            response.status()
        )));
    }
    response
        .json::<ReleaseResponse>()
        .map_err(|err| PublishError::Backend(format!("github release parse failed: {}", err)))
}

fn create_release(
    client: &Client,
    token: &str,
    repo: &str,
    request: &PublishRequest,
) -> Result<ReleaseResponse, PublishError> {
    let url = format!("https://api.github.com/repos/{}/releases", repo);
    let payload = CreateReleaseRequest {
        tag_name: request.tag.clone(),
        name: request.name.clone(),
        body: request.body.clone(),
        draft: false,
        prerelease: false,
    };
    let response = client
        .post(&url)
        .bearer_auth(token)
        .json(&payload)
        .send()
        .map_err(|err| PublishError::Backend(format!("github release create failed: {}", err)))?;
    if !response.status().is_success() {
        return Err(PublishError::Backend(format!(
            "github release create failed: {}",
            response.status()
        )));
    }
    response
        .json::<ReleaseResponse>()
        .map_err(|err| PublishError::Backend(format!("github release parse failed: {}", err)))
}

fn existing_asset_names(release: &ReleaseResponse) -> HashSet<String> {
    release
        .assets
        .as_ref()
        .map(|assets| assets.iter().map(|asset| asset.name.clone()).collect())
        .unwrap_or_default()
}

fn upload_asset(
    client: &Client,
    token: &str,
    upload_url: &str,
    asset: &ReleaseAsset,
) -> Result<(), PublishError> {
    let url = upload_url
        .split('{')
        .next()
        .unwrap_or(upload_url)
        .to_string();
    let upload_url = format!("{}?name={}", url, asset.name);
    let body = fs::read(&asset.path).map_err(|err| {
        PublishError::Io(format!(
            "failed to read asset '{}': {}",
            asset.path.display(),
            err
        ))
    })?;
    let response = client
        .post(&upload_url)
        .bearer_auth(token)
        .header("Content-Type", &asset.content_type)
        .body(body)
        .send()
        .map_err(|err| PublishError::Backend(format!("github upload failed: {}", err)))?;
    if response.status().is_success() {
        Ok(())
    } else {
        Err(PublishError::Backend(format!(
            "github upload failed: {}",
            response.status()
        )))
    }
}
