use super::{random_key, KEY_BYTES};
use security_framework::os::macos::keychain::SecKeychain;
use std::path::Path;
use zeroize::Zeroizing;

const ACCOUNT: &str = "vault-key-v1";

pub fn load_key(_path: &Path, service: &str, create: bool) -> Result<Zeroizing<Vec<u8>>, String> {
    let keychain =
        SecKeychain::default().map_err(|error| format!("无法打开 macOS Keychain：{error}"))?;
    if let Ok((password, _item)) = keychain.find_generic_password(service, ACCOUNT) {
        let key = Zeroizing::new(password.to_owned());
        if key.len() != KEY_BYTES {
            return Err("macOS Keychain 中的设置密钥长度无效".into());
        }
        return Ok(key);
    }
    if !create {
        return Err("macOS Keychain 中没有设置密钥".into());
    }
    let key = random_key();
    keychain
        .set_generic_password(service, ACCOUNT, &key)
        .map_err(|error| format!("无法保存 macOS Keychain 密钥：{error}"))?;
    Ok(key)
}

pub fn reset_key(_path: &Path, service: &str) -> Result<(), String> {
    let keychain =
        SecKeychain::default().map_err(|error| format!("无法打开 macOS Keychain：{error}"))?;
    if let Ok((_password, item)) = keychain.find_generic_password(service, ACCOUNT) {
        item.delete();
    }
    Ok(())
}
