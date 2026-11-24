use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::services::curseforge_api::{CurseForgeClient, CurseForgeFile, CurseForgeProject};

const MODRINTH_API_BASE: &str = "https://api.modrinth.com/v2";
const MAX_DEPENDENCY_DEPTH: u32 = 10;
/// Modrinth resolves `version_files` lookups in batches of this size.
const MODRINTH_BATCH_SIZE: usize = 100;

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
    /// Set when the project has no usable version for this instance.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unresolved_reason: Option<String>,
    /// Game versions accepted by the selected version (empty when unresolved).
    #[serde(default)]
    pub game_versions: Vec<String>,
    /// Loaders accepted by the selected version (empty when unresolved).
    #[serde(default)]
    pub loaders: Vec<String>,
    #[serde(default)]
    pub children: Vec<ResolvedDependency>,
}

/// Identifies the installed copy of a project so the preview can explain a
/// version change in words ("2.1.0" or a file name) instead of raw ids.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledModRef {
    pub source: DependencySource,
    pub project_id: String,
    pub enabled: bool,
    pub filename: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestedVersion {
    pub version_id: String,
    pub requested_by: String,
}

/// Project that asks for another project, together with the version it will
/// end up with.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyOrigin {
    pub project_id: String,
    pub source: DependencySource,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictType {
    /// Two nodes in the tree demand different versions of the same project.
    VersionMismatch,
    /// A required project has no version for this game version and loader.
    Unresolved,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyConflict {
    pub project_id: String,
    pub source: DependencySource,
    #[serde(default)]
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
    #[serde(default = "default_conflict_type")]
    pub kind: ConflictType,
    pub requested_by: Vec<DependencyOrigin>,
    #[serde(default)]
    pub requested_versions: Vec<RequestedVersion>,
    /// Label of the installed copy, filled by `annotate_installed_versions`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub installed_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

fn default_conflict_type() -> ConflictType {
    ConflictType::VersionMismatch
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyResolutionResult {
    #[serde(default)]
    pub tree: Vec<ResolvedDependency>,
    #[serde(default)]
    pub conflicts: Vec<DependencyConflict>,
    /// Projects any resolved node declares incompatible. The front compares
    /// them with the installed mods because the resolver has no inventory.
    #[serde(default)]
    pub incompatible: Vec<DependencyOrigin>,
}

// ── Raw API shapes ────────────────────────────────────────────────────────────

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
    game_versions: Vec<String>,
    #[serde(default)]
    loaders: Vec<String>,
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

// ── Constraint model ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NodeRole {
    /// Part of the install plan; its presence is required.
    Hard,
    /// Suggested by a resolved node; listed but never installed implicitly.
    Optional,
    /// Declared incompatible with a resolved node; only used for warnings.
    Incompatible,
}

impl NodeRole {
    fn kind(self) -> DependencyKind {
        match self {
            Self::Hard => DependencyKind::Required,
            Self::Optional => DependencyKind::Optional,
            Self::Incompatible => DependencyKind::Incompatible,
        }
    }
}

#[derive(Debug, Default)]
struct ModrinthCandidates {
    /// Versions for this instance and loader, newest first.
    compatible: Vec<ModrinthVersionBrief>,
    /// Versions referenced by dependencies, kept even when incompatible so a
    /// conflict can name the version that was actually requested.
    referenced: HashMap<String, ModrinthVersionBrief>,
}

impl ModrinthCandidates {
    fn find(&self, version_id: &str) -> Option<&ModrinthVersionBrief> {
        self.compatible
            .iter()
            .find(|v| v.id == version_id)
            .or_else(|| self.referenced.get(version_id))
    }

    fn is_compatible(&self, version: &ModrinthVersionBrief) -> bool {
        self.compatible.iter().any(|v| v.id == version.id)
    }

    fn newest_compatible(&self) -> Option<&ModrinthVersionBrief> {
        self.compatible.first()
    }
}

#[derive(Debug, Default)]
struct CurseForgeCandidates {
    compatible: Vec<CurseForgeFile>,
    referenced: HashMap<u32, CurseForgeFile>,
}

impl CurseForgeCandidates {
    fn find(&self, file_id: u32) -> Option<&CurseForgeFile> {
        self.compatible
            .iter()
            .find(|f| f.id == file_id)
            .or_else(|| self.referenced.get(&file_id))
    }

    fn is_compatible(&self, file: &CurseForgeFile) -> bool {
        self.compatible.iter().any(|f| f.id == file.id)
    }

    fn newest_compatible(&self) -> Option<&CurseForgeFile> {
        self.compatible.first()
    }
}

#[derive(Debug, Clone)]
struct NodeInfo {
    source: DependencySource,
    project_id: String,
    title: Option<String>,
    icon_url: Option<String>,
    role: NodeRole,
    depth: u32,
}

#[derive(Debug, Default, Clone)]
struct NodeState {
    /// `(parent index, version id)` pairs requested by parents. Incompatible
    /// nodes store an empty version id.
    constraints: Vec<(usize, String)>,
    selected: Option<String>,
    unresolved_reason: Option<String>,
}

fn unique_constraint_ids(constraints: &[(usize, String)]) -> Vec<String> {
    let mut ids: Vec<String> = constraints
        .iter()
        .map(|(_, id)| id.clone())
        .filter(|id| !id.is_empty())
        .collect();
    ids.sort();
    ids.dedup();
    ids
}

/// Picks the version that satisfies every parent: the pinned version when all
/// parents agree, otherwise the newest compatible one. When only referenced
/// (incompatible) versions exist, the newest referenced one is returned so the
/// tree still explains what was asked for.
fn select_modrinth<'a>(
    constraints: &[(usize, String)],
    candidates: &'a ModrinthCandidates,
) -> Option<&'a ModrinthVersionBrief> {
    let ids = unique_constraint_ids(constraints);
    if ids.is_empty() {
        return candidates.newest_compatible();
    }
    if ids.len() == 1 {
        return candidates.find(&ids[0]);
    }
    candidates
        .newest_compatible()
        .or_else(|| candidates.find(&ids[0]))
}

fn select_curseforge<'a>(
    constraints: &[(usize, String)],
    candidates: &'a CurseForgeCandidates,
) -> Option<&'a CurseForgeFile> {
    let ids = unique_constraint_ids(constraints);
    if ids.is_empty() {
        return candidates.newest_compatible();
    }
    let files: Vec<u32> = ids.iter().filter_map(|id| id.parse().ok()).collect();
    if files.len() == 1 {
        return candidates.find(files[0]);
    }
    candidates
        .newest_compatible()
        .or_else(|| candidates.find(files[0]))
}

fn modrinth_file(version: &ModrinthVersionBrief) -> Option<&ModrinthFileBrief> {
    version
        .files
        .iter()
        .find(|f| f.primary)
        .or_else(|| version.files.first())
}

fn is_minecraft_version_label(value: &str) -> bool {
    let mut parts = value.split('.');
    let head = parts.next().unwrap_or_default();
    !head.is_empty()
        && head.chars().all(|c| c.is_ascii_digit())
        && parts.all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
}

fn curseforge_file_loaders(file: &CurseForgeFile) -> Vec<String> {
    if let Some(modules) = &file.modules
        && !modules.is_empty()
    {
        let mut loaders: Vec<String> = modules.iter().map(|m| m.name.to_lowercase()).collect();
        loaders.sort();
        loaders.dedup();
        return loaders;
    }
    let mut loaders: Vec<String> = file
        .game_versions
        .iter()
        .filter(|v| !is_minecraft_version_label(v))
        .map(|v| v.to_lowercase())
        .collect();
    loaders.sort();
    loaders.dedup();
    loaders
}

fn curseforge_player_versions(file: &CurseForgeFile) -> Vec<String> {
    file.game_versions
        .iter()
        .filter(|v| is_minecraft_version_label(v))
        .cloned()
        .collect()
}

// ── Collector ─────────────────────────────────────────────────────────────────

#[derive(Default)]
struct Collector {
    loader: String,
    game_version: String,
    nodes: Vec<NodeInfo>,
    children: Vec<Vec<usize>>,
    index: HashMap<(DependencySource, String), usize>,
    states: Vec<NodeState>,
    modrinth: HashMap<String, ModrinthCandidates>,
    curseforge: HashMap<String, CurseForgeCandidates>,
    modrinth_projects: HashMap<String, ModrinthProjectBrief>,
    curseforge_projects: HashMap<String, CurseForgeProject>,
    conflicts: Vec<DependencyConflict>,
}

impl Collector {
    fn accepts_loader(&self, loader: &str) -> bool {
        let wanted = self.loader.to_lowercase();
        if wanted.is_empty() || wanted == "vanilla" {
            return true;
        }
        loader.eq_ignore_ascii_case(&wanted)
    }

    fn curseforge_file_matches(&self, file: &CurseForgeFile) -> bool {
        if !self.game_version.is_empty()
            && !file.game_versions.iter().any(|v| v == &self.game_version)
        {
            return false;
        }
        let loaders = curseforge_file_loaders(file);
        loaders.is_empty() || loaders.iter().any(|name| self.accepts_loader(name))
    }

    /// Stores a node or returns the existing one so repeated dependencies are
    /// listed once. An optional/incompatible node upgraded to a hard one keeps
    /// its position but becomes installable.
    fn remember(
        &mut self,
        source: DependencySource,
        project_id: &str,
        role: NodeRole,
        depth: u32,
    ) -> (usize, bool) {
        let key = (source, project_id.to_string());
        if let Some(&index) = self.index.get(&key) {
            if role == NodeRole::Hard {
                self.nodes[index].role = NodeRole::Hard;
                self.nodes[index].depth = self.nodes[index].depth.min(depth);
            }
            return (index, false);
        }
        let index = self.nodes.len();
        self.nodes.push(NodeInfo {
            source,
            project_id: project_id.to_string(),
            title: None,
            icon_url: None,
            role,
            depth,
        });
        self.children.push(Vec::new());
        self.states.push(NodeState::default());
        self.index.insert(key, index);
        (index, true)
    }

    fn add_constraint(&mut self, index: usize, parent: usize, version_id: Option<&str>) {
        let constraints = &mut self.states[index].constraints;
        if constraints.iter().any(|(id, _)| *id == parent) {
            if let Some(version_id) = version_id
                && let Some(entry) = constraints.iter_mut().find(|(id, _)| *id == parent)
            {
                entry.1 = version_id.to_string();
            }
            return;
        }
        constraints.push((parent, version_id.unwrap_or_default().to_string()));
    }

    fn push_child(&mut self, parent: usize, child: usize) {
        if !self.children[parent].contains(&child) {
            self.children[parent].push(child);
        }
    }

    async fn modrinth_candidates(
        &mut self,
        project_id: &str,
        hints: &[String],
    ) -> Result<(), String> {
        if !self.modrinth.contains_key(project_id) {
            let compatible =
                fetch_modrinth_project_versions(project_id, &self.loader, &self.game_version)
                    .await?;
            self.modrinth.insert(
                project_id.to_string(),
                ModrinthCandidates {
                    compatible,
                    referenced: HashMap::new(),
                },
            );
        }
        let missing: Vec<String> = hints
            .iter()
            .filter(|id| {
                !id.is_empty()
                    && self
                        .modrinth
                        .get(project_id)
                        .is_some_and(|c| c.find(id).is_none())
            })
            .cloned()
            .collect();
        if missing.is_empty() {
            return Ok(());
        }
        let fetched = fetch_modrinth_versions(&missing).await;
        let entry = self
            .modrinth
            .get_mut(project_id)
            .expect("project candidates cached");
        for version in fetched {
            entry.referenced.insert(version.id.clone(), version);
        }
        Ok(())
    }

    async fn ensure_modrinth_project(&mut self, project_id: &str) {
        if self.modrinth_projects.contains_key(project_id) {
            return;
        }
        if let Ok(project) = fetch_modrinth_project(project_id).await {
            self.modrinth_projects
                .insert(project_id.to_string(), project);
        }
    }

    async fn curseforge_candidates(&mut self, mod_id: u32, hints: &[String]) -> Result<(), String> {
        let key = mod_id.to_string();
        if !self.curseforge.contains_key(&key) {
            let files = fetch_curseforge_project_files(mod_id).await?;
            let compatible = files
                .into_iter()
                .filter(|file| file.is_available && self.curseforge_file_matches(file))
                .collect();
            self.curseforge.insert(
                key.clone(),
                CurseForgeCandidates {
                    compatible,
                    referenced: HashMap::new(),
                },
            );
        }
        let missing: Vec<u32> = hints
            .iter()
            .filter_map(|id| id.parse::<u32>().ok())
            .filter(|id| {
                self.curseforge
                    .get(&key)
                    .is_some_and(|c| c.find(*id).is_none())
            })
            .collect();
        if missing.is_empty() {
            return Ok(());
        }
        let fetched = fetch_curseforge_files(&missing).await;
        let entry = self
            .curseforge
            .get_mut(&key)
            .expect("project candidates cached");
        for file in fetched {
            entry.referenced.insert(file.id, file);
        }
        Ok(())
    }

    /// Fills in title and icon for a node that is only listed (optional or
    /// incompatible); without this the preview would show the raw project id.
    async fn describe_node(&mut self, index: usize) {
        let info = self.nodes[index].clone();
        match info.source {
            DependencySource::Modrinth => {
                self.ensure_modrinth_project(&info.project_id).await;
                if let Some(project) = self.modrinth_projects.get(&info.project_id) {
                    self.nodes[index].title = Some(project.title.clone());
                    self.nodes[index].icon_url = project.icon_url.clone();
                }
            }
            DependencySource::Curseforge => {
                if let Ok(mod_id) = info.project_id.parse::<u32>() {
                    self.ensure_curseforge_project(mod_id).await;
                }
                if let Some(project) = self.curseforge_projects.get(&info.project_id) {
                    self.nodes[index].title = Some(project.name.clone());
                    self.nodes[index].icon_url = project.logo.as_ref().map(|l| l.url.clone());
                }
            }
        }
    }

    async fn ensure_curseforge_project(&mut self, mod_id: u32) {
        let key = mod_id.to_string();
        if self.curseforge_projects.contains_key(&key) {
            return;
        }
        if let Ok(project) = fetch_curseforge_project(mod_id).await {
            self.curseforge_projects.insert(key, project);
        }
    }

    async fn collect_modrinth(
        &mut self,
        project_id: &str,
        role: NodeRole,
        depth: u32,
        parent: Option<usize>,
        pinned: Option<&str>,
    ) -> Result<usize, String> {
        let (index, is_new) = self.remember(DependencySource::Modrinth, project_id, role, depth);
        if let Some(parent) = parent {
            self.add_constraint(index, parent, pinned);
        }
        if !is_new || role != NodeRole::Hard || depth > MAX_DEPENDENCY_DEPTH {
            return Ok(index);
        }

        let hints: Vec<String> = self.states[index]
            .constraints
            .iter()
            .map(|(_, v)| v.clone())
            .collect();
        self.modrinth_candidates(project_id, &hints).await?;
        self.ensure_modrinth_project(project_id).await;
        if let Some(project) = self.modrinth_projects.get(project_id) {
            self.nodes[index].title = Some(project.title.clone());
            self.nodes[index].icon_url = project.icon_url.clone();
        }

        let Some(selected) = self
            .modrinth
            .get(project_id)
            .and_then(|c| select_modrinth(&self.states[index].constraints, c))
            .cloned()
        else {
            self.states[index].unresolved_reason =
                Some(format!("No hay versión compatible de {}", project_id));
            return Ok(index);
        };

        let mut deps: Vec<(DependencyKind, String, Option<String>)> = Vec::new();
        for dep in &selected.dependencies {
            let kind = DependencyKind::from_modrinth(&dep.dependency_type);
            let Some(sub_project) = &dep.project_id else {
                continue;
            };
            deps.push((kind, sub_project.clone(), dep.version_id.clone()));
        }
        // Load the versions pinned by the parent before descending so children
        // do not issue a second request for the same project.
        for (kind, sub_project, version) in &deps {
            if *kind != DependencyKind::Required || sub_project == project_id {
                continue;
            }
            let hint: &[String] = match version {
                Some(version) => std::slice::from_ref(version),
                None => &[],
            };
            self.modrinth_candidates(sub_project, hint).await?;
        }

        for (kind, sub_project, version) in deps {
            let child_role = match kind {
                DependencyKind::Optional => NodeRole::Optional,
                DependencyKind::Incompatible => NodeRole::Incompatible,
                _ => NodeRole::Hard,
            };
            let child = if child_role == NodeRole::Hard {
                Box::pin(self.collect_modrinth(
                    &sub_project,
                    child_role,
                    depth + 1,
                    Some(index),
                    version.as_deref(),
                ))
                .await?
            } else {
                let (child, _) = self.remember(
                    DependencySource::Modrinth,
                    &sub_project,
                    child_role,
                    depth + 1,
                );
                self.add_constraint(child, index, version.as_deref());
                self.describe_node(child).await;
                child
            };
            self.push_child(index, child);
        }
        self.solve(index);
        Ok(index)
    }

    async fn collect_curseforge(
        &mut self,
        mod_id: u32,
        role: NodeRole,
        depth: u32,
        parent: Option<usize>,
        pinned: Option<u32>,
    ) -> Result<usize, String> {
        let project_id = mod_id.to_string();
        let (index, is_new) = self.remember(DependencySource::Curseforge, &project_id, role, depth);
        if let Some(parent) = parent {
            self.add_constraint(index, parent, pinned.map(|id| id.to_string()).as_deref());
        }
        if !is_new || role != NodeRole::Hard || depth > MAX_DEPENDENCY_DEPTH {
            return Ok(index);
        }

        let hints: Vec<String> = self.states[index]
            .constraints
            .iter()
            .map(|(_, v)| v.clone())
            .collect();
        self.curseforge_candidates(mod_id, &hints).await?;
        self.ensure_curseforge_project(mod_id).await;
        if let Some(project) = self.curseforge_projects.get(&project_id) {
            self.nodes[index].title = Some(project.name.clone());
            self.nodes[index].icon_url = project.logo.as_ref().map(|l| l.url.clone());
        }

        let Some(selected) = self
            .curseforge
            .get(&project_id)
            .and_then(|c| select_curseforge(&self.states[index].constraints, c))
            .cloned()
        else {
            self.states[index].unresolved_reason =
                Some(format!("No hay archivo compatible del mod {}", mod_id));
            return Ok(index);
        };

        let mut deps: Vec<(DependencyKind, u32, Option<u32>)> = Vec::new();
        for dep in &selected.dependencies {
            let Some(sub_mod_id) = dep.mod_id else {
                continue;
            };
            let kind = DependencyKind::from_curseforge_relation(dep.relation_type)
                .unwrap_or(DependencyKind::Optional);
            deps.push((kind, sub_mod_id, dep.file_id));
        }
        let sub_hints: Vec<String> = deps
            .iter()
            .filter(|(kind, _, _)| *kind == DependencyKind::Required)
            .filter_map(|(_, _, file_id)| file_id.map(|id| id.to_string()))
            .collect();

        for (kind, sub_mod_id, file_id) in deps {
            self.curseforge_candidates(sub_mod_id, &sub_hints).await?;
            let child_role = match kind {
                DependencyKind::Optional => NodeRole::Optional,
                DependencyKind::Incompatible => NodeRole::Incompatible,
                _ => NodeRole::Hard,
            };
            let child = if child_role == NodeRole::Hard {
                Box::pin(self.collect_curseforge(
                    sub_mod_id,
                    child_role,
                    depth + 1,
                    Some(index),
                    file_id,
                ))
                .await?
            } else {
                let (child, _) = self.remember(
                    DependencySource::Curseforge,
                    &sub_mod_id.to_string(),
                    child_role,
                    depth + 1,
                );
                self.add_constraint(child, index, file_id.map(|id| id.to_string()).as_deref());
                self.describe_node(child).await;
                child
            };
            self.push_child(index, child);
        }
        self.solve(index);
        Ok(index)
    }

    /// Picks the version for a node and records any conflict its constraints
    /// cause. Safe to call repeatedly: the selected version and the conflict
    /// list entry are replaced instead of appended.
    fn solve(&mut self, index: usize) {
        if self.nodes[index].role != NodeRole::Hard {
            return;
        }
        let info = self.nodes[index].clone();
        let constraints = self.states[index].constraints.clone();
        let ids = unique_constraint_ids(&constraints);

        let selected = match info.source {
            DependencySource::Modrinth => {
                let candidates = self.modrinth.get(&info.project_id);
                candidates
                    .and_then(|c| select_modrinth(&constraints, c))
                    .map(|version| {
                        (
                            version.id.clone(),
                            candidates.is_some_and(|c| c.is_compatible(version)),
                        )
                    })
            }
            DependencySource::Curseforge => {
                let candidates = self.curseforge.get(&info.project_id);
                candidates
                    .and_then(|c| select_curseforge(&constraints, c))
                    .map(|file| {
                        (
                            file.id.to_string(),
                            candidates.is_some_and(|c| c.is_compatible(file)),
                        )
                    })
            }
        };

        let Some((version_id, compatible)) = selected else {
            self.states[index].selected = None;
            if self.states[index].unresolved_reason.is_none() {
                self.states[index].unresolved_reason =
                    Some(format!("Sin versión disponible para {}", info.project_id));
            }
            self.record_conflict(index, Some(ConflictType::Unresolved));
            return;
        };
        self.states[index].selected = Some(version_id);
        if !compatible {
            self.states[index].unresolved_reason = Some(format!(
                "La versión pedida de {} no es compatible con Minecraft {}{}",
                info.title.as_deref().unwrap_or(&info.project_id),
                self.game_version,
                if self.loader.is_empty() {
                    String::new()
                } else {
                    format!(" / {}", self.loader)
                }
            ));
            self.record_conflict(index, Some(ConflictType::Unresolved));
            return;
        }
        // Running solve() again (final pass) must replace, not append, the
        // entry for this project.
        self.states[index].unresolved_reason = None;
        if ids.len() > 1 {
            self.record_conflict(index, Some(ConflictType::VersionMismatch));
        } else {
            self.record_conflict(index, None);
        }
    }

    fn record_conflict(&mut self, index: usize, kind: Option<ConflictType>) {
        let conflict = kind.map(|kind| self.build_conflict(index, kind));
        let project_id = self.nodes[index].project_id.clone();
        let source = self.nodes[index].source;
        if let Some(position) = self
            .conflicts
            .iter()
            .position(|c| c.project_id == project_id && c.source == source)
        {
            match conflict {
                Some(conflict) => self.conflicts[position] = conflict,
                None => {
                    self.conflicts.remove(position);
                }
            }
            return;
        }
        if let Some(conflict) = conflict {
            self.conflicts.push(conflict);
        }
    }

    fn build_conflict(&self, index: usize, kind: ConflictType) -> DependencyConflict {
        let info = &self.nodes[index];
        let state = &self.states[index];
        let mut requested_by = Vec::new();
        let mut requested_versions = Vec::new();
        let mut seen = HashSet::new();
        for (parent, version_id) in &state.constraints {
            let Some(parent_node) = self.nodes.get(*parent) else {
                continue;
            };
            if !seen.insert((parent_node.project_id.clone(), version_id.clone())) {
                continue;
            }
            requested_by.push(DependencyOrigin {
                project_id: parent_node.project_id.clone(),
                source: parent_node.source,
                version_id: self.states[*parent].selected.clone(),
            });
            if !version_id.is_empty() {
                requested_versions.push(RequestedVersion {
                    version_id: version_id.clone(),
                    requested_by: parent_node.project_id.clone(),
                });
            }
        }
        let reason = match kind {
            ConflictType::VersionMismatch => Some(format!(
                "Los padres piden versiones distintas ({})",
                unique_constraint_ids(&state.constraints).join(", ")
            )),
            ConflictType::Unresolved => state.unresolved_reason.clone(),
        };
        DependencyConflict {
            project_id: info.project_id.clone(),
            source: info.source,
            title: info
                .title
                .clone()
                .unwrap_or_else(|| info.project_id.clone()),
            icon_url: info.icon_url.clone(),
            kind,
            requested_by,
            requested_versions,
            installed_version: None,
            reason,
        }
    }

    fn build_node(&self, index: usize) -> ResolvedDependency {
        let info = &self.nodes[index];
        let state = &self.states[index];
        let mut filename = String::new();
        let mut download_url = None;
        let mut game_versions = Vec::new();
        let mut loaders = Vec::new();
        match info.source {
            DependencySource::Modrinth => {
                let version = state
                    .selected
                    .as_deref()
                    .and_then(|id| self.modrinth.get(&info.project_id)?.find(id));
                if let Some(version) = version {
                    game_versions = version.game_versions.clone();
                    loaders = version.loaders.clone();
                    if let Some(file) = modrinth_file(version) {
                        filename = file.filename.clone();
                        download_url = Some(file.url.clone());
                    }
                }
            }
            DependencySource::Curseforge => {
                let file = state
                    .selected
                    .as_deref()
                    .and_then(|id| id.parse::<u32>().ok())
                    .and_then(|id| self.curseforge.get(&info.project_id)?.find(id));
                if let Some(file) = file {
                    game_versions = curseforge_player_versions(file);
                    loaders = curseforge_file_loaders(file);
                    filename = file.file_name.clone();
                    download_url = file.download_url.clone();
                }
            }
        }
        let children = self.children[index]
            .iter()
            .map(|&child| self.build_node(child))
            .collect();
        ResolvedDependency {
            project_id: info.project_id.clone(),
            version_id: state.selected.clone(),
            source: info.source,
            title: info
                .title
                .clone()
                .unwrap_or_else(|| info.project_id.clone()),
            icon_url: info.icon_url.clone(),
            filename,
            download_url,
            kind: info.role.kind(),
            depth: info.depth,
            unresolved_reason: state.unresolved_reason.clone(),
            game_versions,
            loaders,
            children,
        }
    }

    fn incompatible_projects(&self) -> Vec<DependencyOrigin> {
        self.nodes
            .iter()
            .filter(|node| node.role == NodeRole::Incompatible)
            .map(|node| DependencyOrigin {
                project_id: node.project_id.clone(),
                source: node.source,
                version_id: None,
            })
            .collect()
    }
}

// ── Entry point ───────────────────────────────────────────────────────────────

pub async fn resolve_dependencies(
    requests: Vec<DependencyRequest>,
    loader: String,
    game_version: String,
    installed: Vec<InstalledModRef>,
) -> Result<DependencyResolutionResult, String> {
    let mut collector = Collector {
        loader,
        game_version,
        ..Collector::default()
    };

    let mut roots = Vec::new();
    for request in requests {
        let role = match request.kind {
            DependencyKind::Optional => NodeRole::Optional,
            DependencyKind::Incompatible => NodeRole::Incompatible,
            _ => NodeRole::Hard,
        };
        let index = match request.source {
            DependencySource::Modrinth => {
                Box::pin(collector.collect_modrinth(
                    &request.project_id,
                    role,
                    0,
                    None,
                    request.version_id.as_deref(),
                ))
                .await?
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
                Box::pin(collector.collect_curseforge(mod_id, role, 0, None, file_id)).await?
            }
        };
        roots.push(index);
    }

    // Constraints discovered late (a child pinning a version its parent also
    // needs) must be reflected in every node, so solve once more in order.
    for index in 0..collector.nodes.len() {
        collector.solve(index);
    }

    let tree = roots
        .iter()
        .map(|&index| collector.build_node(index))
        .collect();
    let incompatible = collector.incompatible_projects();
    Ok(DependencyResolutionResult {
        tree,
        conflicts: annotate_installed_versions(collector.conflicts, &installed),
        incompatible,
    })
}

/// Fills `installed_version` with a human readable label: the metadata version
/// when the installed file shares content with the selected version, the file
/// name when the provider reports a different one, and the raw id only when
/// nothing else identifies it.
fn annotate_installed_versions(
    conflicts: Vec<DependencyConflict>,
    installed: &[InstalledModRef],
) -> Vec<DependencyConflict> {
    conflicts
        .into_iter()
        .map(|mut conflict| {
            let Some(local) = installed.iter().find(|local| {
                local.enabled
                    && local.source == conflict.source
                    && local.project_id == conflict.project_id
            }) else {
                return conflict;
            };
            // The label comes from the installed file: its metadata version,
            // or its name when the provider did not record one.
            conflict.installed_version = Some(
                local
                    .version
                    .as_deref()
                    .filter(|version| !version.trim().is_empty())
                    .map(str::to_string)
                    .unwrap_or_else(|| display_filename(&local.filename)),
            );
            conflict
        })
        .collect()
}

fn display_filename(filename: &str) -> String {
    filename
        .strip_suffix(".disabled")
        .unwrap_or(filename)
        .to_string()
}

// ── Network helpers ───────────────────────────────────────────────────────────

async fn fetch_modrinth_project(project_id: &str) -> Result<ModrinthProjectBrief, String> {
    let url = format!("{}/project/{}", MODRINTH_API_BASE, project_id);
    let value = crate::commands::market::get_json(&url).await?;
    serde_json::from_value(value)
        .map_err(|e| format!("Error al parsear proyecto de Modrinth: {}", e))
}

fn modrinth_versions_url(project_id: &str, loader: &str, game_version: &str) -> String {
    let mut url = format!("{}/project/{}/version", MODRINTH_API_BASE, project_id);
    let mut first = true;
    {
        let mut append_param = |url: &mut String, name: &str, value: &str| {
            url.push(if first { '?' } else { '&' });
            first = false;
            url.push_str(name);
            url.push('=');
            url.push_str(&urlencoding::encode(value));
        };
        let loader_lower = loader.to_lowercase();
        if !loader_lower.is_empty() && loader_lower != "vanilla" {
            append_param(
                &mut url,
                "loaders",
                &serde_json::to_string(&vec![loader_lower]).unwrap_or_default(),
            );
        }
        if !game_version.is_empty() {
            append_param(
                &mut url,
                "game_versions",
                &serde_json::to_string(&vec![game_version]).unwrap_or_default(),
            );
        }
    }
    url
}

async fn fetch_modrinth_project_versions(
    project_id: &str,
    loader: &str,
    game_version: &str,
) -> Result<Vec<ModrinthVersionBrief>, String> {
    let url = modrinth_versions_url(project_id, loader, game_version);
    let value = crate::commands::market::get_json(&url).await?;
    serde_json::from_value(value)
        .map_err(|e| format!("Error al parsear versiones de Modrinth: {}", e))
}

async fn fetch_modrinth_version(version_id: &str) -> Result<ModrinthVersionBrief, String> {
    let url = format!("{}/version/{}", MODRINTH_API_BASE, version_id);
    let value = crate::commands::market::get_json(&url).await?;
    serde_json::from_value(value)
        .map_err(|e| format!("Error al parsear versión de Modrinth: {}", e))
}

/// Looks up several versions at once. Failures are dropped so a single missing
/// version never fails the whole resolution.
async fn fetch_modrinth_versions(version_ids: &[String]) -> Vec<ModrinthVersionBrief> {
    let mut versions = Vec::new();
    for chunk in version_ids.chunks(MODRINTH_BATCH_SIZE) {
        if chunk.len() == 1 {
            if let Ok(version) = fetch_modrinth_version(&chunk[0]).await {
                versions.push(version);
            }
            continue;
        }
        let url = format!("{}/version_files", MODRINTH_API_BASE);
        let body = serde_json::json!({ "ids": chunk });
        let Ok(resp) = crate::core::http_client::HTTP
            .post(&url)
            .json(&body)
            .send()
            .await
        else {
            continue;
        };
        if !resp.status().is_success() {
            // Fall back to individual lookups when the batch endpoint rejects
            // the payload.
            for id in chunk {
                if let Ok(version) = fetch_modrinth_version(id).await {
                    versions.push(version);
                }
            }
            continue;
        }
        let Ok(value) = resp.json::<serde_json::Value>().await else {
            continue;
        };
        let Some(object) = value.as_object() else {
            continue;
        };
        for id in chunk {
            if let Some(value) = object.get(id)
                && let Ok(version) = serde_json::from_value::<ModrinthVersionBrief>(value.clone())
            {
                versions.push(version);
            } else if let Ok(version) = fetch_modrinth_version(id).await {
                versions.push(version);
            }
        }
    }
    versions
}

async fn fetch_curseforge_project(mod_id: u32) -> Result<CurseForgeProject, String> {
    let client = CurseForgeClient::from_settings_or_default();
    client.get_project(mod_id).await.map_err(|e| e.to_string())
}

async fn fetch_curseforge_project_files(mod_id: u32) -> Result<Vec<CurseForgeFile>, String> {
    let client = CurseForgeClient::from_settings_or_default();
    client
        .get_all_project_files(mod_id, None, None)
        .await
        .map_err(|e| e.to_string())
}

async fn fetch_curseforge_files(file_ids: &[u32]) -> Vec<CurseForgeFile> {
    let client = CurseForgeClient::from_settings_or_default();
    client.get_mod_files(file_ids).await.unwrap_or_default()
}

#[cfg(test)]
#[path = "../tests/services/dependency_resolver.rs"]
mod tests;
