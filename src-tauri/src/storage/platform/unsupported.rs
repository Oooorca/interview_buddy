use std::path::Path;

pub(crate) fn same_path(left: &Path, right: &Path) -> bool {
    left == right
}

pub(crate) fn path_text(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}
