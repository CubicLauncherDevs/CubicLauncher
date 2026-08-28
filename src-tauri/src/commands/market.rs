use crate::services::curseforge_api::{
    CurseForgeClient, CurseForgeFile, CurseForgeSearchResponse, CurseForgeSortField,
};

// ── Modrinth commands ─────────────────────────────────────────────────────────

const MODRINTH_API_BASE: &str = "https://api.modrinth.com/v2";

fn build_modrinth_facets(
    loader: &str,
    game_version: &Option<String>,
    category: &Option<String>,
    project_type: &str,
) -> serde_json::Value {
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

pub(crate) async fn get_json(url: &str) -> Result<serde_json::Value, String> {
    use crate::core::http_client::HTTP;
    use serde_json::Value;
    use std::collections::HashMap;
    use std::sync::LazyLock;
    use std::sync::Mutex;
    use std::time::Instant;

    const CACHE_TTL: u64 = 300;
    const CACHE_MAX: usize = 200;

    struct CacheEntry {
        data: Value,
        fetched_at: Instant,
    }

    struct ApiCache {
        entries: Mutex<HashMap<String, CacheEntry>>,
    }

    static CACHE: LazyLock<ApiCache> = LazyLock::new(|| ApiCache {
        entries: Mutex::new(HashMap::new()),
    });

    {
        let mut map = CACHE.entries.lock().unwrap();
        if let Some(entry) = map.get(url) {
            if entry.fetched_at.elapsed().as_secs() < CACHE_TTL {
                return Ok(entry.data.clone());
            }
            map.remove(url);
        }
    }

    let resp = HTTP
        .get(url)
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!(
            "API error {}: {}",
            status,
            body.chars().take(200).collect::<String>()
        ));
    }

    let data: Value = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    {
        let mut map = CACHE.entries.lock().unwrap();
        if map.len() >= CACHE_MAX
            && let Some(oldest) = map
                .iter()
                .min_by_key(|(_, e)| e.fetched_at)
                .map(|(k, _)| k.clone())
        {
            map.remove(&oldest);
        }
        map.insert(
            url.to_string(),
            CacheEntry {
                data: data.clone(),
                fetched_at: Instant::now(),
            },
        );
    }

    Ok(data)
}

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
) -> Result<serde_json::Value, String> {
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
pub async fn get_modrinth_project(project_id: String) -> Result<serde_json::Value, String> {
    let url = format!("{}/project/{}", MODRINTH_API_BASE, project_id);
    get_json(&url).await
}

#[tauri::command]
pub async fn get_modrinth_project_versions(
    project_id: String,
    loader: Option<String>,
    game_version: Option<String>,
) -> Result<serde_json::Value, String> {
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
pub async fn get_modrinth_version(version_id: String) -> Result<serde_json::Value, String> {
    let url = format!("{}/version/{}", MODRINTH_API_BASE, version_id);
    get_json(&url).await
}

#[tauri::command]
pub async fn get_modrinth_latest_versions(
    hashes: Vec<String>,
    algorithm: String,
    loaders: Vec<String>,
    game_versions: Vec<String>,
) -> Result<serde_json::Value, String> {
    let url = format!("{}/version_files/update", MODRINTH_API_BASE);
    let body = serde_json::json!({
        "hashes": hashes,
        "algorithm": algorithm,
        "loaders": loaders,
        "game_versions": game_versions,
    });

    let resp = crate::core::http_client::HTTP
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Modrinth update request failed: {}", e))?;

    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(format!(
            "Modrinth update error {}: {}",
            status,
            text.chars().take(200).collect::<String>()
        ));
    }

    resp.json::<serde_json::Value>()
        .await
        .map_err(|e| format!("Failed to parse Modrinth update response: {}", e))
}

// ── CurseForge commands ───────────────────────────────────────────────────────

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn search_curseforge(
    query: String,
    loader: String,
    game_version: Option<String>,
    category: Option<String>,
    index: String,
    limit: u32,
    offset: u32,
    class_id: Option<u32>,
) -> Result<CurseForgeSearchResponse, String> {
    let client = CurseForgeClient::from_settings_or_default();
    let sort_field = CurseForgeSortField::from_index(&index);
    let category_id = category.and_then(|c| c.parse::<u32>().ok());

    client
        .search(crate::services::curseforge_api::SearchParams {
            query: &query,
            loader: Some(&loader),
            game_version: game_version.as_deref(),
            category_id,
            sort_field,
            sort_order: crate::services::curseforge_api::SortOrder::Desc,
            index: offset,
            page_size: limit,
            class_id: class_id.unwrap_or(crate::services::curseforge_api::MODS_CLASS_ID),
        })
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn search_curseforge_modpacks(
    query: String,
    loader: String,
    game_version: Option<String>,
    category: Option<String>,
    index: String,
    limit: u32,
    offset: u32,
) -> Result<CurseForgeSearchResponse, String> {
    let client = CurseForgeClient::from_settings_or_default();
    let sort_field = CurseForgeSortField::from_index(&index);
    let category_id = category.and_then(|c| c.parse::<u32>().ok());

    client
        .search_modpacks(
            &query,
            Some(&loader),
            game_version.as_deref(),
            category_id,
            sort_field,
            offset,
            limit,
        )
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_curseforge_project(
    mod_id: u32,
) -> Result<crate::services::curseforge_api::CurseForgeProject, String> {
    let client = CurseForgeClient::from_settings_or_default();
    client.get_project(mod_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_curseforge_project_description(mod_id: u32) -> Result<String, String> {
    let client = CurseForgeClient::from_settings_or_default();
    client
        .get_project_description(mod_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_curseforge_project_files(
    mod_id: u32,
    loader: Option<String>,
    game_version: Option<String>,
) -> Result<Vec<CurseForgeFile>, String> {
    let client = CurseForgeClient::from_settings_or_default();
    client
        .get_project_files(mod_id, loader.as_deref(), game_version.as_deref())
        .await
        .map(|response| response.data)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_curseforge_file_download_url(
    mod_id: u32,
    file_id: u32,
    file_name: String,
) -> Result<String, String> {
    let client = CurseForgeClient::from_settings_or_default();
    client
        .get_file_download_url(mod_id, file_id)
        .await
        .or_else(|e| {
            tracing::warn!(
                mod_id,
                file_id,
                file_name = %file_name,
                error = %e,
                "CurseForge official download URL failed, falling back to CDN"
            );
            Ok(crate::services::curseforge_api::curseforge_cdn_url(
                file_id, &file_name,
            ))
        })
}
