use crate::security::{random_key, KEY_BYTES};
use std::{fs, io::Write, path::Path};
use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::{LocalFree, HLOCAL},
        Security::Cryptography::{
            CryptProtectData, CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
        },
    },
};
use zeroize::Zeroizing;

const ENTROPY_PREFIX: &str = "Interview Buddy DPAPI vault key v1|";

pub fn load_key(path: &Path, service: &str, create: bool) -> Result<Zeroizing<Vec<u8>>, String> {
    if path.is_file() {
        let protected = fs::read(path).map_err(|error| format!("无法读取 DPAPI 密钥：{error}"))?;
        let key = unprotect(&protected, service)?;
        if key.len() != KEY_BYTES {
            return Err("DPAPI 密钥长度无效".into());
        }
        return Ok(key);
    }
    if !create {
        return Err("加密设置密钥不存在".into());
    }
    let key = random_key();
    let protected = protect(&key, service)?;
    atomic_write_new(path, &protected)?;
    Ok(key)
}

pub fn reset_key(path: &Path, _service: &str) -> Result<(), String> {
    if path.is_file() {
        fs::remove_file(path).map_err(|error| format!("无法重置 DPAPI 密钥：{error}"))?;
    }
    Ok(())
}

fn protect(data: &[u8], service: &str) -> Result<Vec<u8>, String> {
    crypt(data, service, true)
}

fn unprotect(data: &[u8], service: &str) -> Result<Zeroizing<Vec<u8>>, String> {
    crypt(data, service, false).map(Zeroizing::new)
}

fn crypt(data: &[u8], service: &str, protect: bool) -> Result<Vec<u8>, String> {
    let entropy = format!("{ENTROPY_PREFIX}{service}").into_bytes();
    let input = blob(data)?;
    let entropy_blob = blob(&entropy)?;
    let mut output = CRYPT_INTEGER_BLOB::default();
    let result = unsafe {
        if protect {
            CryptProtectData(
                &input,
                PCWSTR::null(),
                Some(&entropy_blob),
                None,
                None,
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        } else {
            CryptUnprotectData(
                &input,
                None,
                Some(&entropy_blob),
                None,
                None,
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        }
    };
    result.map_err(|error| format!("Windows DPAPI 操作失败：{error}"))?;
    if output.pbData.is_null() {
        return Err("Windows DPAPI 返回了空数据".into());
    }
    let bytes =
        unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize) }.to_vec();
    unsafe {
        let _ = LocalFree(Some(HLOCAL(output.pbData.cast())));
    }
    Ok(bytes)
}

fn blob(data: &[u8]) -> Result<CRYPT_INTEGER_BLOB, String> {
    Ok(CRYPT_INTEGER_BLOB {
        cbData: data
            .len()
            .try_into()
            .map_err(|_| "DPAPI 输入过大".to_string())?,
        pbData: data.as_ptr() as *mut u8,
    })
}

fn atomic_write_new(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "无法确定安全文件目录".to_string())?;
    fs::create_dir_all(parent).map_err(|error| format!("无法创建安全文件目录：{error}"))?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)
        .map_err(|error| format!("无法创建安全临时文件：{error}"))?;
    temporary
        .write_all(bytes)
        .map_err(|error| format!("无法写入安全临时文件：{error}"))?;
    temporary
        .as_file_mut()
        .sync_all()
        .map_err(|error| format!("无法同步安全临时文件：{error}"))?;
    temporary
        .persist_noclobber(path)
        .map_err(|error| format!("无法保存安全文件：{}", error.error))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dpapi_round_trip_and_entropy_binding() {
        let protected = protect(b"vault-key", "test.service").expect("protect");
        assert_ne!(protected, b"vault-key");
        assert_eq!(
            unprotect(&protected, "test.service")
                .expect("unprotect")
                .as_slice(),
            b"vault-key"
        );
        assert!(unprotect(&protected, "other.service").is_err());
    }
}
