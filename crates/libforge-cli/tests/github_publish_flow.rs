use std::fs;
use std::path::{Path, PathBuf};

use libforge_cli::commands::{build, bundle, publish};

fn temp_dir(name: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    path.push(format!("libforge-e2e-github-{}-{}", name, stamp));
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
fn end_to_end_publish_github_if_configured() {
    let run = std::env::var("LIBFORGE_E2E_GITHUB").ok();
    if run.as_deref() != Some("1") {
        eprintln!("Skipping GitHub publish test (set LIBFORGE_E2E_GITHUB=1 to enable).");
        return;
    }
    let repo = match std::env::var("LIBFORGE_PUBLISH_REPO") {
        Ok(value) => value,
        Err(_) => {
            eprintln!("Skipping GitHub publish test (LIBFORGE_PUBLISH_REPO not set).");
            return;
        }
    };
    let token = match std::env::var("GITHUB_TOKEN") {
        Ok(value) => value,
        Err(_) => {
            eprintln!("Skipping GitHub publish test (GITHUB_TOKEN not set).");
            return;
        }
    };
    let private_key = match std::env::var("LIBFORGE_PRIVATE_KEY") {
        Ok(value) => value,
        Err(_) => {
            eprintln!("Skipping GitHub publish test (LIBFORGE_PRIVATE_KEY not set).");
            return;
        }
    };

    let dir = temp_dir("flow");
    let crate_name = "e2e-demo";
    write_sample_crate(&dir, crate_name);

    let target = host_target_triple();
    fs::write(
        dir.join("libforge.yaml"),
        format!(
            "build:\n  targets:\n    - {}\nprecompiled_binaries:\n  repository: {}\n  public_key: deadbeef\n",
            target, repo
        ),
    )
    .expect("write libforge.yaml");

    let _build_outcome = build::run(build::BuildArgs {
        manifest_dir: dir.clone(),
        target: None,
        profile: "release".to_string(),
        executor: build::BuildExecutorKind::Cargo,
        cross_image: None,
    })
    .expect("build");

    let dist_dir = dir.join("dist");
    let bundle_outcome = bundle::run(bundle::BundleArgs {
        manifest_dir: dir.clone(),
        target: None,
        output_dir: dist_dir.clone(),
        profile: "release".to_string(),
    })
    .expect("bundle");

    let result = publish::run(publish::PublishArgs {
        manifest: bundle_outcome.manifest_path,
        assets_dir: Some(dist_dir),
        asset_files: vec![],
        out_dir: None,
        repository: repo,
        github_token: token,
        private_key_hex: private_key,
    })
    .expect("publish");

    assert!(!result.uploaded.is_empty());
}
