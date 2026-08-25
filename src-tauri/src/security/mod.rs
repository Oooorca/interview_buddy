mod envelope;
mod platform;

use aes_gcm::aead::{Generate, Key};
use aes_gcm::Aes256Gcm;
use std::path::Path;
use zeroize::Zeroizing;

pub use envelope::{decrypt_envelope, encrypt_envelope, EnvelopeKind};

pub const KEY_BYTES: usize = 32;

pub fn random_key() -> Zeroizing<Vec<u8>> {
    Zeroizing::new(Key::<Aes256Gcm>::generate().to_vec())
}

pub fn load_key(path: &Path, service: &str, create: bool) -> Result<Zeroizing<Vec<u8>>, String> {
    platform::load_key(path, service, create)
}

pub fn reset_key(path: &Path, service: &str) -> Result<(), String> {
    platform::reset_key(path, service)
}
