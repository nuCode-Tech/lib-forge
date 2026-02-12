use std::fs;
use std::path::{Path, PathBuf};

use xforge_cli::commands::{build, bundle, keygen, publish};
use xforge_core::manifest::{deserialize_manifest, signing_payload};
use xforge_core::security::{parse_public_key_hex, verify};
use xforge_publish::local::LocalPublisher;
use xforge_publish::release::{publish_release, PublishRequest};

fn temp_dir(name: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    path.push(format!("xforge-e2e-{}-{}", name, stamp));
    fs::create_dir_all(&path).expect("create temp dir");
    path
}

fn write_sample_crate(dir: &Path, name: &str) {
    fs::create_dir_all(dir.join("src")).expect("create src");
    fs::write(
        dir.join("src").join("lib.rs"),
        "pub fn demo() -> u32 { 42 }",
    )
    .expect("write lib.rs");
    fs::write(
        dir.join("Cargo.toml"),
        format!("[package]\nname = \"{}\"\nversion = \"0.1.0\"\n", name),
    )
    .expect("write Cargo.toml");
    fs::write(
        dir.join("Cargo.lock"),
        format!("[[package]]\nname = \"{}\"\nversion = \"0.1.0\"\n", name),
    )
    .expect("write Cargo.lock");
}

fn host_target_triple() -> String {
    let output = std::process::Command::new("rustc")
        .arg("-vV")
        .output()
        .expect("rustc -vV");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("utf8");
    for line in stdout.lines() {
        if let Some(triple) = line.strip_prefix("host: ") {
            return triple.trim().to_string();
        }
    }
    panic!("missing host target");
}

#[test]
fn end_to_end_build_bundle_publish_local() {
    let dir = temp_dir("flow");
    let crate_name = "e2e-demo";
    write_sample_crate(&dir, crate_name);

    let target = host_target_triple();
    fs::write(
        dir.join("xforge.yaml"),
        format!(
            "build:\n  targets:\n    - {}\nprecompiled_binaries:\n  repository: local/demo\n  public_key: deadbeef\n",
            target
        ),
    )
    .expect("write xforge.yaml");

    let build_outcome = build::run(build::BuildArgs {
        manifest_dir: dir.clone(),
        target: None,
        profile: "release".to_string(),
        executor: build::BuildExecutorKind::Cargo,
        cross_image: None,
    })
    .expect("build");
    assert!(build_outcome.build_id.starts_with("b1-"));

    let dist_dir = dir.join("dist");
    let bundle_outcome = bundle::run(bundle::BundleArgs {
        manifest_dir: dir.clone(),
        target: None,
        output_dir: dist_dir.clone(),
        profile: "release".to_string(),
    })
    .expect("bundle");
    assert!(bundle_outcome.manifest_path.exists());
    assert!(!bundle_outcome.archive_paths.is_empty());

    let keys = keygen::run().expect("keygen");
    let signed = publish::prepare_signed_assets(
        &bundle_outcome.manifest_path,
        Some(&dist_dir),
        &[],
        Some(&dist_dir),
        &keys.private_key_hex,
    )
    .expect("sign assets");

    let local_out = dir.join("local-release");
    let publisher = LocalPublisher::new(local_out.clone()).expect("publisher");
    let request = PublishRequest {
        repository: "local/demo".to_string(),
        tag: signed.build_id.clone(),
        name: format!("xforge {}", signed.build_id),
        body: format!("XForge release {}", signed.build_id),
        build_id: signed.build_id.clone(),
        manifest_path: signed.signed_manifest_path.clone(),
        assets: signed.assets.clone(),
    };
    let outcome = publish_release(&publisher, request).expect("publish");
    assert!(!outcome.uploaded.is_empty());

    let signed_manifest_contents =
        fs::read_to_string(&signed.signed_manifest_path).expect("read manifest");
    let manifest = deserialize_manifest(&signed_manifest_contents).expect("parse manifest");
    let signing = manifest.signing.as_ref().expect("signing block");
    let public_key = parse_public_key_hex(&signing.public_key).expect("public key");
    let payload = signing_payload(&manifest).expect("payload");
    let signature_bytes = hex::decode(&signing.signature).expect("signature hex");
    let ok = verify(&public_key, &payload, &signature_bytes).expect("verify");
    assert!(ok);
}
