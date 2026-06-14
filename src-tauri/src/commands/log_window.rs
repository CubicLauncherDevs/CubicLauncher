use crate::services::launcher::{LogLine, get_log_history};
use dashmap::DashMap;
use std::sync::OnceLock;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

static LOG_WINDOWS: OnceLock<DashMap<String, String>> = OnceLock::new();

#[tauri::command]
pub fn get_log_history_cmd(instance_id: String) -> Vec<LogLine> {
    get_log_history(&instance_id)
}

#[tauri::command]
pub async fn open_log_window(
    app: AppHandle,
    instance_id: String,
    instance_name: String,
) -> Result<(), String> {
    let label = format!("log-{}", instance_id);

    let map = LOG_WINDOWS.get_or_init(DashMap::new);
    if let Some(existing) = map.get(&instance_id) {
        if let Some(w) = app.get_webview_window(existing.value()) {
            let _ = w.set_focus();
            return Ok(());
        }
    }

    let encoded_id = urlencoding::encode(&instance_id);
    let encoded_name = urlencoding::encode(&instance_name);
    let path = format!("/?log={}&name={}", encoded_id, encoded_name);

    WebviewWindowBuilder::new(&app, &label, WebviewUrl::App(path.into()))
        .title(format!("Logs — {}", instance_name))
        .inner_size(800.0, 500.0)
        .min_inner_size(400.0, 300.0)
        .resizable(true)
        .decorations(true)
        .transparent(false)
        .build()
        .map_err(|e| e.to_string())?;

    map.insert(instance_id, label);
    Ok(())
}
