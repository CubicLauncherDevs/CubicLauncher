use crate::core::errors::CoreError;
use crate::services::SettingsManager;
use serde::Serialize;
use std::path::Path;
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

#[command]
pub fn detect_java_paths() -> Result<JavaPaths, String> {
    info!("Detectando rutas de Java");
    let mut paths = JavaPaths {
        jre8: String::new(),
        jre17: String::new(),
        jre21: String::new(),
        jre25: String::new(),
    };

    #[cfg(target_os = "windows")]
    {
        // Simple heuristic for Windows
        let base_dirs = [
            "C:\\Program Files\\Java",
            "C:\\Program Files\\Eclipse Adoptium",
            "C:\\Program Files\\AdoptOpenJDK",
        ];

        for base in base_dirs {
            if let Ok(entries) = std::fs::read_dir(base) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        let name = match path.file_name() {
                            Some(n) => n.to_string_lossy().to_lowercase(),
                            None => String::new(),
                        };
                        let exact_java = path.join("bin").join("javaw.exe");
                        if exact_java.exists() {
                            if name.contains("1.8") || name.contains("-8") {
                                if paths.jre8.is_empty() {
                                    paths.jre8 = exact_java.to_string_lossy().into_owned();
                                }
                            } else if name.contains("-17") || name.contains("17.") {
                                if paths.jre17.is_empty() {
                                    paths.jre17 = exact_java.to_string_lossy().into_owned();
                                }
                            } else if name.contains("-21") || name.contains("21.") {
                                if paths.jre21.is_empty() {
                                    paths.jre21 = exact_java.to_string_lossy().into_owned();
                                }
                            } else if name.contains("-25") || name.contains("25.") {
                                if paths.jre25.is_empty() {
                                    paths.jre25 = exact_java.to_string_lossy().into_owned();
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        // Simple heuristic for Linux
        let base_dir = "/usr/lib/jvm";
        info!("Escaneando {} en busca de JVMs", base_dir);
        if let Ok(entries) = std::fs::read_dir(base_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let name = match path.file_name() {
                        Some(n) => n.to_string_lossy().to_lowercase(),
                        None => String::new(),
                    };
                    let exact_java = path.join("bin").join("java");
                    if exact_java.exists() {
                        if name.contains("-8-") || name.contains("1.8.0") {
                            if paths.jre8.is_empty() {
                                paths.jre8 = exact_java.to_string_lossy().into_owned();
                            }
                        } else if name.contains("-17-")
                            || name.ends_with("-17")
                            || name.contains("17-")
                        {
                            if paths.jre17.is_empty() {
                                paths.jre17 = exact_java.to_string_lossy().into_owned();
                            }
                        } else if name.contains("-21-")
                            || name.ends_with("-21")
                            || name.contains("21-")
                        {
                            if paths.jre21.is_empty() {
                                paths.jre21 = exact_java.to_string_lossy().into_owned();
                            }
                        } else if (name.contains("-25-")
                            || name.ends_with("-25")
                            || name.contains("25-"))
                            && paths.jre25.is_empty()
                        {
                            paths.jre25 = exact_java.to_string_lossy().into_owned();
                        }
                    }
                }
            }
        }

        // Fallbacks if not found
        if paths.jre8.is_empty() && Path::new("/usr/bin/java").exists() {
            paths.jre8 = "/usr/bin/java".to_string();
        }
        if paths.jre17.is_empty() && Path::new("/usr/bin/java").exists() {
            paths.jre17 = "/usr/bin/java".to_string();
        }
        if paths.jre21.is_empty() && Path::new("/usr/bin/java").exists() {
            paths.jre21 = "/usr/bin/java".to_string();
        }
        if paths.jre25.is_empty() && Path::new("/usr/bin/java").exists() {
            paths.jre25 = "/usr/bin/java".to_string();
        }
    }

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
