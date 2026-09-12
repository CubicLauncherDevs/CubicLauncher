use super::launch::validate_uuid;
use crate::services::{InstanceManager, server_manager as servers, server_status};
use tauri::ipc::Channel;

async fn read_list(instance_id: &str) -> Result<servers::ServerList, String> {
    validate_uuid(instance_id)?;
    let handle = InstanceManager::get()
        .get_handle(instance_id)
        .await
        .ok_or_else(|| servers::error("SERVERS_NOT_FOUND", ""))?;
    let guard = handle
        .try_lock_files()
        .map_err(|e| servers::error("SERVERS_BUSY", e))?;
    let dir = handle.get_instance_dir().await;
    tokio::task::spawn_blocking(move || {
        let _guard = guard;
        servers::list(&dir)
    })
    .await
    .map_err(|e| servers::error("SERVERS_READ", e))?
}

#[tauri::command]
pub async fn get_instance_servers(instance_id: String) -> Result<servers::ServerList, String> {
    read_list(&instance_id).await
}

#[tauri::command]
pub async fn get_instance_server_icons(
    instance_id: String,
    revision: String,
    indices: Vec<usize>,
) -> Result<Vec<servers::ServerIcon>, String> {
    validate_uuid(&instance_id)?;
    let handle = InstanceManager::get()
        .get_handle(&instance_id)
        .await
        .ok_or_else(|| servers::error("SERVERS_NOT_FOUND", ""))?;
    let guard = handle
        .try_lock_files()
        .map_err(|e| servers::error("SERVERS_BUSY", e))?;
    let dir = handle.get_instance_dir().await;
    tokio::task::spawn_blocking(move || {
        let _guard = guard;
        servers::icons(&dir, &revision, indices)
    })
    .await
    .map_err(|e| servers::error("SERVERS_READ", e))?
}

#[tauri::command]
pub async fn instance_server_action(
    instance_id: String,
    revision: String,
    action: servers::ServerAction,
) -> Result<servers::ServerList, String> {
    validate_uuid(&instance_id)?;
    let handle = InstanceManager::get()
        .get_handle(&instance_id)
        .await
        .ok_or_else(|| servers::error("SERVERS_NOT_FOUND", ""))?;
    let guard = handle
        .try_lock_files()
        .map_err(|e| servers::error("SERVERS_BUSY", e))?;
    if handle.is_busy() {
        return Err(servers::error("SERVERS_BUSY", ""));
    }
    let dir = handle.get_instance_dir().await;
    tokio::task::spawn_blocking(move || {
        let _guard = guard;
        servers::apply(&dir, &revision, action)
    })
    .await
    .map_err(|e| servers::error("SERVERS_WRITE", e))?
}

#[tauri::command]
pub async fn start_instance_server_ping(
    window: tauri::WebviewWindow,
    instance_id: String,
    targets: Vec<server_status::PingTarget>,
    on_status: Channel<server_status::ServerStatusEvent>,
) -> Result<String, String> {
    validate_uuid(&instance_id)?;
    if InstanceManager::get()
        .get_handle(&instance_id)
        .await
        .is_none()
    {
        return Err(servers::error("SERVERS_NOT_FOUND", ""));
    }
    // Read-only network requests use the displayed snapshot. File revisions
    // are still checked when fetching icons or modifying servers.dat.
    server_status::start(window.label(), targets, on_status)
}

#[tauri::command]
pub fn cancel_instance_server_ping(window: tauri::WebviewWindow, request_id: String) {
    server_status::cancel(window.label(), &request_id);
}
