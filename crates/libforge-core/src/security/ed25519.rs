use ed25519_dalek::{Signature, SigningKey, Signer, Verifier, VerifyingKey};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SigningError {
    InvalidHex,
    InvalidPublicKeyLength { len: usize },
    InvalidPrivateKeyLength { len: usize },
    InvalidSignatureLength { len: usize },
    InvalidPublicKey,
}

impl std::fmt::Display for SigningError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SigningError::InvalidHex => write!(f, "invalid hex string"),
            SigningError::InvalidPublicKeyLength { len } => {
                write!(f, "public key must be 32 bytes, got {}", len)
            }
            SigningError::InvalidPrivateKeyLength { len } => {
                write!(f, "private key must be 64 bytes, got {}", len)
            }
            SigningError::InvalidSignatureLength { len } => {
                write!(f, "signature must be 64 bytes, got {}", len)
            }
            SigningError::InvalidPublicKey => write!(f, "invalid public key"),
        }
    }
}

impl std::error::Error for SigningError {}

pub fn parse_public_key_hex(hex: &str) -> Result<[u8; 32], SigningError> {
    let bytes = hex::decode(hex).map_err(|_| SigningError::InvalidHex)?;
    if bytes.len() != 32 {
        return Err(SigningError::InvalidPublicKeyLength { len: bytes.len() });
    }
    let len = bytes.len();
    Ok(bytes
        .try_into()
        .map_err(|_| SigningError::InvalidPublicKeyLength { len })?)
}

pub fn parse_private_key_hex(hex: &str) -> Result<[u8; 64], SigningError> {
    let bytes = hex::decode(hex).map_err(|_| SigningError::InvalidHex)?;
    if bytes.len() != 64 {
        return Err(SigningError::InvalidPrivateKeyLength { len: bytes.len() });
    }
    let len = bytes.len();
    Ok(bytes
        .try_into()
        .map_err(|_| SigningError::InvalidPrivateKeyLength { len })?)
}

pub fn sign(private_key: &[u8; 64], payload: &[u8]) -> Result<Vec<u8>, SigningError> {
    let secret_bytes: [u8; 32] = private_key[0..32]
        .try_into()
        .map_err(|_| SigningError::InvalidPrivateKeyLength { len: private_key.len() })?;
    let signing_key = SigningKey::from_bytes(&secret_bytes);
    let signature: Signature = signing_key.sign(payload);
    Ok(signature.to_bytes().to_vec())
}

pub fn public_key_from_private_key(private_key: &[u8; 64]) -> Result<[u8; 32], SigningError> {
    let secret_bytes: [u8; 32] = private_key[0..32]
        .try_into()
        .map_err(|_| SigningError::InvalidPrivateKeyLength { len: private_key.len() })?;
    let signing_key = SigningKey::from_bytes(&secret_bytes);
    Ok(signing_key.verifying_key().to_bytes())
}

pub fn verify(
    public_key: &[u8; 32],
    payload: &[u8],
    signature: &[u8],
) -> Result<bool, SigningError> {
    let verifying_key =
        VerifyingKey::from_bytes(public_key).map_err(|_| SigningError::InvalidPublicKey)?;
    if signature.len() != 64 {
        return Err(SigningError::InvalidSignatureLength {
            len: signature.len(),
        });
    }
    let sig_bytes: [u8; 64] = signature
        .try_into()
        .map_err(|_| SigningError::InvalidSignatureLength { len: signature.len() })?;
    let signature = Signature::from_bytes(&sig_bytes);
    Ok(verifying_key.verify(payload, &signature).is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_verify_round_trip() {
        let secret = [7u8; 32];
        let signing_key = SigningKey::from_bytes(&secret);
        let public = signing_key.verifying_key().to_bytes();
        let mut keypair_bytes = [0u8; 64];
        keypair_bytes[0..32].copy_from_slice(&secret);
        keypair_bytes[32..64].copy_from_slice(&public);
        let payload = b"hello";
        let signature = sign(&keypair_bytes, payload).expect("sign");
        let ok = verify(&public, payload, &signature).expect("verify");
        assert!(ok);
        let derived = public_key_from_private_key(&keypair_bytes).expect("public");
        assert_eq!(derived, public);
    }

    #[test]
    fn invalid_key_lengths() {
        assert!(matches!(
            parse_public_key_hex("aa"),
            Err(SigningError::InvalidPublicKeyLength { .. })
        ));
        assert!(matches!(
            parse_private_key_hex("aa"),
            Err(SigningError::InvalidPrivateKeyLength { .. })
        ));
    }
}
