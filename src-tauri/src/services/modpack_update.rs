//! Pack upgrades are staged first, compared against the original inventory, and journaled.
use super::modpack::{self, PackState, Result, checked_path};
use super::{InstanceHandle, compute_file_sha1};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

pub const JOURNAL_DIR: &str = ".modpack-transaction";
const PREVIEW_TTL: Duration = Duration::from_secs(1800);

#[derive(Clone, Serialize)]
pub struct PackVersion {
    pub id: String,
    pub name: String,
    pub version: String,
}

#[derive(Clone, Serialize)]
pub struct Preview {
    pub token: String,
    pub name: String,
    pub version: String,
    pub game_version: String,
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub changed: Vec<String>,
    pub conflicts: Vec<String>,
}

struct Prepared {
    id: String,
    root: PathBuf,
    stage: tempfile::TempDir,
    old: PackState,
    next: PackState,
    preview: Preview,
    snapshot: BTreeMap<String, Option<String>>,
    collisions: BTreeMap<String, Vec<String>>,
    replacements: BTreeMap<String, Vec<String>>,
    runtime: String,
    created: Instant,
}

static PREVIEWS: LazyLock<Mutex<HashMap<String, Prepared>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent(concat!("CubicLauncher/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|e| e.to_string())
}

fn project(state: &PackState) -> Result<&str> {
    state
        .project_id
        .as_deref()
        .ok_or_else(|| "Este pack no tiene un proyecto remoto asociado".into())
}

pub async fn versions(state: &PackState) -> Result<Vec<PackVersion>> {
    let project = project(state)?;
    if state.source == "modrinth" {
        let response: Vec<serde_json::Value> = client()?
            .get(format!(
                "https://api.modrinth.com/v2/project/{}/version",
                urlencoding::encode(project)
            ))
            .send()
            .await
            .map_err(|e| e.to_string())?
            .error_for_status()
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())?;
        response
            .into_iter()
            .map(|v| {
                Ok(PackVersion {
                    id: string(&v, "id")?,
                    name: string(&v, "name")?,
                    version: string(&v, "version_number")?,
                })
            })
            .collect()
    } else if state.source == "curseforge" {
        let response = super::curseforge_api::CurseForgeClient::from_settings_or_default()
            .get_project_files(
                project
                    .parse()
                    .map_err(|_| "Proyecto CurseForge inválido")?,
                None,
                None,
            )
            .await
            .map_err(|e| e.to_string())?;
        Ok(response
            .data
            .into_iter()
            .filter(|f| f.is_available && !f.is_server_pack)
            .map(|f| PackVersion {
                id: f.id.to_string(),
                name: f.display_name.clone(),
                version: f.display_name,
            })
            .collect())
    } else {
        Err("Seleccioná un archivo local para actualizar este pack".into())
    }
}

fn string(value: &serde_json::Value, key: &str) -> Result<String> {
    value[key]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| format!("La respuesta no contiene {key}"))
}

async fn download_archive(state: &PackState, version: &str, destination: &Path) -> Result<()> {
    let project = project(state)?;
    let (url, hash) = if state.source == "modrinth" {
        let value: serde_json::Value = client()?
            .get(format!(
                "https://api.modrinth.com/v2/version/{}",
                urlencoding::encode(version)
            ))
            .send()
            .await
            .map_err(|e| e.to_string())?
            .error_for_status()
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())?;
        if value["project_id"].as_str() != Some(project) {
            return Err("La versión no pertenece a este modpack".into());
        }
        let files = value["files"].as_array().ok_or("Versión sin archivos")?;
        let file = files
            .iter()
            .find(|f| f["primary"].as_bool() == Some(true))
            .or_else(|| files.first())
            .ok_or("Versión sin archivos")?;
        (
            string(file, "url")?,
            file["hashes"]["sha1"].as_str().map(str::to_owned),
        )
    } else if state.source == "curseforge" {
        let file_id: u32 = version.parse().map_err(|_| "Versión CurseForge inválida")?;
        let files = super::curseforge_api::CurseForgeClient::from_settings_or_default()
            .get_mod_files(&[file_id])
            .await
            .map_err(|e| e.to_string())?;
        let file = files.first().ok_or("Versión no disponible")?;
        if file.mod_id.map(|id| id.to_string()).as_deref() != Some(project) || file.is_server_pack {
            return Err("La versión no pertenece al modpack de cliente seleccionado".into());
        }
        (
            file.download_url.clone().unwrap_or_else(|| {
                super::curseforge_api::curseforge_cdn_url(file.id, &file.file_name)
            }),
            file.hashes
                .iter()
                .find(|h| h.algo == 1)
                .map(|h| h.value.clone()),
        )
    } else {
        return Err("Pack sin origen remoto".into());
    };
    let mut response = client()?
        .get(&url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?;
    let mut target = tokio::fs::File::create(destination)
        .await
        .map_err(|e| e.to_string())?;
    use tokio::io::AsyncWriteExt;
    while let Some(bytes) = response.chunk().await.map_err(|e| e.to_string())? {
        target.write_all(&bytes).await.map_err(|e| e.to_string())?;
    }
    target.flush().await.map_err(|e| e.to_string())?;
    drop(target);
    if let Some(hash) = hash {
        let path = destination.to_path_buf();
        let actual = tokio::task::spawn_blocking(move || compute_file_sha1(&path))
            .await
            .map_err(|e| e.to_string())??;
        if !hash.eq_ignore_ascii_case(&actual) {
            return Err("El archivo descargado no coincide con su hash".into());
        }
    }
    Ok(())
}

async fn stage_archive(archive: &Path, destination: &Path) -> Result<PackState> {
    // Validate all paths, including overrides, before the installer writes anything.
    let mut state = modpack::inspect_archive(archive).await?;
    let archive_path = archive.to_path_buf();
    let mrpack = tokio::task::spawn_blocking(move || -> Result<bool> {
        let archive = zip::ZipArchive::new(File::open(archive_path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        Ok(archive.file_names().any(|n| n == "modrinth.index.json"))
    })
    .await
    .map_err(|e| e.to_string())??;
    let shared = crate::core::PathManager::get().get_shared_dir();
    if mrpack {
        cubrinth::mrpack::install_mrpack(archive, destination, &shared, None)
            .await
            .map_err(|e| e.to_string())?;
    } else {
        super::curseforge_modpack::install_curseforge_modpack(archive, destination, &shared, None)
            .await
            .map_err(|e| e.to_string())?;
    }
    let root = destination.to_path_buf();
    tokio::task::spawn_blocking(move || {
        modpack::verify_files(&root, &mut state)?;
        Ok(state)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Legacy inventories are reconstructed from the exact published archive, never from
/// the instance's current directory (which already contains the user's additions).
pub async fn restore_legacy(root: &Path, state: PackState) -> Result<PackState> {
    if !state.needs_inventory {
        return Ok(state);
    }
    let version = state
        .version_id
        .as_deref()
        .ok_or("No se conoce la versión original del pack")?;
    let temp = tempfile::Builder::new()
        .prefix(".modpack-legacy-")
        .tempdir()
        .map_err(|e| e.to_string())?;
    let archive = temp.path().join("pack.zip");
    download_archive(&state, version, &archive).await?;
    let files = temp.path().join("files");
    tokio::fs::create_dir(&files)
        .await
        .map_err(|e| e.to_string())?;
    let mut restored = stage_archive(&archive, &files).await?;
    restored.source = state.source;
    restored.project_id = state.project_id;
    restored.version_id = state.version_id;
    restored.locked = state.locked;
    // Hashless historical entries cannot safely support destructive updates.
    if restored.files.values().any(|f| f.sha1.is_empty()) {
        return Err(
            "El manifiesto original no contiene hashes suficientes para reconstruir el pack".into(),
        );
    }
    modpack::save(root, &restored)?;
    Ok(restored)
}

pub async fn prepare(
    handle: &InstanceHandle,
    version_id: Option<String>,
    local_path: Option<String>,
) -> Result<Preview> {
    let root = handle.get_instance_dir().await;
    let old = modpack::read_state(&root)
        .await?
        .ok_or("La instancia no está basada en un modpack")?;
    let old = restore_legacy(&root, old).await?;
    let stage = tempfile::Builder::new()
        .prefix(".modpack-preview-")
        .tempdir()
        .map_err(|e| e.to_string())?;
    let archive = stage.path().join("pack.zip");
    let remote = match (version_id.as_deref(), local_path) {
        (Some(version), None) => {
            download_archive(&old, version, &archive).await?;
            true
        }
        (None, Some(path)) => {
            tokio::fs::copy(path, &archive)
                .await
                .map_err(|e| e.to_string())?;
            false
        }
        _ => return Err("Elegí una versión publicada o un archivo local".into()),
    };
    let files_dir = stage.path().join("files");
    tokio::fs::create_dir(&files_dir)
        .await
        .map_err(|e| e.to_string())?;
    let mut next = stage_archive(&archive, &files_dir).await?;
    if remote {
        next.source = old.source.clone();
        next.project_id = old.project_id.clone();
        next.version_id = version_id;
    }
    next.locked = old.locked;
    let runtime = handle.get_version().await.to_string();
    let id = handle.uuid.to_string();
    let prepared =
        tokio::task::spawn_blocking(move || build_preview(id, root, stage, old, next, runtime))
            .await
            .map_err(|e| e.to_string())??;
    let preview = prepared.preview.clone();
    {
        let mut previews = PREVIEWS.lock().map_err(|e| e.to_string())?;
        previews.retain(|_, p| p.created.elapsed() < PREVIEW_TTL && p.id != prepared.id);
        if previews.len() >= 8 {
            return Err("Hay demasiadas vistas previas abiertas".into());
        }
        previews.insert(preview.token.clone(), prepared);
    }
    let token = preview.token.clone();
    tokio::spawn(async move {
        tokio::time::sleep(PREVIEW_TTL).await;
        if let Ok(mut previews) = PREVIEWS.lock() {
            previews.remove(&token);
        }
    });
    Ok(preview)
}

fn current_hash(root: &Path, relative: &str) -> Result<Option<String>> {
    let path = checked_path(root, relative)?;
    match fs::metadata(&path) {
        Ok(m) if m.is_file() => compute_file_sha1(&path).map(Some),
        Ok(_) => Err(format!("Se esperaba un archivo: {relative}")),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

fn addon_paths(root: &Path) -> Result<Vec<String>> {
    let mut paths = Vec::new();
    for folder in ["mods", "resourcepacks", "shaderpacks"] {
        let path = checked_path(root, folder)?;
        let entries = match fs::read_dir(path) {
            Ok(entries) => entries,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => return Err(e.to_string()),
        };
        for entry in entries {
            let entry = entry.map_err(|e| e.to_string())?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') || name.ends_with(".crep") {
                continue;
            }
            if entry.file_type().map_err(|e| e.to_string())?.is_file() {
                paths.push(format!("{folder}/{name}"));
            }
        }
    }
    Ok(paths)
}

/// IDs in a JAR are stable across filename/version changes. A readable display name
/// is not sufficient to decide whether two user mods are the same project.
pub(super) fn mod_ids(path: &Path) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    let Ok(file) = File::open(path) else {
        return ids;
    };
    let Ok(mut zip) = zip::ZipArchive::new(file) else {
        return ids;
    };
    for name in ["fabric.mod.json", "quilt.mod.json"] {
        if let Ok(entry) = zip.by_name(name)
            && entry.size() < 1024 * 1024
            && let Ok(value) = serde_json::from_reader::<_, serde_json::Value>(entry)
        {
            let id = if name == "fabric.mod.json" {
                value["id"].as_str()
            } else {
                value["quilt_loader"]["id"].as_str()
            };
            if let Some(id) = id {
                ids.insert(id.to_string());
            }
        }
    }
    for name in ["META-INF/mods.toml", "META-INF/neoforge.mods.toml"] {
        use std::io::Read;
        if let Ok(mut entry) = zip.by_name(name)
            && entry.size() < 1024 * 1024
        {
            let mut text = String::new();
            if entry.read_to_string(&mut text).is_ok()
                && let Ok(value) = toml::from_str::<toml::Value>(&text)
                && let Some(mods) = value.get("mods").and_then(|v| v.as_array())
            {
                for m in mods {
                    if let Some(id) = m.get("modId").and_then(|v| v.as_str()) {
                        ids.insert(id.to_string());
                    }
                }
            }
        }
    }
    if let Ok(entry) = zip.by_name("mcmod.info")
        && entry.size() < 1024 * 1024
        && let Ok(value) = serde_json::from_reader::<_, serde_json::Value>(entry)
    {
        let mods: Vec<&serde_json::Value> = value
            .as_array()
            .or_else(|| value.get("modList").and_then(|v| v.as_array()))
            .map(|items| items.iter().collect())
            .unwrap_or_else(|| vec![&value]);
        for item in mods {
            if let Some(id) = item.get("modid").and_then(|v| v.as_str()) {
                ids.insert(id.to_string());
            }
        }
    }
    ids
}

fn snapshot(
    root: &Path,
    old: &PackState,
    next: &PackState,
) -> Result<BTreeMap<String, Option<String>>> {
    let mut paths: BTreeSet<String> = old.files.keys().chain(next.files.keys()).cloned().collect();
    for p in paths.clone() {
        if p.starts_with("mods/") {
            paths.insert(
                p.strip_suffix(".disabled")
                    .map(str::to_owned)
                    .unwrap_or_else(|| format!("{p}.disabled")),
            );
        }
    }
    paths.extend(addon_paths(root)?);
    paths
        .into_iter()
        .map(|p| Ok((p.clone(), current_hash(root, &p)?)))
        .collect()
}

fn build_preview(
    id: String,
    root: PathBuf,
    stage: tempfile::TempDir,
    old: PackState,
    next: PackState,
    runtime: String,
) -> Result<Prepared> {
    let snapshot = snapshot(&root, &old, &next)?;
    let mut preview = Preview {
        token: uuid::Uuid::new_v4().to_string(),
        name: next.name.clone(),
        version: next.version.clone(),
        game_version: next.game_version.clone(),
        added: vec![],
        removed: vec![],
        changed: vec![],
        conflicts: vec![],
    };
    let mut collisions = BTreeMap::new();
    // Reject portable-path aliases instead of letting install/delete operations
    // address the same physical file on case-insensitive filesystems.
    let mut portable = BTreeMap::<String, String>::new();
    for path in old
        .files
        .keys()
        .chain(next.files.keys())
        .chain(snapshot.keys())
    {
        let key = path.to_lowercase();
        if let Some(previous) = portable.insert(key, path.clone())
            && previous != *path
        {
            return Err(format!(
                "Rutas del pack que sólo difieren en mayúsculas: {previous} / {path}"
            ));
        }
    }
    let mut replacements = BTreeMap::new();
    for (path, file) in &next.files {
        match old.files.get(path) {
            None => preview.added.push(path.clone()),
            Some(old) if old.sha1 != file.sha1 => preview.changed.push(path.clone()),
            _ => {}
        }
    }
    preview.removed = old
        .files
        .keys()
        .filter(|p| !next.files.contains_key(*p))
        .cloned()
        .collect();
    for removed in &preview.removed {
        let old_ids = old.identities.get(removed).cloned().unwrap_or_else(|| {
            let path = root.join(removed);
            let disabled = root.join(format!("{removed}.disabled"));
            mod_ids(if path.exists() { &path } else { &disabled })
        });
        if old_ids.is_empty() {
            continue;
        }
        for added in &preview.added {
            let new_ids = next
                .identities
                .get(added)
                .cloned()
                .unwrap_or_else(|| mod_ids(&stage.path().join("files").join(added)));
            if !old_ids.is_disjoint(&new_ids) {
                replacements
                    .entry(removed.clone())
                    .or_insert_with(Vec::new)
                    .push(added.clone());
            }
        }
    }
    let changes: Vec<_> = preview
        .added
        .iter()
        .chain(&preview.changed)
        .chain(&preview.removed)
        .cloned()
        .collect();
    for path in &changes {
        let current = snapshot.get(path).and_then(|s| s.as_deref());
        let original = old.files.get(path).map(|f| f.sha1.as_str());
        if current != original {
            preview.conflicts.push(path.clone());
        }
        let disabled = format!("{path}.disabled");
        if snapshot.get(&disabled).is_some_and(|s| s.is_some()) {
            collisions
                .entry(path.clone())
                .or_insert_with(Vec::new)
                .push(disabled);
            preview.conflicts.push(path.clone());
        }
    }
    let user_mods: Vec<_> = addon_paths(&root)?
        .into_iter()
        .filter(|p| {
            p.starts_with("mods/")
                && !old.files.contains_key(p)
                && !old
                    .files
                    .contains_key(p.strip_suffix(".disabled").unwrap_or(p))
        })
        .map(|p| {
            let ids = mod_ids(&root.join(&p));
            (p, ids)
        })
        .collect();
    for path in preview
        .added
        .iter()
        .chain(&preview.changed)
        .filter(|p| p.starts_with("mods/"))
    {
        let ids = mod_ids(&stage.path().join("files").join(path));
        for (user, user_ids) in &user_mods {
            if user != path && !ids.is_disjoint(user_ids) {
                collisions
                    .entry(path.clone())
                    .or_insert_with(Vec::new)
                    .push(user.clone());
                preview.conflicts.push(path.clone());
            }
        }
    }
    preview.conflicts.sort();
    preview.conflicts.dedup();
    Ok(Prepared {
        id,
        root,
        stage,
        old,
        next,
        preview,
        snapshot,
        collisions,
        replacements,
        runtime,
        created: Instant::now(),
    })
}

pub fn cancel(id: &str, token: &str) -> Result<()> {
    let mut previews = PREVIEWS.lock().map_err(|e| e.to_string())?;
    if previews.get(token).is_some_and(|p| p.id == id) {
        previews.remove(token);
    }
    Ok(())
}

#[derive(Serialize, Deserialize)]
struct Journal {
    paths: BTreeMap<String, bool>,
    committed: bool,
}

fn persist_json(path: &Path, value: &impl Serialize) -> Result<()> {
    use std::io::Write;
    let parent = path.parent().ok_or("Ruta inválida")?;
    let mut temp = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    serde_json::to_writer(&mut temp, value).map_err(|e| e.to_string())?;
    temp.flush().map_err(|e| e.to_string())?;
    temp.as_file().sync_all().map_err(|e| e.to_string())?;
    temp.persist(path).map_err(|e| e.to_string())?;
    sync_directory(parent)?;
    Ok(())
}

fn sync_directory(path: &Path) -> Result<()> {
    #[cfg(unix)]
    File::open(path)
        .and_then(|file| file.sync_all())
        .map_err(|e| e.to_string())?;
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

fn sync_backup_directories(path: &Path) -> Result<()> {
    for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        if entry.file_type().map_err(|e| e.to_string())?.is_dir() {
            sync_backup_directories(&entry.path())?;
        }
    }
    sync_directory(path)
}

fn atomic_copy(source: &Path, destination: &Path) -> Result<()> {
    let parent = destination.parent().ok_or("Ruta inválida")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let mut temp = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    std::io::copy(
        &mut File::open(source).map_err(|e| e.to_string())?,
        &mut temp,
    )
    .map_err(|e| e.to_string())?;
    temp.as_file().sync_all().map_err(|e| e.to_string())?;
    temp.persist(destination).map_err(|e| e.to_string())?;
    sync_directory(parent)?;
    Ok(())
}

fn journal_path(root: &Path, path: &str) -> Result<PathBuf> {
    if matches!(path, modpack::STATE_FILE | "instance.cub") {
        Ok(root.join(path))
    } else {
        checked_path(root, path)
    }
}

/// Called on startup before loading instance.cub, and on any failed upgrade.
pub fn recover(root: &Path) -> Result<()> {
    let transaction = root.join(JOURNAL_DIR);
    if !transaction.exists() {
        return Ok(());
    }
    let journal_path = transaction.join("journal.json");
    if !journal_path.exists() {
        // No mutations occur before the complete backup journal is durable.
        return fs::remove_dir_all(transaction).map_err(|e| e.to_string());
    }
    let journal: Journal =
        serde_json::from_reader(File::open(&journal_path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    if !journal.committed {
        for (relative, existed) in journal.paths {
            let target = journal_path_for_recovery(root, &relative)?;
            if existed {
                atomic_copy(&transaction.join("backup").join(&relative), &target)?;
            } else if target.exists() {
                fs::remove_file(&target).map_err(|e| e.to_string())?;
                sync_directory(target.parent().ok_or("Ruta inválida")?)?;
            }
        }
    }
    fs::remove_dir_all(transaction).map_err(|e| e.to_string())
}

fn journal_path_for_recovery(root: &Path, path: &str) -> Result<PathBuf> {
    journal_path(root, path)
}

fn apply_files(prepared: &Prepared, resolutions: &BTreeMap<String, String>) -> Result<()> {
    let changes: Vec<_> = prepared
        .preview
        .added
        .iter()
        .chain(&prepared.preview.changed)
        .chain(&prepared.preview.removed)
        .collect();
    let mut operations = BTreeMap::<String, Option<PathBuf>>::new();
    let mut next = prepared.next.clone();
    let mut kept: BTreeSet<String> = resolutions
        .iter()
        .filter(|(_, value)| *value == "keep")
        .map(|(p, _)| p.clone())
        .collect();
    for (old, replacements) in &prepared.replacements {
        if kept.contains(old) {
            for replacement in replacements {
                if resolutions.get(replacement).is_some_and(|r| r == "replace") {
                    return Err(format!(
                        "Resoluciones incompatibles: conservar {old} impide instalar {replacement}"
                    ));
                }
                kept.insert(replacement.clone());
                next.files.remove(replacement);
                next.identities.remove(replacement);
            }
        }
    }
    for path in &kept {
        if prepared.preview.removed.contains(path)
            && let Some(file) = prepared.old.files.get(path)
        {
            next.files.insert(path.clone(), file.clone());
            if let Some(ids) = prepared.old.identities.get(path) {
                next.identities.insert(path.clone(), ids.clone());
            }
        }
    }
    for path in changes {
        if kept.contains(path) {
            continue;
        }
        operations.insert(
            path.clone(),
            prepared
                .next
                .files
                .contains_key(path)
                .then(|| prepared.stage.path().join("files").join(path)),
        );
    }
    for path in operations.clone().keys() {
        if let Some(collisions) = prepared.collisions.get(path) {
            for collision in collisions {
                if kept.contains(collision) {
                    return Err(format!(
                        "Resoluciones incompatibles: instalar {path} reemplazaría {collision}, que elegiste conservar"
                    ));
                }
                // Installing another next-pack file at this path already replaces
                // the colliding user file. Never override that operation with delete.
                operations.entry(collision.clone()).or_insert(None);
            }
        }
    }
    stage_file_changes(&prepared.root, &operations)?;
    modpack::save(&prepared.root, &next)?;
    Ok(())
}

/// Caller holds the instance files guard. The shared journal also recovers mod
/// version replacements on startup, before any instance can be launched.
pub(crate) fn stage_file_changes(
    root: &Path,
    operations: &BTreeMap<String, Option<PathBuf>>,
) -> Result<()> {
    let transaction = root.join(JOURNAL_DIR);
    fs::create_dir(&transaction).map_err(|e| e.to_string())?;
    let mut journal = Journal {
        paths: BTreeMap::new(),
        committed: false,
    };
    for path in operations
        .keys()
        .map(String::as_str)
        .chain([modpack::STATE_FILE, "instance.cub"])
    {
        let source = journal_path(root, path)?;
        let exists = source.exists();
        if exists {
            atomic_copy(&source, &transaction.join("backup").join(path))?;
        }
        journal.paths.insert(path.into(), exists);
    }
    sync_backup_directories(&transaction)?;
    persist_json(&transaction.join("journal.json"), &journal)?;
    sync_directory(root)?;
    for (relative, source) in operations {
        let target = checked_path(root, relative)?;
        if let Some(source) = source {
            atomic_copy(source, &target)?;
        } else if target.exists() {
            fs::remove_file(&target).map_err(|e| e.to_string())?;
            sync_directory(target.parent().ok_or("Ruta inválida")?)?;
        }
    }
    Ok(())
}

pub(crate) fn commit_file_changes(root: &Path) -> Result<()> {
    let transaction = root.join(JOURNAL_DIR);
    let path = transaction.join("journal.json");
    let mut journal: Journal =
        serde_json::from_reader(File::open(&path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    journal.committed = true;
    persist_json(&path, &journal)?;
    if let Err(error) = fs::remove_dir_all(transaction) {
        tracing::warn!(%error, "File changes committed; backup cleanup deferred until restart");
    }
    Ok(())
}

pub async fn apply(
    handle: &InstanceHandle,
    token: String,
    resolutions: BTreeMap<String, String>,
) -> Result<()> {
    let root = handle.get_instance_dir().await;
    if root.join(JOURNAL_DIR).exists() {
        return Err("Hay una actualización pendiente de recuperación. Reiniciá el launcher para restaurarla".into());
    }
    let runtime = handle.get_version().await.to_string();
    let prepared = {
        let mut previews = PREVIEWS.lock().map_err(|e| e.to_string())?;
        let preview = previews
            .get(&token)
            .ok_or("La vista previa expiró; volvé a prepararla")?;
        if preview.id != handle.uuid.as_ref() {
            return Err("La vista previa pertenece a otra instancia".into());
        }
        for conflict in &preview.preview.conflicts {
            if !resolutions
                .get(conflict)
                .is_some_and(|r| r == "keep" || r == "replace")
            {
                return Err(format!("Falta resolver el conflicto: {conflict}"));
            }
        }
        if resolutions
            .keys()
            .any(|p| !preview.preview.conflicts.contains(p))
        {
            return Err("Resolución de conflicto desconocida".into());
        }
        previews.remove(&token).ok_or("La vista previa expiró")?
    };
    let next_version = prepared.next.game_version.clone();
    let result = tokio::task::spawn_blocking(move || -> Result<()> {
        if prepared.created.elapsed() >= PREVIEW_TTL
            || prepared.root != root
            || prepared.runtime != runtime
            || modpack::load(&root)?.as_ref() != Some(&prepared.old)
            || snapshot(&root, &prepared.old, &prepared.next)? != prepared.snapshot
        {
            return Err("La instancia cambió desde la vista previa; volvé a prepararla".into());
        }
        apply_files(&prepared, &resolutions)
    })
    .await
    .map_err(|e| e.to_string())?;
    let root = handle.get_instance_dir().await;
    if let Err(error) = result {
        let recovery = tokio::task::spawn_blocking(move || recover(&root))
            .await
            .map_err(|e| e.to_string())?;
        return Err(match recovery {
            Ok(()) => error,
            Err(e) => format!("{error}. No se pudo restaurar el respaldo: {e}"),
        });
    }
    let previous = handle.get_version().await.to_string();
    handle.set_version(next_version.clone()).await;
    // Caller holds the files guard; save through the dedicated guarded method below.
    if let Err(error) = handle.save_pack_version().await {
        handle.set_version(previous).await;
        let recovery = tokio::task::spawn_blocking(move || recover(&root))
            .await
            .map_err(|e| e.to_string())?;
        recovery?;
        return Err(error);
    }
    let commit = tokio::task::spawn_blocking(move || -> Result<()> {
        let transaction = root.join(JOURNAL_DIR);
        let path = transaction.join("journal.json");
        let mut journal: Journal =
            serde_json::from_reader(File::open(&path).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        journal.committed = true;
        persist_json(&path, &journal)?;
        if let Err(error) = fs::remove_dir_all(transaction) {
            tracing::warn!(%error, "Pack updated; backup cleanup deferred until restart");
        }
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?;
    if let Err(error) = commit {
        handle.set_version(previous).await;
        let root = handle.get_instance_dir().await;
        tokio::task::spawn_blocking(move || recover(&root))
            .await
            .map_err(|e| e.to_string())??;
        return Err(error);
    }
    super::DownloadQueue::get().enqueue(next_version).await;
    Ok(())
}

#[cfg(test)]
#[path = "../tests/services/modpack_update.rs"]
mod tests;
