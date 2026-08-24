use aes_gcm::{
    aead::{Aead, Generate, KeyInit, Payload},
    Aes256Gcm, Key, Nonce,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

const FORMAT: &str = "interview-buddy-encrypted";
const ALGORITHM: &str = "AES-256-GCM";
const KEY_ID: &str = "vault-key-v1";
const VERSION: u8 = 1;
const MAX_PLAINTEXT: usize = 1024 * 1024;
const MAX_ENVELOPE: usize = 2 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvelopeKind {
    Settings,
    StoragePointer,
}

impl EnvelopeKind {
    fn text(self) -> &'static str {
        match self {
            Self::Settings => "settings",
            Self::StoragePointer => "storage-pointer",
        }
    }

    fn aad(self, service: &str) -> String {
        format!("{service}|{}|v{VERSION}", self.text())
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EncryptedEnvelope {
    format: String,
    version: u8,
    kind: String,
    algorithm: String,
    key_id: String,
    nonce: String,
    ciphertext: String,
}

pub fn encrypt_envelope(
    key: &[u8],
    service: &str,
    kind: EnvelopeKind,
    plaintext: &[u8],
) -> Result<Vec<u8>, String> {
    validate_key(key)?;
    if plaintext.len() > MAX_PLAINTEXT {
        return Err("设置内容超过 1 MiB 安全上限".into());
    }
    let key = Key::<Aes256Gcm>::try_from(key).map_err(|_| "设置密钥长度无效")?;
    let cipher = Aes256Gcm::new(&key);
    let nonce = Nonce::generate();
    let ciphertext = cipher
        .encrypt(
            &nonce,
            Payload {
                msg: plaintext,
                aad: kind.aad(service).as_bytes(),
            },
        )
        .map_err(|_| "设置加密失败".to_string())?;
    let envelope = EncryptedEnvelope {
        format: FORMAT.into(),
        version: VERSION,
        kind: kind.text().into(),
        algorithm: ALGORITHM.into(),
        key_id: KEY_ID.into(),
        nonce: URL_SAFE_NO_PAD.encode(nonce.as_slice()),
        ciphertext: URL_SAFE_NO_PAD.encode(ciphertext),
    };
    serde_json::to_vec_pretty(&envelope).map_err(|error| error.to_string())
}

pub fn decrypt_envelope(
    key: &[u8],
    service: &str,
    expected_kind: EnvelopeKind,
    encoded: &[u8],
) -> Result<Zeroizing<Vec<u8>>, String> {
    validate_key(key)?;
    if encoded.len() > MAX_ENVELOPE {
        return Err("加密设置文件超过 2 MiB 安全上限".into());
    }
    let envelope: EncryptedEnvelope =
        serde_json::from_slice(encoded).map_err(|_| "加密设置外壳格式无效".to_string())?;
    if envelope.format != FORMAT
        || envelope.version != VERSION
        || envelope.kind != expected_kind.text()
        || envelope.algorithm != ALGORITHM
        || envelope.key_id != KEY_ID
    {
        return Err("加密设置版本、用途或算法不受支持".into());
    }
    let nonce = URL_SAFE_NO_PAD
        .decode(envelope.nonce)
        .map_err(|_| "加密设置 nonce 无效".to_string())?;
    if nonce.len() != 12 {
        return Err("加密设置 nonce 长度无效".into());
    }
    let ciphertext = URL_SAFE_NO_PAD
        .decode(envelope.ciphertext)
        .map_err(|_| "加密设置密文无效".to_string())?;
    let key = Key::<Aes256Gcm>::try_from(key).map_err(|_| "设置密钥长度无效")?;
    let cipher = Aes256Gcm::new(&key);
    let nonce = Nonce::try_from(nonce.as_slice()).map_err(|_| "加密设置 nonce 长度无效")?;
    let plaintext = cipher
        .decrypt(
            &nonce,
            Payload {
                msg: &ciphertext,
                aad: expected_kind.aad(service).as_bytes(),
            },
        )
        .map_err(|_| "无法验证或解密设置".to_string())?;
    if plaintext.len() > MAX_PLAINTEXT {
        return Err("解密后的设置超过 1 MiB 安全上限".into());
    }
    Ok(Zeroizing::new(plaintext))
}

fn validate_key(key: &[u8]) -> Result<(), String> {
    if key.len() != super::KEY_BYTES {
        return Err("设置密钥长度无效".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_round_trip_and_random_nonce() {
        let key = [7u8; 32];
        let first =
            encrypt_envelope(&key, "test.app", EnvelopeKind::Settings, b"secret").expect("encrypt");
        let second = encrypt_envelope(&key, "test.app", EnvelopeKind::Settings, b"secret")
            .expect("encrypt again");
        assert_ne!(first, second);
        assert_eq!(
            decrypt_envelope(&key, "test.app", EnvelopeKind::Settings, &first)
                .expect("decrypt")
                .as_slice(),
            b"secret"
        );
    }

    #[test]
    fn envelope_rejects_tampering_wrong_key_and_kind() {
        let key = [3u8; 32];
        let mut encoded =
            encrypt_envelope(&key, "test.app", EnvelopeKind::Settings, b"secret").unwrap();
        let last = encoded.len() - 3;
        encoded[last] ^= 1;
        assert!(decrypt_envelope(&key, "test.app", EnvelopeKind::Settings, &encoded).is_err());

        let encoded =
            encrypt_envelope(&key, "test.app", EnvelopeKind::Settings, b"secret").unwrap();
        assert!(
            decrypt_envelope(&[4u8; 32], "test.app", EnvelopeKind::Settings, &encoded).is_err()
        );
        assert!(
            decrypt_envelope(&key, "test.app", EnvelopeKind::StoragePointer, &encoded).is_err()
        );
    }

    #[test]
    fn envelope_rejects_tampered_nonce_ciphertext_aad_and_metadata() {
        let key = [9u8; 32];
        let encoded =
            encrypt_envelope(&key, "test.app", EnvelopeKind::Settings, b"classified").unwrap();
        let mut envelope: EncryptedEnvelope = serde_json::from_slice(&encoded).unwrap();
        envelope.nonce = URL_SAFE_NO_PAD.encode([0u8; 12]);
        assert!(decrypt_envelope(
            &key,
            "test.app",
            EnvelopeKind::Settings,
            &serde_json::to_vec(&envelope).unwrap()
        )
        .is_err());

        let mut envelope: EncryptedEnvelope = serde_json::from_slice(&encoded).unwrap();
        let mut ciphertext = URL_SAFE_NO_PAD.decode(&envelope.ciphertext).unwrap();
        ciphertext[0] ^= 0x80;
        envelope.ciphertext = URL_SAFE_NO_PAD.encode(ciphertext);
        assert!(decrypt_envelope(
            &key,
            "test.app",
            EnvelopeKind::Settings,
            &serde_json::to_vec(&envelope).unwrap()
        )
        .is_err());
        assert!(decrypt_envelope(&key, "other.app", EnvelopeKind::Settings, &encoded).is_err());

        let mut envelope: EncryptedEnvelope = serde_json::from_slice(&encoded).unwrap();
        envelope.version = 2;
        assert!(decrypt_envelope(
            &key,
            "test.app",
            EnvelopeKind::Settings,
            &serde_json::to_vec(&envelope).unwrap()
        )
        .is_err());
    }

    #[test]
    fn envelope_enforces_size_limits() {
        let key = [1u8; 32];
        assert!(encrypt_envelope(
            &key,
            "test.app",
            EnvelopeKind::Settings,
            &vec![0; MAX_PLAINTEXT + 1]
        )
        .is_err());
        assert!(decrypt_envelope(
            &key,
            "test.app",
            EnvelopeKind::Settings,
            &vec![b'x'; MAX_ENVELOPE + 1]
        )
        .is_err());
    }
}
