use std::process::{Command, Stdio};

use xforge_core::build_plan::{BuildEnvVar, BuildPlan, BuiltArtifact};

use crate::builder::{BuildError, BuildExecutor, BuildResult};

#[derive(Clone, Debug, Default)]
pub struct CargoExecutor;

impl CargoExecutor {
    pub fn new() -> Self {
        Self
    }
}

impl BuildExecutor for CargoExecutor {
    fn execute(&self, plan: &BuildPlan) -> BuildResult<Vec<BuiltArtifact>> {
        let mut artifacts = Vec::with_capacity(plan.targets.len());
        for target in &plan.targets {
            let mut command = Command::new("cargo");
            command
                .arg("build")
                .args(profile_args(&plan.profile.name))
                .arg("--target")
                .arg(&target.rust_target_triple)
                .arg("--manifest-path")
                .arg(&target.cargo_manifest_path)
                .args(&plan.profile.cargo_args)
                .args(&target.cargo_args)
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .current_dir(&target.working_dir);
            if !target.cargo_features.is_empty() {
                command
                    .arg("--features")
                    .arg(target.cargo_features.join(","));
            }
            apply_rustflags(&plan.profile.rustflags, &mut command);
            apply_env(&plan.profile.env, &mut command);
            apply_env(&target.env, &mut command);
            apply_toolchain(&plan.profile.toolchain.channel, &mut command);
            let status = command
                .status()
                .map_err(|error| BuildError::new(format!("cargo build failed: {}", error)))?;
            if !status.success() {
                return Err(BuildError::new(format!(
                    "cargo build exited with status {}",
                    status
                )));
            }
            artifacts.push(target.artifact.clone());
        }
        Ok(artifacts)
    }
}

fn profile_args(profile: &str) -> Vec<String> {
    if profile == "release" {
        vec!["--release".to_string()]
    } else {
        vec!["--profile".to_string(), profile.to_string()]
    }
}

fn apply_rustflags(flags: &[String], command: &mut Command) {
    if flags.is_empty() {
        return;
    }
    let joined = flags.join(" ");
    command.env("RUSTFLAGS", joined);
}

fn apply_env(values: &[BuildEnvVar], command: &mut Command) {
    for entry in values {
        command.env(&entry.key, &entry.value);
    }
}

fn apply_toolchain(channel: &Option<String>, command: &mut Command) {
    if let Some(channel) = channel {
        command.env("RUSTUP_TOOLCHAIN", channel);
    }
}
