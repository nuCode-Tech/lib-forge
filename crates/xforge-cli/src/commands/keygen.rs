use ed25519_dalek::SigningKey;
use rand_core::OsRng;

pub struct KeygenOutput {
    pub public_key_hex: String,
    pub private_key_hex: String,
}

pub fn run() -> Result<KeygenOutput, String> {
    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);
    let secret = signing_key.to_bytes();
    let public = signing_key.verifying_key().to_bytes();
    let mut private = [0u8; 64];
    private[0..32].copy_from_slice(&secret);
    private[32..64].copy_from_slice(&public);
    Ok(KeygenOutput {
        public_key_hex: hex::encode(public),
        private_key_hex: hex::encode(private),
    })
}
