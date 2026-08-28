use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DependencyKind {
    Required,
    Optional,
    Embedded,
    Incompatible,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DependencySource {
    Modrinth,
    Curseforge,
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
