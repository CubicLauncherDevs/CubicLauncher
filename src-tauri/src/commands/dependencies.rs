use crate::services::dependency_resolver::{DependencyResolutionResult, DependencySource};

#[tauri::command]
pub async fn resolve_mod_dependencies(
    _source: DependencySource,
    _project_id: String,
    _version_id: Option<String>,
    _loader: String,
    _game_version: String,
) -> Result<DependencyResolutionResult, String> {
    // TODO: implementar lógica de resolución recursiva en fase 2.
    Ok(DependencyResolutionResult {
        tree: vec![],
        conflicts: vec![],
    })
}
