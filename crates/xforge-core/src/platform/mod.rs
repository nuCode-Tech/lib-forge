pub mod android;
pub mod apple;
pub mod key;
pub mod linux;
pub mod windows;

pub use key::{
    all_platform_keys, all_rust_targets, binding_support, is_supported_rust_target,
    packaging_support, platforms_for_rust_target, registry, BindingSupport, PackagingFormat,
    PackagingSupport, PlatformDescriptor, PlatformKey, PlatformKeyError, SupportStatus,
};
