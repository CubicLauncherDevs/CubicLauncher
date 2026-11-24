use crate::services::dependency_resolver::{
    DependencyRequest, DependencyResolutionResult, InstalledModRef,
};

#[tauri::command]
pub async fn resolve_mod_dependencies(
    requests: Vec<DependencyRequest>,
    loader: String,
    game_version: String,
    installed: Option<Vec<InstalledModRef>>,
) -> Result<DependencyResolutionResult, String> {
    // Older frontends do not send the inventory; conflicts then keep the raw
    // version ids instead of readable labels.
    let installed = installed
        .unwrap_or_default()
        .into_iter()
        .filter(|local| !local.project_id.is_empty())
        .collect();
    crate::services::dependency_resolver::resolve_dependencies(
        requests,
        loader,
        game_version,
        installed,
    )
    .await
}
