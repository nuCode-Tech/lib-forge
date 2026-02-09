use std::fs;
use std::path::PathBuf;

use libforge_core::security::ed25519::{parse_private_key_hex, sign};

pub struct SignArgs {
    pub file: PathBuf,
    pub out: Option<PathBuf>,
    pub private_key_hex: String,
}

pub fn run(args: SignArgs) -> Result<PathBuf, String> {
    let payload = fs::read(&args.file)
        .map_err(|err| format!("failed to read file '{}': {}", args.file.display(), err))?;
    let private_key =
        parse_private_key_hex(&args.private_key_hex).map_err(|err| err.to_string())?;
    let signature = sign(&private_key, &payload).map_err(|err| err.to_string())?;
    let out_path = args
        .out
        .unwrap_or_else(|| PathBuf::from(format!("{}.sig", args.file.display())));
    fs::write(&out_path, signature)
        .map_err(|err| format!("failed to write signature '{}': {}", out_path.display(), err))?;
    Ok(out_path)
}
