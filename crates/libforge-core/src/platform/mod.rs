pub mod android;
pub mod apple;
pub mod key;
pub mod linux;
pub mod windows;

pub use key::{
    binding_support, packaging_support, platforms_for_rust_target, registry, Architecture,
    BindingSupport, PackagingFormat, PackagingSupport, PlatformDescriptor, PlatformFamily,
    PlatformKey, PlatformKeyError, PlatformOs, SupportStatus,
};
