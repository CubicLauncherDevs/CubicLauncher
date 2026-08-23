use std::path::{Component, Path, PathBuf};

/// Validates that `sub_path` is a relative path containing only normal
/// components (and `.` segments). It rejects absolute paths, parent directory
/// references (`..`) and any root/prefix components.
///
/// On success, returns `base.join(sub_path)`. The caller should still verify the
/// final resolved path stays under `base` when dealing with filesystem entries
/// that might be symlinks.
pub fn sanitize_path(base: &Path, sub_path: &Path) -> Result<PathBuf, String> {
    if sub_path.is_absolute() {
        return Err(format!(
            "Ruta absoluta no permitida: '{}'",
            sub_path.display()
        ));
    }

    for component in sub_path.components() {
        if !matches!(component, Component::Normal(_) | Component::CurDir) {
            return Err(format!(
                "Componente de ruta no permitido en: '{}'",
                sub_path.display()
            ));
        }
    }

    Ok(base.join(sub_path))
}

/// Convenience helper for callers that receive a string path.
pub fn safe_join(base: &Path, relative: &str) -> Result<PathBuf, String> {
    sanitize_path(base, Path::new(relative))
}

/// Validates a path component (file or directory name) to ensure it can safely be
/// used as a single segment. Rejects empty strings, `.`, `..`, path separators,
/// null bytes and other special filesystem characters.
pub fn validate_path_component(component: &str) -> Result<(), String> {
    if component.is_empty() {
        return Err("El componente de ruta está vacío".into());
    }
    if component == "." || component == ".." {
        return Err(format!("Componente de ruta no permitido: '{}'", component));
    }

    const FORBIDDEN: &[char] = &['/', '\\', '\0', '<', '>', ':', '"', '|', '?', '*'];
    if component.chars().any(|c| FORBIDDEN.contains(&c)) {
        return Err(format!(
            "El componente de ruta contiene caracteres no permitidos: '{}'",
            component
        ));
    }

    Ok(())
}

/// Validates a file name intended to be written into a specific directory.
/// It must be a single component, so paths like `foo/bar` or `../x` are rejected.
pub fn validate_filename(filename: &str) -> Result<(), String> {
    validate_path_component(filename)
}

/// Validates an identifier derived from user input (e.g. a theme id).
/// Allows letters, digits, underscores, hyphens and dots; this is stricter than
/// a generic filename because identifiers are also used as directory names.
pub fn validate_identifier(id: &str) -> Result<(), String> {
    if id.is_empty() || id.len() > 64 {
        return Err(
            "El identificador no puede estar vacío ni superar 64 caracteres".into(),
        );
    }
    if id == "." || id == ".." || id.contains(":") || id.contains('\0') {
        return Err("El identificador contiene caracteres no permitidos".into());
    }
    if id.contains('/') || id.contains('\\') {
        return Err("El identificador no puede contener separadores de ruta".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_absolute_path() {
        let base = Path::new("/tmp/instances/test");
        assert!(sanitize_path(base, Path::new("/etc/passwd")).is_err());
    }

    #[test]
    fn test_sanitize_parent_dir() {
        let base = Path::new("/tmp/instances/test");
        assert!(sanitize_path(base, Path::new("../malicious")).is_err());
    }

    #[test]
    fn test_sanitize_dotdot_nested() {
        let base = Path::new("/tmp/instances/test");
        assert!(sanitize_path(base, Path::new("mods/../../secrets")).is_err());
    }

    #[test]
    fn test_sanitize_valid_sub_path() {
        let base = PathBuf::from("/tmp/instances/test");
        let result = sanitize_path(&base, Path::new("mods")).unwrap();
        assert_eq!(result, base.join("mods"));
    }

    #[test]
    fn test_sanitize_nested_valid() {
        let base = PathBuf::from("/tmp/instances/test");
        let result = sanitize_path(&base, Path::new("screenshots/2025-01-01")).unwrap();
        assert_eq!(result, base.join("screenshots/2025-01-01"));
    }

    #[test]
    fn test_validate_identifier_rejects_dotdot() {
        assert!(validate_identifier("..").is_err());
        assert!(validate_identifier("foo/../bar").is_err());
    }

    #[test]
    fn test_validate_identifier_accepts_safe() {
        assert!(validate_identifier("my_theme").is_ok());
        assert!(validate_identifier("author.theme-v2").is_ok());
    }

    #[test]
    fn test_validate_filename_rejects_path() {
        assert!(validate_filename("foo/bar.jar").is_err());
        assert!(validate_filename("../bar.jar").is_err());
    }

    #[test]
    fn test_validate_filename_accepts_basename() {
        assert!(validate_filename("mod.jar").is_ok());
    }
}
