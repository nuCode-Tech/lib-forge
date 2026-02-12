#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BindingLanguage {
    Kotlin,
    Swift,
    Python,
    Dart,
}

impl BindingLanguage {
    pub fn as_str(self) -> &'static str {
        match self {
            BindingLanguage::Kotlin => "kotlin",
            BindingLanguage::Swift => "swift",
            BindingLanguage::Python => "python",
            BindingLanguage::Dart => "dart",
        }
    }
}

impl std::fmt::Display for BindingLanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for BindingLanguage {
    type Err = BindingMetadataError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "kotlin" => Ok(BindingLanguage::Kotlin),
            "swift" => Ok(BindingLanguage::Swift),
            "python" => Ok(BindingLanguage::Python),
            "dart" => Ok(BindingLanguage::Dart),
            _ => Err(BindingMetadataError::UnknownBinding(value.to_string())),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BindingMetadata {
    Swift(SwiftBinding),
    Kotlin(KotlinBinding),
    Python(PythonBinding),
    Dart(DartBinding),
}

impl BindingMetadata {
    pub fn language(&self) -> BindingLanguage {
        match self {
            BindingMetadata::Swift(_) => BindingLanguage::Swift,
            BindingMetadata::Kotlin(_) => BindingLanguage::Kotlin,
            BindingMetadata::Python(_) => BindingLanguage::Python,
            BindingMetadata::Dart(_) => BindingLanguage::Dart,
        }
    }

    pub fn validate(&self) -> Result<(), BindingMetadataError> {
        match self {
            BindingMetadata::Swift(binding) => binding.validate(),
            BindingMetadata::Kotlin(binding) => binding.validate(),
            BindingMetadata::Python(binding) => binding.validate(),
            BindingMetadata::Dart(binding) => binding.validate(),
        }
    }

    pub fn canonical_string(&self) -> String {
        match self {
            BindingMetadata::Swift(binding) => binding.canonical_string(),
            BindingMetadata::Kotlin(binding) => binding.canonical_string(),
            BindingMetadata::Python(binding) => binding.canonical_string(),
            BindingMetadata::Dart(binding) => binding.canonical_string(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SwiftBinding {
    pub toolchain: String,
    pub deployment_target: String,
}

impl SwiftBinding {
    pub fn validate(&self) -> Result<(), BindingMetadataError> {
        if self.deployment_target.trim().is_empty() {
            return Err(BindingMetadataError::MissingField {
                binding: "swift",
                field: "deployment_target",
            });
        }
        if self.toolchain.trim().is_empty() {
            return Err(BindingMetadataError::MissingField {
                binding: "swift",
                field: "toolchain",
            });
        }
        Ok(())
    }

    pub fn canonical_string(&self) -> String {
        format!(
            "swift:toolchain={};deployment_target={}",
            self.toolchain.trim(),
            self.deployment_target.trim()
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KotlinBinding {
    pub min_sdk: u32,
    pub jvm_target: String,
    pub ndk_abis: Vec<String>,
}

impl KotlinBinding {
    pub fn validate(&self) -> Result<(), BindingMetadataError> {
        if self.min_sdk == 0 {
            return Err(BindingMetadataError::MissingField {
                binding: "kotlin",
                field: "min_sdk",
            });
        }
        if self.jvm_target.trim().is_empty() {
            return Err(BindingMetadataError::MissingField {
                binding: "kotlin",
                field: "jvm_target",
            });
        }
        if self.ndk_abis.is_empty() || self.ndk_abis.iter().any(|value| value.trim().is_empty()) {
            return Err(BindingMetadataError::MissingField {
                binding: "kotlin",
                field: "ndk_abis",
            });
        }
        Ok(())
    }

    pub fn canonical_string(&self) -> String {
        let mut ndk_abis = self
            .ndk_abis
            .iter()
            .map(|value| value.trim().to_string())
            .collect::<Vec<String>>();
        ndk_abis.sort();
        format!(
            "kotlin:min_sdk={};jvm_target={};ndk_abis={}",
            self.min_sdk,
            self.jvm_target.trim(),
            ndk_abis.join(","),
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PythonBinding {
    pub abi_tag: String,
    pub platform_tag: String,
}

impl PythonBinding {
    pub fn validate(&self) -> Result<(), BindingMetadataError> {
        if self.abi_tag.trim().is_empty() {
            return Err(BindingMetadataError::MissingField {
                binding: "python",
                field: "abi_tag",
            });
        }
        if self.platform_tag.trim().is_empty() {
            return Err(BindingMetadataError::MissingField {
                binding: "python",
                field: "platform_tag",
            });
        }
        Ok(())
    }

    pub fn canonical_string(&self) -> String {
        format!(
            "python:abi_tag={};platform_tag={}",
            self.abi_tag.trim(),
            self.platform_tag.trim()
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DartBinding {
    pub sdk_constraint: String,
    pub ffi_abi: String,
}

impl DartBinding {
    pub fn validate(&self) -> Result<(), BindingMetadataError> {
        if self.sdk_constraint.trim().is_empty() {
            return Err(BindingMetadataError::MissingField {
                binding: "dart",
                field: "sdk_constraint",
            });
        }
        if self.ffi_abi.trim().is_empty() {
            return Err(BindingMetadataError::MissingField {
                binding: "dart",
                field: "ffi_abi",
            });
        }
        Ok(())
    }

    pub fn canonical_string(&self) -> String {
        format!(
            "dart:sdk_constraint={};ffi_abi={}",
            self.sdk_constraint.trim(),
            self.ffi_abi.trim()
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BindingMetadataSet {
    pub bindings: Vec<BindingMetadata>,
}

impl BindingMetadataSet {
    pub fn validate(&self) -> Result<(), BindingMetadataError> {
        for binding in &self.bindings {
            binding.validate()?;
        }
        Ok(())
    }

    pub fn canonical_string(&self) -> String {
        let mut entries = self
            .bindings
            .iter()
            .map(|binding| binding.canonical_string())
            .collect::<Vec<String>>();
        entries.sort();
        entries.join("|")
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BindingMetadataError {
    MissingField {
        binding: &'static str,
        field: &'static str,
    },
    UnknownBinding(String),
}

impl std::fmt::Display for BindingMetadataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BindingMetadataError::MissingField { binding, field } => {
                write!(f, "binding '{}' missing ABI field '{}'", binding, field)
            }
            BindingMetadataError::UnknownBinding(value) => {
                write!(f, "unknown binding language '{}'", value)
            }
        }
    }
}

impl std::error::Error for BindingMetadataError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_string_is_order_independent() {
        let binding = BindingMetadata::Kotlin(KotlinBinding {
            min_sdk: 21,
            jvm_target: "1.8".to_string(),
            ndk_abis: vec!["x86_64".to_string(), "arm64-v8a".to_string()],
        });
        assert_eq!(
            binding.canonical_string(),
            "kotlin:min_sdk=21;jvm_target=1.8;ndk_abis=arm64-v8a,x86_64"
        );

        let binding = BindingMetadata::Python(PythonBinding {
            abi_tag: "cp311".to_string(),
            platform_tag: "manylinux_2_28".to_string(),
        });
        assert_eq!(
            binding.canonical_string(),
            "python:abi_tag=cp311;platform_tag=manylinux_2_28"
        );
    }
}
