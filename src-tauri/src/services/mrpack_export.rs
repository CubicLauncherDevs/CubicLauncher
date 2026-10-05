//! Selective, streaming Modrinth pack export.
use super::instance_manager::InstanceHandle;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use sha2::Sha512;
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::time::Duration;
use zellkern::{GameVersion, Loader};

type Result<T> = std::result::Result<T, String>;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportEntry {
    /// Strict, instance-relative, slash-separated path. Directories have no trailing slash.
    pub path: String,
    pub is_dir: bool,
    /// Recursive size for directories; all sizes are uncompressed bytes.
    pub size: u64,
    pub default_selected: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportPreview {
    pub name: String,
    pub version_id: String,
    pub summary: String,
    pub author: String,
    pub dependencies: BTreeMap<String, String>,
    pub entries: Vec<ExportEntry>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExportRequest {
    pub name: String,
    pub version_id: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub author: String,
    /// Exact leaf paths from the preview, never directory prefixes or globs.
    pub selected_paths: Vec<String>,
}

pub struct ExportInput {
    pub root: PathBuf,
    pub name: String,
    pub dependencies: BTreeMap<String, String>,
    pub icon: Option<PathBuf>,
}

pub async fn prepare_export(handle: &InstanceHandle) -> Result<ExportInput> {
    validate_minecraft_jar(&handle.get_minecraft_jar().await)?;
    let version = handle.get_version().await;
    let dependencies = dependencies(&GameVersion::from_version_id(&version))?;
    Ok(ExportInput {
        root: handle.get_instance_dir().await,
        name: handle.get_name().await.to_string(),
        dependencies,
        icon: handle
            .get_icon_absolute()
            .await
            .map(|p| PathBuf::from(p.as_ref())),
    })
}

fn validate_minecraft_jar(config: &super::minecraft_jar::MinecraftJarConfig) -> Result<()> {
    if config.files().next().is_some() {
        return Err(
            "El formato .mrpack no puede representar un JAR de Minecraft personalizado ni jar mods"
                .into(),
        );
    }
    Ok(())
}

fn dependencies(version: &GameVersion) -> Result<BTreeMap<String, String>> {
    let loader = match &version.loader {
        Loader::Vanilla => None,
        Loader::Fabric(v) => Some(("fabric-loader", v)),
        Loader::Forge(v) => Some(("forge", v)),
        Loader::NeoForge(v) => Some(("neoforge", v)),
        Loader::Quilt(v) => Some(("quilt-loader", v)),
        Loader::OptiFine(_) => {
            return Err("El formato .mrpack no puede representar OptiFine como loader".into());
        }
    };
    if version.mc_version.trim().is_empty() || loader.is_some_and(|(_, v)| v.trim().is_empty()) {
        return Err(
            "La instancia no tiene una versión de Minecraft/loader válida para .mrpack".into(),
        );
    }
    let mut deps = BTreeMap::from([("minecraft".into(), version.mc_version.clone())]);
    if let Some((key, value)) = loader {
        deps.insert(key.into(), value.clone());
    }
    Ok(deps)
}

fn valid_relative(path: &str) -> bool {
    !path.is_empty()
        && !path.contains(['\\', ':'])
        && !path.chars().any(char::is_control)
        && path
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
        && Path::new(path)
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
}

fn internal(path: &str) -> bool {
    path.split('/').any(|part| {
        let part = part.to_ascii_lowercase();
        part.ends_with(".crep")
            || part.starts_with(".modpack-")
            || matches!(
                part.as_str(),
                "instance.cub" | "upstream.json" | "modpack.cub.json"
            )
    }) || matches!(
        path.to_ascii_lowercase().as_str(),
        "icon.png"
            | "icon.jpg"
            | "icon.jpeg"
            | "icon.webp"
            | "icon.gif"
            | "icon.svg"
            | "instance.cfg"
            | "mmc-pack.json"
            | "cubic-manifest.json"
            | "config.json"
    ) || matches!(
        path.split('/').next(),
        Some("cubic-jar" | "cubic-jar-cache" | ".cubic" | ".git")
    )
}

fn selected_by_default(path: &str) -> bool {
    matches!(
        path.split('/').next(),
        Some(
            "mods"
                | "config"
                | "scripts"
                | "kubejs"
                | "defaultconfigs"
                | "resourcepacks"
                | "shaderpacks"
        )
    )
}

fn downloadable_content(path: &str) -> bool {
    // Configuration, scripts, and every other user file always remain overrides.
    let Some((directory, file)) = path.split_once('/') else {
        return false;
    };
    match directory {
        "mods" => file.to_ascii_lowercase().ends_with(".jar"),
        "resourcepacks" | "shaderpacks" => file.to_ascii_lowercase().ends_with(".zip"),
        _ => false,
    }
}

/// Recheck every ancestor; neither selected leaves nor parent directories may be links.
fn source_path(root: &Path, relative: &str) -> Result<PathBuf> {
    if !valid_relative(relative) || internal(relative) {
        return Err(format!("Ruta de exportación no permitida: {relative}"));
    }
    let mut path = root.to_path_buf();
    for part in relative.split('/') {
        path.push(part);
        let meta = fs::symlink_metadata(&path).map_err(|e| format!("{relative}: {e}"))?;
        if meta.file_type().is_symlink() || (!meta.is_dir() && !meta.is_file()) {
            return Err(format!(
                "No se exportan enlaces ni archivos especiales: {relative}"
            ));
        }
    }
    let canonical = fs::canonicalize(&path).map_err(|e| e.to_string())?;
    if !canonical.starts_with(root) {
        return Err(format!("Ruta fuera de la instancia: {relative}"));
    }
    Ok(path)
}

fn canonical_root(input: &ExportInput) -> Result<PathBuf> {
    let metadata = fs::symlink_metadata(&input.root).map_err(|e| e.to_string())?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err("El directorio de instancia no puede ser un enlace".into());
    }
    fs::canonicalize(&input.root).map_err(|e| e.to_string())
}

pub fn preview(input: &ExportInput) -> Result<ExportPreview> {
    let root = canonical_root(input)?;
    let icon = input.icon.as_ref().and_then(|p| fs::canonicalize(p).ok());
    let mut entries = Vec::new();
    scan(&root, "", icon.as_deref(), &mut entries)?;
    Ok(ExportPreview {
        name: input.name.clone(),
        version_id: "1.0.0".into(),
        summary: String::new(),
        author: super::modpack::load(&root)
            .ok()
            .flatten()
            .and_then(|pack| pack.author)
            .unwrap_or_default(),
        dependencies: input.dependencies.clone(),
        entries,
    })
}

fn scan(
    root: &Path,
    relative: &str,
    icon: Option<&Path>,
    entries: &mut Vec<ExportEntry>,
) -> Result<u64> {
    let dir = if relative.is_empty() {
        root.to_owned()
    } else {
        source_path(root, relative)?
    };
    let mut children = fs::read_dir(dir)
        .map_err(|e| e.to_string())?
        .collect::<std::io::Result<Vec<_>>>()
        .map_err(|e| e.to_string())?;
    children.sort_by_key(|entry| entry.file_name());
    let mut total = 0_u64;
    for child in children {
        let Some(name) = child.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        let path = if relative.is_empty() {
            name
        } else {
            format!("{relative}/{name}")
        };
        if !valid_relative(&path) || internal(&path) {
            continue;
        }
        let kind = child.file_type().map_err(|e| e.to_string())?;
        if kind.is_symlink() || (!kind.is_file() && !kind.is_dir()) {
            continue;
        }
        let source = source_path(root, &path)?;
        if icon.is_some_and(|icon| fs::canonicalize(&source).ok().as_deref() == Some(icon)) {
            continue;
        }
        let index = entries.len();
        entries.push(ExportEntry {
            path: path.clone(),
            is_dir: kind.is_dir(),
            size: 0,
            default_selected: selected_by_default(&path),
        });
        let size = if kind.is_dir() {
            scan(root, &path, icon, entries)?
        } else {
            fs::metadata(source).map_err(|e| e.to_string())?.len()
        };
        entries[index].size = size;
        total = total
            .checked_add(size)
            .ok_or("Tamaño total fuera de rango")?;
    }
    Ok(total)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct IndexFile {
    path: String,
    hashes: BTreeMap<String, String>,
    downloads: Vec<String>,
    file_size: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Index<'a> {
    format_version: u8,
    game: &'static str,
    version_id: &'a str,
    name: &'a str,
    #[serde(skip_serializing_if = "str::is_empty")]
    summary: &'a str,
    // Optional extension: the core mrpack format does not define an author field.
    // Readers unaware of it can ignore it without affecting pack installation.
    #[serde(skip_serializing_if = "str::is_empty")]
    author: &'a str,
    dependencies: &'a BTreeMap<String, String>,
    files: Vec<IndexFile>,
}

pub struct Snapshot {
    // Private immutable copies guarantee index hashes describe exactly the exported bytes,
    // even if an external editor changes the instance during the network lookup.
    staging: tempfile::TempDir,
    files: Vec<IndexFile>,
    request: ExportRequest,
    dependencies: BTreeMap<String, String>,
    destination: PathBuf,
}

fn validate_request(request: &ExportRequest) -> Result<()> {
    if request.name.trim().is_empty() || request.version_id.trim().is_empty() {
        return Err("El nombre y la versión del modpack son obligatorios".into());
    }
    if request.name.len() > 512
        || request.version_id.len() > 512
        || request.summary.len() > 16384
        || request.author.chars().count() > 512
    {
        return Err("Los metadatos del modpack son demasiado largos".into());
    }
    Ok(())
}

fn destination(root: &Path, dest: &Path) -> Result<PathBuf> {
    if !dest.is_absolute()
        || dest.components().any(|c| matches!(c, Component::ParentDir))
        || !dest
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("mrpack"))
    {
        return Err("El destino debe ser una ruta absoluta a un archivo .mrpack, sin '..'".into());
    }
    let parent = fs::canonicalize(dest.parent().ok_or("Destino inválido")?)
        .map_err(|e| format!("Directorio de destino: {e}"))?;
    let dest = parent.join(dest.file_name().ok_or("Destino inválido")?);
    if dest.starts_with(root) {
        return Err("Guarda el .mrpack fuera del directorio de la instancia".into());
    }
    match fs::symlink_metadata(&dest) {
        Ok(meta) if meta.file_type().is_symlink() || !meta.is_file() => {
            return Err("El destino no puede ser un enlace ni un directorio".into());
        }
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(e.to_string()),
        _ => {}
    }
    Ok(dest)
}

/// Runs on spawn_blocking. Stream each selected source through both hashers into a
/// private snapshot; memory usage does not depend on the size of the selected files.
pub fn snapshot(input: &ExportInput, request: ExportRequest, dest: &Path) -> Result<Snapshot> {
    validate_request(&request)?;
    let root = canonical_root(input)?;
    let destination = destination(&root, dest)?;
    let available = preview(input)?
        .entries
        .into_iter()
        .filter(|e| !e.is_dir)
        .map(|e| e.path)
        .collect::<BTreeSet<_>>();
    let selected = request.selected_paths.iter().collect::<BTreeSet<_>>();
    let staging = tempfile::Builder::new()
        .prefix(".mrpack-snapshot-")
        .tempdir_in(destination.parent().unwrap())
        .map_err(|e| e.to_string())?;
    let mut files = Vec::new();
    for relative in selected {
        if !valid_relative(relative) || !available.contains(relative) {
            return Err(format!("Archivo no exportable o ausente: {relative}"));
        }
        let path = source_path(&root, relative)?;
        let mut source = File::open(&path).map_err(|e| e.to_string())?;
        // Check again after opening, before reading bytes (also catches link swaps).
        source_path(&root, relative)?;
        let opened = source.metadata().map_err(|e| e.to_string())?;
        if !opened.is_file() {
            return Err(format!("No es un archivo regular: {relative}"));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            let current = fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
            if current.ino() != opened.ino() || current.dev() != opened.dev() {
                return Err(format!(
                    "El archivo cambió durante la exportación: {relative}"
                ));
            }
        }
        let mut target = File::create(staging.path().join(files.len().to_string()))
            .map_err(|e| e.to_string())?;
        let mut sha1 = Sha1::new();
        let mut sha512 = Sha512::new();
        let mut size = 0_u64;
        let mut buffer = [0_u8; 64 * 1024];
        loop {
            let count = source.read(&mut buffer).map_err(|e| e.to_string())?;
            if count == 0 {
                break;
            }
            target
                .write_all(&buffer[..count])
                .map_err(|e| e.to_string())?;
            sha1.update(&buffer[..count]);
            sha512.update(&buffer[..count]);
            size = size
                .checked_add(count as u64)
                .ok_or("Archivo demasiado grande")?;
        }
        files.push(IndexFile {
            path: relative.clone(),
            hashes: BTreeMap::from([
                (
                    "sha1".into(),
                    sha1.finalize().iter().map(|b| format!("{b:02x}")).collect(),
                ),
                (
                    "sha512".into(),
                    sha512
                        .finalize()
                        .iter()
                        .map(|b| format!("{b:02x}"))
                        .collect(),
                ),
            ]),
            downloads: Vec::new(),
            file_size: size,
        });
    }
    Ok(Snapshot {
        staging,
        files,
        request,
        dependencies: input.dependencies.clone(),
        destination,
    })
}

#[derive(Debug, Deserialize)]
struct RemoteFile {
    hashes: BTreeMap<String, String>,
    url: String,
    size: u64,
}

#[derive(Debug, Deserialize)]
struct RemoteVersion {
    files: Vec<RemoteFile>,
}

/// Mrpack's approved direct-download hosts. Reject credentials, alternate ports,
/// fragments, landing pages, and hosts merely ending in an approved hostname.
fn valid_download(raw: &str) -> bool {
    let Ok(url) = url::Url::parse(raw) else {
        return false;
    };
    if url.scheme() != "https"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some()
        || url.fragment().is_some()
        || url.query().is_some()
    {
        return false;
    }
    let path = url.path();
    let Some(filename) = path.rsplit('/').next() else {
        return false;
    };
    if !(filename.to_ascii_lowercase().ends_with(".jar")
        || filename.to_ascii_lowercase().ends_with(".zip"))
    {
        return false;
    }
    match url.host_str() {
        Some("cdn.modrinth.com") => path.starts_with("/data/"),
        Some("github.com") => path.contains("/releases/download/"),
        Some("raw.githubusercontent.com") => path.split('/').count() >= 5,
        Some("gitlab.com") => path.contains("/-/releases/") && path.contains("/downloads/"),
        _ => false,
    }
}

fn apply_resolved(files: &mut [IndexFile], versions: &BTreeMap<String, RemoteVersion>) {
    for file in files.iter_mut().filter(|f| downloadable_content(&f.path)) {
        let Some(version) = versions.get(&file.hashes["sha1"]) else {
            continue;
        };
        // A version can contain several files. Never use its primary/latest file.
        if let Some(remote) = version.files.iter().find(|remote| {
            remote.size == file.file_size
                && remote.hashes.get("sha1") == file.hashes.get("sha1")
                && remote.hashes.get("sha512") == file.hashes.get("sha512")
                && valid_download(&remote.url)
        }) {
            file.downloads = vec![remote.url.clone()];
        }
    }
}

/// Batch exact-file resolution, without /update or any loader/version substitution.
/// Lookup failures are harmless: selected bytes are preserved as overrides.
pub async fn resolve_downloads(snapshot: &mut Snapshot) -> Result<()> {
    let hashes: Vec<_> = snapshot
        .files
        .iter()
        .filter(|f| downloadable_content(&f.path))
        .map(|f| f.hashes["sha1"].clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    if hashes.is_empty() {
        return Ok(());
    }
    let client = reqwest::Client::builder()
        .user_agent("CubicLauncher/mrpack-export")
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;
    for batch in hashes.chunks(100) {
        let response = client
            .post("https://api.modrinth.com/v2/version_files")
            .json(&serde_json::json!({ "hashes": batch, "algorithm": "sha1" }))
            .send()
            .await;
        let response = match response.and_then(reqwest::Response::error_for_status) {
            Ok(response) => response,
            Err(error) => {
                tracing::warn!(%error, "Modrinth lookup unavailable; exporting overrides");
                break;
            }
        };
        match response.json::<BTreeMap<String, RemoteVersion>>().await {
            Ok(versions) => apply_resolved(&mut snapshot.files, &versions),
            Err(error) => {
                tracing::warn!(%error, "Invalid Modrinth response; exporting overrides");
                break;
            }
        }
    }
    Ok(())
}

/// Runs on spawn_blocking. NamedTempFile removes partial output on every failure;
/// persist atomically replaces the destination only after the ZIP is finalized.
pub fn write_archive(snapshot: Snapshot) -> Result<PathBuf> {
    let mut temp = tempfile::Builder::new()
        .prefix(".mrpack-export-")
        .tempfile_in(snapshot.destination.parent().ok_or("Destino inválido")?)
        .map_err(|e| e.to_string())?;
    {
        let mut zip = zip::ZipWriter::new(temp.as_file_mut());
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        let index = Index {
            format_version: 1,
            game: "minecraft",
            version_id: snapshot.request.version_id.trim(),
            name: snapshot.request.name.trim(),
            summary: snapshot.request.summary.trim(),
            author: snapshot.request.author.trim(),
            dependencies: &snapshot.dependencies,
            files: snapshot
                .files
                .iter()
                .filter(|f| !f.downloads.is_empty())
                .cloned()
                .collect(),
        };
        zip.start_file("modrinth.index.json", options)
            .map_err(|e| e.to_string())?;
        serde_json::to_writer_pretty(&mut zip, &index).map_err(|e| e.to_string())?;
        for (i, file) in snapshot
            .files
            .iter()
            .enumerate()
            .filter(|(_, f)| f.downloads.is_empty())
        {
            zip.start_file(
                format!("overrides/{}", file.path),
                options.large_file(file.file_size >= u32::MAX as u64),
            )
            .map_err(|e| e.to_string())?;
            let mut source = File::open(snapshot.staging.path().join(i.to_string()))
                .map_err(|e| e.to_string())?;
            let copied = std::io::copy(&mut source, &mut zip).map_err(|e| e.to_string())?;
            if copied != file.file_size {
                return Err(format!("El snapshot cambió: {}", file.path));
            }
        }
        zip.finish().map_err(|e| e.to_string())?;
    }
    temp.as_file().sync_all().map_err(|e| e.to_string())?;
    temp.persist(&snapshot.destination)
        .map_err(|e| e.to_string())?;
    Ok(snapshot.destination.clone())
}

#[cfg(test)]
#[path = "../tests/services/mrpack_export.rs"]
mod tests;
