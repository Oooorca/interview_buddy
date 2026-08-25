use std::path::Path;

pub(crate) fn same_path(left: &Path, right: &Path) -> bool {
    path_text(left).eq_ignore_ascii_case(&path_text(right))
}

pub(crate) fn path_text(path: &Path) -> String {
    let text = path.to_string_lossy();
    if let Some(rest) = text.strip_prefix(r"\\?\UNC\") {
        return format!(r"\\{rest}");
    }
    if let Some(rest) = text.strip_prefix(r"\\?\") {
        return rest.to_string();
    }
    text.into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_path_hides_windows_extended_length_prefix() {
        assert_eq!(
            path_text(Path::new(r"\\?\C:\Tools\Interview Buddy\cache")),
            r"C:\Tools\Interview Buddy\cache"
        );
        assert_eq!(
            path_text(Path::new(r"\\?\UNC\server\share\cache")),
            r"\\server\share\cache"
        );
    }
}
