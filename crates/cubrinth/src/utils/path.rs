use std::path::{Component, Path, PathBuf};

/// Returns `base.join(rel)` only if `rel` is a relative path without parent
/// directory references (`..`), absolute roots or prefixes.
pub fn safe_join(base: &Path, rel: &str) -> Result<PathBuf, String> {
    let rel_path = Path::new(rel);
    if rel_path.is_absolute() {
        return Err(format!("Absolute path not allowed: '{}'", rel));
    }
    for component in rel_path.components() {
        if !matches!(component, Component::Normal(_) | Component::CurDir) {
            return Err(format!("Path traversal attempt: '{}'", rel));
        }
    }
    Ok(base.join(rel_path))
}

/// Validates that a ZIP entry path is safe to extract.
/// Returns the enclosed path if it contains only normal components, otherwise
/// `None`.
pub fn safe_zip_entry_name(entry_name: &str) -> Option<&Path> {
    let path = Path::new(entry_name);
    if path.is_absolute() {
        return None;
    }
    for component in path.components() {
        if !matches!(component, Component::Normal(_) | Component::CurDir) {
            return None;
        }
    }
    Some(path)
}
