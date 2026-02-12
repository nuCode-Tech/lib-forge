pub mod ed25519;

pub use ed25519::{
    parse_private_key_hex, parse_public_key_hex, public_key_from_private_key, sign, verify,
    SigningError,
};
