mod envelope;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
mod unsupported;
#[cfg(target_os = "windows")]
mod windows;

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
    #[cfg(target_os = "windows")]
    return windows::load_key(path, service, create);
    #[cfg(target_os = "macos")]
    return macos::load_key(path, service, create);
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    return unsupported::load_key(path, service, create);
}

pub fn reset_key(path: &Path, service: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    return windows::reset_key(path, service);
    #[cfg(target_os = "macos")]
    return macos::reset_key(path, service);
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    return unsupported::reset_key(path, service);
}
