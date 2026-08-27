use crate::core::errors::CoreError;
use crate::services::SettingsManager;
use aqua::parse_java_major_version;
use serde::Serialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use sysinfo::{MemoryRefreshKind, RefreshKind, System};
use tauri::command;
use tracing::{info, warn};

#[command]
pub fn get_settings() -> Result<SettingsManager, String> {
    info!("Accediendo a configuración actual");
    Ok(SettingsManager::snapshot())
}

#[command]
pub async fn update_settings(mut new_settings: SettingsManager) -> Result<(), String> {
    info!("Actualizando configuración");
    if new_settings.min_memory == 0 {
        warn!("min_memory no puede ser 0, usando 1");
    }
    if new_settings.max_memory == 0 {
        warn!("max_memory no puede ser 0, usando 2");
    }
    if new_settings.min_memory > new_settings.max_memory {
        return Err(
            CoreError::Other("min_memory no puede ser mayor que max_memory".to_string())
                .to_string(),
        );
    }
    new_settings.console_history_limit = new_settings.console_history_limit.clamp(100, 5000);
    SettingsManager::write(|s| {
        for new_user in &mut new_settings.user {
            let _ = new_user.load_tokens();
        }
        *s = new_settings;
    })?;
    SettingsManager::save().await?;
    info!("Configuración actualizada y guardada");
    Ok(())
}

#[derive(Serialize)]
pub struct JavaPaths {
    jre8: String,
    jre17: String,
    jre21: String,
    jre25: String,
}

#[derive(Debug, Default)]
struct JavaPathSlots {
    jre8: Option<String>,
    jre17: Option<String>,
    jre21: Option<String>,
    jre25: Option<String>,
}

impl JavaPathSlots {
    fn set_if_empty(&mut self, major: u8, path: String) -> bool {
        let slot = match major {
            8 => &mut self.jre8,
            17 => &mut self.jre17,
            21 => &mut self.jre21,
            25 => &mut self.jre25,
            _ => return false,
        };
        if slot.is_some() {
            return false;
        }
        *slot = Some(path);
        true
    }

    fn into_java_paths(self) -> JavaPaths {
        JavaPaths {
            jre8: self.jre8.unwrap_or_default(),
            jre17: self.jre17.unwrap_or_default(),
            jre21: self.jre21.unwrap_or_default(),
            jre25: self.jre25.unwrap_or_default(),
        }
    }
}

/// Name of the Java binary used for version detection. `javaw.exe` is used for
/// launching, but `java.exe` is used here because it prints `-version` output.
fn detection_binary_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "java.exe"
    } else {
        "java"
    }
}

fn java_home_candidates() -> Vec<PathBuf> {
    let mut out = Vec::new();

    // $JAVA_HOME always wins.
    if let Ok(java_home) = std::env::var("JAVA_HOME") {
        let candidate = Path::new(&java_home)
            .join("bin")
            .join(detection_binary_name());
        out.push(candidate);
    }

    #[cfg(target_os = "windows")]
    {
        let base_dirs = [
            PathBuf::from(r"C:\Program Files\Java"),
            PathBuf::from(r"C:\Program Files\Eclipse Adoptium"),
            PathBuf::from(r"C:\Program Files\AdoptOpenJDK"),
            PathBuf::from(r"C:\Program Files\Microsoft"),
            PathBuf::from(r"C:\Program Files\Amazon Corretto"),
        ];
        for base in &base_dirs {
            push_subdirectory_bins(base, &mut out);
        }
    }

    #[cfg(target_os = "macos")]
    {
        // Typical locations for installed JDKs/JREs on macOS.
        let mac_vm_base = PathBuf::from("/Library/Java/JavaVirtualMachines");
        if let Ok(entries) = std::fs::read_dir(&mac_vm_base) {
            for entry in entries.flatten() {
                let home = entry.path().join("Contents").join("Home");
                out.push(home.join("bin").join(detection_binary_name()));
            }
        }
        let brew_bases = [
            PathBuf::from("/opt/homebrew/opt"),
            PathBuf::from("/usr/local/opt"),
        ];
        for base in &brew_bases {
            push_matching_subdirectory_bins(base, "openjdk", &mut out);
        }
        push_subdirectory_bins(Path::new("/usr/lib/jvm"), &mut out);
    }

    #[cfg(target_os = "linux")]
    {
        push_subdirectory_bins(Path::new("/usr/lib/jvm"), &mut out);
        if Path::new("/usr/bin/java").exists() {
            out.push(PathBuf::from("/usr/bin/java"));
        }
    }

    out
}

fn push_subdirectory_bins(base: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(base) else {
        return;
    };
    for entry in entries.flatten() {
        let candidate = entry.path().join("bin").join(detection_binary_name());
        if candidate.exists() {
            out.push(candidate);
        }
    }
}

#[cfg(target_os = "macos")]
fn push_matching_subdirectory_bins(base: &Path, prefix: &str, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(base) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy().to_lowercase();
        if !name.starts_with(prefix) {
            continue;
        }
        let candidate = entry
            .path()
            .join("libexec")
            .join("openjdk.jdk")
            .join("Contents")
            .join("Home")
            .join("bin")
            .join(detection_binary_name());
        if candidate.exists() {
            out.push(candidate);
        }
        // Direct symlinked Home.
        let candidate2 = entry.path().join("bin").join(detection_binary_name());
        if candidate2.exists() {
            out.push(candidate2);
        }
    }
}

fn detect_java_major_version_sync(java_bin: &Path) -> Option<u8> {
    let output = std::process::Command::new(java_bin)
        .arg("-version")
        .output()
        .ok()?;

    let text = String::from_utf8_lossy(if output.stderr.is_empty() {
        &output.stdout
    } else {
        &output.stderr
    });

    parse_java_major_version(&text)
}

#[command]
pub fn detect_java_paths() -> Result<JavaPaths, String> {
    info!("Detectando rutas de Java");

    let candidates = java_home_candidates();
    let mut seen = HashSet::new();
    let mut slots = JavaPathSlots::default();

    for candidate in candidates {
        let canonical = std::fs::canonicalize(&candidate).unwrap_or_else(|_| candidate.clone());
        if !canonical.exists() || !seen.insert(canonical.clone()) {
            continue;
        }

        if let Some(major) = detect_java_major_version_sync(&canonical) {
            info!("Detectado Java {} en {}", major, canonical.display());
            slots.set_if_empty(major, canonical.to_string_lossy().into_owned());
        }
    }

    let paths = slots.into_java_paths();
    info!(
        "Rutas Java detectadas: JRE8={}, JRE17={}, JRE21={}, JRE25={}",
        paths.jre8, paths.jre17, paths.jre21, paths.jre25
    );
    Ok(paths)
}

#[derive(Serialize, Copy, Clone)]
pub struct RecommendedRam {
    pub total_gb: u32,
    pub min_gb: u32,
    pub max_gb: u32,
}

fn calculate_recommended_gb(total_gb: u32) -> u32 {
    if total_gb <= 4 {
        2
    } else if total_gb <= 8 {
        (((total_gb as f64 / 3.0 * 2.0).round() / 2.0).clamp(2.0, 3.0)) as u32
    } else {
        (((total_gb as f64 / 4.0 * 2.0).round() / 2.0).clamp(3.0, 8.0)) as u32
    }
}

static RECOMMENDED_RAM: OnceLock<RecommendedRam> = OnceLock::new();

#[command]
pub fn get_recommended_ram() -> Result<RecommendedRam, String> {
    Ok(*RECOMMENDED_RAM.get_or_init(|| {
        let sys = System::new_with_specifics(
            RefreshKind::new().with_memory(MemoryRefreshKind::new().with_ram()),
        );
        let total_gb = (sys.total_memory() / 1024 / 1024).max(1) as u32;
        let max_gb = calculate_recommended_gb(total_gb);
        let min_gb = (max_gb / 2).max(1);

        info!(
            "RAM detectada: {} GB, min: {} GB, max: {} GB",
            total_gb, min_gb, max_gb
        );

        RecommendedRam {
            total_gb,
            min_gb,
            max_gb,
        }
    }))
}
