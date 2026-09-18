use crate::core::http_client::HTTP;
use crate::core::path_manager::PathManager;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::time::{Duration, Instant};
use tokio::fs;
use tokio::sync::Mutex;
use tracing::warn;

const BUNDLED_LOCALES: [(&str, &str); 2] = [
    ("es-ES", include_str!("../../../src/lib/i18n/es-ES.json")),
    ("en-US", include_str!("../../../src/lib/i18n/en-US.json")),
];

#[derive(Debug, Deserialize)]
struct LocaleMetadata {
    id: String,
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    languages: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StoredLocale {
    code: String,
    id: String,
    data: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocaleEntry {
    code: String,
    id: String,
    label: String,
    #[serde(default)]
    flag: String,
    #[serde(default)]
    installed: bool,
}

const API_BASE: &str = "https://i18n.cubiclauncher.org";
const CHECK_TTL: Duration = Duration::from_secs(300);
const RETRY_TTL: Duration = Duration::from_secs(30);
const CATALOG_TTL: Duration = Duration::from_secs(900);
const MAX_CHECKS: usize = 32;

#[derive(Default)]
struct LocaleService {
    // Only metadata is retained here. Local reads never wait for network I/O.
    disk: Mutex<Option<Vec<LocaleEntry>>>,
    checks: Mutex<VecDeque<(String, Instant, bool)>>,
    catalog: Mutex<Option<(Instant, Vec<LocaleEntry>, bool)>>,
}

static LOCALES: LazyLock<LocaleService> = LazyLock::new(LocaleService::default);

fn locales_dir() -> PathBuf {
    PathManager::get().get_settings_dir().join("i18n")
}

fn parse_locale_metadata(data: &str) -> Result<(String, String), String> {
    let metadata: LocaleMetadata =
        serde_json::from_str(data).map_err(|e| format!("Invalid locale JSON: {e}"))?;

    if metadata.id.is_empty()
        || !metadata
            .id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-')
    {
        return Err(format!("Invalid locale id: {}", metadata.id));
    }

    let code = metadata
        .id
        .split('-')
        .next()
        .filter(|code| !code.is_empty())
        .ok_or_else(|| format!("Invalid locale id: {}", metadata.id))?
        .to_ascii_lowercase();

    Ok((code, metadata.id))
}

async fn ensure_bundled_locales(dir: &Path) -> Result<(), String> {
    fs::create_dir_all(dir).await.map_err(|e| e.to_string())?;

    for (id, data) in BUNDLED_LOCALES {
        let path = dir.join(format!("{id}.json"));
        let is_current = fs::read_to_string(&path)
            .await
            .is_ok_and(|stored| stored == data);

        if !is_current {
            fs::write(path, data).await.map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}

async fn read_stored_locales(dir: &Path) -> Result<Vec<LocaleEntry>, String> {
    let mut entries = fs::read_dir(dir).await.map_err(|e| e.to_string())?;
    let mut paths = Vec::new();

    while let Some(entry) = entries.next_entry().await.map_err(|e| e.to_string())? {
        let path = entry.path();
        let is_json = path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("json"));

        if entry
            .file_type()
            .await
            .map_err(|e| e.to_string())?
            .is_file()
            && is_json
        {
            paths.push(path);
        }
    }

    let mut locales = BTreeMap::new();
    for original_path in paths {
        let data = match fs::read_to_string(&original_path).await {
            Ok(data) => data,
            Err(error) => {
                warn!(path = %original_path.display(), %error, "Could not read locale file");
                continue;
            }
        };
        let (code, id) = match parse_locale_metadata(&data) {
            Ok(metadata) => metadata,
            Err(error) => {
                warn!(path = %original_path.display(), %error, "Ignoring invalid locale file");
                continue;
            }
        };

        let canonical_path = dir.join(format!("{id}.json"));
        if original_path != canonical_path {
            if canonical_path.exists() {
                if let Err(error) = fs::remove_file(&original_path).await {
                    warn!(path = %original_path.display(), %error, "Could not remove legacy locale file");
                }
                continue;
            }

            if let Err(error) = fs::rename(&original_path, &canonical_path).await {
                warn!(
                    from = %original_path.display(),
                    to = %canonical_path.display(),
                    %error,
                    "Could not migrate locale file"
                );
            }
        }

        let metadata: LocaleMetadata = serde_json::from_str(&data).map_err(|e| e.to_string())?;
        locales.insert(
            id.clone(),
            LocaleEntry {
                label: metadata
                    .languages
                    .get(&code)
                    .cloned()
                    .unwrap_or_else(|| id.clone()),
                flag: locale_flag(&id),
                code,
                id,
                installed: true,
            },
        );
    }

    Ok(locales.into_values().collect())
}

async fn save_locale_to(dir: &Path, data: String) -> Result<(), String> {
    let (code, id) = parse_locale_metadata(&data)?;
    fs::create_dir_all(dir).await.map_err(|e| e.to_string())?;

    let path = dir.join(format!("{id}.json"));
    // Readers must never observe a partially written dictionary.
    let destination = path.clone();
    let parent = dir.to_path_buf();
    tokio::task::spawn_blocking(move || -> Result<(), String> {
        use std::io::Write;
        let mut staged = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
        staged
            .write_all(data.as_bytes())
            .map_err(|e| e.to_string())?;
        staged.persist(destination).map_err(|e| e.to_string())?;
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())??;

    let legacy_path = dir.join(format!("{code}.json"));
    if legacy_path != path
        && legacy_path.exists()
        && let Err(error) = fs::remove_file(&legacy_path).await
    {
        warn!(path = %legacy_path.display(), %error, "Could not remove legacy locale file");
    }

    Ok(())
}

#[tauri::command]
pub async fn save_locale(data: String) -> Result<(), String> {
    LOCALES.save(&locales_dir(), data).await
}

fn validate_code(code: &str) -> Result<(), String> {
    if !(2..=16).contains(&code.len()) || !code.bytes().all(|b| b.is_ascii_lowercase()) {
        return Err("Invalid locale code".into());
    }
    Ok(())
}

fn locale_flag(id: &str) -> String {
    id.split('-')
        .skip(1)
        .find(|part| part.len() == 2 && part.bytes().all(|b| b.is_ascii_alphabetic()))
        .map(|region| {
            region
                .to_ascii_uppercase()
                .bytes()
                .filter_map(|b| char::from_u32(b as u32 + 127397))
                .collect()
        })
        .unwrap_or_default()
}

async fn read_locale_from(dir: &Path, code: &str) -> Result<Option<StoredLocale>, String> {
    validate_code(code)?;
    let mut entries = match fs::read_dir(dir).await {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.to_string()),
    };
    let mut paths = Vec::new();
    while let Some(entry) = entries.next_entry().await.map_err(|e| e.to_string())? {
        let path = entry.path();
        if path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
            && path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .is_some_and(|stem| {
                    stem.split('-')
                        .next()
                        .is_some_and(|part| part.eq_ignore_ascii_case(code))
                })
            && entry
                .file_type()
                .await
                .map_err(|e| e.to_string())?
                .is_file()
        {
            paths.push(path);
        }
    }
    // Match the previous short-code selection (last canonical ID), before legacy files.
    paths.sort_by(|a, b| {
        let legacy = |p: &Path| p.file_stem().is_some_and(|s| s == code);
        legacy(a).cmp(&legacy(b)).then_with(|| b.cmp(a))
    });
    for path in paths {
        let Ok(data) = fs::read_to_string(&path).await else {
            continue;
        };
        let Ok((found, id)) = parse_locale_metadata(&data) else {
            continue;
        };
        if found != code {
            continue;
        }
        if path == dir.join(format!("{code}.json")) {
            save_locale_to(dir, data.clone()).await?;
        }
        return Ok(Some(StoredLocale {
            code: found,
            id,
            data,
        }));
    }
    Ok(None)
}

fn version_of(locale: &StoredLocale) -> Option<String> {
    serde_json::from_str::<LocaleMetadata>(&locale.data)
        .ok()?
        .version
}

fn changed_locale(
    locale: Option<StoredLocale>,
    known_version: Option<&str>,
) -> Option<StoredLocale> {
    locale
        .filter(|locale| known_version.is_none() || version_of(locale).as_deref() != known_version)
}

impl LocaleService {
    async fn load(&self, dir: &Path, code: &str) -> Result<Option<StoredLocale>, String> {
        let _guard = self.disk.lock().await;
        read_locale_from(dir, code).await
    }

    async fn save(&self, dir: &Path, data: String) -> Result<(), String> {
        let mut metadata = self.disk.lock().await;
        save_locale_to(dir, data).await?;
        *metadata = None;
        Ok(())
    }

    async fn refresh(
        &self,
        dir: &Path,
        code: &str,
        known_version: Option<&str>,
        client: &reqwest::Client,
        base: &str,
    ) -> Result<Option<StoredLocale>, String> {
        validate_code(code)?;
        // Serialize checks across WebViews; dictionaries themselves stay on disk.
        let mut checks = self.checks.lock().await;
        let local = self.load(dir, code).await?;
        if checks.iter().any(|(key, at, success)| {
            key == code && at.elapsed() < if *success { CHECK_TTL } else { RETRY_TTL }
        }) {
            return Ok(changed_locale(local, known_version));
        }
        let result = async {
            if let Some(version) = local.as_ref().and_then(version_of) {
                #[derive(Deserialize)]
                struct Version {
                    version: String,
                }
                let remote: Version = client
                    .get(format!("{base}/{code}/version"))
                    .timeout(Duration::from_secs(10))
                    .send()
                    .await
                    .map_err(|e| e.to_string())?
                    .error_for_status()
                    .map_err(|e| e.to_string())?
                    .json()
                    .await
                    .map_err(|e| e.to_string())?;
                if version == remote.version {
                    return Ok(local);
                }
            }
            let data = client
                .get(format!("{base}/{code}"))
                .timeout(Duration::from_secs(10))
                .send()
                .await
                .map_err(|e| e.to_string())?
                .error_for_status()
                .map_err(|e| e.to_string())?
                .text()
                .await
                .map_err(|e| e.to_string())?;
            let (found, id) = parse_locale_metadata(&data)?;
            if found != code {
                return Err("Locale response does not match requested language".into());
            }
            self.save(dir, data.clone()).await?;
            Ok(Some(StoredLocale {
                code: found,
                id,
                data,
            }))
        }
        .await;
        checks.retain(|(key, _, _)| key != code);
        if checks.len() >= MAX_CHECKS {
            checks.pop_front();
        }
        checks.push_back((code.to_owned(), Instant::now(), result.is_ok()));
        result.map(|locale| changed_locale(locale, known_version))
    }

    async fn list(
        &self,
        dir: &Path,
        client: &reqwest::Client,
        base: &str,
    ) -> Result<Vec<LocaleEntry>, String> {
        let installed = {
            let mut metadata = self.disk.lock().await;
            if metadata.is_none() {
                ensure_bundled_locales(dir).await?;
                *metadata = Some(read_stored_locales(dir).await?);
            }
            metadata.as_ref().unwrap().clone()
        };
        let mut catalog = self.catalog.lock().await;
        if catalog.as_ref().is_none_or(|(at, _, success)| {
            at.elapsed() >= if *success { CATALOG_TTL } else { RETRY_TTL }
        }) {
            let response = async {
                client
                    .get(format!("{base}/locales"))
                    .timeout(Duration::from_secs(10))
                    .send()
                    .await?
                    .error_for_status()?
                    .json::<Vec<LocaleEntry>>()
                    .await
            }
            .await;
            let success = response.is_ok();
            let entries = match response {
                Ok(entries) => entries,
                Err(error) => {
                    warn!(%error, "Could not refresh locale catalog; keeping installed languages");
                    catalog
                        .as_ref()
                        .map(|(_, entries, _)| entries.clone())
                        .unwrap_or_default()
                }
            };
            *catalog = Some((Instant::now(), entries, success));
        }
        let mut combined: Vec<LocaleEntry> = Vec::new();
        for entry in &catalog.as_ref().unwrap().1 {
            if validate_code(&entry.code).is_ok()
                && !combined.iter().any(|existing| existing.code == entry.code)
            {
                let mut entry = entry.clone();
                entry.installed = false;
                combined.push(entry);
            }
        }
        for entry in installed {
            if let Some(remote) = combined.iter_mut().find(|remote| remote.code == entry.code) {
                remote.installed = true;
            } else {
                combined.push(entry);
            }
        }
        Ok(combined)
    }
}

#[tauri::command]
pub async fn load_locale(code: String) -> Result<Option<StoredLocale>, String> {
    LOCALES.load(&locales_dir(), &code).await
}

#[tauri::command]
pub async fn refresh_locale(
    code: String,
    known_version: Option<String>,
) -> Result<Option<StoredLocale>, String> {
    LOCALES
        .refresh(
            &locales_dir(),
            &code,
            known_version.as_deref(),
            &HTTP,
            API_BASE,
        )
        .await
}

#[tauri::command]
pub async fn list_locales() -> Result<Vec<LocaleEntry>, String> {
    LOCALES.list(&locales_dir(), &HTTP, API_BASE).await
}

#[cfg(test)]
#[path = "../tests/commands/i18n.rs"]
mod tests;
