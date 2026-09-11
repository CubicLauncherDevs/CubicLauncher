//! Lightweight world discovery and streaming filesystem operations.
//! Listing never traverses region/player files. Only small metadata DTOs are cached.
use crate::core::validate_filename;
use fastnbt::Value;
use flate2::{Compression, read::GzDecoder, write::GzEncoder};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{Arc, LazyLock, Mutex},
    time::{Instant, SystemTime},
};
use zip::{ZipArchive, ZipWriter, write::SimpleFileOptions};

type Result<T> = std::result::Result<T, String>;
const MAX_LEVEL_BYTES: u64 = 16 * 1024 * 1024;
// Limit simultaneous disk-heavy operations across all instances.
pub static IO_SLOTS: LazyLock<Arc<tokio::sync::Semaphore>> =
    LazyLock::new(|| Arc::new(tokio::sync::Semaphore::new(2)));

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldDto {
    pub folder: String,
    pub name: String,
    pub version: Option<String>,
    pub game_mode: Option<i32>,
    pub hardcore: bool,
    pub last_played: Option<i64>,
    pub icon: Option<String>,
    pub icon_revision: Option<String>,
    pub seed: Option<String>,
    pub metadata_error: bool,
}

#[derive(Clone, Deserialize)]
struct Level {
    #[serde(rename = "Data")]
    data: LevelData,
}

#[derive(Clone, Default, Deserialize)]
#[serde(default)]
struct LevelData {
    #[serde(rename = "LevelName")]
    name: Option<String>,
    #[serde(rename = "LastPlayed")]
    last_played: Option<i64>,
    #[serde(rename = "GameType")]
    game_mode: Option<i32>,
    hardcore: Option<i8>,
    #[serde(rename = "Version")]
    version: Option<LevelVersion>,
    #[serde(rename = "RandomSeed")]
    random_seed: Option<i64>,
    #[serde(rename = "WorldGenSettings")]
    world_gen_settings: Option<WorldGenSettings>,
}

impl LevelData {
    fn seed(&self) -> Option<String> {
        self.world_gen_settings
            .as_ref()
            .and_then(|settings| settings.seed)
            .or(self.random_seed)
            .map(|value| value.to_string())
    }
}

#[derive(Clone, Default, Deserialize)]
#[serde(default)]
struct WorldGenSettings {
    seed: Option<i64>,
}

#[derive(Clone, Deserialize)]
struct LevelVersion {
    #[serde(rename = "Name")]
    name: Option<String>,
}

#[derive(Clone, PartialEq)]
struct Stamp(Option<SystemTime>, u64);

#[derive(Clone)]
struct CachedLevel {
    stamps: (Option<Stamp>, Option<Stamp>),
    data: LevelData,
    error: bool,
}

static CACHE: LazyLock<Mutex<HashMap<PathBuf, CachedLevel>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn io(error: impl std::fmt::Display) -> String {
    error.to_string()
}

fn regular_file(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|m| m.is_file())
}

fn stamp(path: &Path) -> Option<Stamp> {
    let m = fs::symlink_metadata(path).ok()?;
    m.is_file().then(|| Stamp(m.modified().ok(), m.len()))
}

fn read_level(path: &Path) -> Result<Vec<u8>> {
    if !regular_file(path) {
        return Err("level.dat no es un archivo regular".into());
    }
    let mut bytes = Vec::new();
    GzDecoder::new(File::open(path).map_err(io)?)
        .take(MAX_LEVEL_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(io)?;
    if bytes.len() as u64 > MAX_LEVEL_BYTES {
        return Err("level.dat supera el límite de 16 MiB".into());
    }
    Ok(bytes)
}

fn metadata(world: &Path) -> CachedLevel {
    let stamps = (
        stamp(&world.join("level.dat")),
        stamp(&world.join("level.dat_old")),
    );
    if let Some(cached) = CACHE.lock().unwrap_or_else(|e| e.into_inner()).get(world)
        && cached.stamps == stamps
    {
        return cached.clone();
    }
    let parse = |name| -> Result<Level> {
        fastnbt::from_bytes(&read_level(&world.join(name))?).map_err(io)
    };
    let primary = parse("level.dat");
    let error = primary.is_err();
    let data = primary
        .or_else(|_| parse("level.dat_old"))
        .map(|level| level.data)
        .unwrap_or_default();
    let cached = CachedLevel {
        stamps,
        data,
        error,
    };
    let mut cache = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    if cache.len() >= 2048 {
        cache.clear();
    }
    cache.insert(world.to_path_buf(), cached.clone());
    cached
}

/// A direct child only; neither saves nor individual worlds may be symlinks.
pub fn saves_dir(instance: &Path, create: bool) -> Result<PathBuf> {
    let instance = instance.canonicalize().map_err(io)?;
    let saves = instance.join("saves");
    if create && !saves.try_exists().map_err(io)? {
        fs::create_dir(&saves).map_err(io)?;
    }
    match fs::symlink_metadata(&saves) {
        Ok(m) if m.is_dir() => Ok(saves),
        Err(e) if !create && e.kind() == std::io::ErrorKind::NotFound => Ok(saves),
        _ => Err("La carpeta saves no es un directorio regular".into()),
    }
}

pub fn world_dir(saves: &Path, folder: &str) -> Result<PathBuf> {
    validate_filename(folder)?;
    let path = saves.join(folder);
    if !fs::symlink_metadata(&path).map_err(io)?.is_dir() {
        return Err("El mundo no es un directorio regular".into());
    }
    if !regular_file(&path.join("level.dat")) && !regular_file(&path.join("level.dat_old")) {
        return Err("La carpeta no contiene un mundo de Minecraft Java".into());
    }
    Ok(path)
}

pub fn list_worlds(saves: &Path) -> Result<Vec<WorldDto>> {
    let entries = match fs::read_dir(saves) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(io(e)),
    };
    let mut worlds = Vec::new();
    for entry in entries {
        let entry = entry.map_err(io)?;
        if !entry.file_type().map_err(io)?.is_dir() {
            continue;
        }
        let Some(folder) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        if folder.starts_with(".cubic-world-") {
            continue;
        }
        let Ok(path) = world_dir(saves, &folder) else {
            continue;
        };
        let cached = metadata(&path);
        let data = cached.data.clone();
        let seed = data.seed();
        let icon = path.join("icon.png");
        let icon_stamp = stamp(&icon);
        worlds.push(WorldDto {
            name: data.name.unwrap_or_else(|| folder.clone()),
            folder,
            version: data.version.and_then(|v| v.name),
            game_mode: data.game_mode,
            hardcore: data.hardcore == Some(1),
            last_played: data.last_played.filter(|v| *v > 0),
            icon: icon_stamp
                .as_ref()
                .map(|_| icon.to_string_lossy().into_owned()),
            icon_revision: icon_stamp.map(|s| format!("{:?}-{}", s.0, s.1)),
            seed,
            metadata_error: cached.error,
        });
    }
    worlds.sort_by(|a, b| {
        b.last_played
            .cmp(&a.last_played)
            .then_with(|| a.folder.cmp(&b.folder))
    });
    Ok(worlds)
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldProgress {
    pub bytes: u64,
    pub files: u64,
}

pub struct Progress<'a> {
    callback: &'a mut dyn FnMut(WorldProgress),
    last: Instant,
    bytes: u64,
    files: u64,
}

impl<'a> Progress<'a> {
    pub fn new(callback: &'a mut dyn FnMut(WorldProgress)) -> Self {
        Self {
            callback,
            last: Instant::now(),
            bytes: 0,
            files: 0,
        }
    }
    fn add(&mut self, bytes: u64) {
        self.bytes += bytes;
        if self.last.elapsed().as_millis() >= 150 {
            self.flush();
        }
    }
    pub fn flush(&mut self) {
        (self.callback)(WorldProgress {
            bytes: self.bytes,
            files: self.files,
        });
        self.last = Instant::now();
    }
}

fn transfer(
    reader: &mut impl Read,
    writer: &mut impl Write,
    progress: &mut Progress<'_>,
) -> Result<()> {
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let n = reader.read(&mut buffer).map_err(io)?;
        if n == 0 {
            break;
        }
        writer.write_all(&buffer[..n]).map_err(io)?;
        progress.add(n as u64);
    }
    progress.files += 1;
    Ok(())
}

// Iterative, bounded-memory traversal. Never follow symlinks (including directory links).
fn walk(root: &Path, mut visit: impl FnMut(&Path, &Path, bool) -> Result<()>) -> Result<()> {
    let mut stack = vec![fs::read_dir(root).map_err(io)?];
    while let Some(entries) = stack.last_mut() {
        let Some(entry) = entries.next() else {
            stack.pop();
            continue;
        };
        let entry = entry.map_err(io)?;
        let path = entry.path();
        let kind = entry.file_type().map_err(io)?;
        if !kind.is_file() && !kind.is_dir() {
            return Err(format!(
                "No se admiten enlaces ni archivos especiales: {}",
                path.display()
            ));
        }
        let relative = path.strip_prefix(root).map_err(io)?;
        visit(&path, relative, kind.is_dir())?;
        if kind.is_dir() {
            if stack.len() >= 128 {
                return Err("Demasiados niveles de carpetas".into());
            }
            stack.push(fs::read_dir(path).map_err(io)?);
        }
    }
    Ok(())
}

pub fn world_size(world: &Path, progress: &mut Progress<'_>) -> Result<u64> {
    let mut bytes = 0;
    walk(world, |path, _, dir| {
        if !dir {
            let len = fs::metadata(path).map_err(io)?.len();
            bytes += len;
            progress.add(len);
            progress.files += 1;
        }
        Ok(())
    })?;
    Ok(bytes)
}

fn copy_tree(source: &Path, target: &Path, progress: &mut Progress<'_>) -> Result<()> {
    walk(source, |path, relative, dir| {
        let dest = target.join(relative);
        if dir {
            fs::create_dir(&dest).map_err(io)?;
        } else if relative != Path::new("session.lock") {
            let mut output = File::create(dest).map_err(io)?;
            transfer(&mut File::open(path).map_err(io)?, &mut output, progress)?;
        }
        Ok(())
    })
}

fn publish(stage: &Path, saves: &Path, name: &str) -> Result<String> {
    let name = name.trim_end_matches(['.', ' ']);
    let base = if validate_filename(name).is_ok() && name.len() <= 120 && !name.starts_with('.') {
        name.trim_end_matches(['.', ' '])
    } else {
        "World"
    };
    for suffix in 0..10000 {
        let name = if suffix == 0 {
            base.to_string()
        } else {
            format!("{base} ({suffix})")
        };
        let target = saves.join(&name);
        match fs::create_dir(&target) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(io(e)),
        }
        // Reserve the name without clobbering, then move top-level entries on the same filesystem.
        let result = (|| -> Result<()> {
            for entry in fs::read_dir(stage).map_err(io)? {
                let entry = entry.map_err(io)?;
                fs::rename(entry.path(), target.join(entry.file_name())).map_err(io)?;
            }
            Ok(())
        })();
        if let Err(error) = result {
            let _ = fs::remove_dir_all(target);
            return Err(error);
        }
        return Ok(name);
    }
    Err("No se pudo encontrar un nombre de carpeta disponible".into())
}

pub fn duplicate(world: &Path, saves: &Path, progress: &mut Progress<'_>) -> Result<String> {
    let stage = tempfile::Builder::new()
        .prefix(".cubic-world-")
        .tempdir_in(saves)
        .map_err(io)?;
    copy_tree(world, stage.path(), progress)?;
    publish(
        stage.path(),
        saves,
        &world.file_name().unwrap_or_default().to_string_lossy(),
    )
}

pub fn rename_world(world: &Path, name: &str) -> Result<()> {
    let name = name.trim();
    if name.is_empty() || name.len() > 256 || name.chars().any(char::is_control) {
        return Err(
            "El nombre debe tener entre 1 y 256 bytes y no contener caracteres de control".into(),
        );
    }
    let level_path = world.join("level.dat");
    let mut root: HashMap<String, Value> =
        fastnbt::from_bytes(&read_level(&level_path)?).map_err(io)?;
    let Some(Value::Compound(data)) = root.get_mut("Data") else {
        return Err("level.dat no contiene Data".into());
    };
    data.insert("LevelName".into(), Value::String(name.into()));
    let bytes = fastnbt::to_bytes(&root).map_err(io)?;
    let mut temp = tempfile::NamedTempFile::new_in(world).map_err(io)?;
    let mut gzip = GzEncoder::new(temp.as_file_mut(), Compression::fast());
    gzip.write_all(&bytes).map_err(io)?;
    gzip.finish().map_err(io)?;
    temp.as_file().sync_all().map_err(io)?;
    temp.persist(level_path).map_err(io)?;
    CACHE
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .remove(world);
    Ok(())
}

pub fn delete_world(world: &Path) -> Result<()> {
    fs::remove_dir_all(world).map_err(io)?;
    CACHE
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .remove(world);
    Ok(())
}

pub fn copy_world_seed(world: &Path) -> Result<Option<String>> {
    let cached = metadata(world);
    Ok(cached.data.seed())
}

pub fn reset_world_icon(world: &Path) -> Result<bool> {
    let icon = world.join("icon.png");
    if regular_file(&icon) {
        fs::remove_file(&icon).map_err(io)?;
        CACHE
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(world);
        Ok(true)
    } else {
        Ok(false)
    }
}

pub fn export_zip(world: &Path, destination: &Path, progress: &mut Progress<'_>) -> Result<()> {
    if destination.try_exists().map_err(io)? {
        return Err(
            "Ya existe un archivo en ese destino. Elige un nombre nuevo para el ZIP".into(),
        );
    }
    if !destination.is_absolute()
        || !destination
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("zip"))
    {
        return Err("El destino debe ser una ruta absoluta a un ZIP nuevo".into());
    }
    let parent = destination
        .parent()
        .ok_or("Destino inválido")?
        .canonicalize()
        .map_err(io)?;
    if parent.starts_with(world.parent().ok_or("Mundo inválido")?) {
        return Err("Guarda el ZIP fuera de la carpeta saves".into());
    }
    let temp = tempfile::NamedTempFile::new_in(parent).map_err(io)?;
    let mut zip = ZipWriter::new(temp.reopen().map_err(io)?);
    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .compression_level(Some(1))
        .large_file(true);
    walk(world, |path, relative, dir| {
        let name = relative
            .to_str()
            .ok_or("Nombre de archivo no válido")?
            .replace('\\', "/");
        if dir {
            zip.add_directory(format!("{name}/"), options).map_err(io)?;
        } else if relative != Path::new("session.lock") {
            zip.start_file(name, options).map_err(io)?;
            transfer(&mut File::open(path).map_err(io)?, &mut zip, progress)?;
        }
        Ok(())
    })?;
    zip.finish().map_err(io)?.sync_all().map_err(io)?;
    temp.persist_noclobber(destination).map_err(io)?;
    Ok(())
}

fn zip_path(name: &str) -> Result<PathBuf> {
    // Validate both separators on every platform and reject ambiguous Windows names.
    let normalized = name.replace('\\', "/");
    let mut result = PathBuf::new();
    for part in normalized.trim_end_matches('/').split('/') {
        validate_filename(part)?;
        if part.ends_with(['.', ' ']) {
            return Err("Nombre ambiguo en ZIP".into());
        }
        result.push(part);
    }
    Ok(result)
}

pub fn import_world(source: &Path, saves: &Path, progress: &mut Progress<'_>) -> Result<String> {
    let source = source.canonicalize().map_err(io)?;
    let stage = tempfile::Builder::new()
        .prefix(".cubic-world-")
        .tempdir_in(saves)
        .map_err(io)?;
    if source.is_dir() {
        if saves.starts_with(&source) {
            return Err("El origen contiene la carpeta de destino".into());
        }
        if !regular_file(&source.join("level.dat")) && !regular_file(&source.join("level.dat_old"))
        {
            return Err("Selecciona la carpeta que contiene level.dat".into());
        }
        copy_tree(&source, stage.path(), progress)?;
        return publish(
            stage.path(),
            saves,
            &source.file_name().unwrap_or_default().to_string_lossy(),
        );
    }
    let mut archive = ZipArchive::new(File::open(&source).map_err(io)?).map_err(io)?;
    let mut roots = std::collections::HashSet::new();
    for index in 0..archive.len() {
        let file = archive.by_index(index).map_err(io)?;
        let path = zip_path(file.name())?;
        if file.is_symlink() {
            return Err("El ZIP contiene enlaces simbólicos".into());
        }
        if !file.is_dir()
            && path
                .file_name()
                .is_some_and(|n| n == "level.dat" || n == "level.dat_old")
            && !path.components().any(|p| p.as_os_str() == "__MACOSX")
        {
            roots.insert(path.parent().unwrap_or(Path::new("")).to_path_buf());
        }
    }
    if roots.len() != 1 {
        return Err("El ZIP debe contener exactamente un mundo de Minecraft Java".into());
    }
    let root = roots.into_iter().next().unwrap();
    for index in 0..archive.len() {
        let mut file = archive.by_index(index).map_err(io)?;
        let path = zip_path(file.name())?;
        let Ok(relative) = path.strip_prefix(&root) else {
            continue;
        };
        if relative.as_os_str().is_empty() || relative == Path::new("session.lock") {
            continue;
        }
        let dest = stage.path().join(relative);
        if file.is_dir() {
            fs::create_dir_all(dest).map_err(io)?;
        } else {
            fs::create_dir_all(dest.parent().ok_or("Ruta ZIP inválida")?).map_err(io)?;
            let mut output = File::options()
                .write(true)
                .create_new(true)
                .open(dest)
                .map_err(io)?;
            transfer(&mut file, &mut output, progress)?;
        }
    }
    let name = root
        .file_name()
        .or_else(|| source.file_stem())
        .unwrap_or_default()
        .to_string_lossy();
    publish(stage.path(), saves, &name)
}

#[cfg(test)]
#[path = "world_manager_tests.rs"]
mod tests;
