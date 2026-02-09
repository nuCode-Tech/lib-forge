use std::fs;
use std::path::PathBuf;

use libforge_core::security::ed25519::{parse_public_key_hex, verify};

pub struct VerifyArgs {
    pub file: PathBuf,
    pub signature: PathBuf,
    pub public_key_hex: String,
}

pub fn run(args: VerifyArgs) -> Result<bool, String> {
    let payload = fs::read(&args.file)
        .map_err(|err| format!("failed to read file '{}': {}", args.file.display(), err))?;
    let signature = fs::read(&args.signature).map_err(|err| {
        format!(
            "failed to read signature '{}': {}",
            args.signature.display(),
            err
        )
    })?;
    let public_key =
        parse_public_key_hex(&args.public_key_hex).map_err(|err| err.to_string())?;
    verify(&public_key, &payload, &signature).map_err(|err| err.to_string())
}
