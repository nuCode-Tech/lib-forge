use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: &str = "libforge.manifest.v1";

/// The canonical `libforge.manifest.v1` contract.
///
/// Required fields:
/// - `schemaVersion`
/// - `package`
/// - `build`
/// - `artifacts`
/// - `bindings`
/// - `platforms`
///
/// Optional fields within each section default to benign values so adapters
/// continue to work when those sections evolve. Forward compatibility is also
/// enabled by Serde's allowance of extra properties so new tooling can add
/// optional metadata without breaking older deserializers.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    #[serde(default = "default_schema_version")]
    pub schema_version: String,
    pub package: Package,
    pub build: Build,
    pub artifacts: Artifacts,
    pub bindings: Bindings,
    pub platforms: Platforms,
}

fn default_schema_version() -> String {
    SCHEMA_VERSION.to_string()
}

/// Metadata that identifies the distribution.
///
/// Required fields are `name` and `version`. The remaining fields are optional
/// helpers that adapters can leverage for display or attribution.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Package {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub license: Option<String>,
    #[serde(default)]
    pub authors: Vec<String>,
    #[serde(default)]
    pub repository: Option<String>,
}

/// Encodes the identity of the build that produced the manifest.
///
/// The `id` and `identity` fields are required to tie the manifest back to the
/// exact invocation that generated it. Optional metadata such as timestamps,
/// engines, and profiles add provenance but default to `None` when omitted.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Build {
    pub id: String,
    pub identity: BuildIdentity,
    #[serde(default)]
    pub timestamp: Option<String>,
    #[serde(default)]
    pub engine: Option<String>,
}

/// Information that describes the host, toolchain, and configuration used for
/// the build.
///
/// `host` and `toolchain` are required. `profile` is optional and `features` is
/// an optional list that defaults to an empty vector so callers can push
/// additional tags without missing the field entirely.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildIdentity {
    pub host: String,
    pub toolchain: String,
    #[serde(default)]
    pub profile: Option<String>,
    #[serde(default)]
    pub features: Vec<String>,
}

/// Describes how artifacts are named and how their checksums are collected.
///
/// The `naming` block is required, while `checksums` is optional and defaults to
/// an empty list. This section is the single source of truth for artifact
/// naming because every adapter can interpret the template, delimiter, and
/// inclusion flags consistently.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Artifacts {
    pub naming: ArtifactNaming,
    #[serde(default)]
    pub checksums: Vec<String>,
}

/// The naming template that tooling must honor when emitting artifacts.
///
/// `template` is required. `delimiter`, `includePlatform`, and
/// `includeBinding` are optional with documented defaults so serializers that
/// omit them still have a deterministic naming scheme.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactNaming {
    #[serde(default = "default_artifact_template")]
    pub template: String,
    #[serde(default = "default_artifact_delimiter")]
    pub delimiter: String,
    #[serde(default = "default_include_platform")]
    pub include_platform: bool,
    #[serde(default = "default_include_binding")]
    pub include_binding: bool,
}

fn default_artifact_template() -> String {
    "{package.name}-{package.version}-{platform}".to_string()
}

fn default_artifact_delimiter() -> String {
    "-".to_string()
}

fn default_include_platform() -> bool {
    true
}

fn default_include_binding() -> bool {
    true
}

/// Captures binding compatibility information for adapters.
///
/// The `catalog` list must be present and should enumerate every binding that
/// can be distributed. `primary` is optional metadata that tooling can read to
/// highlight the preferred binding for the manifest.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Bindings {
    pub catalog: Vec<BindingDescriptor>,
    #[serde(default)]
    pub primary: Option<String>,
}

/// Single entry that documents name, version, and target compatibility for a
/// binding.
///
/// Both `name` and `version` are required. The lists of `platforms` and
/// `artifacts` default to empty, which keeps backward compatibility even if
/// the manifest later enumerates new bindings.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BindingDescriptor {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub platforms: Vec<String>,
    #[serde(default)]
    pub artifacts: Vec<String>,
}

/// Defines every platform that the manifest resolves.
///
/// `default` is the platform that tooling should fall back to when none is
/// explicitly requested. `targets` is required and should at least include the
/// explicit fallback platform.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Platforms {
    pub default: String,
    pub targets: Vec<Platform>,
}

/// Details for a single platform.
///
/// `name` is required. The `triples`, `bindings`, and `artifacts` lists default
/// to empty collections so they can be omitted and still produce a valid
/// manifest. `description` is optional and may describe selection or ordering
/// hints.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Platform {
    pub name: String,
    #[serde(default)]
    pub triples: Vec<String>,
    #[serde(default)]
    pub bindings: Vec<String>,
    #[serde(default)]
    pub artifacts: Vec<String>,
    #[serde(default)]
    pub description: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_MANIFEST_JSON: &str = r#"
{
  "schemaVersion": "libforge.manifest.v1",
  "package": {
    "name": "libforge-cargo",
    "version": "0.1.0",
    "description": "Core manifest contract example.",
    "license": "Apache-2.0",
    "authors": ["LibForge Team"],
    "repository": "https://github.com/stax/lib-forge"
  },
  "build": {
    "id": "build-20260128",
    "identity": {
      "host": "linux",
      "toolchain": "rustc 1.78.0",
      "profile": "release",
      "features": ["release", "deterministic"]
    },
    "timestamp": "2026-01-28T00:00:00Z",
    "engine": "cargo"
  },
  "artifacts": {
    "naming": {
      "template": "{package.name}-{package.version}-{platform}",
      "delimiter": "-",
      "includePlatform": true,
      "includeBinding": true
    },
    "checksums": ["sha256"]
  },
  "bindings": {
    "primary": "dart",
    "catalog": [
      {
        "name": "dart",
        "version": "3.0.0",
        "platforms": ["linux-x86_64", "android-arm64"],
        "artifacts": ["bundle"]
      },
      {
        "name": "python",
        "version": "3.11",
        "platforms": ["linux-x86_64"],
        "artifacts": ["wheel"]
      }
    ]
  },
  "platforms": {
    "default": "linux-x86_64",
    "targets": [
      {
        "name": "linux-x86_64",
        "triples": ["x86_64-unknown-linux-gnu"],
        "bindings": ["dart", "python"],
        "artifacts": ["bundle", "wheel"],
        "description": "Primary developer linux target"
      },
      {
        "name": "android-arm64",
        "triples": ["armv7-linux-androideabi", "aarch64-linux-android"],
        "bindings": ["dart"],
        "artifacts": ["bundle"]
      }
    ]
  }
}
"#;

    #[test]
    fn example_manifest_deserializes() {
        let manifest: Manifest =
            serde_json::from_str(SAMPLE_MANIFEST_JSON).expect("example manifest should parse");
        assert_eq!(manifest.schema_version, SCHEMA_VERSION);
        assert_eq!(manifest.package.name, "libforge-cargo");
        assert_eq!(manifest.build.identity.host, "linux");
        assert!(manifest.artifacts.naming.include_binding);
        assert_eq!(manifest.bindings.catalog.len(), 2);
        assert_eq!(manifest.platforms.default, "linux-x86_64");
        assert!(manifest
            .platforms
            .targets
            .iter()
            .any(|platform| platform.name == "android-arm64"));
    }
}
