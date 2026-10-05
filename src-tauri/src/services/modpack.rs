//! Authoritative pack provenance. This is persistent instance data, not a mod cache.
use super::curseforge_api::{CurseForgeClient, curseforge_cdn_url};
use crate::commands::instance::mods::ModDto;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};

pub const STATE_FILE: &str = "modpack.cub.json";
pub type Result<T> = std::result::Result<T, String>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackFile {
    pub sha1: String,
    #[serde(default)]
    pub downloads: Vec<String>,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackState {
    pub schema_version: u32,
    pub source: String,
    pub project_id: Option<String>,
    pub version_id: Option<String>,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    pub version: String,
    pub game_version: String,
    pub locked: bool,
    pub files: BTreeMap<String, PackFile>,
    #[serde(default)]
    pub identities: BTreeMap<String, std::collections::BTreeSet<String>>,
    /// Legacy instances need an exact archive before individual file ownership is known.
    #[serde(default)]
    pub needs_inventory: bool,
}

pub fn valid_path(relative: &str) -> Result<()> {
    if relative.is_empty()
        || relative.contains(['\\', ':'])
        || relative.chars().any(char::is_control)
        || relative
            .split('/')
            .any(|p| p.is_empty() || p == "." || p == "..")
        || !Path::new(relative)
            .components()
            .all(|p| matches!(p, Component::Normal(_)))
    {
        return Err(format!("Ruta de modpack inválida: {relative}"));
    }
    if relative.split('/').any(|p| {
        let p = p.to_ascii_lowercase();
        p.ends_with(".crep")
            || p.starts_with(".modpack-")
            || matches!(
                p.as_str(),
                "instance.cub"
                    | "upstream.json"
                    | "modpack.cub.json"
                    | ".git"
                    | "cubic-jar"
                    | "cubic-jar-cache"
            )
    }) {
        return Err(format!(
            "El modpack contiene metadatos reservados: {relative}"
        ));
    }
    Ok(())
}

/// Also reject linked ancestors, including for destinations that do not exist yet.
pub fn checked_path(root: &Path, relative: &str) -> Result<PathBuf> {
    valid_path(relative)?;
    let mut path = root.to_path_buf();
    for part in relative.split('/') {
        path.push(part);
        match fs::symlink_metadata(&path) {
            Ok(m) if m.file_type().is_symlink() => {
                return Err(format!("Enlace no permitido: {relative}"));
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(path)
}

pub fn load(root: &Path) -> Result<Option<PackState>> {
    let path = root.join(STATE_FILE);
    match fs::read(&path) {
        Ok(bytes) => {
            let state: PackState = serde_json::from_slice(&bytes)
                .map_err(|e| format!("No se puede leer el registro del modpack: {e}"))?;
            if state.schema_version != 1 {
                return Err("Versión de registro de modpack no compatible".into());
            }
            for relative in state.files.keys() {
                valid_path(relative)?;
            }
            Ok(Some(state))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => legacy(root),
        Err(e) => Err(e.to_string()),
    }
}

fn legacy(root: &Path) -> Result<Option<PackState>> {
    let bytes = match fs::read(root.join("upstream.json")) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.to_string()),
    };
    let upstream: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    let source = match upstream["type"].as_str() {
        Some("modrinth-modpack") => "modrinth",
        Some("curseforge-modpack") => "curseforge",
        _ => return Ok(None),
    };
    let id = |key: &str| {
        upstream.get(key).and_then(|v| {
            v.as_str()
                .map(str::to_owned)
                .or_else(|| v.as_u64().map(|v| v.to_string()))
        })
    };
    Ok(Some(PackState {
        schema_version: 1,
        source: source.into(),
        project_id: id("projectId"),
        version_id: id(if source == "modrinth" {
            "versionId"
        } else {
            "fileId"
        }),
        name: root
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
        version: String::new(),
        author: None,
        game_version: String::new(),
        locked: true,
        files: BTreeMap::new(),
        identities: BTreeMap::new(),
        needs_inventory: true,
    }))
}

pub fn save(root: &Path, state: &PackState) -> Result<()> {
    let mut temp = tempfile::NamedTempFile::new_in(root).map_err(|e| e.to_string())?;
    serde_json::to_writer_pretty(&mut temp, state).map_err(|e| e.to_string())?;
    temp.flush().map_err(|e| e.to_string())?;
    temp.as_file().sync_all().map_err(|e| e.to_string())?;
    temp.persist(root.join(STATE_FILE))
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn read_state(root: &Path) -> Result<Option<PackState>> {
    let root = root.to_path_buf();
    tokio::task::spawn_blocking(move || load(&root))
        .await
        .map_err(|e| e.to_string())?
}

pub async fn validate_archive_paths(path: &Path) -> Result<()> {
    let path = path.to_path_buf();
    tokio::task::spawn_blocking(move || archive_data(&path).map(|_| ()))
        .await
        .map_err(|e| e.to_string())?
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct IndexFile {
    path: String,
    hashes: BTreeMap<String, String>,
    downloads: Vec<String>,
    file_size: u64,
    env: Option<BTreeMap<String, String>>,
}

struct ArchiveData {
    state: PackState,
    curseforge_files: Vec<(u32, u32)>,
    overrides: BTreeMap<String, PackFile>,
}

fn archive_data(path: &Path) -> Result<ArchiveData> {
    let mut archive = zip::ZipArchive::new(File::open(path).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let mrpack = archive.file_names().any(|n| n == "modrinth.index.json");
    let mut files = BTreeMap::new();
    let mut curseforge_files = Vec::new();
    let (name, version, author, game_version, overrides_prefix) = if mrpack {
        let metadata = cubrinth::mrpack::parse_mrpack(path).map_err(|e| e.to_string())?;
        let json: serde_json::Value = serde_json::from_reader(
            archive
                .by_name("modrinth.index.json")
                .map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        let entries: Vec<IndexFile> =
            serde_json::from_value(json["files"].clone()).map_err(|e| e.to_string())?;
        for entry in entries {
            valid_path(&entry.path)?;
            if entry
                .env
                .as_ref()
                .and_then(|e| e.get("client"))
                .is_some_and(|e| e == "unsupported")
            {
                continue;
            }
            if entry.downloads.is_empty() {
                return Err(format!("Archivo sin descarga: {}", entry.path));
            }
            files.insert(
                entry.path,
                PackFile {
                    sha1: entry.hashes.get("sha1").cloned().unwrap_or_default(),
                    downloads: entry.downloads,
                    size: entry.file_size,
                },
            );
        }
        (
            metadata.name,
            metadata.version_id,
            metadata.author,
            metadata.game_version,
            "overrides/".to_string(),
        )
    } else {
        let metadata =
            super::curseforge_modpack::parse_curseforge_modpack(path).map_err(|e| e.to_string())?;
        let manifest: super::curseforge_modpack::CurseForgeModpackManifest =
            serde_json::from_reader(
                archive
                    .by_name("manifest.json")
                    .map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
        valid_path(&manifest.overrides)?;
        curseforge_files = manifest
            .files
            .into_iter()
            .filter(|f| f.required)
            .map(|f| (f.project_id, f.file_id))
            .collect();
        (
            metadata.name,
            metadata.version,
            Some(manifest.author).filter(|author| !author.trim().is_empty()),
            metadata.game_version,
            format!("{}/", manifest.overrides),
        )
    };
    let game_version = game_version
        .ok_or("El pack no indica una versión de Minecraft")?
        .to_version_id();
    let mut overrides = BTreeMap::new();
    // Client overrides always win, regardless of archive entry order.
    for prefix in [
        overrides_prefix.as_str(),
        if mrpack { "client-overrides/" } else { "" },
    ] {
        if prefix.is_empty() {
            continue;
        }
        for i in 0..archive.len() {
            let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
            let Some(relative) = entry.name().strip_prefix(prefix).map(str::to_owned) else {
                continue;
            };
            if entry.is_dir() {
                continue;
            }
            valid_path(&relative)?;
            let mut hash = Sha1::new();
            let mut buffer = [0u8; 65536];
            let mut size = 0;
            loop {
                let n = entry.read(&mut buffer).map_err(|e| e.to_string())?;
                if n == 0 {
                    break;
                }
                hash.update(&buffer[..n]);
                size += n as u64;
            }
            overrides.insert(
                relative,
                PackFile {
                    sha1: hash.finalize().iter().map(|b| format!("{b:02x}")).collect(),
                    downloads: vec![],
                    size,
                },
            );
        }
    }
    Ok(ArchiveData {
        state: PackState {
            schema_version: 1,
            source: "local".into(),
            project_id: None,
            version_id: None,
            name,
            author,
            version,
            game_version,
            locked: true,
            files,
            identities: BTreeMap::new(),
            needs_inventory: false,
        },
        curseforge_files,
        overrides,
    })
}

pub async fn inspect_archive(path: &Path) -> Result<PackState> {
    let path = path.to_path_buf();
    let mut data = tokio::task::spawn_blocking(move || archive_data(&path))
        .await
        .map_err(|e| e.to_string())??;
    if !data.curseforge_files.is_empty() {
        let client = CurseForgeClient::from_settings_or_default();
        let ids: Vec<u32> = data.curseforge_files.iter().map(|(_, id)| *id).collect();
        let mut projects: Vec<u32> = data.curseforge_files.iter().map(|(id, _)| *id).collect();
        projects.sort_unstable();
        projects.dedup();
        let files = client
            .get_mod_files(&ids)
            .await
            .map_err(|e| e.to_string())?;
        let projects = client
            .get_projects(&projects)
            .await
            .map_err(|e| e.to_string())?;
        for (project, file_id) in &data.curseforge_files {
            let file = files
                .iter()
                .find(|f| f.id == *file_id)
                .ok_or_else(|| format!("Archivo CurseForge no disponible: {file_id}"))?;
            let class = projects
                .iter()
                .find(|p| p.id == *project)
                .and_then(|p| p.class_id);
            let folder = match class {
                Some(12) => "resourcepacks",
                Some(6552) => "shaderpacks",
                _ => "mods",
            };
            let relative = format!("{folder}/{}", file.file_name);
            valid_path(&relative)?;
            data.state.files.insert(
                relative,
                PackFile {
                    sha1: file
                        .hashes
                        .iter()
                        .find(|h| h.algo == 1)
                        .map(|h| h.value.clone())
                        .unwrap_or_default(),
                    downloads: vec![
                        file.download_url
                            .clone()
                            .unwrap_or_else(|| curseforge_cdn_url(file.id, &file.file_name)),
                    ],
                    size: file.file_length,
                },
            );
        }
    }
    data.state.files.extend(data.overrides);
    Ok(data.state)
}

pub fn verify_files(root: &Path, state: &mut PackState) -> Result<()> {
    for (relative, file) in &mut state.files {
        let path = checked_path(root, relative)?;
        let hash = super::compute_file_sha1(&path)?;
        let size = fs::metadata(&path).map_err(|e| e.to_string())?.len();
        if (!file.sha1.is_empty() && !file.sha1.eq_ignore_ascii_case(&hash)) || file.size != size {
            return Err(format!(
                "El archivo del pack no coincide con su manifiesto: {relative}"
            ));
        }
        file.sha1 = hash;
        if relative.starts_with("mods/") {
            state
                .identities
                .insert(relative.clone(), super::modpack_update::mod_ids(&path));
        }
    }
    Ok(())
}

pub async fn record_install(
    root: &Path,
    archive: &Path,
    source: &str,
    project_id: Option<String>,
    version_id: Option<String>,
) -> Result<()> {
    let mut state = inspect_archive(archive).await?;
    state.source = source.into();
    state.project_id = project_id;
    state.version_id = version_id;
    let root = root.to_path_buf();
    tokio::task::spawn_blocking(move || {
        verify_files(&root, &mut state)?;
        save(&root, &state)
    })
    .await
    .map_err(|e| e.to_string())?
}

fn owned<'a>(state: &'a PackState, relative: &str) -> Option<&'a PackFile> {
    let canonical = relative.strip_suffix(".disabled").unwrap_or(relative);
    state
        .files
        .iter()
        .find(|(path, _)| {
            path.strip_suffix(".disabled")
                .unwrap_or(path)
                .eq_ignore_ascii_case(canonical)
        })
        .map(|(_, file)| file)
}

pub async fn ensure_paths_mutable(root: &Path, relative_paths: &[String]) -> Result<()> {
    let root = root.to_path_buf();
    let paths = relative_paths.to_vec();
    tokio::task::spawn_blocking(move || {
        let state = load(&root)?;
        for relative in paths {
            checked_path(&root, &relative)?;
            if let Some(state) = &state
                && state.locked
                && (state.needs_inventory || owned(state, &relative).is_some())
            {
                return Err(format!(
                    "Este archivo es provisto por: {}. Desbloqueá el modpack para modificarlo.",
                    state.name
                ));
            }
        }
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

pub async fn annotate_mods(root: &Path, subdir: &str, mods: &mut [ModDto]) -> Result<()> {
    let Some(state) = read_state(root).await? else {
        return Ok(());
    };
    for item in mods {
        let relative = format!("{subdir}/{}", item.filename);
        if let Some(file) = owned(&state, &relative) {
            item.pack_name = Some(state.name.clone());
            item.pack_locked = state.locked;
            item.pack_modified =
                !file.sha1.eq_ignore_ascii_case(&item.sha1) || item.filename.ends_with(".disabled");
        } else if state.needs_inventory {
            // Unknown ownership stays visible; don't pretend legacy files are user additions.
            item.pack_locked = state.locked;
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "../tests/services/modpack.rs"]
mod tests;
