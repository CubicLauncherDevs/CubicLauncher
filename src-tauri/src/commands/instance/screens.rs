use crate::core::{PathManager, validate_filename};
use crate::services::InstanceManager;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use std::collections::VecDeque;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::SystemTime;
use tokio::sync::Semaphore;
use tracing::{info, warn};

use super::launch::validate_uuid;

fn screenshots_dir(instance_name: &str) -> Option<PathBuf> {
    validate_filename(instance_name).ok()?;
    Some(
        PathManager::get()
            .get_instance_dir()
            .join(instance_name)
            .join("screenshots"),
    )
}

// Capture metadata once per entry, never inside a sort comparator.
fn screenshot_entries(dir: &Path) -> impl Iterator<Item = (SystemTime, PathBuf)> {
    std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if !path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("png"))
            {
                return None;
            }
            let metadata = path.metadata().ok()?;
            metadata
                .is_file()
                .then(|| (metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH), path))
        })
}

fn sorted_screenshots(dir: &Path) -> Vec<String> {
    let mut screenshots: Vec<_> = screenshot_entries(dir).collect();
    screenshots.sort_unstable_by(|a, b| b.cmp(a));
    screenshots
        .into_iter()
        .map(|(_, path)| path.to_string_lossy().into_owned())
        .collect()
}

fn latest_screenshot(dir: &Path) -> Option<String> {
    screenshot_entries(dir)
        .max()
        .map(|(_, path)| path.to_string_lossy().into_owned())
}

#[tauri::command]
pub async fn get_instance_screenshot(instance_name: String) -> Option<String> {
    let dir = screenshots_dir(&instance_name)?;
    tokio::task::spawn_blocking(move || latest_screenshot(&dir))
        .await
        .ok()
        .flatten()
}

#[tauri::command]
pub async fn get_all_instance_screenshots(instance_name: String) -> Vec<String> {
    let Some(dir) = screenshots_dir(&instance_name) else {
        return Vec::new();
    };
    tokio::task::spawn_blocking(move || sorted_screenshots(&dir))
        .await
        .unwrap_or_default()
}

const THUMBNAIL_WIDTH: u32 = 400;
const THUMBNAIL_HEIGHT: u32 = 225;
const THUMBNAIL_CACHE_BYTES: usize = 4 * 1024 * 1024;
const THUMBNAIL_CACHE_ENTRIES: usize = 64;
// PNG decoding needs the source pixels temporarily. Only one decoder may run,
// including across windows, and it must never block an async runtime thread.
static THUMBNAIL_WORKERS: Semaphore = Semaphore::const_new(1);
static THUMBNAILS: LazyLock<Mutex<ThumbnailCache>> =
    LazyLock::new(|| Mutex::new(ThumbnailCache::default()));

type FileStamp = (SystemTime, u64);

struct CachedThumbnail {
    path: PathBuf,
    stamp: FileStamp,
    data: Arc<str>,
}

#[derive(Default)]
struct ThumbnailCache {
    entries: VecDeque<CachedThumbnail>,
    bytes: usize,
}

impl ThumbnailCache {
    fn get(&mut self, path: &Path, stamp: FileStamp) -> Option<Arc<str>> {
        let index = self.entries.iter().position(|entry| entry.path == path)?;
        let entry = self.entries.remove(index)?;
        if entry.stamp != stamp {
            self.bytes -= entry.data.len();
            return None;
        }
        let data = entry.data.clone();
        self.entries.push_back(entry);
        Some(data)
    }

    fn insert(&mut self, path: PathBuf, stamp: FileStamp, data: Arc<str>) {
        if data.len() > THUMBNAIL_CACHE_BYTES {
            return;
        }
        while self.entries.len() >= THUMBNAIL_CACHE_ENTRIES
            || self.bytes + data.len() > THUMBNAIL_CACHE_BYTES
        {
            if let Some(entry) = self.entries.pop_front() {
                self.bytes -= entry.data.len();
            }
        }
        self.bytes += data.len();
        self.entries
            .push_back(CachedThumbnail { path, stamp, data });
    }
}

fn file_stamp(path: &Path) -> Option<FileStamp> {
    let metadata = path.metadata().ok()?;
    metadata
        .is_file()
        .then_some((metadata.modified().ok()?, metadata.len()))
}

fn thumbnail_path(dir: &Path, filename: &str) -> Option<PathBuf> {
    validate_filename(filename).ok()?;
    if !Path::new(filename)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("png"))
    {
        return None;
    }
    let dir = dir.canonicalize().ok()?;
    let path = dir.join(filename).canonicalize().ok()?;
    path.starts_with(&dir).then_some(path)
}

fn make_thumbnail(path: &Path) -> Option<Arc<str>> {
    let mut reader = image::ImageReader::open(path).ok()?;
    reader.set_format(image::ImageFormat::Png);
    let mut limits = image::Limits::default();
    limits.max_alloc = Some(128 * 1024 * 1024);
    reader.limits(limits);
    let decoded = reader.decode().ok()?;
    let preview = decoded.thumbnail(THUMBNAIL_WIDTH, THUMBNAIL_HEIGHT);
    drop(decoded);
    let mut png = Cursor::new(Vec::new());
    preview.write_to(&mut png, image::ImageFormat::Png).ok()?;
    Some(Arc::from(format!(
        "data:image/png;base64,{}",
        STANDARD.encode(png.into_inner())
    )))
}

fn cached_thumbnail(dir: &Path, filename: &str, cache: &Mutex<ThumbnailCache>) -> Option<Arc<str>> {
    let path = thumbnail_path(dir, filename)?;
    let stamp = file_stamp(&path)?;
    if let Some(data) = cache
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(&path, stamp)
    {
        return Some(data);
    }
    let data = make_thumbnail(&path)?;
    // A screenshot can be replaced while it is decoded; don't cache that result.
    if file_stamp(&path) != Some(stamp) {
        return None;
    }
    cache
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(path, stamp, data.clone());
    Some(data)
}

#[tauri::command]
pub async fn get_screenshot_thumbnail(
    instance_id: String,
    filename: String,
) -> Result<Option<Arc<str>>, String> {
    validate_uuid(&instance_id)?;
    validate_filename(&filename)?;
    let handle = InstanceManager::get()
        .get_handle(&instance_id)
        .await
        .ok_or("Instancia no encontrada")?;
    let dir = handle.get_instance_dir().await.join("screenshots");
    let permit = THUMBNAIL_WORKERS
        .acquire()
        .await
        .map_err(|e| e.to_string())?;
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        cached_thumbnail(&dir, &filename, &THUMBNAILS)
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_instance_cover_image(instance_id: String, path: String) {
    if let Err(e) = validate_uuid(&instance_id) {
        warn!("{}", e);
        return;
    }
    info!(
        "Estableciendo cover image para instancia {}: {}",
        instance_id, path
    );
    let manager = InstanceManager::get();
    if let Some(handle) = manager.get_handle(&instance_id).await {
        if handle.is_busy() {
            warn!(
                "Intento de establecer cover image en instancia ocupada {}",
                instance_id
            );
            return;
        }
        handle.set_cover_image(Some(PathBuf::from(path))).await;
        if let Err(e) = handle.save_if_dirty().await {
            warn!(
                "Error guardando cover image de instancia {}: {:?}",
                instance_id, e
            );
        }
    } else {
        warn!(
            "Instancia {} no encontrada para establecer cover image",
            instance_id
        );
    }
}

#[tauri::command]
pub async fn reset_instance_cover_image(instance_id: String) {
    if let Err(e) = validate_uuid(&instance_id) {
        warn!("{}", e);
        return;
    }
    info!("Reseteando cover image para instancia {}", instance_id);
    let manager = InstanceManager::get();
    if let Some(handle) = manager.get_handle(&instance_id).await {
        if handle.is_busy() {
            warn!(
                "Intento de resetear cover image en instancia ocupada {}",
                instance_id
            );
            return;
        }
        handle.set_cover_image(None).await;
        if let Err(e) = handle.save_if_dirty().await {
            warn!(
                "Error guardando reset cover image de instancia {}: {:?}",
                instance_id, e
            );
        }
    } else {
        warn!(
            "Instancia {} no encontrada para resetear cover image",
            instance_id
        );
    }
}

#[tauri::command]
pub async fn get_instance_banner(instance_id: String) -> Option<String> {
    if validate_uuid(&instance_id).is_err() {
        return None;
    }
    let manager = InstanceManager::get();
    let handle = manager.get_handle(&instance_id).await?;

    if let Some(path) = handle.get_cover_image().await {
        return Some(path.to_string_lossy().to_string());
    }

    get_instance_screenshot(handle.get_name().await.to_string()).await
}

#[cfg(test)]
#[path = "../../tests/commands/instance/screens.rs"]
mod tests;
