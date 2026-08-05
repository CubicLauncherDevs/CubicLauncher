use crate::core::http_client::HTTP;
use crate::services::launcher::{LogLine, get_log_history, sanitize_with_user};
use dashmap::DashMap;
use std::sync::OnceLock;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

static LOG_WINDOWS: OnceLock<DashMap<String, String>> = OnceLock::new();

fn remove_log_window(instance_id: &str) {
    if let Some(map) = LOG_WINDOWS.get() {
        map.remove(instance_id);
    }
}

#[tauri::command]
pub fn get_log_history_cmd(instance_id: String, limit: Option<usize>) -> Vec<LogLine> {
    get_log_history(&instance_id, limit)
}

/// Abre la ventana de logs de una instancia. Puede llamarse tanto desde
/// comandos Tauri como directamente desde el backend.
pub async fn open_log_window_for_instance(
    app: AppHandle,
    instance_id: String,
    instance_name: String,
) -> Result<(), String> {
    let label = format!("log-{}", instance_id);

    let map = LOG_WINDOWS.get_or_init(DashMap::new);
    if let Some(existing) = map.get(&instance_id)
        && let Some(w) = app.get_webview_window(existing.value())
    {
        let _ = w.set_focus();
        return Ok(());
    }

    let encoded_id = urlencoding::encode(&instance_id);
    let encoded_name = urlencoding::encode(&instance_name);
    let path = format!("/?log={}&name={}", encoded_id, encoded_name);

    let window = WebviewWindowBuilder::new(&app, &label, WebviewUrl::App(path.into()))
        .title(format!("Logs — {}", instance_name))
        .inner_size(800.0, 500.0)
        .min_inner_size(400.0, 300.0)
        .resizable(true)
        .decorations(true)
        .build()
        .map_err(|e| e.to_string())?;

    let id = instance_id.clone();
    window.on_window_event(move |event| {
        if let tauri::WindowEvent::CloseRequested { .. } = event {
            remove_log_window(&id);
        }
    });

    map.insert(instance_id, label);
    Ok(())
}

#[tauri::command]
pub async fn open_log_window(
    app: AppHandle,
    instance_id: String,
    instance_name: String,
) -> Result<(), String> {
    open_log_window_for_instance(app, instance_id, instance_name).await
}

#[tauri::command]
pub async fn upload_log_to_mclogs(content: String) -> Result<String, String> {
    let content = sanitize_with_user(&content);
    let resp = HTTP
        .post("https://api.mclo.gs/1/log")
        .form(&[("content", content.as_str())])
        .send()
        .await
        .map_err(|e| format!("Error de red: {}", e))?;

    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Error al leer respuesta: {}", e))?;

    if json["success"] == true {
        json["url"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| "URL no encontrada en la respuesta".to_string())
    } else {
        Err(json["error"]
            .as_str()
            .unwrap_or("Error desconocido")
            .to_string())
    }
}
