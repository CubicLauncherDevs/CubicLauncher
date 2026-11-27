//! Garantiza que `xrandr` esté disponible para LWJGL 2.
//!
//! LWJGL 2.9.4 enumera los modos de pantalla ejecutando el comando externo
//! `xrandr -q`. Si no está en `PATH`, `XRandR.populate()` falla y el juego
//! muere con `ArrayIndexOutOfBoundsException: 0` en
//! `LinuxDisplay.getAvailableDisplayModes`.

#[cfg(target_os = "linux")]
use std::path::{Path, PathBuf};

#[cfg(target_os = "linux")]
const COMMON_PATHS: &[&str] = &[
    "/run/current-system/sw/bin/xrandr",
    "/usr/bin/xrandr",
    "/usr/local/bin/xrandr",
    "/bin/xrandr",
];

#[cfg(target_os = "linux")]
pub(crate) fn ensure_xrandr(env: &mut std::collections::HashMap<String, String>) {
    let inherited = env
        .get("PATH")
        .cloned()
        .or_else(|| std::env::var("PATH").ok());
    let configured = std::env::var_os("CUBIC_XRANDR").map(PathBuf::from);
    ensure_xrandr_in(env, inherited.as_deref(), configured, COMMON_PATHS);
}

#[cfg(not(target_os = "linux"))]
pub(crate) fn ensure_xrandr(_env: &mut std::collections::HashMap<String, String>) {}

#[cfg(target_os = "linux")]
fn ensure_xrandr_in(
    env: &mut std::collections::HashMap<String, String>,
    inherited: Option<&str>,
    configured: Option<PathBuf>,
    common: &[&str],
) {
    let path = inherited.unwrap_or_default();
    if find_in_path("xrandr", path).is_some() {
        return;
    }

    let binary = configured
        .filter(|candidate| is_executable(candidate))
        .or_else(|| {
            common
                .iter()
                .map(PathBuf::from)
                .find(|candidate| is_executable(candidate))
        });

    let Some(binary) = binary else {
        log::warn!(
            "xrandr no está disponible; LWJGL 2 podría no poder enumerar los modos de pantalla"
        );
        return;
    };
    let Some(dir) = binary.parent() else {
        return;
    };

    let mut value = dir.display().to_string();
    if !path.is_empty() {
        value.push(':');
        value.push_str(path);
    }
    log::info!("xrandr añadido al PATH: {}", binary.display());
    env.insert("PATH".into(), value);
}

#[cfg(target_os = "linux")]
fn find_in_path(name: &str, path: &str) -> Option<PathBuf> {
    path.split(':')
        .filter(|dir| !dir.is_empty())
        .map(|dir| Path::new(dir).join(name))
        .find(|candidate| is_executable(candidate))
}

#[cfg(target_os = "linux")]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;

    path.metadata()
        .map(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};

    fn fake_xrandr(dir: &Path) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;

        let binary = dir.join("xrandr");
        std::fs::write(&binary, b"#!/bin/sh\n").unwrap();
        std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o755)).unwrap();
        binary
    }

    #[test]
    fn keeps_the_path_when_xrandr_is_already_available() {
        let temp = tempfile::tempdir().unwrap();
        fake_xrandr(temp.path());
        let original = temp.path().display().to_string();
        let mut env = HashMap::from([("PATH".to_string(), original.clone())]);
        ensure_xrandr_in(&mut env, Some(original.as_str()), None, &[]);
        assert_eq!(env.get("PATH"), Some(&original));
    }

    #[test]
    fn prepends_the_configured_xrandr_to_the_path() {
        let temp = tempfile::tempdir().unwrap();
        let binary = fake_xrandr(temp.path());
        let mut env = HashMap::new();
        ensure_xrandr_in(&mut env, Some("/nonexistent"), Some(binary), &[]);
        assert_eq!(
            env.get("PATH").unwrap(),
            &format!("{}:/nonexistent", temp.path().display())
        );
    }

    #[test]
    fn leaves_the_path_untouched_when_xrandr_is_missing() {
        let mut env = HashMap::from([("PATH".to_string(), "/nonexistent".to_string())]);
        ensure_xrandr_in(&mut env, Some("/nonexistent"), None, &[]);
        assert_eq!(env.get("PATH").unwrap(), "/nonexistent");
    }
}
