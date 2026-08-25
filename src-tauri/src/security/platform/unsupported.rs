use std::path::Path;
use zeroize::Zeroizing;

pub fn load_key(_path: &Path, _service: &str, _create: bool) -> Result<Zeroizing<Vec<u8>>, String> {
    Err("当前平台不支持安全设置存储".into())
}

pub fn reset_key(_path: &Path, _service: &str) -> Result<(), String> {
    Err("当前平台不支持安全设置存储".into())
}
