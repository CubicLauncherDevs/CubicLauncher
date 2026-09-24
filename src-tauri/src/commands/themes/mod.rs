use crate::commands::themes::v1::{ThemeEntry, ThemeFile, ThemePreview};
use crate::commands::themes::v2::{ThemeDef, ThemeMeta, V2Theme, flatten_variables};
use crate::core::errors::{CoreError, FsError};
use crate::core::{PathManager, safe_join, sanitize_path, validate_identifier};
use crate::services::SettingsManager;
use crate::theme_watcher::ThemeWatcher;
use compact_str::CompactString;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::exists;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use tauri::command;
use tracing::{error, info, warn};
mod v1;
mod v2;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FontFace {
    pub family: CompactString,
    pub src: CompactString,
    #[serde(default)]
    pub format: Option<CompactString>,
    #[serde(default)]
    pub weight: Option<CompactString>,
    #[serde(default)]
    pub style: Option<CompactString>,
}

#[derive(Debug, Serialize, Clone)]
pub struct ThemeResponse {
    pub name: String,
    pub author: String,
    pub version: String,
    pub r#type: String,
    pub variables: HashMap<String, String>,
    pub bg_image: Option<String>,
    pub bg_music: Option<String>,
    pub bg_image_blur: Option<f64>,
    pub bg_image_opacity: Option<f64>,
    pub fonts: Vec<FontFace>,
    pub icons: HashMap<String, String>,
    pub inject_css: Option<String>,
}

trait Theme {
    fn get_name(&self) -> CompactString;
    fn get_author(&self) -> CompactString;
    fn get_version(&self) -> CompactString; // semver
    fn to_theme_res(&self) -> ThemeResponse;
}

pub(crate) trait ZipImportable: Sized {
    const ZIP_TARGET_FILE: &'static str;
    fn parse_import(content: &str) -> Result<Self, String>;
    fn import_name(&self) -> &str;
    fn import_author(&self) -> &str;
    fn import_version(&self) -> &str;
}

fn build_theme_id(name: &str, author: &str) -> Result<String, String> {
    let normalized_name = name.to_lowercase().replace(' ', "_");
    let theme_id = if author.is_empty() {
        normalized_name
    } else {
        let normalized_author = author.to_lowercase().replace(' ', "_");
        format!("{}_{}", normalized_name, normalized_author)
    };
    validate_identifier(&theme_id)?;
    Ok(theme_id)
}

/// Resolves a theme asset reference relative to the theme base directory.
/// Absolute paths and `file:` URLs are returned as-is; relative paths are
/// validated to avoid directory traversal.
fn resolve_theme_asset(theme_base: &Path, reference: &str) -> Result<Option<PathBuf>, String> {
    if reference.starts_with('/') || reference.starts_with("file:") {
        return Ok(Some(PathBuf::from(reference)));
    }
    if reference.is_empty() {
        return Ok(None);
    }
    Ok(Some(sanitize_path(theme_base, Path::new(reference))?))
}

fn extract_preview(vars: &HashMap<String, String>) -> ThemePreview {
    ThemePreview {
        bg: vars
            .get("--bg-main")
            .or_else(|| vars.get("--bg-card"))
            .or_else(|| vars.get("--bg-sidebar"))
            .cloned()
            .unwrap_or_else(|| "#0c0c0c".into()),
        accent: vars
            .get("--accent")
            .cloned()
            .unwrap_or_else(|| "#ffffff".into()),
        text: vars
            .get("--text-primary")
            .cloned()
            .unwrap_or_else(|| "#d8d8d8".into()),
    }
}

const MAX_ICON_SIZE: u64 = 2 * 1024 * 1024;
const MAX_BG_SIZE: u64 = 25 * 1024 * 1024;

#[derive(Debug)]
enum BgValidationError {
    TooLarge,
    InvalidImage,
    Io,
}

fn validate_background_image(path: &std::path::Path) -> Result<(), BgValidationError> {
    let mut file = std::fs::File::open(path).map_err(|_| BgValidationError::Io)?;
    let meta = file.metadata().map_err(|_| BgValidationError::Io)?;
    if meta.len() > MAX_BG_SIZE {
        return Err(BgValidationError::TooLarge);
    }

    let mut buf = [0u8; 16];
    file.read_exact(&mut buf)
        .map_err(|_| BgValidationError::Io)?;

    if infer::is_image(&buf) {
        Ok(())
    } else {
        Err(BgValidationError::InvalidImage)
    }
}

fn validate_theme_icon(path: &str) -> bool {
    if path.is_empty() {
        return false;
    }
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());

    match ext.as_deref() {
        Some("svg") => std::fs::metadata(path).is_ok(),
        Some("png" | "webp" | "jpg" | "jpeg") => {
            if let Ok(meta) = std::fs::metadata(path)
                && meta.len() > MAX_ICON_SIZE
            {
                warn!("Icono demasiado grande ({} bytes): {}", meta.len(), path);
                return false;
            }
            let is_image = std::fs::File::open(path)
                .ok()
                .and_then(|mut f| {
                    let mut buf = [0u8; 16];
                    f.read_exact(&mut buf).ok()?;
                    Some(infer::is_image(&buf))
                })
                .unwrap_or(false);
            if !is_image {
                warn!("Icono no es una imagen válida: {}", path);
            }
            is_image
        }
        _ => {
            warn!("Extensión de icono no soportada: {}", path);
            false
        }
    }
}

const THEME_MUSIC_FILE: &str = "BG.mp3";

fn find_theme_music(theme_base: &Path, background: Option<&str>) -> Option<String> {
    let mut candidates = Vec::with_capacity(3);

    if let Some(parent) = background
        .map(Path::new)
        .and_then(Path::parent)
        .filter(|parent| parent.starts_with(theme_base))
    {
        candidates.push(parent.join(THEME_MUSIC_FILE));
    }
    candidates.push(theme_base.join("assets").join(THEME_MUSIC_FILE));
    candidates.push(theme_base.join(THEME_MUSIC_FILE));

    candidates.into_iter().find_map(|path| {
        std::fs::symlink_metadata(&path)
            .ok()
            .filter(|metadata| metadata.file_type().is_file())
            .map(|_| path.to_string_lossy().into_owned())
    })
}

// Serialize installation/removal, not extraction. Transactions are siblings of
// installed themes on the same filesystem, with no listable metadata at their root.
static INSTALL_LOCK: parking_lot::Mutex<()> = parking_lot::Mutex::new(());

struct StagedTheme {
    root: PathBuf,
}

impl StagedTheme {
    fn new(themes_dir: &Path) -> Result<Self, String> {
        std::fs::create_dir_all(themes_dir).map_err(|e| e.to_string())?;
        let root = themes_dir.join(format!(".theme-import-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&root).map_err(|e| e.to_string())?;
        let staged = Self { root };
        std::fs::create_dir(staged.root.join("theme")).map_err(|e| e.to_string())?;
        Ok(staged)
    }

    fn install(&self, destination: &Path) -> Result<(), String> {
        let backup = self.root.join("backup");
        let had_previous = match std::fs::symlink_metadata(destination) {
            Ok(meta) if meta.is_dir() => true,
            Ok(_) => {
                return Err(format!(
                    "Destino de theme no es un directorio: {:?}",
                    destination
                ));
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => false,
            Err(e) => return Err(e.to_string()),
        };
        if had_previous {
            std::fs::rename(destination, &backup).map_err(|e| e.to_string())?;
        }
        if let Err(e) = std::fs::rename(self.root.join("theme"), destination) {
            if had_previous && let Err(rollback) = std::fs::rename(&backup, destination) {
                // Never delete the only remaining copy if the filesystem also
                // refuses rollback; report its recovery path to the caller.
                return Err(format!(
                    "Error instalando theme: {e}; error restaurando: {rollback}. Copia conservada en {}",
                    backup.display()
                ));
            }
            return Err(format!("Error instalando theme: {e}"));
        }
        if had_previous && let Err(e) = std::fs::remove_dir_all(&backup) {
            warn!(
                "Theme instalado; no se pudo limpiar backup {:?}: {}",
                backup, e
            );
        }
        Ok(())
    }
}

impl Drop for StagedTheme {
    fn drop(&mut self) {
        if self.root.join("backup").exists() {
            warn!("Backup de theme conservado en {:?}", self.root);
        } else if let Err(e) = std::fs::remove_dir_all(&self.root) {
            warn!("No se pudo limpiar staging {:?}: {}", self.root, e);
        }
    }
}

fn import_zip_inner<T: ZipImportable>(zip_path: &str) -> Result<Option<ThemeEntry>, String> {
    let themes_dir = PathManager::get().get_themes_dir();
    let Some((entry, staged)) = stage_zip::<T>(zip_path, themes_dir)? else {
        return Ok(None);
    };
    let _install = INSTALL_LOCK.lock();
    let destination = themes_dir.join(entry.id.as_str());
    let watch = ThemeWatcher::pause_import(&destination);
    staged.install(&destination)?;
    watch.finish();
    info!("Theme importado: id='{}'", entry.id);
    Ok(Some(entry))
}

fn stage_zip<T: ZipImportable>(
    zip_path: &str,
    themes_dir: &Path,
) -> Result<Option<(ThemeEntry, StagedTheme)>, String> {
    let source = std::path::Path::new(zip_path);
    if !source.exists() {
        return Err(FsError::NotFound(zip_path.to_string()).to_string());
    }

    let file = std::fs::File::open(source).map_err(|e| {
        FsError::ReadFile {
            path: zip_path.to_string(),
            source: e,
        }
        .to_string()
    })?;

    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| CoreError::Other(format!("Archivo ZIP inválido: {}", e)).to_string())?;

    let target = T::ZIP_TARGET_FILE;
    let mut manifest = None;
    for i in 0..archive.len() {
        let entry = archive
            .by_index(i)
            .map_err(|e| CoreError::Other(format!("Error leyendo ZIP: {}", e)).to_string())?;
        // Older Windows exports used native separators, including in wrappers.
        let name = entry.name().replace('\\', "/");
        if name == target || name.ends_with(&format!("/{target}")) {
            if manifest.is_some() {
                return Err(CoreError::Other(format!(
                    "ZIP inválido: múltiples {} encontrados",
                    target
                ))
                .to_string());
            }
            manifest = Some((i, name));
        }
    }
    let Some((entry_index, entry_name)) = manifest else {
        return Ok(None);
    };

    let content = {
        let mut buf = String::new();
        let mut entry = archive.by_index(entry_index).map_err(|e| {
            CoreError::Other(format!("Error leyendo {}: {}", target, e)).to_string()
        })?;
        entry.read_to_string(&mut buf).map_err(|e| {
            CoreError::Other(format!("Error leyendo {}: {}", target, e)).to_string()
        })?;
        buf
    };

    let theme_file: T = T::parse_import(&content)?;

    let (name_str, author_str, version_str) = {
        let n = theme_file.import_name();
        let a = theme_file.import_author();
        let v = theme_file.import_version();
        (n.to_owned(), a.to_owned(), v.to_owned())
    };

    let theme_id = build_theme_id(&name_str, &author_str)?;

    let prefix = if entry_name == target {
        String::new()
    } else {
        entry_name.strip_suffix(target).unwrap_or("").to_string()
    };
    let prefix_path = Path::new(&prefix);
    if !prefix.is_empty() {
        for component in prefix_path.components() {
            if !matches!(component, Component::Normal(_) | Component::CurDir) {
                return Err(
                    CoreError::Other(format!("Prefijo ZIP inválido: '{}'", prefix)).to_string(),
                );
            }
        }
    }
    let prefix_path: PathBuf = prefix_path
        .components()
        .filter_map(|component| match component {
            Component::Normal(name) => Some(name),
            _ => None,
        })
        .collect();

    let staged = StagedTheme::new(themes_dir)?;
    let theme_dir = staged.root.join("theme");
    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| CoreError::Other(format!("Error leyendo ZIP: {}", e)).to_string())?;

        let name = entry.name().replace('\\', "/");
        // Reject drive prefixes and links on every OS. Validate before lexical
        // normalization: ZIP 8's enclosed_name() removes roots and internal `..`.
        if name.contains([':', '\0'])
            || entry
                .unix_mode()
                .is_some_and(|mode| mode & 0o170000 == 0o120000)
        {
            return Err(format!("Entrada ZIP insegura: {}", entry.name()));
        }
        sanitize_path(&theme_dir, Path::new(&name))?;
        let enclosed: PathBuf = Path::new(&name)
            .components()
            .filter_map(|component| match component {
                Component::Normal(name) => Some(name),
                _ => None,
            })
            .collect();

        let relative = match enclosed.strip_prefix(&prefix_path) {
            Ok(r) => r,
            Err(_) => continue,
        };

        if relative.as_os_str().is_empty() {
            continue;
        }

        // Double-check the resolved path stays under the theme directory.
        let out_path = sanitize_path(&theme_dir, relative)?;

        if name.ends_with('/') {
            std::fs::create_dir_all(&out_path)
                .map_err(|e| format!("Error creando directorio {:?}: {}", out_path, e))?;
            continue;
        }
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Error creando directorio {:?}: {}", parent, e))?;
        }

        let mut out_file = std::fs::File::create(&out_path).map_err(|e| {
            FsError::WriteFile {
                path: out_path.to_string_lossy().to_string(),
                source: e,
            }
            .to_string()
        })?;

        std::io::copy(&mut entry, &mut out_file).map_err(|e| {
            FsError::WriteFile {
                path: out_path.to_string_lossy().to_string(),
                source: e,
            }
            .to_string()
        })?;
    }

    load_user_theme(&theme_dir, &theme_id)?;

    Ok(Some((
        ThemeEntry {
            id: theme_id.into(),
            name: name_str.into(),
            author: author_str.to_lowercase().into(),
            version: version_str.into(),
            r#type: "user".into(),
            preview: None,
            icon: None,
        },
        staged,
    )))
}

#[command]
pub fn list_themes() -> Result<Vec<ThemeEntry>, String> {
    let themes_dir = PathManager::get().get_themes_dir();
    let mut themes = Vec::new();

    let entries = match std::fs::read_dir(themes_dir) {
        Ok(e) => e,
        Err(_) => {
            info!("Directorio de themes no encontrado: {:?}", themes_dir);
            return Ok(themes);
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let theme_file = if path.join("theme.json").exists() {
            path.join("theme.json")
        } else if path.join("Meta.toml").exists() {
            path.join("Meta.toml")
        } else {
            continue;
        };
        let id = match path.file_name() {
            Some(name) => name.to_string_lossy().to_string(),
            None => continue,
        };
        let content = match std::fs::read_to_string(&theme_file) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let entry = if theme_file.extension().map(|e| e == "toml").unwrap_or(false) {
            let theme: ThemeMeta = match toml::from_str(&content) {
                Ok(t) => t,
                Err(_) => continue,
            };
            let (preview, icon) = path
                .join("Definition.toml")
                .exists()
                .then(|| -> Option<(Option<ThemePreview>, Option<String>)> {
                    let def_content = std::fs::read_to_string(path.join("Definition.toml")).ok()?;
                    let definitions: ThemeDef = toml::from_str(&def_content).ok()?;
                    let vars = flatten_variables(&definitions);
                    let preview = Some(extract_preview(&vars));
                    let icon = definitions.icons.preview.as_ref().and_then(|icon_path| {
                        let abs = path.join(icon_path);
                        let abs_str = abs.to_string_lossy().to_string();
                        if validate_theme_icon(&abs_str) {
                            Some(abs_str)
                        } else {
                            None
                        }
                    });
                    Some((preview, icon))
                })
                .flatten()
                .unwrap_or((None, None));
            ThemeEntry {
                id: id.into(),
                name: theme.name,
                author: theme.author.to_lowercase(),
                version: theme.version,
                r#type: "v2".into(),
                preview,
                icon,
            }
        } else {
            let theme: ThemeFile = match serde_json::from_str(&content) {
                Ok(t) => t,
                Err(_) => continue,
            };
            let preview = Some(extract_preview(&theme.variables));
            ThemeEntry {
                id: id.into(),
                name: theme.name.to_lowercase(),
                author: theme.author.to_lowercase(),
                version: theme.version,
                r#type: theme.r#type,
                preview,
                icon: None,
            }
        };
        themes.push(entry);
    }

    info!("{} temas listados", themes.len());
    Ok(themes)
}

#[command]
pub fn get_user_theme(id: String) -> Result<ThemeResponse, String> {
    info!("Leyendo theme '{}'", id);
    validate_identifier(&id)?;
    let theme_base = PathManager::get().get_themes_dir().join(&id);
    load_user_theme(&theme_base, &id)
}

fn load_user_theme(theme_base: &Path, id: &str) -> Result<ThemeResponse, String> {
    let meta_path = theme_base.join("Meta.toml");
    let exists_meta_toml = match exists(&meta_path) {
        Ok(e) => e,
        Err(e) => return Err(e.to_string()),
    };

    if exists_meta_toml {
        // Si existe Meta.toml entonces tomamos que el theme es v2
        info!("EL theme {id} tiene Meta.toml, se cargara como V2");
        let meta_bytes = std::fs::read(&meta_path).map_err(|e| FsError::ReadFile {
            path: meta_path.to_string_lossy().into(),
            source: e,
        })?;
        let def_path = theme_base.join("Definition.toml");
        let definition_bytes = std::fs::read(&def_path).map_err(|e| FsError::ReadFile {
            path: def_path.to_string_lossy().into(),
            source: e,
        })?;
        //serializar archivos a toml
        let metadata: ThemeMeta =
            toml::from_slice(&meta_bytes).map_err(|e| CoreError::Serialize(e.to_string()))?;
        let mut definitions: ThemeDef =
            toml::from_slice(&definition_bytes).map_err(|e| CoreError::Serialize(e.to_string()))?;

        //verificar si existe la referencia al backgroudn
        if let Some(ref bg) = definitions.background.reference_path
            && let Some(resolved) = resolve_theme_asset(theme_base, bg.as_ref())?
        {
            definitions.background.reference_path = Some(resolved.to_string_lossy().to_string());
        }

        // validar imagen de fondo
        if let Some(ref bg) = definitions.background.reference_path {
            match validate_background_image(std::path::Path::new(bg)) {
                Ok(()) => {}
                Err(BgValidationError::TooLarge) => {
                    warn!("Theme '{}': background demasiado grande, ignorando", id);
                    definitions.background.reference_path = None;
                }
                Err(BgValidationError::InvalidImage) => {
                    warn!("Theme '{}': bg_image no es una imagen válida", id);
                    definitions.background.reference_path = None;
                }
                Err(BgValidationError::Io) => {
                    definitions.background.reference_path = None;
                }
            }
        }

        // Resolver y validar iconos del theme
        if let Some(ref preview) = definitions.icons.preview
            && let Some(resolved) = resolve_theme_asset(theme_base, preview.as_ref())?
        {
            definitions.icons.preview = Some(resolved.to_string_lossy().to_string());
        }
        if let Some(ref preview) = definitions.icons.preview
            && !validate_theme_icon(preview)
        {
            warn!("Theme '{}': preview icon inválido, ignorando", id);
            definitions.icons.preview = None;
        }

        for items in definitions.icons.groups.values_mut() {
            for path in items.values_mut() {
                if let Some(resolved) = resolve_theme_asset(theme_base, path.as_ref())? {
                    *path = resolved.to_string_lossy().to_string();
                }
            }
        }
        definitions.icons.groups.retain(|group, items| {
            items.retain(|name, path| {
                let valid = validate_theme_icon(path);
                if !valid {
                    warn!(
                        "Theme '{}': icon '{}:{}' inválido, ignorando",
                        id, group, name
                    );
                }
                valid
            });
            !items.is_empty()
        });

        for font in &mut definitions.fonts {
            if let Some(resolved) = resolve_theme_asset(theme_base, font.src.as_ref())? {
                font.src = resolved.to_string_lossy().to_string().into();
            }
        }

        let inject_css_path = theme_base.join("Inject.css");
        let inject = if inject_css_path.exists() {
            let content =
                std::fs::read_to_string(&inject_css_path).map_err(|e| FsError::ReadFile {
                    path: inject_css_path.to_string_lossy().into_owned(),
                    source: e,
                })?;
            info!("Inject.css leido, {} bytes", content.len());
            Some(content)
        } else {
            info!("Inject.css no encontrado en {:?}", inject_css_path);
            None
        };

        let v2 = V2Theme {
            meta: metadata,
            theme: definitions,
        };
        let mut intermediate: ThemeResponse = v2.to_theme_res();
        intermediate.inject_css = inject;
        intermediate.bg_music = find_theme_music(theme_base, intermediate.bg_image.as_deref());
        info!("Theme V2 convertido a intermediario correctamente");
        Ok(intermediate)
    } else {
        // v1
        let theme_json_path = theme_base.join("theme.json");

        let content = std::fs::read_to_string(&theme_json_path).map_err(|e| {
            FsError::ReadFile {
                path: theme_json_path.to_string_lossy().to_string(),
                source: e,
            }
            .to_string()
        })?;

        let mut theme: ThemeFile = serde_json::from_str(&content)
            .map_err(|e| CoreError::Other(format!("Theme '{}' inválido: {}", id, e)).to_string())?;

        // Resolver bg_image relativa al directorio del theme si no es absoluta
        if let Some(ref bg) = theme.bg_image
            && let Some(resolved) = resolve_theme_asset(theme_base, bg.as_ref())?
        {
            theme.bg_image = Some(resolved.to_string_lossy().to_string());
        }

        // Validar imagen de fondo
        if let Some(ref bg) = theme.bg_image {
            match validate_background_image(std::path::Path::new(bg)) {
                Ok(()) => {}
                Err(BgValidationError::TooLarge) => {
                    warn!("Theme '{}': bg_image demasiado grande, ignorando", id);
                    theme.bg_image_warning_key = Some("themes.warning.largeFile".into());
                    theme.bg_image = None;
                }
                Err(BgValidationError::InvalidImage) => {
                    warn!("Theme '{}': bg_image no es una imagen válida", id);
                    theme.bg_image_warning_key = Some("themes.warning.notAnImage".into());
                    theme.bg_image = None;
                }
                Err(BgValidationError::Io) => {
                    theme.bg_image = None;
                }
            }
        }

        // Resolver rutas de fuentes relativas al directorio del theme
        for font in &mut theme.fonts {
            if let Some(resolved) = resolve_theme_asset(theme_base, font.src.as_ref())? {
                font.src = resolved.to_string_lossy().to_string().into();
            }
        }
        let mut intermediate = theme.to_theme_res();
        intermediate.bg_music = find_theme_music(theme_base, intermediate.bg_image.as_deref());
        Ok(intermediate)
    }
}

#[command]
pub async fn set_theme(id: String) -> Result<(), String> {
    info!("Cambiando tema a '{}'", id);
    SettingsManager::write(|s| {
        s.theme = id.clone().into();
    })?;

    SettingsManager::save().await?;

    info!("Tema cambiado a '{}'", id);
    Ok(())
}

#[command]
pub fn get_current_theme() -> Result<String, String> {
    let theme = SettingsManager::read().theme.to_string();
    info!("Tema actual: '{}'", theme);
    Ok(theme)
}

#[command]
pub fn get_themes_dir_path() -> Result<String, String> {
    let path = PathManager::get()
        .get_themes_dir()
        .to_string_lossy()
        .to_string();
    info!("Ruta de directorio de themes: {}", path);
    Ok(path)
}

#[command]
pub fn import_theme(source_path: String) -> Result<ThemeEntry, String> {
    info!("Importando theme desde '{}'", source_path);
    let source = std::path::Path::new(&source_path);
    if !source.exists() {
        error!("Archivo de theme no existe: {}", source_path);
        return Err(FsError::NotFound(source_path.clone()).to_string());
    }

    let content = std::fs::read_to_string(source).map_err(|e| {
        FsError::ReadFile {
            path: source_path.clone(),
            source: e,
        }
        .to_string()
    })?;

    let theme_file: ThemeFile = serde_json::from_str(&content).map_err(|e| {
        CoreError::Other(format!("El archivo no es un theme válido: {}", e)).to_string()
    })?;

    let theme_id = build_theme_id(&theme_file.name, &theme_file.author)?;
    let theme_dir = PathManager::get().get_themes_dir().join(&theme_id);
    let _install = INSTALL_LOCK.lock();

    if theme_dir.exists() {
        error!("El theme '{}' ya existe", theme_file.name);
        return Err(CoreError::Other(format!(
            "Ya existe un theme con el nombre '{}'",
            theme_file.name
        ))
        .to_string());
    }

    let staged = StagedTheme::new(PathManager::get().get_themes_dir())?;
    let staged_dir = staged.root.join("theme");
    let dest_path = staged_dir.join("theme.json");
    std::fs::write(&dest_path, &content).map_err(|e| {
        FsError::WriteFile {
            path: dest_path.to_string_lossy().to_string(),
            source: e,
        }
        .to_string()
    })?;

    // Si el bg_image es una ruta relativa, intentar copiar el archivo
    if let Some(ref bg) = theme_file.bg_image
        && !bg.starts_with('/')
        && !bg.starts_with("file:")
        && let Some(resolved_src) = source.parent().and_then(|p| safe_join(p, bg.as_ref()).ok())
        && let Some(resolved_dest) = resolve_theme_asset(&staged_dir, bg.as_ref())?
    {
        info!("Copiando bg_image a {:?}", resolved_dest);
        if let Err(e) = std::fs::copy(&resolved_src, &resolved_dest) {
            warn!("Error copiando bg_image a {:?}: {}", resolved_dest, e);
        }
    }

    load_user_theme(&staged_dir, &theme_id)?;
    let watch = ThemeWatcher::pause_import(&theme_dir);
    staged.install(&theme_dir)?;
    watch.finish();
    info!(
        "Theme importado: id='{}', name='{}'",
        theme_id, theme_file.name
    );
    let preview = Some(extract_preview(&theme_file.variables));
    Ok(ThemeEntry {
        id: theme_id.into(),
        name: theme_file.name,
        author: theme_file.author.to_lowercase(),
        version: theme_file.version,
        r#type: "user".into(),
        preview,
        icon: None,
    })
}

#[command]
pub fn import_theme_cbth(cbth_path: String) -> Result<ThemeEntry, String> {
    info!("Importando theme CBTH desde '{}'", cbth_path);
    match import_zip_inner::<ThemeMeta>(&cbth_path)? {
        Some(entry) => Ok(entry),
        None => Err(CoreError::Other("ZIP inválido: no se encontró Meta.toml".into()).to_string()),
    }
}

#[tauri::command]
pub fn import_theme_zip(zip_path: String) -> Result<ThemeEntry, String> {
    info!("Importando theme ZIP desde '{}'", zip_path);
    match import_zip_inner::<ThemeFile>(&zip_path)? {
        Some(entry) => Ok(entry),
        None => {
            info!("No se encontró theme.json, intentando como tema V2");
            import_theme_cbth(zip_path)
        }
    }
}

#[command]
pub fn remove_theme(id: String) -> Result<(), String> {
    info!("Eliminando theme '{}'", id);
    validate_identifier(&id)?;
    let _install = INSTALL_LOCK.lock();
    let theme_dir = PathManager::get().get_themes_dir().join(&id);
    if !theme_dir.exists() {
        return Err(FsError::NotFound(theme_dir.to_string_lossy().to_string()).to_string());
    }
    std::fs::remove_dir_all(&theme_dir).map_err(|e| FsError::Remove {
        path: theme_dir.to_string_lossy().to_string(),
        source: e,
    })?;
    info!("Theme '{}' eliminado", id);
    Ok(())
}

#[command]
pub fn export_theme(id: String, dest: String) -> Result<String, String> {
    info!("Exportando theme '{}' a '{}'", id, dest);
    validate_identifier(&id)?;

    let output = std::path::PathBuf::from(&dest);
    if output.file_name().is_none() {
        return Err(FsError::InvalidPath(dest.clone()).to_string());
    }
    if output.extension().map(|e| e != "zip").unwrap_or(true) {
        return Err(CoreError::Other("La exportación debe ser un archivo .zip".into()).to_string());
    }

    // Reject destinations that traverse out of the intended location to avoid
    // silently overwriting arbitrary files.
    for component in output.components() {
        if matches!(component, Component::ParentDir) {
            return Err(FsError::InvalidPath(dest.clone()).to_string());
        }
    }

    let theme_dir = PathManager::get().get_themes_dir().join(&id);
    if !theme_dir.exists() {
        return Err(FsError::NotFound(theme_dir.to_string_lossy().to_string()).to_string());
    }

    export_theme_directory(&theme_dir, &output)?;
    let out_path = output.to_string_lossy().to_string();
    info!("Theme exportado a '{}'", out_path);
    Ok(out_path)
}

fn export_theme_directory(theme_dir: &Path, output: &Path) -> Result<(), String> {
    let file = std::fs::File::create(output).map_err(|e| FsError::WriteFile {
        path: output.to_string_lossy().to_string(),
        source: e,
    })?;
    let mut zip_writer = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    fn add_dir_to_zip(
        zip: &mut zip::ZipWriter<std::fs::File>,
        dir: &std::path::Path,
        prefix: &std::path::Path,
        options: zip::write::SimpleFileOptions,
    ) -> Result<(), String> {
        for entry in std::fs::read_dir(dir).map_err(|e| FsError::ReadDir {
            path: dir.to_string_lossy().to_string(),
            source: e,
        })? {
            let entry = entry.map_err(|e| FsError::ReadDir {
                path: dir.to_string_lossy().to_string(),
                source: e,
            })?;
            let path = entry.path();
            let relative = path
                .strip_prefix(prefix)
                .map_err(|_| CoreError::Other("Error calculando ruta relativa".into()))?;
            if path.is_dir() {
                zip.add_directory(relative.to_string_lossy().replace('\\', "/"), options)
                    .map_err(|e| CoreError::Other(format!("Error agregando directorio: {}", e)))?;
                add_dir_to_zip(zip, &path, prefix, options)?;
            } else {
                let data = std::fs::read(&path).map_err(|e| FsError::ReadFile {
                    path: path.to_string_lossy().to_string(),
                    source: e,
                })?;
                zip.start_file(relative.to_string_lossy().replace('\\', "/"), options)
                    .map_err(|e| CoreError::Other(format!("Error agregando archivo: {}", e)))?;
                std::io::Write::write_all(&mut *zip, &data)
                    .map_err(|e| CoreError::Other(format!("Error escribiendo ZIP: {}", e)))?;
            }
        }
        Ok(())
    }

    add_dir_to_zip(&mut zip_writer, theme_dir, theme_dir, options)?;

    zip_writer
        .finish()
        .map_err(|e| CoreError::Other(format!("Error finalizando ZIP: {}", e)))?;

    Ok(())
}

#[cfg(test)]
#[path = "../../tests/commands/themes/import.rs"]
mod import_tests;
