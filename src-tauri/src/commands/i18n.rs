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
    #[serde(skip_serializing_if = "Option::is_none")]
    version: Option<String>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    percentage: Option<f64>,
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

struct ParsedLocale {
    code: String,
    id: String,
    version: Option<String>,
    label: String,
}

// Parses a dictionary once, exposing every field callers need.
fn parse_locale(data: &str) -> Result<ParsedLocale, String> {
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

    let label = metadata
        .languages
        .get(&code)
        .cloned()
        .unwrap_or_else(|| metadata.id.clone());

    Ok(ParsedLocale {
        code,
        id: metadata.id,
        version: metadata.version,
        label,
    })
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
        let parsed = match parse_locale(&data) {
            Ok(parsed) => parsed,
            Err(error) => {
                warn!(path = %original_path.display(), %error, "Ignoring invalid locale file");
                continue;
            }
        };

        let id = parsed.id;
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

        locales.insert(
            id.clone(),
            LocaleEntry {
                label: parsed.label,
                flag: locale_flag(&id),
                code: parsed.code,
                id,
                installed: true,
                percentage: None,
            },
        );
    }

    Ok(locales.into_values().collect())
}

// Writes a dictionary whose identity was already parsed by the caller.
async fn write_locale_file(dir: &Path, data: String, code: &str, id: &str) -> Result<(), String> {
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

    // New data invalidates any previously stored validator.
    remove_etag(dir, id).await;

    Ok(())
}

async fn save_locale_to(dir: &Path, data: String) -> Result<(), String> {
    let parsed = parse_locale(&data)?;
    write_locale_file(dir, data, &parsed.code, &parsed.id).await
}

fn etag_path(dir: &Path, id: &str) -> PathBuf {
    dir.join(format!("{id}.etag"))
}

async fn read_etag(dir: &Path, id: &str) -> Option<String> {
    let raw = fs::read_to_string(etag_path(dir, id)).await.ok()?;
    let tag = raw.trim();
    (!tag.is_empty()).then(|| tag.to_owned())
}

async fn write_etag(dir: &Path, id: &str, etag: &str) -> Result<(), String> {
    let tag = etag.trim();
    if tag.is_empty() {
        return Ok(());
    }
    fs::write(etag_path(dir, id), tag)
        .await
        .map_err(|e| e.to_string())
}

async fn remove_etag(dir: &Path, id: &str) {
    match fs::remove_file(etag_path(dir, id)).await {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => warn!(%error, "Could not remove locale etag"),
    }
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
        let Ok(parsed) = parse_locale(&data) else {
            continue;
        };
        if parsed.code != code {
            continue;
        }
        if path == dir.join(format!("{code}.json")) {
            save_locale_to(dir, data.clone()).await?;
        }
        return Ok(Some(StoredLocale {
            code: parsed.code,
            id: parsed.id,
            version: parsed.version,
            data,
        }));
    }
    Ok(None)
}

fn version_of(locale: &StoredLocale) -> Option<String> {
    locale.version.clone()
}

fn changed_locale(
    locale: Option<StoredLocale>,
    known_version: Option<&str>,
) -> Option<StoredLocale> {
    locale
        .filter(|locale| known_version.is_none() || version_of(locale).as_deref() != known_version)
}

enum Download {
    NotModified,
    Updated(StoredLocale),
}

impl LocaleService {
    async fn load(&self, dir: &Path, code: &str) -> Result<Option<StoredLocale>, String> {
        let _guard = self.disk.lock().await;
        read_locale_from(dir, code).await
    }

    async fn save(&self, dir: &Path, data: String) -> Result<(), String> {
        let parsed = parse_locale(&data)?;
        self.store(dir, data, &parsed.code, &parsed.id).await
    }

    // Persists a dictionary whose identity was already parsed, invalidating the cache.
    async fn store(&self, dir: &Path, data: String, code: &str, id: &str) -> Result<(), String> {
        let mut metadata = self.disk.lock().await;
        write_locale_file(dir, data, code, id).await?;
        *metadata = None;
        Ok(())
    }

    async fn download(
        &self,
        dir: &Path,
        code: &str,
        local: Option<&StoredLocale>,
        if_none_match: Option<&str>,
        client: &reqwest::Client,
        base: &str,
    ) -> Result<Download, String> {
        let mut request = client
            .get(format!("{base}/{code}"))
            .timeout(Duration::from_secs(10));
        if let Some(tag) = if_none_match {
            request = request.header(reqwest::header::IF_NONE_MATCH, tag);
        }

        let response = request.send().await.map_err(|e| e.to_string())?;

        // A 304 has an empty body, so it must be handled before error/body parsing.
        if response.status() == reqwest::StatusCode::NOT_MODIFIED {
            return Ok(Download::NotModified);
        }

        let response = response.error_for_status().map_err(|e| e.to_string())?;

        // Capture the validator before the body is consumed.
        let etag = response
            .headers()
            .get(reqwest::header::ETAG)
            .and_then(|value| value.to_str().ok())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned);

        let data = response.text().await.map_err(|e| e.to_string())?;
        let parsed = parse_locale(&data)?;
        if parsed.code != code {
            return Err("Locale response does not match requested language".into());
        }

        // Rewriting an identical dictionary only burns disk I/O; the validator still refreshes.
        if local.is_none_or(|stored| stored.data != data) {
            self.store(dir, data.clone(), &parsed.code, &parsed.id)
                .await?;
        }
        if let Some(etag) = etag {
            write_etag(dir, &parsed.id, &etag).await?;
        }

        Ok(Download::Updated(StoredLocale {
            code: parsed.code,
            id: parsed.id,
            version: parsed.version,
            data,
        }))
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
            let etag = match local.as_ref() {
                Some(locale) => read_etag(dir, &locale.id).await,
                None => None,
            };

            // A stored validator lets us skip the download entirely on 304.
            if etag.is_some() {
                return match self
                    .download(dir, code, local.as_ref(), etag.as_deref(), client, base)
                    .await?
                {
                    Download::NotModified => Ok(local),
                    Download::Updated(locale) => Ok(Some(locale)),
                };
            }

            // Without a validator, keep the cheap version probe before downloading.
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

            match self
                .download(dir, code, local.as_ref(), None, client, base)
                .await?
            {
                Download::Updated(locale) => Ok(Some(locale)),
                Download::NotModified => Ok(local),
            }
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
