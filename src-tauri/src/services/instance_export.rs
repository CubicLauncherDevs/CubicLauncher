//! Exportación de instancias de CubicLauncher a un ZIP compatible con
//! MultiMC / PolyMC / Prism Launcher.
//!
//! El archivo generado contiene:
//! - `instance.cfg`        metadatos de la instancia (nombre, memoria, icono).
//! - `mmc-pack.json`       componentes de Minecraft + loader.
//! - `.minecraft/`         directorio de juego (mods, saves, resourcepacks, etc.).
//! - `icon.png`            icono personalizado de la instancia, si existe.
//! - `cubic-manifest.json` metadatos extra propios de CubicLauncher.

use crate::services::InstOverrides;
use crate::services::instance_manager::InstanceHandle;
use serde::Serialize;
use std::io::Write;
use std::path::{Path, PathBuf};
use tracing::{info, warn};
use zellkern::{GameVersion, Loader};

/// Carpetas/archivos del directorio de juego que se incluyen en el ZIP.
const GAME_DATA_ENTRIES: &[&str] = &[
    "mods",
    "resourcepacks",
    "shaderpacks",
    "saves",
    "screenshots",
    "config",
    "scripts",
    "defaultconfigs",
    "kubejs",
    "data",
    "options.txt",
    "optionsof.txt",
    "servers.dat",
];

/// Datos recolectados de la instancia antes de comprimir en un hilo aparte.
pub struct ExportInput {
    pub uuid: String,
    pub name: String,
    pub version_id: String,
    pub mc_version: String,
    pub loader_name: String,
    pub loader_version: Option<String>,
    pub loader_mmc_uid: Option<&'static str>,
    pub instance_dir: PathBuf,
    pub min_memory: u32,
    pub max_memory: u32,
    pub overrides: Option<InstOverrides>,
    pub icon_src: Option<PathBuf>,
}

/// Prepara la información necesaria para exportar una instancia.
pub async fn prepare_export(handle: &InstanceHandle) -> Result<ExportInput, String> {
    let name = handle.get_name().await.to_string();
    let version_id = handle.get_version().await.to_string();
    let instance_dir = handle.get_instance_dir().await;
    let min_memory = handle.get_min_memory().await;
    let max_memory = handle.get_max_memory().await;
    let overrides = handle.get_overrides().await;
    let uuid = handle.uuid.to_string();

    let icon_src = handle.get_icon_absolute().await.and_then(|abs| {
        let path = PathBuf::from(abs.as_ref());
        path.is_file().then_some(path)
    });

    let game_version = GameVersion::from_version_id(&version_id);
    let loader_name = game_version.loader.name().to_string();
    let loader_version = game_version.loader.version().map(|s| s.to_string());
    let loader_mmc_uid = loader_to_mmc_uid(&game_version.loader);
    let mc_version = game_version.mc_version;

    Ok(ExportInput {
        uuid,
        name,
        version_id,
        mc_version,
        loader_name,
        loader_version,
        loader_mmc_uid,
        instance_dir,
        min_memory,
        max_memory,
        overrides,
        icon_src,
    })
}

/// Genera el ZIP en la ruta indicada.
pub fn export_to_zip(input: &ExportInput, dest: &Path) -> Result<PathBuf, String> {
    info!(
        "Exportando instancia '{}' a '{}'",
        input.name,
        dest.display()
    );

    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("No se pudo crear el directorio destino: {e}"))?;
    }

    let file =
        std::fs::File::create(dest).map_err(|e| format!("No se pudo crear el archivo ZIP: {e}"))?;

    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    let manifest = build_cubic_manifest(input);
    write_zip_file(
        &mut zip,
        "cubic-manifest.json",
        manifest.as_bytes(),
        options,
    )?;

    let cfg = build_instance_cfg(input);
    write_zip_file(&mut zip, "instance.cfg", cfg.as_bytes(), options)?;

    let mmc_pack = build_mmc_pack(input);
    write_zip_file(&mut zip, "mmc-pack.json", mmc_pack.as_bytes(), options)?;

    if let Some(icon) = &input.icon_src {
        let data =
            std::fs::read(icon).map_err(|e| format!("No se pudo leer el icono {:?}: {e}", icon))?;
        write_zip_file(&mut zip, "icon.png", &data, options)?;
    }

    zip.add_directory(".minecraft", options)
        .map_err(|e| format!("No se pudo añadir directorio .minecraft: {e}"))?;

    for entry in GAME_DATA_ENTRIES {
        let src = input.instance_dir.join(entry);
        if !src.exists() {
            continue;
        }
        let zip_path = PathBuf::from(".minecraft").join(entry);
        add_path_to_zip(&mut zip, &src, &zip_path, options)
            .map_err(|e| format!("Error comprimiendo {:?}: {e}", src))?;
    }

    zip.finish()
        .map_err(|e| format!("Error finalizando ZIP: {e}"))?;

    info!("Instancia exportada exitosamente a '{}'", dest.display());
    Ok(dest.to_path_buf())
}

fn write_zip_file(
    zip: &mut zip::ZipWriter<std::fs::File>,
    name: &str,
    data: &[u8],
    options: zip::write::SimpleFileOptions,
) -> Result<(), String> {
    zip.start_file(name, options)
        .map_err(|e| format!("No se pudo iniciar archivo '{name}' en el ZIP: {e}"))?;
    zip.write_all(data)
        .map_err(|e| format!("No se pudo escribir '{name}' en el ZIP: {e}"))?;
    Ok(())
}

fn add_path_to_zip(
    zip: &mut zip::ZipWriter<std::fs::File>,
    src: &Path,
    zip_path: &Path,
    options: zip::write::SimpleFileOptions,
) -> Result<(), std::io::Error> {
    if src.is_dir() {
        zip.add_directory(zip_path.to_string_lossy(), options)?;
        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            add_path_to_zip(
                zip,
                &entry.path(),
                &zip_path.join(entry.file_name()),
                options,
            )?;
        }
    } else {
        zip.start_file(zip_path.to_string_lossy(), options)?;
        let data = std::fs::read(src)?;
        zip.write_all(&data)?;
    }
    Ok(())
}

fn build_instance_cfg(input: &ExportInput) -> String {
    let mut lines = vec!["[General]".to_string(), format!("name={}", input.name)];

    if input.icon_src.is_some() {
        lines.push("iconKey=icon".to_string());
    }

    lines.push(format!("MinMemAlloc={}", input.min_memory));
    lines.push(format!("MaxMemAlloc={}", input.max_memory));

    lines.join("\n") + "\n"
}

#[derive(Serialize)]
struct MmcComponent {
    uid: &'static str,
    version: String,
}

#[derive(Serialize)]
struct MmcPack {
    components: Vec<MmcComponent>,
}

fn build_mmc_pack(input: &ExportInput) -> String {
    let mut components = vec![MmcComponent {
        uid: "net.minecraft",
        version: input.mc_version.clone(),
    }];

    if let Some(uid) = input.loader_mmc_uid {
        if let Some(version) = &input.loader_version {
            components.push(MmcComponent {
                uid,
                version: version.clone(),
            });
        } else {
            warn!(
                "Loader '{}' sin versión para instancia '{}', se exporta como Vanilla",
                input.loader_name, input.name
            );
        }
    }

    serde_json::to_string_pretty(&MmcPack { components }).expect("mmc-pack.json serializable")
}

#[derive(Serialize)]
struct CubicManifest {
    format_version: u8,
    exported_by: &'static str,
    uuid: String,
    name: String,
    version_id: String,
    mc_version: String,
    loader: String,
    loader_version: Option<String>,
    min_memory: u32,
    max_memory: u32,
    overrides: Option<InstOverrides>,
}

fn build_cubic_manifest(input: &ExportInput) -> String {
    let manifest = CubicManifest {
        format_version: 1,
        exported_by: "CubicLauncher",
        uuid: input.uuid.clone(),
        name: input.name.clone(),
        version_id: input.version_id.clone(),
        mc_version: input.mc_version.clone(),
        loader: input.loader_name.clone(),
        loader_version: input.loader_version.clone(),
        min_memory: input.min_memory,
        max_memory: input.max_memory,
        overrides: input.overrides,
    };

    serde_json::to_string_pretty(&manifest).expect("manifest serializable")
}

fn loader_to_mmc_uid(loader: &Loader) -> Option<&'static str> {
    match loader {
        Loader::Vanilla => None,
        Loader::Fabric(_) => Some("net.fabricmc.fabric-loader"),
        Loader::Forge(_) => Some("net.minecraftforge"),
        Loader::NeoForge(_) => Some("net.neoforged"),
        Loader::Quilt(_) => Some("org.quiltmc.quilt-loader"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loader_to_mmc_uid() {
        assert_eq!(loader_to_mmc_uid(&Loader::Vanilla), None);
        assert_eq!(
            loader_to_mmc_uid(&Loader::Fabric("0.15.0".into())),
            Some("net.fabricmc.fabric-loader")
        );
        assert_eq!(
            loader_to_mmc_uid(&Loader::NeoForge("21.0.0".into())),
            Some("net.neoforged")
        );
    }

    #[test]
    fn test_build_mmc_pack_json() {
        let input = ExportInput {
            uuid: "uuid".into(),
            name: "Test".into(),
            version_id: "1.21-fabric-0.15.0".into(),
            mc_version: "1.21".into(),
            loader_name: "Fabric".into(),
            loader_version: Some("0.15.0".into()),
            loader_mmc_uid: loader_to_mmc_uid(&Loader::Fabric("0.15.0".into())),
            instance_dir: PathBuf::new(),
            min_memory: 512,
            max_memory: 2048,
            overrides: None,
            icon_src: None,
        };
        let json = build_mmc_pack(&input);
        assert!(json.contains("net.minecraft"));
        assert!(json.contains("net.fabricmc.fabric-loader"));
        assert!(json.contains("0.15.0"));
    }

    #[test]
    fn test_build_instance_cfg() {
        let input = ExportInput {
            uuid: "uuid".into(),
            name: "MiInstancia".into(),
            version_id: "1.20.1".into(),
            mc_version: "1.20.1".into(),
            loader_name: "Vanilla".into(),
            loader_version: None,
            loader_mmc_uid: None,
            instance_dir: PathBuf::new(),
            min_memory: 1024,
            max_memory: 4096,
            overrides: None,
            icon_src: None,
        };
        let cfg = build_instance_cfg(&input);
        assert!(cfg.contains("name=MiInstancia"));
        assert!(cfg.contains("MinMemAlloc=1024"));
        assert!(cfg.contains("MaxMemAlloc=4096"));
        assert!(!cfg.contains("iconKey"));
    }
}
