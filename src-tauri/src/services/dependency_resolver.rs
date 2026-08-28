use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::services::curseforge_api::{CurseForgeClient, CurseForgeFile, CurseForgeProject};

const MODRINTH_API_BASE: &str = "https://api.modrinth.com/v2";
const MAX_DEPENDENCY_DEPTH: u32 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DependencyKind {
    Required,
    Optional,
    Embedded,
    Incompatible,
}

impl DependencyKind {
    fn from_modrinth(value: &str) -> Self {
        match value.to_lowercase().as_str() {
            "required" => Self::Required,
            "optional" => Self::Optional,
            "embedded" => Self::Embedded,
            "incompatible" => Self::Incompatible,
            _ => Self::Required,
        }
    }

    fn from_curseforge_relation(relation_type: u32) -> Option<Self> {
        match relation_type {
            1 => Some(Self::Embedded),     // embeddedLibrary
            2 => Some(Self::Optional),     // optionalDependency
            3 => Some(Self::Required),     // requiredDependency
            4 => Some(Self::Optional),     // tool
            5 => Some(Self::Incompatible), // incompatible
            6 => Some(Self::Embedded),     // include
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "lowercase")]
pub enum DependencySource {
    Modrinth,
    Curseforge,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyRequest {
    pub source: DependencySource,
    pub project_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version_id: Option<String>,
    pub kind: DependencyKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedDependency {
    pub project_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version_id: Option<String>,
    pub source: DependencySource,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
    pub filename: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub download_url: Option<String>,
    pub kind: DependencyKind,
    pub depth: u32,
    #[serde(default)]
    pub children: Vec<ResolvedDependency>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestedVersion {
    pub version_id: String,
    pub requested_by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyConflict {
    pub project_id: String,
    pub source: DependencySource,
    #[serde(default)]
    pub requested_versions: Vec<RequestedVersion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyResolutionResult {
    #[serde(default)]
    pub tree: Vec<ResolvedDependency>,
    #[serde(default)]
    pub conflicts: Vec<DependencyConflict>,
}

#[derive(Debug, Clone, Deserialize)]
struct ModrinthFileBrief {
    url: String,
    filename: String,
    #[serde(default)]
    primary: bool,
}

#[derive(Debug, Clone, Deserialize)]
struct ModrinthDependencyBrief {
    #[serde(default)]
    dependency_type: String,
    #[serde(default)]
    project_id: Option<String>,
    #[serde(default)]
    version_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct ModrinthVersionBrief {
    id: String,
    #[serde(default)]
    files: Vec<ModrinthFileBrief>,
    #[serde(default)]
    dependencies: Vec<ModrinthDependencyBrief>,
}

#[derive(Debug, Clone, Deserialize)]
struct ModrinthProjectBrief {
    title: String,
    #[serde(default)]
    icon_url: Option<String>,
}

fn record_version(
    source: DependencySource,
    project_id: &str,
    version_id: &str,
    requested_by: &str,
    visited: &mut HashMap<(DependencySource, String), String>,
) -> Option<DependencyConflict> {
    let key = (source, project_id.to_string());

    if let Some(existing_version) = visited.get(&key) {
        if existing_version != version_id {
            return Some(DependencyConflict {
                project_id: project_id.to_string(),
                source,
                requested_versions: vec![
                    RequestedVersion {
                        version_id: existing_version.clone(),
                        requested_by: "otra dependencia".to_string(),
                    },
                    RequestedVersion {
                        version_id: version_id.to_string(),
                        requested_by: requested_by.to_string(),
                    },
                ],
            });
        }
        return None;
    }

    visited.insert(key, version_id.to_string());
    None
}

pub async fn resolve_dependencies(
    requests: Vec<DependencyRequest>,
    loader: String,
    game_version: String,
) -> Result<DependencyResolutionResult, String> {
    let mut visited: HashMap<(DependencySource, String), String> = HashMap::new();
    let mut conflicts: Vec<DependencyConflict> = Vec::new();
    let mut tree: Vec<ResolvedDependency> = Vec::new();

    for request in requests {
        let requested_by = request.project_id.clone();
        match request.source {
            DependencySource::Modrinth => {
                let node = Box::pin(resolve_modrinth_node(
                    &request.project_id,
                    request.version_id.as_deref(),
                    &loader,
                    &game_version,
                    request.kind,
                    0,
                    &requested_by,
                    &mut visited,
                    &mut conflicts,
                ))
                .await?;
                if let Some(dep) = node {
                    tree.push(dep);
                }
            }
            DependencySource::Curseforge => {
                let mod_id = request.project_id.parse::<u32>().map_err(|_| {
                    format!(
                        "El project_id de CurseForge debe ser numérico: {}",
                        request.project_id
                    )
                })?;
                let file_id = request
                    .version_id
                    .as_deref()
                    .and_then(|v| v.parse::<u32>().ok());
                let node = Box::pin(resolve_curseforge_node(
                    mod_id,
                    file_id,
                    &loader,
                    &game_version,
                    request.kind,
                    0,
                    &requested_by,
                    &mut visited,
                    &mut conflicts,
                ))
                .await?;
                if let Some(dep) = node {
                    tree.push(dep);
                }
            }
        }
    }

    Ok(DependencyResolutionResult { tree, conflicts })
}

#[allow(clippy::too_many_arguments)]
async fn resolve_modrinth_node(
    project_id: &str,
    version_id: Option<&str>,
    loader: &str,
    game_version: &str,
    kind: DependencyKind,
    depth: u32,
    requested_by: &str,
    visited: &mut HashMap<(DependencySource, String), String>,
    conflicts: &mut Vec<DependencyConflict>,
) -> Result<Option<ResolvedDependency>, String> {
    if depth > MAX_DEPENDENCY_DEPTH {
        return Ok(None);
    }

    let version = if let Some(vid) = version_id {
        fetch_modrinth_version(vid).await?
    } else {
        let versions = fetch_modrinth_project_versions(project_id, loader, game_version).await?;
        versions
            .into_iter()
            .next()
            .ok_or_else(|| format!("No se encontró versión compatible para {}", project_id))?
    };

    if let Some(conflict) = record_version(
        DependencySource::Modrinth,
        project_id,
        &version.id,
        requested_by,
        visited,
    ) {
        conflicts.push(conflict);
        return Ok(None);
    }

    let project = fetch_modrinth_project(project_id).await;
    let (title, icon_url) = project
        .as_ref()
        .map(|p| (p.title.clone(), p.icon_url.clone()))
        .unwrap_or_else(|_| (project_id.to_string(), None));

    let primary_file = version
        .files
        .iter()
        .find(|f| f.primary)
        .or_else(|| version.files.first())
        .ok_or_else(|| format!("La versión {} no tiene archivos", version.id))?;

    let mut children = Vec::new();
    for sub in &version.dependencies {
        let sub_kind = DependencyKind::from_modrinth(&sub.dependency_type);
        let Some(sub_project_id) = &sub.project_id else {
            continue;
        };

        match sub_kind {
            DependencyKind::Required | DependencyKind::Embedded => {
                let child = Box::pin(resolve_modrinth_node(
                    sub_project_id,
                    sub.version_id.as_deref(),
                    loader,
                    game_version,
                    sub_kind,
                    depth + 1,
                    &format!("{}@{}", project_id, version.id),
                    visited,
                    conflicts,
                ))
                .await?;
                if let Some(c) = child {
                    children.push(c);
                }
            }
            DependencyKind::Optional => {
                let leaf = resolve_modrinth_leaf(
                    sub_project_id,
                    sub.version_id.as_deref(),
                    loader,
                    game_version,
                )
                .await;
                if let Some(l) = leaf {
                    children.push(l.with_depth(depth + 1).with_kind(DependencyKind::Optional));
                }
            }
            DependencyKind::Incompatible => {
                children.push(ResolvedDependency {
                    project_id: sub_project_id.clone(),
                    version_id: sub.version_id.clone(),
                    source: DependencySource::Modrinth,
                    title: sub_project_id.clone(),
                    icon_url: None,
                    filename: String::new(),
                    download_url: None,
                    kind: DependencyKind::Incompatible,
                    depth: depth + 1,
                    children: vec![],
                });
            }
        }
    }

    Ok(Some(ResolvedDependency {
        project_id: project_id.to_string(),
        version_id: Some(version.id),
        source: DependencySource::Modrinth,
        title,
        icon_url,
        filename: primary_file.filename.clone(),
        download_url: Some(primary_file.url.clone()),
        kind,
        depth,
        children,
    }))
}

async fn resolve_modrinth_leaf(
    project_id: &str,
    version_id: Option<&str>,
    loader: &str,
    game_version: &str,
) -> Option<ResolvedDependency> {
    let project = fetch_modrinth_project(project_id).await.ok()?;
    let version = if let Some(vid) = version_id {
        fetch_modrinth_version(vid).await.ok()?
    } else {
        fetch_modrinth_project_versions(project_id, loader, game_version)
            .await
            .ok()?
            .into_iter()
            .next()?
    };

    let primary_file = version
        .files
        .iter()
        .find(|f| f.primary)
        .or_else(|| version.files.first())?;

    Some(ResolvedDependency {
        project_id: project_id.to_string(),
        version_id: Some(version.id),
        source: DependencySource::Modrinth,
        title: project.title,
        icon_url: project.icon_url,
        filename: primary_file.filename.clone(),
        download_url: Some(primary_file.url.clone()),
        kind: DependencyKind::Optional,
        depth: 0,
        children: vec![],
    })
}

#[allow(clippy::too_many_arguments)]
async fn resolve_curseforge_node(
    mod_id: u32,
    file_id: Option<u32>,
    loader: &str,
    game_version: &str,
    kind: DependencyKind,
    depth: u32,
    requested_by: &str,
    visited: &mut HashMap<(DependencySource, String), String>,
    conflicts: &mut Vec<DependencyConflict>,
) -> Result<Option<ResolvedDependency>, String> {
    if depth > MAX_DEPENDENCY_DEPTH {
        return Ok(None);
    }

    let project_id_string = mod_id.to_string();
    let file = if let Some(fid) = file_id {
        fetch_curseforge_file(fid).await?
    } else {
        let files = fetch_curseforge_project_files(mod_id, loader, game_version).await?;
        files
            .into_iter()
            .next()
            .ok_or_else(|| format!("No se encontró archivo compatible para el mod {}", mod_id))?
    };

    let file_id_string = file.id.to_string();
    if let Some(conflict) = record_version(
        DependencySource::Curseforge,
        &project_id_string,
        &file_id_string,
        requested_by,
        visited,
    ) {
        conflicts.push(conflict);
        return Ok(None);
    }

    let project = fetch_curseforge_project(mod_id).await;
    let (title, icon_url) = project
        .as_ref()
        .map(|p| {
            let icon = p.logo.as_ref().map(|l| l.url.clone());
            (p.name.clone(), icon)
        })
        .unwrap_or_else(|_| (project_id_string.clone(), None));

    let download_url = if let Some(url) = &file.download_url {
        Some(url.clone())
    } else {
        resolve_curseforge_download_url(mod_id, file.id, &file.file_name)
            .await
            .ok()
    };

    let mut children = Vec::new();
    for sub in &file.dependencies {
        let Some(sub_mod_id) = sub.mod_id else {
            continue;
        };
        let sub_kind = DependencyKind::from_curseforge_relation(sub.relation_type)
            .unwrap_or(DependencyKind::Optional);

        match sub_kind {
            DependencyKind::Required | DependencyKind::Embedded => {
                let child = Box::pin(resolve_curseforge_node(
                    sub_mod_id,
                    None,
                    loader,
                    game_version,
                    sub_kind,
                    depth + 1,
                    &format!("{}@{}", project_id_string, file.id),
                    visited,
                    conflicts,
                ))
                .await?;
                if let Some(c) = child {
                    children.push(c);
                }
            }
            DependencyKind::Optional => {
                let leaf = resolve_curseforge_leaf(sub_mod_id, None, loader, game_version).await;
                if let Some(l) = leaf {
                    children.push(l.with_depth(depth + 1).with_kind(DependencyKind::Optional));
                }
            }
            DependencyKind::Incompatible => {
                children.push(ResolvedDependency {
                    project_id: sub_mod_id.to_string(),
                    version_id: None,
                    source: DependencySource::Curseforge,
                    title: sub_mod_id.to_string(),
                    icon_url: None,
                    filename: String::new(),
                    download_url: None,
                    kind: DependencyKind::Incompatible,
                    depth: depth + 1,
                    children: vec![],
                });
            }
        }
    }

    Ok(Some(ResolvedDependency {
        project_id: project_id_string,
        version_id: Some(file_id_string),
        source: DependencySource::Curseforge,
        title,
        icon_url,
        filename: file.file_name,
        download_url,
        kind,
        depth,
        children,
    }))
}

async fn resolve_curseforge_leaf(
    mod_id: u32,
    file_id: Option<u32>,
    loader: &str,
    game_version: &str,
) -> Option<ResolvedDependency> {
    let project = fetch_curseforge_project(mod_id).await.ok()?;
    let file = if let Some(fid) = file_id {
        fetch_curseforge_file(fid).await.ok()?
    } else {
        fetch_curseforge_project_files(mod_id, loader, game_version)
            .await
            .ok()?
            .into_iter()
            .next()?
    };

    let download_url = if let Some(url) = &file.download_url {
        Some(url.clone())
    } else {
        resolve_curseforge_download_url(mod_id, file.id, &file.file_name)
            .await
            .ok()
    };

    Some(ResolvedDependency {
        project_id: mod_id.to_string(),
        version_id: Some(file.id.to_string()),
        source: DependencySource::Curseforge,
        title: project.name,
        icon_url: project.logo.as_ref().map(|l| l.url.clone()),
        filename: file.file_name,
        download_url,
        kind: DependencyKind::Optional,
        depth: 0,
        children: vec![],
    })
}

async fn fetch_modrinth_project(project_id: &str) -> Result<ModrinthProjectBrief, String> {
    let url = format!("{}/project/{}", MODRINTH_API_BASE, project_id);
    let value = crate::commands::market::get_json(&url).await?;
    serde_json::from_value(value)
        .map_err(|e| format!("Error al parsear proyecto de Modrinth: {}", e))
}

async fn fetch_modrinth_version(version_id: &str) -> Result<ModrinthVersionBrief, String> {
    let url = format!("{}/version/{}", MODRINTH_API_BASE, version_id);
    let value = crate::commands::market::get_json(&url).await?;
    serde_json::from_value(value)
        .map_err(|e| format!("Error al parsear versión de Modrinth: {}", e))
}

async fn fetch_modrinth_project_versions(
    project_id: &str,
    loader: &str,
    game_version: &str,
) -> Result<Vec<ModrinthVersionBrief>, String> {
    let mut url = format!("{}/project/{}/version", MODRINTH_API_BASE, project_id);
    let mut first = true;

    let mut append_param = |name: &str, value: &str| {
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

    let loader_lower = loader.to_lowercase();
    if !loader_lower.is_empty() && loader_lower != "vanilla" {
        append_param(
            "loaders",
            &serde_json::to_string(&vec![loader_lower]).unwrap_or_default(),
        );
    }
    if !game_version.is_empty() {
        append_param(
            "game_versions",
            &serde_json::to_string(&vec![game_version]).unwrap_or_default(),
        );
    }

    let value = crate::commands::market::get_json(&url).await?;
    serde_json::from_value(value)
        .map_err(|e| format!("Error al parsear versiones de Modrinth: {}", e))
}

async fn fetch_curseforge_project(mod_id: u32) -> Result<CurseForgeProject, String> {
    let client = CurseForgeClient::from_settings_or_default();
    client.get_project(mod_id).await.map_err(|e| e.to_string())
}

async fn fetch_curseforge_project_files(
    mod_id: u32,
    loader: &str,
    game_version: &str,
) -> Result<Vec<CurseForgeFile>, String> {
    let client = CurseForgeClient::from_settings_or_default();
    client
        .get_project_files(mod_id, Some(loader), Some(game_version))
        .await
        .map(|response| response.data)
        .map_err(|e| e.to_string())
}

async fn fetch_curseforge_file(file_id: u32) -> Result<CurseForgeFile, String> {
    let client = CurseForgeClient::from_settings_or_default();
    let files = client
        .get_mod_files(&[file_id])
        .await
        .map_err(|e| e.to_string())?;
    files
        .into_iter()
        .next()
        .ok_or_else(|| format!("No se encontró el archivo {} de CurseForge", file_id))
}

async fn resolve_curseforge_download_url(
    mod_id: u32,
    file_id: u32,
    file_name: &str,
) -> Result<String, String> {
    let client = CurseForgeClient::from_settings_or_default();
    client
        .get_file_download_url(mod_id, file_id)
        .await
        .or_else(|_| {
            Ok(crate::services::curseforge_api::curseforge_cdn_url(
                file_id, file_name,
            ))
        })
        .map_err(|e: crate::services::curseforge_api::CurseForgeError| e.to_string())
}

trait DependencyExt {
    fn with_depth(self, depth: u32) -> Self;
    fn with_kind(self, kind: DependencyKind) -> Self;
}

impl DependencyExt for ResolvedDependency {
    fn with_depth(mut self, depth: u32) -> Self {
        self.depth = depth;
        self
    }

    fn with_kind(mut self, kind: DependencyKind) -> Self {
        self.kind = kind;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dependency_kind_from_modrinth() {
        assert_eq!(
            DependencyKind::from_modrinth("required"),
            DependencyKind::Required
        );
        assert_eq!(
            DependencyKind::from_modrinth("optional"),
            DependencyKind::Optional
        );
        assert_eq!(
            DependencyKind::from_modrinth("embedded"),
            DependencyKind::Embedded
        );
        assert_eq!(
            DependencyKind::from_modrinth("incompatible"),
            DependencyKind::Incompatible
        );
        assert_eq!(
            DependencyKind::from_modrinth("unknown"),
            DependencyKind::Required
        );
        assert_eq!(
            DependencyKind::from_modrinth("OPTIONAL"),
            DependencyKind::Optional
        );
    }

    #[test]
    fn dependency_kind_from_curseforge_relation() {
        assert_eq!(
            DependencyKind::from_curseforge_relation(1),
            Some(DependencyKind::Embedded)
        );
        assert_eq!(
            DependencyKind::from_curseforge_relation(2),
            Some(DependencyKind::Optional)
        );
        assert_eq!(
            DependencyKind::from_curseforge_relation(3),
            Some(DependencyKind::Required)
        );
        assert_eq!(
            DependencyKind::from_curseforge_relation(5),
            Some(DependencyKind::Incompatible)
        );
        assert_eq!(DependencyKind::from_curseforge_relation(99), None);
    }

    #[test]
    fn record_version_first_insert() {
        let mut visited = HashMap::new();
        let conflict = record_version(
            DependencySource::Modrinth,
            "abc123",
            "v1",
            "root",
            &mut visited,
        );

        assert!(conflict.is_none());
        assert_eq!(
            visited.get(&(DependencySource::Modrinth, "abc123".to_string())),
            Some(&"v1".to_string())
        );
    }

    #[test]
    fn record_version_same_version_is_no_conflict() {
        let mut visited = HashMap::new();
        visited.insert(
            (DependencySource::Modrinth, "abc123".to_string()),
            "v1".to_string(),
        );

        let conflict = record_version(
            DependencySource::Modrinth,
            "abc123",
            "v1",
            "dep-a",
            &mut visited,
        );

        assert!(conflict.is_none());
    }

    #[test]
    fn record_version_different_version_is_conflict() {
        let mut visited = HashMap::new();
        visited.insert(
            (DependencySource::Curseforge, "98765".to_string()),
            "file-1".to_string(),
        );

        let conflict = record_version(
            DependencySource::Curseforge,
            "98765",
            "file-2",
            "dep-b",
            &mut visited,
        );

        assert!(conflict.is_some());
        let conflict = conflict.unwrap();
        assert_eq!(conflict.project_id, "98765");
        assert_eq!(conflict.source, DependencySource::Curseforge);
        assert_eq!(conflict.requested_versions.len(), 2);
        assert_eq!(conflict.requested_versions[0].version_id, "file-1");
        assert_eq!(conflict.requested_versions[1].version_id, "file-2");
        assert_eq!(conflict.requested_versions[1].requested_by, "dep-b");
    }

    #[test]
    fn record_version_tracks_sources_independently() {
        let mut visited = HashMap::new();

        record_version(
            DependencySource::Modrinth,
            "abc123",
            "v1",
            "root",
            &mut visited,
        );
        let conflict = record_version(
            DependencySource::Curseforge,
            "abc123",
            "file-1",
            "root",
            &mut visited,
        );

        assert!(conflict.is_none());
        assert_eq!(visited.len(), 2);
    }
}
