use libforge_core::artifact::layout::ArchiveLayout;
use libforge_core::build_plan::BuiltArtifact;

mod common;
pub mod android;
pub mod tar;
pub mod xcframework;
pub mod zip;

pub use android::AarPacker;
pub use tar::TarGzPacker;
pub use xcframework::XcframeworkPacker;
pub use zip::ZipPacker;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PackFormat {
    Zip,
    TarGz,
    XCFramework,
    AAR,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackInput {
    pub artifact: BuiltArtifact,
    pub layout: ArchiveLayout,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackRequest {
    pub format: PackFormat,
    pub inputs: Vec<PackInput>,
    pub output_dir: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackResult {
    pub format: PackFormat,
    pub output_paths: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PackError {
    InvalidRequest { message: String },
    Io { message: String },
}

impl std::fmt::Display for PackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PackError::InvalidRequest { message } => write!(f, "invalid pack request: {}", message),
            PackError::Io { message } => write!(f, "pack i/o error: {}", message),
        }
    }
}

impl std::error::Error for PackError {}

pub trait PackExecutor {
    fn pack(&self, request: &PackRequest) -> Result<PackResult, PackError>;
}
