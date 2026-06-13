use crate::core::errors::CoreError;

#[tauri::command]
pub fn open_url(url: String) -> Result<(), String> {
    if !url.starts_with("https://") && !url.starts_with("http://") {
        return Err(
            CoreError::Other("URL debe comenzar con http:// o https://".to_string()).to_string(),
        );
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", &url])
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&url)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&url)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Una URL que comienza con `https://` debe ser aceptada.
    #[test]
    fn test_open_url_https() {
        assert!(open_url("https://example.com".into()).is_ok());
    }

    /// Una URL que comienza con `http://` también debe ser aceptada.
    #[test]
    fn test_open_url_http() {
        assert!(open_url("http://example.com".into()).is_ok());
    }

    /// Cualquier otro protocolo (ej. `ftp://`) debe ser rechazado por seguridad.
    #[test]
    fn test_open_url_ftp_rejected() {
        assert!(open_url("ftp://example.com".into()).is_err());
    }

    /// Una URL vacía no es válida.
    #[test]
    fn test_open_url_empty() {
        assert!(open_url(String::new()).is_err());
    }

    /// Una URL sin protocolo debe ser rechazada.
    /// `open_url` solo acepta http/https explícitos.
    #[test]
    fn test_open_url_no_protocol() {
        assert!(open_url("example.com".into()).is_err());
    }
}
