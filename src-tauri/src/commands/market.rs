use std::collections::HashMap;
use std::sync::LazyLock;
use std::sync::Mutex;
use std::time::Instant;

use serde_json::Value;

use crate::core::http_client::HTTP;

const MODRINTH_API_BASE: &str = "https://api.modrinth.com/v2";
const CURSEFORGE_API_BASE: &str = "https://api.curseforge.com/v1";
const CURSEFORGE_API_KEY: &str = "$2a$10$v4G8m2LV2QhjUu5l.G24Ieqdp4JTEEQ6bRsZjvpa0YncCVaDaqBP6";
const MINECRAFT_GAME_ID: u32 = 432;

const CACHE_TTL: u64 = 300;
const CACHE_MAX: usize = 200;

struct CacheEntry {
    data: Value,
    fetched_at: Instant,
}

struct ApiCache {
    entries: Mutex<HashMap<String, CacheEntry>>,
}

impl ApiCache {
    fn get(&self, key: &str) -> Option<Value> {
        let mut map = self.entries.lock().unwrap();
        if let Some(entry) = map.get(key) {
            if entry.fetched_at.elapsed().as_secs() < CACHE_TTL {
                return Some(entry.data.clone());
            }
            map.remove(key);
        }
        None
    }

    fn set(&self, key: String, data: Value) {
        let mut map = self.entries.lock().unwrap();
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

fn curseforge_loader_id(loader: &str) -> u32 {
    match loader.to_lowercase().as_str() {
        "fabric" => 4,
        "forge" => 1,
        "neoforge" => 6,
        "quilt" => 5,
        _ => 4,
    }
}

fn build_modrinth_facets(
    loader: &str,
    game_version: &Option<String>,
    category: &Option<String>,
    project_type: &str,
) -> Value {
    let mut facets: Vec<Vec<String>> = Vec::new();

    let loader_lower = loader.to_lowercase();
    if !loader_lower.is_empty() && loader_lower != "vanilla" {
        facets.push(vec![format!("categories:{}", loader_lower)]);
    }
    if let Some(gv) = game_version
        && !gv.is_empty()
    {
        facets.push(vec![format!("versions:{}", gv)]);
    }
    facets.push(vec![format!("project_type:{}", project_type)]);
    if let Some(cat) = category
        && !cat.is_empty()
    {
        facets.push(vec![format!("categories:{}", cat.to_lowercase())]);
    }

    serde_json::to_value(facets).unwrap_or_default()
}

async fn get_json(url: &str) -> Result<Value, String> {
    if let Some(cached) = CACHE.get(url) {
        return Ok(cached);
    }

    let resp = HTTP
        .get(url)
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        if body.len() > 200 {
            return Err(format!("API error {}: {}...", status, &body[..200]));
        }
        return Err(format!("API error {}: {}", status, body));
    }

    let data: Value = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    CACHE.set(url.to_string(), data.clone());
    Ok(data)
}

// ── Modrinth commands ──

#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn search_modrinth(
    query: String,
    loader: String,
    game_version: Option<String>,
    category: Option<String>,
    index: String,
    limit: u32,
    offset: u32,
    project_type: String,
) -> Result<Value, String> {
    let facets = build_modrinth_facets(&loader, &game_version, &category, &project_type);

    let facets_str = facets.to_string();
    let mut url = format!("{}/search", MODRINTH_API_BASE);
    url.push_str(&format!(
        "?query={}&facets={}&index={}&limit={}&offset={}",
        urlencoding::encode(&query),
        urlencoding::encode(&facets_str),
        urlencoding::encode(&index),
        limit,
        offset
    ));

    get_json(&url).await
}

#[tauri::command]
pub async fn get_modrinth_project(project_id: String) -> Result<Value, String> {
    let url = format!("{}/project/{}", MODRINTH_API_BASE, project_id);
    get_json(&url).await
}

#[tauri::command]
pub async fn get_modrinth_project_versions(
    project_id: String,
    loader: Option<String>,
    game_version: Option<String>,
) -> Result<Value, String> {
    let mut url = format!("{}/project/{}/version", MODRINTH_API_BASE, project_id);
    let mut first = true;

    let mut append_param = |url: &mut String, name: &str, value: &str| {
        if first {
            url.push('?');
            first = false;
        } else {
            url.push('&');
        }
        url.push_str(name);
        url.push('=');
        url.push_str(&urlencoding::encode(value));
    };

    append_param(&mut url, "include_changelog", "false");

    if let Some(l) = loader {
        let l = l.to_lowercase();
        if !l.is_empty() && l != "vanilla" {
            append_param(
                &mut url,
                "loaders",
                &serde_json::to_string(&vec![l]).unwrap_or_default(),
            );
        }
    }
    if let Some(gv) = game_version
        && !gv.is_empty()
    {
        append_param(
            &mut url,
            "game_versions",
            &serde_json::to_string(&vec![gv]).unwrap_or_default(),
        );
    }

    get_json(&url).await
}

#[tauri::command]
pub async fn get_modrinth_version(version_id: String) -> Result<Value, String> {
    let url = format!("{}/version/{}", MODRINTH_API_BASE, version_id);
    get_json(&url).await
}

#[tauri::command]
pub async fn get_modrinth_latest_versions(
    hashes: Vec<String>,
    algorithm: String,
    loaders: Vec<String>,
    game_versions: Vec<String>,
) -> Result<Value, String> {
    let url = format!("{}/version_files/update", MODRINTH_API_BASE);

    let cache_key = format!(
        "POST {} {} {} {} {}",
        url,
        serde_json::to_string(&hashes).unwrap_or_default(),
        algorithm,
        serde_json::to_string(&loaders).unwrap_or_default(),
        serde_json::to_string(&game_versions).unwrap_or_default(),
    );

    if let Some(cached) = CACHE.get(&cache_key) {
        return Ok(cached);
    }

    let body = serde_json::json!({
        "hashes": hashes,
        "algorithm": algorithm,
        "loaders": loaders,
        "game_versions": game_versions,
    });

    let resp = HTTP
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Modrinth update request failed: {}", e))?;

    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        if text.len() > 200 {
            return Err(format!(
                "Modrinth update error {}: {}...",
                status,
                &text[..200]
            ));
        }
        return Err(format!("Modrinth update error {}: {}", status, text));
    }

    let data: Value = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse Modrinth update response: {}", e))?;

    CACHE.set(cache_key, data.clone());
    Ok(data)
}

// ── CurseForge commands ──

async fn get_json_cf(url: &str) -> Result<Value, String> {
    if let Some(cached) = CACHE.get(url) {
        return Ok(cached);
    }

    let resp = HTTP
        .get(url)
        .header("x-api-key", CURSEFORGE_API_KEY)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| format!("CurseForge request failed: {}", e))?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        if body.len() > 200 {
            return Err(format!(
                "CurseForge API error {}: {}...",
                status,
                &body[..200]
            ));
        }
        return Err(format!("CurseForge API error {}: {}", status, body));
    }

    let data: Value = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse CurseForge response: {}", e))?;

    CACHE.set(url.to_string(), data.clone());
    Ok(data)
}

#[tauri::command]
pub async fn search_curseforge(
    query: String,
    loader: String,
    game_version: Option<String>,
    category: Option<String>,
    index: String,
    limit: u32,
    offset: u32,
) -> Result<Value, String> {
    let mut url = format!("{}/mods/search", CURSEFORGE_API_BASE);
    let mut first = true;

    let mut append = |url: &mut String, name: &str, value: &str| {
        if first {
            url.push('?');
            first = false;
        } else {
            url.push('&');
        }
        url.push_str(name);
        url.push('=');
        url.push_str(&urlencoding::encode(value));
    };

    append(&mut url, "gameId", &MINECRAFT_GAME_ID.to_string());

    if !query.is_empty() {
        append(&mut url, "searchFilter", &query);
    }

    append(&mut url, "pageSize", &limit.min(50).to_string());
    append(&mut url, "index", &offset.to_string());
    append(&mut url, "classId", "6");

    let loader_lower = loader.to_lowercase();
    if loader_lower != "vanilla" {
        append(
            &mut url,
            "modLoaderType",
            &curseforge_loader_id(&loader).to_string(),
        );
    }
    if let Some(gv) = &game_version
        && !gv.is_empty()
    {
        append(&mut url, "gameVersion", gv);
    }
    if let Some(cat) = &category
        && !cat.is_empty()
    {
        append(&mut url, "categoryId", cat);
    }

    match index.as_str() {
        "downloads" => append(&mut url, "sortOrder", "desc"),
        "newest" => {
            append(&mut url, "sortField", "2");
            append(&mut url, "sortOrder", "desc");
        }
        _ => append(&mut url, "sortOrder", "desc"),
    }

    get_json_cf(&url).await
}

#[tauri::command]
pub async fn get_curseforge_project(mod_id: u32) -> Result<Value, String> {
    let url = format!("{}/mods/{}", CURSEFORGE_API_BASE, mod_id);

    if let Some(cached) = CACHE.get(&url) {
        return Ok(cached);
    }

    let resp = HTTP
        .get(&url)
        .header("x-api-key", CURSEFORGE_API_KEY)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| format!("CurseForge request failed: {}", e))?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        if body.len() > 200 {
            return Err(format!(
                "CurseForge API error {}: {}...",
                status,
                &body[..200]
            ));
        }
        return Err(format!("CurseForge API error {}: {}", status, body));
    }

    let raw: Value = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse CurseForge response: {}", e))?;

    let data = raw.get("data").cloned().unwrap_or(raw);

    CACHE.set(url, data.clone());
    Ok(data)
}

#[tauri::command]
pub async fn get_curseforge_project_files(
    mod_id: u32,
    loader: Option<String>,
    game_version: Option<String>,
) -> Result<Value, String> {
    let mut url = format!("{}/mods/{}/files", CURSEFORGE_API_BASE, mod_id);
    let mut first = true;

    let mut append = |url: &mut String, name: &str, value: &str| {
        if first {
            url.push('?');
            first = false;
        } else {
            url.push('&');
        }
        url.push_str(name);
        url.push('=');
        url.push_str(&urlencoding::encode(value));
    };

    append(&mut url, "pageSize", "100");

    if let Some(gv) = &game_version
        && !gv.is_empty()
    {
        append(&mut url, "gameVersion", gv);
    }
    if let Some(l) = &loader {
        let l_lower = l.to_lowercase();
        if l_lower != "vanilla" {
            append(
                &mut url,
                "modLoaderType",
                &curseforge_loader_id(l).to_string(),
            );
        }
    }

    if let Some(cached) = CACHE.get(&url) {
        return Ok(cached);
    }

    let resp = HTTP
        .get(&url)
        .header("x-api-key", CURSEFORGE_API_KEY)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| format!("CurseForge request failed: {}", e))?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        if body.len() > 200 {
            return Err(format!(
                "CurseForge API error {}: {}...",
                status,
                &body[..200]
            ));
        }
        return Err(format!("CurseForge API error {}: {}", status, body));
    }

    let raw: Value = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse CurseForge response: {}", e))?;

    let data = raw.get("data").cloned().unwrap_or(raw);

    CACHE.set(url, data.clone());
    Ok(data)
}

#[tauri::command]
pub async fn get_curseforge_file_download_url(mod_id: u32, file_id: u32) -> Result<String, String> {
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

    let resp = HTTP
        .get(&url)
        .header("x-api-key", CURSEFORGE_API_KEY)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| format!("CurseForge request failed: {}", e))?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        if body.len() > 200 {
            return Err(format!(
                "CurseForge API error {}: {}...",
                status,
                &body[..200]
            ));
        }
        return Err(format!("CurseForge API error {}: {}", status, body));
    }

    let raw: Value = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse CurseForge response: {}", e))?;

    let download_url = raw
        .pointer("/data/downloadUrl")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    if !download_url.is_empty() {
        CACHE.set(cache_key, Value::String(download_url.clone()));
    }

    Ok(download_url)
}
