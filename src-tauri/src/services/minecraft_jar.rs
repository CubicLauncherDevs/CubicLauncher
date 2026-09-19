//! Instance-local Minecraft JAR inputs and deterministic jar-mod assembly.
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

pub const INPUT_DIR: &str = "cubic-jar";
const MAX_ENTRY_SIZE: u64 = 256 * 1024 * 1024;
const MAX_ARCHIVE_SIZE: u64 = 2 * 1024 * 1024 * 1024;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct MinecraftJarConfig {
    pub replacement: Option<JarFile>,
    #[serde(default)]
    pub mods: Vec<JarMod>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct JarFile {
    pub file: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct JarMod {
    #[serde(flatten)]
    pub archive: JarFile,
    pub enabled: bool,
}

impl MinecraftJarConfig {
    pub fn files(&self) -> impl Iterator<Item = &JarFile> {
        self.replacement
            .iter()
            .chain(self.mods.iter().map(|m| &m.archive))
    }

    pub fn is_active(&self) -> bool {
        self.replacement.is_some() || self.mods.iter().any(|m| m.enabled)
    }

    pub fn validate(&self) -> Result<(), String> {
        let mut seen = HashSet::new();
        for file in self.files() {
            // Only generated filenames are accepted, including in imported metadata.
            let stem = file
                .file
                .strip_suffix(".jar")
                .ok_or("Nombre de JAR inválido")?;
            uuid::Uuid::parse_str(stem).map_err(|_| "Nombre de JAR inválido")?;
            if !seen.insert(&file.file) {
                return Err("Archivo JAR duplicado en la configuración".into());
            }
        }
        Ok(())
    }
}

fn open_archive(path: &Path) -> Result<zip::ZipArchive<File>, String> {
    let file = File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    zip::ZipArchive::new(file)
        .map_err(|e| format!("Archivo JAR/ZIP inválido ({}): {e}", path.display()))
}

fn entry_name(entry: &zip::read::ZipFile<'_, File>) -> Result<String, String> {
    let name = entry.name();
    if entry.enclosed_name().is_none() || name.contains('\\') || name.contains('\0') {
        return Err(format!("Ruta inválida dentro del JAR: {name}"));
    }
    if entry.size() > MAX_ENTRY_SIZE || entry.is_symlink() {
        return Err(format!("Entrada JAR no admitida: {name}"));
    }
    Ok(name.to_owned())
}

pub fn validate_archive(path: &Path, replacement: bool) -> Result<(), String> {
    let mut archive = open_archive(path)?;
    let mut size = 0_u64;
    let mut files = 0;
    let mut classes = false;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        let name = entry_name(&entry)?;
        if entry.is_dir() {
            continue;
        }
        size = size
            .checked_add(entry.size())
            .ok_or("JAR demasiado grande")?;
        if size > MAX_ARCHIVE_SIZE {
            return Err("El contenido descomprimido del JAR supera 2 GiB".into());
        }
        // Read through the CRC check before accepting or replacing an existing input.
        std::io::copy(&mut entry, &mut std::io::sink()).map_err(|e| e.to_string())?;
        files += 1;
        classes |= name.ends_with(".class");
    }
    if files == 0 || (replacement && !classes) {
        return Err("El archivo está vacío o no contiene clases de Minecraft".into());
    }
    Ok(())
}

pub fn import_file(
    instance_dir: &Path,
    source: &Path,
    replacement: bool,
) -> Result<JarFile, String> {
    let ext = source
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or_default();
    if !ext.eq_ignore_ascii_case("jar") && (replacement || !ext.eq_ignore_ascii_case("zip")) {
        return Err("Selecciona un archivo .jar (o .zip para mods)".into());
    }
    let dir = instance_dir.join(INPUT_DIR);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let mut temp = tempfile::NamedTempFile::new_in(&dir).map_err(|e| e.to_string())?;
    let mut input = File::open(source).map_err(|e| e.to_string())?;
    std::io::copy(&mut input, &mut temp).map_err(|e| e.to_string())?;
    validate_archive(temp.path(), replacement)?;
    let file = format!("{}.jar", uuid::Uuid::new_v4());
    temp.persist(dir.join(&file)).map_err(|e| e.to_string())?;
    Ok(JarFile {
        file,
        name: source
            .file_name()
            .ok_or("Archivo sin nombre")?
            .to_string_lossy()
            .into_owned(),
    })
}

fn is_signature(name: &str) -> bool {
    let upper = name.to_ascii_uppercase();
    let Some(tail) = upper.strip_prefix("META-INF/") else {
        return false;
    };
    !tail.contains('/')
        && (tail == "MANIFEST.MF"
            || tail.starts_with("SIG-")
            || [".SF", ".RSA", ".DSA", ".EC"]
                .iter()
                .any(|ext| tail.ends_with(ext)))
}

/// Later enabled mods win. Rebuild on every launch, so repairs/version changes
/// cannot leave a stale patched JAR. The shared source is always read-only.
pub fn prepare(
    instance_dir: &Path,
    original: &Path,
    config: &MinecraftJarConfig,
) -> Result<Option<PathBuf>, String> {
    config.validate()?;
    if !config.is_active() {
        return Ok(None);
    }
    let dir = instance_dir.join(INPUT_DIR);
    let base = config
        .replacement
        .as_ref()
        .map(|f| dir.join(&f.file))
        .unwrap_or_else(|| original.to_owned());
    if !config.mods.iter().any(|m| m.enabled) {
        validate_archive(&base, true)?;
        return Ok(Some(base));
    }
    let sources: Vec<PathBuf> = std::iter::once(base)
        .chain(
            config
                .mods
                .iter()
                .filter(|m| m.enabled)
                .map(|m| dir.join(&m.archive.file)),
        )
        .collect();
    let mut archives = sources
        .iter()
        .map(|p| open_archive(p))
        .collect::<Result<Vec<_>, _>>()?;
    let mut entries = BTreeMap::new();
    for (source, archive) in archives.iter_mut().enumerate() {
        for index in 0..archive.len() {
            let entry = archive.by_index(index).map_err(|e| e.to_string())?;
            let name = entry_name(&entry)?;
            if !entry.is_dir() && !is_signature(&name) {
                entries.insert(name, (source, index));
            }
        }
    }
    let output_dir = instance_dir.join("cubic-jar-cache");
    std::fs::create_dir_all(&output_dir).map_err(|e| e.to_string())?;
    let mut temp = tempfile::NamedTempFile::new_in(&output_dir).map_err(|e| e.to_string())?;
    let mut writer = zip::ZipWriter::new(temp.as_file_mut());
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    let mut total = 0_u64;
    for (name, (source, index)) in entries {
        let mut entry = archives[source]
            .by_index(index)
            .map_err(|e| e.to_string())?;
        total = total
            .checked_add(entry.size())
            .ok_or("JAR demasiado grande")?;
        if total > MAX_ARCHIVE_SIZE {
            return Err("El JAR combinado supera 2 GiB".into());
        }
        writer
            .start_file(name, options)
            .map_err(|e| e.to_string())?;
        std::io::copy(&mut entry, &mut writer).map_err(|e| e.to_string())?;
    }
    writer.finish().map_err(|e| e.to_string())?;
    temp.flush().map_err(|e| e.to_string())?;
    let output = output_dir.join("minecraft.jar");
    temp.persist(&output).map_err(|e| e.to_string())?;
    Ok(Some(output))
}

/// Copy only the referenced, validated inputs, never the generated cache.
pub fn copy_inputs(
    source: &Path,
    target: &Path,
    config: &MinecraftJarConfig,
) -> Result<(), String> {
    config.validate()?;
    for file in config.files() {
        let src = source.join(INPUT_DIR).join(&file.file);
        validate_archive(&src, config.replacement.as_ref() == Some(file))?;
        let dir = target.join(INPUT_DIR);
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        std::fs::copy(src, dir.join(&file.file)).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "../tests/services/minecraft_jar.rs"]
mod tests;
