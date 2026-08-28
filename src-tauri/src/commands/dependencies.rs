use crate::services::dependency_resolver::{DependencyRequest, DependencyResolutionResult};

#[tauri::command]
pub async fn resolve_mod_dependencies(
    requests: Vec<DependencyRequest>,
    loader: String,
    game_version: String,
) -> Result<DependencyResolutionResult, String> {
    crate::services::dependency_resolver::resolve_dependencies(requests, loader, game_version).await
}
