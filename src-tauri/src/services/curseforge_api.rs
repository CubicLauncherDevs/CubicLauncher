use std::collections::HashMap;
use std::sync::LazyLock;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::core::http_client::HTTP;
use crate::services::SettingsManager;

const CURSEFORGE_API_BASE: &str = "https://api.curseforge.com/v1";
const MINECRAFT_GAME_ID: u32 = 432;

const CACHE_TTL: Duration = Duration::from_secs(300);
const CACHE_MAX: usize = 200;

pub const MODS_CLASS_ID: u32 = 6;
pub const MODPACKS_CLASS_ID: u32 = 4471;
pub const RESOURCE_PACKS_CLASS_ID: u32 = 12;
pub const SHADERS_CLASS_ID: u32 = 6552;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CurseForgeLoader {
    Forge = 1,
    Cauldron = 2,
    LiteLoader = 3,
    Fabric = 4,
    Quilt = 5,
    NeoForge = 6,
}

impl CurseForgeLoader {
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "forge" => Some(Self::Forge),
            "cauldron" | "mcpc" => Some(Self::Cauldron),
            "liteloader" => Some(Self::LiteLoader),
            "fabric" => Some(Self::Fabric),
            "quilt" => Some(Self::Quilt),
            "neoforge" => Some(Self::NeoForge),
            _ => None,
        }
    }

    pub fn id(self) -> u32 {
        self as u32
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CurseForgeSortField {
    Featured = 1,
    Popularity = 2,
    LastUpdated = 3,
    Name = 4,
    Author = 5,
    TotalDownloads = 6,
    Category = 7,
    GameVersion = 8,
    EarlyAccess = 9,
    FeaturedReleased = 10,
    ReleasedDate = 11,
    Rating = 12,
}

impl CurseForgeSortField {
    pub fn from_index(index: &str) -> Self {
        match index {
            "newest" => Self::LastUpdated,
            "downloads" => Self::TotalDownloads,
            _ => Self::Popularity,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeAuthor {
    pub name: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub project_id: Option<u32>,
    #[serde(default)]
    pub id: Option<u32>,
    #[serde(default)]
    pub avatar_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeLogo {
    pub id: u32,
    #[serde(default)]
    pub mod_id: Option<u32>,
    pub title: String,
    pub description: String,
    pub thumbnail_url: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeCategory {
    pub id: u32,
    #[serde(default)]
    pub game_id: Option<u32>,
    pub name: String,
    pub slug: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub icon_url: Option<String>,
    #[serde(default)]
    pub date_modified: Option<String>,
    #[serde(default)]
    pub is_class: Option<bool>,
    #[serde(default)]
    pub class_id: Option<u32>,
    #[serde(default)]
    pub parent_category_id: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeFileIndex {
    pub game_version: String,
    pub file_id: u32,
    pub filename: String,
    pub release_type: u32,
    #[serde(default)]
    pub mod_loader: Option<u32>,
    #[serde(default)]
    pub game_version_type_id: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeFileHash {
    pub value: String,
    pub algo: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeFileModule {
    pub name: String,
    pub fingerprint: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeSortableGameVersion {
    pub game_version_name: String,
    pub game_version_padded: String,
    pub game_version: String,
    pub game_version_release_date: String,
    #[serde(default)]
    pub game_version_type_id: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeFile {
    pub id: u32,
    #[serde(default)]
    pub game_id: Option<u32>,
    #[serde(default)]
    pub mod_id: Option<u32>,
    pub is_available: bool,
    pub display_name: String,
    pub file_name: String,
    pub release_type: u32,
    #[serde(default)]
    pub file_status: u32,
    pub hashes: Vec<CurseForgeFileHash>,
    pub file_date: String,
    pub file_length: u64,
    pub download_count: u64,
    pub download_url: Option<String>,
    pub game_versions: Vec<String>,
    pub sortable_game_versions: Option<Vec<CurseForgeSortableGameVersion>>,
    pub dependencies: Vec<CurseForgeFileDependency>,
    #[serde(default)]
    pub alternate_file_id: u32,
    #[serde(default)]
    pub is_server_pack: bool,
    #[serde(default)]
    pub file_fingerprint: u64,
    pub modules: Option<Vec<CurseForgeFileModule>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeFileDependency {
    #[serde(default)]
    pub mod_id: Option<u32>,
    pub relation_type: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeProject {
    pub id: u32,
    #[serde(default)]
    pub game_id: Option<u32>,
    pub name: String,
    pub slug: String,
    #[serde(default)]
    pub links: Option<CurseForgeProjectLinks>,
    pub summary: String,
    pub status: u32,
    pub download_count: u64,
    pub is_featured: bool,
    pub primary_category_id: u32,
    pub categories: Vec<CurseForgeCategory>,
    #[serde(default)]
    pub class_id: Option<u32>,
    pub authors: Vec<CurseForgeAuthor>,
    #[serde(default)]
    pub logo: Option<CurseForgeLogo>,
    #[serde(default)]
    pub screenshots: Vec<CurseForgeScreenshot>,
    pub main_file_id: u32,
    pub latest_files: Vec<CurseForgeFile>,
    pub latest_files_indexes: Vec<CurseForgeFileIndex>,
    pub date_created: String,
    pub date_modified: String,
    pub date_released: String,
    #[serde(default)]
    pub allow_mod_distribution: bool,
    #[serde(default)]
    pub game_popularity_rank: Option<u64>,
    pub is_available: bool,
    #[serde(default)]
    pub thumbs_up_count: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeProjectLinks {
    #[serde(default)]
    pub website_url: Option<String>,
    #[serde(default)]
    pub wiki_url: Option<String>,
    #[serde(default)]
    pub issues_url: Option<String>,
    #[serde(default)]
    pub source_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeScreenshot {
    pub id: u32,
    #[serde(default)]
    pub mod_id: Option<u32>,
    pub title: String,
    pub description: String,
    pub thumbnail_url: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgePagination {
    pub index: u32,
    pub page_size: u32,
    pub result_count: u32,
    pub total_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeSearchResponse {
    pub data: Vec<CurseForgeProject>,
    pub pagination: CurseForgePagination,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeFilesResponse {
    pub data: Vec<CurseForgeFile>,
    pub pagination: CurseForgePagination,
}

#[derive(Debug, thiserror::Error)]
pub enum CurseForgeError {
    #[error("CurseForge API key is not configured")]
    MissingApiKey,
    #[error("HTTP request failed: {0}")]
    Request(#[from] reqwest::Error),
    #[error("API error {status}: {body}")]
    Api { status: u16, body: String },
    #[error("Failed to parse response from {url}: {source}. Body: {body}")]
    ParseWithContext {
        source: serde_json::Error,
        url: String,
        body: String,
    },
    #[error("Failed to parse response: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("Download URL not found")]
    DownloadUrlNotFound,
}

impl From<CurseForgeError> for String {
    fn from(e: CurseForgeError) -> Self {
        e.to_string()
    }
}

struct CacheEntry {
    data: serde_json::Value,
    fetched_at: Instant,
}

struct ApiCache {
    entries: Mutex<HashMap<String, CacheEntry>>,
}

impl ApiCache {
    fn get(&self, key: &str) -> Option<serde_json::Value> {
        let mut map = self.entries.lock();
        if let Some(entry) = map.get(key) {
            if entry.fetched_at.elapsed() < CACHE_TTL {
                return Some(entry.data.clone());
            }
            map.remove(key);
        }
        None
    }

    fn set(&self, key: String, data: serde_json::Value) {
        let mut map = self.entries.lock();
        if map.len() >= CACHE_MAX
            && let Some(oldest) = map
                .iter()
                .min_by_key(|(_, e)| e.fetched_at)
                .map(|(k, _)| k.clone())
        {
            map.remove(&oldest);
        }
        map.insert(
            key,
            CacheEntry {
                data,
                fetched_at: Instant::now(),
            },
        );
    }
}

static CACHE: LazyLock<ApiCache> = LazyLock::new(|| ApiCache {
    entries: Mutex::new(HashMap::new()),
});

#[derive(Clone)]
pub struct CurseForgeClient {
    api_key: String,
}

impl CurseForgeClient {
    /// Creates a client using the configured API key. If no key is configured,
    /// returns a `MissingApiKey` error.
    pub fn from_settings() -> Result<Self, CurseForgeError> {
        let key = SettingsManager::read()
            .curseforge_api_key
            .as_ref()
            .map(|s| s.to_string());
        key.map(|api_key| Self { api_key })
            .ok_or(CurseForgeError::MissingApiKey)
    }

    /// Creates a client from settings if available, otherwise falls back to the
    /// built-in default key. This preserves compatibility while allowing users to
    /// provide their own key.
    pub fn from_settings_or_default() -> Self {
        Self::from_settings().unwrap_or_else(|_| Self::default())
    }

    /// Creates a client with an explicit API key.
    pub fn new(api_key: String) -> Self {
        Self { api_key }
    }

    pub fn default_api_key() -> &'static str {
        "$2a$10$v4G8m2LV2QhjUu5l.G24Ieqdp4JTEEQ6bRsZjvpa0YncCVaDaqBP6"
    }

    fn api_key(&self) -> &str {
        &self.api_key
    }

    fn build_request(&self, method: reqwest::Method, url: &str) -> reqwest::RequestBuilder {
        HTTP.request(method, url)
            .header("x-api-key", self.api_key())
            .header("Accept", "application/json")
    }

    async fn get_json<T: serde::de::DeserializeOwned>(
        &self,
        url: &str,
    ) -> Result<T, CurseForgeError> {
        if let Some(cached) = CACHE.get(url) {
            return Ok(serde_json::from_value(cached)?);
        }

        let resp = self.build_request(reqwest::Method::GET, url).send().await?;
        let status = resp.status();

        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            tracing::error!(
                url = %url,
                status = %status,
                body = %body,
                "CurseForge API request failed"
            );
            return Err(CurseForgeError::Api {
                status: status.as_u16(),
                body,
            });
        }

        let raw: serde_json::Value = resp.json().await?;
        CACHE.set(url.to_string(), raw.clone());
        serde_json::from_value(raw.clone()).map_err(|e| {
            let full_body = raw.to_string();
            tracing::error!(
                url = %url,
                error = %e,
                body = %full_body,
                "Failed to parse CurseForge response"
            );
            CurseForgeError::ParseWithContext {
                source: e,
                url: url.to_string(),
                body: full_body.chars().take(1000).collect(),
            }
        })
    }

    fn encode_param(value: &str) -> String {
        urlencoding::encode(value).into_owned()
    }
}

impl Default for CurseForgeClient {
    fn default() -> Self {
        Self::new(Self::default_api_key().to_string())
    }
}

#[derive(Debug, Clone)]
pub struct SearchParams<'a> {
    pub query: &'a str,
    pub loader: Option<&'a str>,
    pub game_version: Option<&'a str>,
    pub category_id: Option<u32>,
    pub sort_field: CurseForgeSortField,
    pub sort_order: SortOrder,
    pub index: u32,
    pub page_size: u32,
    pub class_id: u32,
}

#[derive(Debug, Clone, Copy)]
pub enum SortOrder {
    Desc,
}

impl SortOrder {
    pub fn as_str(&self) -> &'static str {
        "desc"
    }
}

impl CurseForgeClient {
    #[allow(clippy::too_many_arguments)]
    pub async fn search(
        &self,
        params: SearchParams<'_>,
    ) -> Result<CurseForgeSearchResponse, CurseForgeError> {
        let mut url = format!("{}/mods/search", CURSEFORGE_API_BASE);
        let mut first = true;

        let mut append = |name: &str, value: &str| {
            if first {
                url.push('?');
                first = false;
            } else {
                url.push('&');
            }
            url.push_str(name);
            url.push('=');
            url.push_str(value);
        };

        append("gameId", &MINECRAFT_GAME_ID.to_string());
        if !params.query.is_empty() {
            append("searchFilter", &Self::encode_param(params.query));
        }
        append("pageSize", &params.page_size.min(50).to_string());
        append("index", &params.index.to_string());
        append("classId", &params.class_id.to_string());

        let loader_lower = params.loader.map(|s| s.to_lowercase());
        let skip_loader_filter = params.class_id == MODPACKS_CLASS_ID;

        if let Some(loader) = loader_lower
            && loader != "vanilla"
            && !skip_loader_filter
            && let Some(cf_loader) = CurseForgeLoader::from_name(&loader)
        {
            append("modLoaderType", &cf_loader.id().to_string());
        }
        if let Some(gv) = params.game_version
            && !gv.is_empty()
            && !skip_loader_filter
        {
            append("gameVersion", &Self::encode_param(gv));
        }
        if let Some(cat) = params.category_id {
            append("categoryId", &cat.to_string());
        }

        append("sortField", &(params.sort_field as u32).to_string());
        append("sortOrder", params.sort_order.as_str());

        self.get_json(&url).await
    }

    /// Search modpacks.
    #[allow(clippy::too_many_arguments)]
    pub async fn search_modpacks(
        &self,
        query: &str,
        loader: Option<&str>,
        game_version: Option<&str>,
        sort_field: CurseForgeSortField,
        index: u32,
        page_size: u32,
    ) -> Result<CurseForgeSearchResponse, CurseForgeError> {
        self.search(SearchParams {
            query,
            loader,
            game_version,
            category_id: None,
            sort_field,
            sort_order: SortOrder::Desc,
            index,
            page_size,
            class_id: MODPACKS_CLASS_ID,
        })
        .await
    }

    /// Search mods.
    #[allow(dead_code)]
    #[allow(clippy::too_many_arguments)]
    pub async fn search_mods(
        &self,
        query: &str,
        loader: Option<&str>,
        game_version: Option<&str>,
        category_id: Option<u32>,
        sort_field: CurseForgeSortField,
        index: u32,
        page_size: u32,
    ) -> Result<CurseForgeSearchResponse, CurseForgeError> {
        self.search(SearchParams {
            query,
            loader,
            game_version,
            category_id,
            sort_field,
            sort_order: SortOrder::Desc,
            index,
            page_size,
            class_id: MODS_CLASS_ID,
        })
        .await
    }

    pub async fn get_project(&self, mod_id: u32) -> Result<CurseForgeProject, CurseForgeError> {
        let url = format!("{}/mods/{}", CURSEFORGE_API_BASE, mod_id);
        let resp: serde_json::Value = self.get_json(&url).await?;
        let data = resp.get("data").cloned().unwrap_or(resp);
        Ok(serde_json::from_value(data)?)
    }

    /// Fetches the full HTML description (README) of a project.
    pub async fn get_project_description(&self, mod_id: u32) -> Result<String, CurseForgeError> {
        let url = format!("{}/mods/{}/description", CURSEFORGE_API_BASE, mod_id);
        let resp: serde_json::Value = self.get_json(&url).await?;
        let data = resp.get("data").cloned().unwrap_or(resp);
        Ok(serde_json::from_value(data)?)
    }

    pub async fn get_projects(
        &self,
        mod_ids: &[u32],
    ) -> Result<Vec<CurseForgeProject>, CurseForgeError> {
        if mod_ids.is_empty() {
            return Ok(Vec::new());
        }
        let url = format!("{}/mods", CURSEFORGE_API_BASE);
        let body = serde_json::json!({ "modIds": mod_ids });
        let resp = self
            .build_request(reqwest::Method::POST, &url)
            .json(&body)
            .send()
            .await?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            tracing::error!(
                url = %url,
                status = %status,
                body = %body,
                "CurseForge API request failed"
            );
            return Err(CurseForgeError::Api {
                status: status.as_u16(),
                body,
            });
        }
        let raw: serde_json::Value = resp.json().await?;
        let data = raw.get("data").cloned().unwrap_or(raw);
        Ok(serde_json::from_value(data)?)
    }

    fn make_files_url(mod_id: u32, loader: Option<&str>, game_version: Option<&str>) -> String {
        let mut url = format!("{}/mods/{}/files", CURSEFORGE_API_BASE, mod_id);
        let mut first = true;

        let mut append = |name: &str, value: &str| {
            if first {
                url.push('?');
                first = false;
            } else {
                url.push('&');
            }
            url.push_str(name);
            url.push('=');
            url.push_str(value);
        };

        append("pageSize", "100");
        if let Some(gv) = game_version
            && !gv.is_empty()
        {
            append("gameVersion", &CurseForgeClient::encode_param(gv));
        }
        if let Some(l) = loader {
            let l_lower = l.to_lowercase();
            if l_lower != "vanilla"
                && let Some(cf_loader) = CurseForgeLoader::from_name(l)
            {
                append("modLoaderType", &cf_loader.id().to_string());
            }
        }
        url
    }

    pub async fn get_project_files(
        &self,
        mod_id: u32,
        loader: Option<&str>,
        game_version: Option<&str>,
    ) -> Result<CurseForgeFilesResponse, CurseForgeError> {
        let url = Self::make_files_url(mod_id, loader, game_version);
        let resp: serde_json::Value = self.get_json(&url).await?;
        let data = resp.get("data").cloned().unwrap_or_else(|| resp.clone());
        let pagination = resp.get("pagination").cloned().unwrap_or_else(|| {
            serde_json::to_value(CurseForgePagination {
                index: 0,
                page_size: 0,
                result_count: 0,
                total_count: 0,
            })
            .unwrap_or_default()
        });
        Ok(CurseForgeFilesResponse {
            data: serde_json::from_value(data)?,
            pagination: serde_json::from_value(pagination)?,
        })
    }

    pub async fn get_file_download_url(
        &self,
        mod_id: u32,
        file_id: u32,
    ) -> Result<String, CurseForgeError> {
        let url = format!(
            "{}/mods/{}/files/{}/download-url",
            CURSEFORGE_API_BASE, mod_id, file_id
        );
        let cache_key = format!("download-url-{}-{}", mod_id, file_id);
        if let Some(cached) = CACHE.get(&cache_key)
            && let Some(s) = cached.as_str()
        {
            return Ok(s.to_string());
        }

        let resp: serde_json::Value = self.get_json(&url).await?;
        let download_url = resp
            .pointer("/data")
            .and_then(|v| v.as_str())
            .map(String::from)
            .ok_or(CurseForgeError::DownloadUrlNotFound)?;

        if !download_url.is_empty() {
            CACHE.set(cache_key, serde_json::Value::String(download_url.clone()));
        }
        Ok(download_url)
    }

    pub async fn get_mod_files(
        &self,
        file_ids: &[u32],
    ) -> Result<Vec<CurseForgeFile>, CurseForgeError> {
        if file_ids.is_empty() {
            return Ok(Vec::new());
        }
        let url = format!("{}/mods/files", CURSEFORGE_API_BASE);
        let body = serde_json::json!({ "fileIds": file_ids });
        let resp = self
            .build_request(reqwest::Method::POST, &url)
            .json(&body)
            .send()
            .await?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            tracing::error!(
                url = %url,
                status = %status,
                body = %body,
                "CurseForge API request failed"
            );
            return Err(CurseForgeError::Api {
                status: status.as_u16(),
                body,
            });
        }
        let raw: serde_json::Value = resp.json().await?;
        let data = raw.get("data").cloned().unwrap_or(raw);
        Ok(serde_json::from_value(data)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SEARCH_MODPACKS_JSON: &str =
        include_str!("../../tests/fixtures/curseforge_search_modpacks.json");

    #[test]
    fn parse_search_modpacks_response() {
        let resp: CurseForgeSearchResponse =
            serde_json::from_str(SEARCH_MODPACKS_JSON).expect("search response should parse");

        assert!(!resp.data.is_empty(), "search response should contain mods");
        assert!(
            resp.pagination.total_count > 0,
            "pagination should report results"
        );

        let first = &resp.data[0];
        assert_eq!(first.id, 285109);
        assert_eq!(first.name, "RLCraft");
        assert_eq!(first.class_id, Some(MODPACKS_CLASS_ID));
        assert!(!first.categories.is_empty());
        assert!(!first.authors.is_empty());

        let logo = first
            .logo
            .as_ref()
            .expect("first project should have a logo");
        assert_eq!(logo.mod_id, Some(285109));

        assert!(!first.latest_files.is_empty());
        let first_file = &first.latest_files[0];
        assert_eq!(first_file.mod_id, Some(285109));
        assert!(first_file.download_count > 0);

        assert!(!first.latest_files_indexes.is_empty());
    }

    #[test]
    fn parse_minimal_curseforge_file() {
        let json = r#"{
            "id": 123,
            "gameId": 432,
            "modId": 456,
            "isAvailable": true,
            "displayName": "Test File",
            "fileName": "test.jar",
            "releaseType": 1,
            "fileStatus": 4,
            "hashes": [],
            "fileDate": "2024-01-01T00:00:00Z",
            "fileLength": 1024,
            "downloadCount": 0,
            "downloadUrl": null,
            "gameVersions": ["1.20.1"],
            "dependencies": [],
            "alternateFileId": 0,
            "isServerPack": false,
            "fileFingerprint": 0,
            "modules": null
        }"#;

        let file: CurseForgeFile = serde_json::from_str(json).expect("file should parse");
        assert_eq!(file.id, 123);
        assert_eq!(file.mod_id, Some(456));
        assert_eq!(file.download_url, None);
        assert_eq!(file.game_versions, vec!["1.20.1"]);
    }

    #[test]
    fn curseforge_cdn_url_builds_correctly() {
        assert_eq!(
            curseforge_cdn_url(4612979, "RLCraft 1.12.2 - Release v2.9.3.zip"),
            "https://edge.forgecdn.net/files/4612/979/RLCraft%201.12.2%20-%20Release%20v2.9.3.zip"
        );
        assert_eq!(
            curseforge_cdn_url(2708027, "RLCraft 1.12.2 - Alpha v2.0.zip"),
            "https://edge.forgecdn.net/files/2708/27/RLCraft%201.12.2%20-%20Alpha%20v2.0.zip"
        );
    }
}

/// Builds a fallback CDN download URL for a CurseForge file using its
/// file ID and file name. This is used when the official download-url
/// endpoint returns 403 or no URL.
pub fn curseforge_cdn_url(file_id: u32, file_name: &str) -> String {
    let id_str = file_id.to_string();
    let (part1, part2) = if id_str.len() > 4 {
        id_str.split_at(4)
    } else {
        (id_str.as_str(), "")
    };
    // Remove leading zeros from the second part, matching the CDN path format.
    let part2 = part2.trim_start_matches('0');
    let part2 = if part2.is_empty() { "0" } else { part2 };
    let encoded_name = urlencoding::encode(file_name).replace('+', "%20");
    if part2 == "0" {
        format!("https://edge.forgecdn.net/files/{}/{}", part1, encoded_name)
    } else {
        format!(
            "https://edge.forgecdn.net/files/{}/{}/{}",
            part1, part2, encoded_name
        )
    }
}
