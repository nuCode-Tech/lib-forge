pub mod schema;
pub mod serialize;
pub mod validate;

pub use schema::{
    ArtifactNaming, Artifacts, BindingDescriptor, Bindings, Build, BuildIdentity, Manifest,
    Package, Platform, Platforms,
};
pub use validate::{validate, ManifestError};
