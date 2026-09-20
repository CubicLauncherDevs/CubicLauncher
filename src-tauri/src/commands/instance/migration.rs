use crate::services::instance_import::migration::{
    self, MigrationResult, Progress, Provider, Scan, Selection,
};
use tauri::ipc::Channel;

#[tauri::command]
pub async fn scan_launcher_migration(
    provider: Provider,
    root: Option<String>,
) -> Result<Scan, String> {
    migration::scan(provider, root).await
}

#[tauri::command]
pub async fn migrate_launcher_instances(
    token: String,
    selections: Vec<Selection>,
    on_progress: Channel<Progress>,
) -> Result<Vec<MigrationResult>, String> {
    migration::migrate(token, selections, on_progress).await
}

#[tauri::command]
pub fn cancel_launcher_migration(token: String) {
    migration::cancel(&token);
}
