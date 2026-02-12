pub mod schema;
pub mod serialize;
pub mod validate;

pub use schema::{
    ArtifactNaming, Artifacts, BindingDescriptor, Bindings, Build, BuildIdentity, Manifest,
    Package, Platform, Platforms, Signing,
};
pub use serialize::{
    deserialize_manifest, serialize_manifest, serialize_manifest_pretty, signing_payload,
};
pub use validate::{validate, ManifestError};
