//! Verified, transactional version changes for user-installed mods.
use super::dependency_resolver::{
    self, DependencyKind, DependencyRequest, DependencySource, ResolvedDependency,
};
use super::modpack::{self, Result};
use super::modpack_update;
use super::{AddonManager, InstanceHandle, ModSource, compute_file_sha1, file_fingerprint};
use crate::commands::instance::mods::{PerFileCacheEntry, repo_path};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct VersionRef {
    pub source: String,
    pub project_id: String,
    pub version_id: String,
}

#[derive(Deserialize)]
pub struct ReplaceRequest {
    pub filename: String,
    pub expected_sha1: String,
    pub target: VersionRef,
    pub downloads: Vec<VersionRef>,
}

struct RemoteFile {
    version: VersionRef,
    filename: String,
    url: String,
    sha1: String,
    size: u64,
}

struct InstalledFile {
    filename: String,
    sha1: String,
    source: ModSource,
    ids: BTreeSet<String>,
}

fn provider(source: &str) -> Result<DependencySource> {
    match source {
        "modrinth" => Ok(DependencySource::Modrinth),
        "curseforge" => Ok(DependencySource::Curseforge),
        _ => Err("No se conoce el proveedor de este mod".into()),
    }
}

fn text(value: &serde_json::Value, key: &str) -> Result<String> {
    value[key]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| format!("Respuesta inválida: falta {key}"))
}

fn modrinth_compatible(data: &serde_json::Value, game: &zellkern::GameVersion) -> bool {
    data["game_versions"].as_array().is_some_and(|versions| {
        versions
            .iter()
            .any(|v| v.as_str() == Some(&game.mc_version))
    }) && data["loaders"].as_array().is_some_and(|loaders| {
        loaders.iter().any(|loader| {
            loader
                .as_str()
                .is_some_and(|name| name.eq_ignore_ascii_case(game.loader.name()))
        })
    })
}

async fn remote_file(version: &VersionRef, game: &zellkern::GameVersion) -> Result<RemoteFile> {
    let (filename, url, sha1, size, compatible) = match provider(&version.source)? {
        DependencySource::Modrinth => {
            let client = reqwest::Client::builder()
                .user_agent(concat!("CubicLauncher/", env!("CARGO_PKG_VERSION")))
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .map_err(|e| e.to_string())?;
            let data: serde_json::Value = client
                .get(format!(
                    "https://api.modrinth.com/v2/version/{}",
                    urlencoding::encode(&version.version_id)
                ))
                .send()
                .await
                .map_err(|e| e.to_string())?
                .error_for_status()
                .map_err(|e| e.to_string())?
                .json()
                .await
                .map_err(|e| e.to_string())?;
            if text(&data, "project_id")? != version.project_id {
                return Err("La versión no pertenece al mod seleccionado".into());
            }
            let compatible = modrinth_compatible(&data, game);
            let files = data["files"]
                .as_array()
                .ok_or("La versión no tiene archivos")?;
            let file = files
                .iter()
                .find(|f| f["primary"].as_bool() == Some(true))
                .or_else(|| files.first())
                .ok_or("La versión no tiene archivos")?;
            (
                text(file, "filename")?,
                text(file, "url")?,
                text(&file["hashes"], "sha1")?,
                file["size"].as_u64().ok_or("Falta el tamaño del mod")?,
                compatible,
            )
        }
        DependencySource::Curseforge => {
            let file_id = version
                .version_id
                .parse()
                .map_err(|_| "ID de archivo CurseForge inválido")?;
            let files = super::curseforge_api::CurseForgeClient::from_settings_or_default()
                .get_mod_files(&[file_id])
                .await
                .map_err(|e| e.to_string())?;
            let file = files.first().ok_or("Versión CurseForge no disponible")?;
            if file.mod_id.map(|id| id.to_string()).as_deref() != Some(version.project_id.as_str())
            {
                return Err("La versión no pertenece al mod seleccionado".into());
            }
            let compatible = file.game_versions.iter().any(|v| v == &game.mc_version)
                && file
                    .game_versions
                    .iter()
                    .any(|v| v.eq_ignore_ascii_case(game.loader.name()));
            (
                file.file_name.clone(),
                file.download_url.clone().unwrap_or_else(|| {
                    super::curseforge_api::curseforge_cdn_url(file.id, &file.file_name)
                }),
                file.hashes
                    .iter()
                    .find(|h| h.algo == 1)
                    .map(|h| h.value.clone())
                    .ok_or("La versión no tiene hash SHA-1")?,
                file.file_length,
                compatible,
            )
        }
    };
    crate::core::validate_filename(&filename)?;
    if !filename.to_lowercase().ends_with(".jar")
        || sha1.len() != 40
        || !sha1.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return Err("El proveedor no devolvió un mod JAR verificable".into());
    }
    if !compatible {
        return Err(format!(
            "{filename} no es compatible con Minecraft {} / {}",
            game.mc_version,
            game.loader.name()
        ));
    }
    Ok(RemoteFile {
        version: version.clone(),
        filename,
        url,
        sha1: sha1.to_ascii_lowercase(),
        size,
    })
}

fn node_ref(node: &ResolvedDependency) -> Result<VersionRef> {
    Ok(VersionRef {
        source: match node.source {
            DependencySource::Modrinth => "modrinth",
            DependencySource::Curseforge => "curseforge",
        }
        .into(),
        project_id: node.project_id.clone(),
        version_id: node.version_id.clone().ok_or("Dependencia sin versión")?,
    })
}

fn required_versions(
    nodes: &[ResolvedDependency],
    selected: &BTreeSet<VersionRef>,
    result: &mut BTreeSet<VersionRef>,
) -> Result<()> {
    for node in nodes {
        if matches!(
            node.kind,
            DependencyKind::Incompatible | DependencyKind::Embedded
        ) {
            continue;
        }
        let version = node_ref(node)?;
        if node.kind == DependencyKind::Optional && !selected.contains(&version) {
            continue;
        }
        result.insert(version);
        required_versions(&node.children, selected, result)?;
    }
    Ok(())
}

fn scan(root: &Path) -> Result<Vec<InstalledFile>> {
    let dir = modpack::checked_path(root, "mods")?;
    let repo = ablage::Repo::open(repo_path(&dir));
    let mut files = Vec::new();
    for entry in std::fs::read_dir(&dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let filename = entry.file_name().to_string_lossy().into_owned();
        let lower = filename.to_ascii_lowercase();
        if !lower
            .strip_suffix(".disabled")
            .unwrap_or(&lower)
            .ends_with(".jar")
        {
            continue;
        }
        let path = modpack::checked_path(root, &format!("mods/{filename}"))?;
        let sha1 = compute_file_sha1(&path)?;
        let source = repo
            .get(&filename)
            .or_else(|| repo.get(&sha1))
            .and_then(|entry| postcard::from_bytes::<PerFileCacheEntry>(&entry.data).ok())
            .filter(|entry| entry.sha1.eq_ignore_ascii_case(&sha1))
            .map(|entry| entry.source)
            .unwrap_or(ModSource::Local);
        files.push(InstalledFile {
            filename,
            sha1,
            source,
            ids: modpack_update::mod_ids(&path),
        });
    }
    Ok(files)
}

fn ensure_own(root: &Path, filename: &str) -> Result<()> {
    let Some(pack) = modpack::load(root)? else {
        return Ok(());
    };
    let relative = format!(
        "mods/{}",
        filename.strip_suffix(".disabled").unwrap_or(filename)
    );
    if pack.needs_inventory
        || pack.files.keys().any(|p| {
            p.strip_suffix(".disabled")
                .unwrap_or(p)
                .eq_ignore_ascii_case(&relative)
        })
    {
        return Err(format!(
            "Este mod es provisto por: {}. Usá la actualización del modpack.",
            pack.name
        ));
    }
    Ok(())
}

fn plan_changes(
    root: &Path,
    stage: &Path,
    request: &ReplaceRequest,
    files: &[RemoteFile],
    installed: &[InstalledFile],
) -> Result<(String, BTreeMap<String, Option<PathBuf>>)> {
    let original = installed
        .iter()
        .find(|f| f.filename == request.filename)
        .ok_or("El mod original ya no existe")?;
    if !original.sha1.eq_ignore_ascii_case(&request.expected_sha1) {
        return Err(
            "El mod cambió desde que abriste sus versiones. Actualizá la lista e intentá de nuevo."
                .into(),
        );
    }
    ensure_own(root, &original.filename)?;
    let mut operations = BTreeMap::new();
    let mut result_name = original.filename.clone();
    let mut destinations = BTreeSet::new();
    let mut removed = BTreeSet::new();
    for file in files {
        let root_mod = file.version == request.target;
        let ids = modpack_update::mod_ids(&stage.join(&file.filename));
        let matches: Vec<_> = installed
            .iter()
            .filter(|old| {
                (root_mod && old.filename == original.filename)
                    || (old.source.source_str() == file.version.source
                        && old.source.project_id() == Some(&file.version.project_id))
                    || !ids.is_disjoint(&old.ids)
                    || old.sha1 == file.sha1
            })
            .collect();
        if matches.len() > 1 {
            return Err(format!(
                "Hay varias copias de {} instaladas; elegí cuál conservar antes de cambiar la versión",
                file.version.project_id
            ));
        }
        let old = matches.first().copied();
        if root_mod && old.is_none_or(|old| old.filename != original.filename) {
            return Err("La versión seleccionada no corresponde al archivo original".into());
        }
        if let Some(old) = old
            && old.sha1 == file.sha1
        {
            if !root_mod && old.filename.ends_with(".disabled") {
                return Err(format!("La dependencia {} está desactivada", old.filename));
            }
            continue;
        }
        if !request.downloads.contains(&file.version) {
            return Err(format!(
                "La dependencia {} necesita otra versión. Volvé a preparar el cambio.",
                file.version.project_id
            ));
        }
        if let Some(old) = old {
            ensure_own(root, &old.filename)?;
            if !root_mod && old.filename.ends_with(".disabled") {
                return Err(format!("La dependencia {} está desactivada", old.filename));
            }
            if !removed.insert(old.filename.clone()) {
                return Err("Dos dependencias intentan reemplazar el mismo archivo".into());
            }
        }
        let disabled = root_mod && original.filename.ends_with(".disabled");
        let filename = if disabled {
            format!("{}.disabled", file.filename)
        } else {
            file.filename.clone()
        };
        ensure_own(root, &filename)?;
        let destination = format!("mods/{filename}");
        modpack::checked_path(root, &destination)?;
        if !destinations.insert(filename.to_lowercase()) {
            return Err("Dos versiones usan el mismo nombre de archivo".into());
        }
        if installed.iter().any(|f| {
            f.filename.eq_ignore_ascii_case(&filename)
                && old.is_none_or(|old| old.filename != f.filename)
        }) {
            return Err(format!("Ya existe otro archivo llamado {filename}"));
        }
        if let Some(old) = old
            && old.filename != filename
        {
            if old.filename.eq_ignore_ascii_case(&filename) {
                return Err("Los nombres de archivo sólo difieren en mayúsculas".into());
            }
            operations.insert(format!("mods/{}", old.filename), None);
        }
        operations.insert(destination, Some(stage.join(&file.filename)));
        if root_mod {
            result_name = filename;
        }
    }
    Ok((result_name, operations))
}

fn cache_versions(
    root: &Path,
    files: &[RemoteFile],
    installed_name: &str,
    target: &VersionRef,
) -> Result<()> {
    let dir = root.join("mods");
    let mut repo = ablage::Repo::open(repo_path(&dir));
    for file in files {
        let name = if &file.version == target {
            installed_name
        } else {
            &file.filename
        };
        let path = dir.join(name);
        if !path.is_file() || compute_file_sha1(&path)? != file.sha1 {
            continue;
        }
        let metadata = std::fs::metadata(&path).map_err(|e| e.to_string())?;
        let source = if file.version.source == "modrinth" {
            ModSource::Modrinth {
                project_id: file.version.project_id.clone(),
                version_id: file.version.version_id.clone(),
                slug: None,
            }
        } else {
            ModSource::CurseForge {
                project_id: file.version.project_id.clone(),
                file_id: file.version.version_id.clone(),
            }
        };
        let entry = PerFileCacheEntry {
            sha1: file.sha1.clone(),
            metadata: AddonManager::get_mod_metadata_only(&path),
            source,
        };
        let data = postcard::to_stdvec(&entry).map_err(|e| e.to_string())?;
        repo.put(
            name.to_string(),
            ablage::Entry {
                version: 1,
                fingerprint: file_fingerprint(
                    name,
                    &metadata.modified().map_err(|e| e.to_string())?,
                    metadata.len(),
                ),
                data,
            },
        );
    }
    repo.flush().map_err(|e| e.to_string())
}

pub async fn replace(handle: &InstanceHandle, request: ReplaceRequest) -> Result<String> {
    crate::core::validate_filename(&request.filename)?;
    if request.expected_sha1.len() != 40 {
        return Err("Esperá a que se identifique el archivo instalado y actualizá la lista".into());
    }
    if request.downloads.is_empty() || !request.downloads.contains(&request.target) {
        return Err("La selección no incluye la nueva versión del mod".into());
    }
    let root = handle.get_instance_dir().await;
    let game = zellkern::GameVersion::from_version_id(&handle.get_version().await);
    let snapshot_root = root.clone();
    let original_name = request.filename.clone();
    let installed = tokio::task::spawn_blocking(move || {
        ensure_own(&snapshot_root, &original_name)?;
        scan(&snapshot_root)
    })
    .await
    .map_err(|e| e.to_string())??;
    let result = dependency_resolver::resolve_dependencies(
        vec![DependencyRequest {
            source: provider(&request.target.source)?,
            project_id: request.target.project_id.clone(),
            version_id: Some(request.target.version_id.clone()),
            kind: DependencyKind::Required,
        }],
        game.loader.name().into(),
        game.mc_version.clone(),
    )
    .await?;
    if !result.conflicts.is_empty() {
        return Err("Las dependencias solicitan versiones incompatibles entre sí".into());
    }
    let selected: BTreeSet<_> = request.downloads.iter().cloned().collect();
    let mut needed = BTreeSet::new();
    required_versions(&result.tree, &selected, &mut needed)?;
    if !needed.contains(&request.target) || !selected.is_subset(&needed) {
        return Err("La selección de dependencias no coincide con la versión elegida".into());
    }
    let mut files = Vec::new();
    for version in needed {
        files.push(remote_file(&version, &game).await?);
    }
    let stage = tempfile::Builder::new()
        .prefix(".mod-version-")
        .tempdir()
        .map_err(|e| e.to_string())?;
    let mut names = BTreeSet::new();
    let mut items = Vec::new();
    for file in &files {
        if !names.insert(file.filename.to_lowercase()) {
            return Err("Dos dependencias usan el mismo nombre de archivo".into());
        }
        items.push(
            aqua::DownloadItemSpec::new(
                file.url.clone(),
                stage.path().join(&file.filename),
                &file.filename,
            )
            .with_hash(&file.sha1)
            .with_size(file.size),
        );
    }
    let dm = aqua::DownloadManager::new(crate::core::PathManager::get().get_shared_dir());
    dm.prepare_batch(Box::new(aqua::GenericBatch::new(
        format!("mod-version-{}", handle.uuid),
        items,
    )))
    .await
    .map_err(|e| e.to_string())?
    .download_all(None)
    .await
    .map_err(|e| e.to_string())?;
    tokio::task::spawn_blocking(move || {
        for file in &files {
            let path = stage.path().join(&file.filename);
            if compute_file_sha1(&path)? != file.sha1
                || std::fs::metadata(&path).map_err(|e| e.to_string())?.len() != file.size
            {
                return Err(format!(
                    "La descarga de {} no coincide con su manifiesto",
                    file.filename
                ));
            }
        }
        let fresh = scan(&root)?;
        if fresh.len() != installed.len()
            || fresh.iter().any(|file| {
                !installed
                    .iter()
                    .any(|old| old.filename == file.filename && old.sha1 == file.sha1)
            })
        {
            return Err(
                "Los archivos de la instancia cambiaron durante la descarga. Volvé a intentarlo."
                    .into(),
            );
        }
        let (name, operations) = plan_changes(&root, stage.path(), &request, &files, &fresh)?;
        if !operations.is_empty() {
            if root.join(modpack_update::JOURNAL_DIR).exists() {
                return Err("Hay una operación pendiente de recuperación".into());
            }
            if let Err(error) = modpack_update::stage_file_changes(&root, &operations)
                .and_then(|_| modpack_update::commit_file_changes(&root))
            {
                return Err(match modpack_update::recover(&root) {
                    Ok(()) => error,
                    Err(recovery) => {
                        format!("{error}. No se pudo restaurar el respaldo: {recovery}")
                    }
                });
            }
        }
        if let Err(error) = cache_versions(&root, &files, &name, &request.target) {
            tracing::warn!(%error, "Mod installed; metadata will be rebuilt");
        }
        Ok(name)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
#[path = "../tests/services/mod_versions.rs"]
mod tests;
