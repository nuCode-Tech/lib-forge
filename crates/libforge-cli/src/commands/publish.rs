use std::fs;
use std::path::{Path, PathBuf};

use libforge_core::manifest::{
    deserialize_manifest, serialize_manifest_pretty, signing_payload, Signing,
};
use libforge_core::security::{parse_private_key_hex, parse_public_key_hex, public_key_from_private_key, sign, verify};
use libforge_publish::github::GitHubPublisher;
use libforge_publish::release::{asset_from_path, publish_release, PublishRequest};

pub struct PublishArgs {
    pub manifest: PathBuf,
    pub assets_dir: Option<PathBuf>,
    pub asset_files: Vec<PathBuf>,
    pub out_dir: Option<PathBuf>,
    pub repository: String,
    pub github_token: String,
    pub private_key_hex: String,
}

pub struct PublishResult {
    pub signed_files: Vec<PathBuf>,
    pub uploaded: Vec<String>,
    pub skipped: Vec<String>,
    pub release_url: Option<String>,
}

pub fn run(args: PublishArgs) -> Result<PublishResult, String> {
    let signed = prepare_signed_assets(
        &args.manifest,
        args.assets_dir.as_deref(),
        &args.asset_files,
        args.out_dir.as_deref(),
        &args.private_key_hex,
    )?;

    verify_manifest_signature(&signed.signed_manifest_path)?;

    let publisher = GitHubPublisher::new(args.github_token).map_err(|err| err.to_string())?;
    let request = PublishRequest {
        repository: args.repository,
        tag: signed.build_id.clone(),
        name: format!("libforge {}", signed.build_id),
        body: format!("LibForge release {}", signed.build_id),
        build_id: signed.build_id.clone(),
        manifest_path: signed.signed_manifest_path.clone(),
        assets: signed.assets,
    };
    let outcome = publish_release(&publisher, request).map_err(|err| err.to_string())?;
    Ok(PublishResult {
        signed_files: signed.signed_files,
        uploaded: outcome.uploaded,
        skipped: outcome.skipped,
        release_url: outcome.release_url,
    })
}

pub struct SignedAssets {
    pub build_id: String,
    pub signed_manifest_path: PathBuf,
    pub signed_files: Vec<PathBuf>,
    pub assets: Vec<libforge_publish::ReleaseAsset>,
}

pub fn prepare_signed_assets(
    manifest_path: &Path,
    assets_dir: Option<&Path>,
    asset_files: &[PathBuf],
    out_dir: Option<&Path>,
    private_key_hex: &str,
) -> Result<SignedAssets, String> {
    let manifest_contents = fs::read_to_string(manifest_path).map_err(|err| {
        format!(
            "failed to read manifest '{}': {}",
            manifest_path.display(),
            err
        )
    })?;
    let mut manifest = deserialize_manifest(&manifest_contents)
        .map_err(|err| format!("failed to parse manifest: {}", err))?;
    let build_id = manifest.build.id.clone();

    let private_key = parse_private_key_hex(private_key_hex).map_err(|err| err.to_string())?;
    let public_key =
        public_key_from_private_key(&private_key).map_err(|err| err.to_string())?;

    let payload = signing_payload(&manifest)
        .map_err(|err| format!("failed to build signing payload: {}", err))?;
    let signature = sign(&private_key, &payload).map_err(|err| err.to_string())?;
    let signature_hex = hex::encode(&signature);
    let public_key_hex = hex::encode(public_key);

    manifest.signing = Some(Signing {
        algorithm: "ed25519".to_string(),
        public_key: public_key_hex.clone(),
        signature: signature_hex.clone(),
    });

    let out_dir = out_dir
        .map(|path| path.to_path_buf())
        .or_else(|| manifest_path.parent().map(|path| path.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));
    fs::create_dir_all(&out_dir)
        .map_err(|err| format!("failed to create out dir '{}': {}", out_dir.display(), err))?;

    let signed_manifest = serialize_manifest_pretty(&manifest)
        .map_err(|err| format!("failed to serialize manifest: {}", err))?;
    let manifest_filename = manifest_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("libforge-manifest.json");
    let signed_manifest_path = out_dir.join(manifest_filename);
    fs::write(&signed_manifest_path, signed_manifest.as_bytes()).map_err(|err| {
        format!(
            "failed to write signed manifest '{}': {}",
            signed_manifest_path.display(),
            err
        )
    })?;

    let manifest_sig_path = out_dir.join(format!("{}.sig", manifest_filename));
    fs::write(&manifest_sig_path, &signature).map_err(|err| {
        format!(
            "failed to write manifest signature '{}': {}",
            manifest_sig_path.display(),
            err
        )
    })?;

    let mut signed_files = vec![signed_manifest_path.clone(), manifest_sig_path.clone()];
    let mut assets = Vec::new();
    assets.push(signed_manifest_path.clone());
    assets.push(manifest_sig_path.clone());

    for asset in collect_assets(assets_dir, asset_files)? {
        let sig_path = sign_file(&asset, &out_dir, &private_key)?;
        signed_files.push(sig_path.clone());
        assets.push(asset);
        assets.push(sig_path);
    }

    let release_assets = dedupe_assets(assets)?;

    Ok(SignedAssets {
        build_id,
        signed_manifest_path,
        signed_files,
        assets: release_assets,
    })
}

fn verify_manifest_signature(manifest_path: &Path) -> Result<(), String> {
    let manifest_contents = fs::read_to_string(manifest_path).map_err(|err| {
        format!(
            "failed to read signed manifest '{}': {}",
            manifest_path.display(),
            err
        )
    })?;
    let manifest = deserialize_manifest(&manifest_contents)
        .map_err(|err| format!("failed to parse signed manifest: {}", err))?;
    let signing = manifest
        .signing
        .as_ref()
        .ok_or_else(|| "signed manifest missing signing block".to_string())?;
    if signing.algorithm != "ed25519" {
        return Err(format!(
            "unsupported signing algorithm '{}'",
            signing.algorithm
        ));
    }
    let public_key = parse_public_key_hex(&signing.public_key)
        .map_err(|err| err.to_string())?;
    let signature = hex::decode(&signing.signature)
        .map_err(|err| format!("invalid signature hex: {}", err))?;
    let payload = signing_payload(&manifest)
        .map_err(|err| format!("failed to build signing payload: {}", err))?;
    let ok = verify(&public_key, &payload, &signature).map_err(|err| err.to_string())?;
    if !ok {
        return Err("manifest signature verification failed".to_string());
    }
    Ok(())
}

fn dedupe_assets(paths: Vec<PathBuf>) -> Result<Vec<libforge_publish::ReleaseAsset>, String> {
    use std::collections::HashMap;
    let mut by_name = HashMap::new();
    for path in paths {
        let asset = asset_from_path(&path).map_err(|err| err.to_string())?;
        by_name.entry(asset.name.clone()).or_insert(asset);
    }
    Ok(by_name.into_values().collect())
}

fn collect_assets(dir: Option<&Path>, files: &[PathBuf]) -> Result<Vec<PathBuf>, String> {
    let mut assets = Vec::new();
    if let Some(dir) = dir {
        let entries = fs::read_dir(dir).map_err(|err| {
            format!("failed to read assets dir '{}': {}", dir.display(), err)
        })?;
        for entry in entries {
            let entry = entry.map_err(|err| format!("failed to read assets dir entry: {}", err))?;
            let path = entry.path();
            if path.is_file() && !path.to_string_lossy().ends_with(".sig") {
                assets.push(path);
            }
        }
    }
    for file in files {
        assets.push(file.clone());
    }
    Ok(assets)
}

fn sign_file(
    path: &Path,
    out_dir: &Path,
    private_key: &[u8; 64],
) -> Result<PathBuf, String> {
    let payload = fs::read(path)
        .map_err(|err| format!("failed to read asset '{}': {}", path.display(), err))?;
    let signature = sign(private_key, &payload).map_err(|err| err.to_string())?;
    let filename = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| format!("invalid asset filename '{}'", path.display()))?;
    let sig_path = out_dir.join(format!("{}.sig", filename));
    fs::write(&sig_path, signature)
        .map_err(|err| format!("failed to write signature '{}': {}", sig_path.display(), err))?;
    Ok(sig_path)
}
