use crate::core::path_manager::PathManager;
use std::fs;
use tokio::fs as tokio_fs;

#[tauri::command]
pub async fn save_locale(lang: String, data: String) -> Result<(), String> {
	let dir = PathManager::get().get_settings_dir().join("i18n");
	fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

	let path = dir.join(format!("{lang}.json"));
	tokio_fs::write(&path, &data)
		.await
		.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn load_locale(lang: String) -> Result<Option<String>, String> {
	let path = PathManager::get()
		.get_settings_dir()
		.join("i18n")
		.join(format!("{lang}.json"));

	if path.exists() {
		tokio_fs::read_to_string(&path)
			.await
			.map(Some)
			.map_err(|e| e.to_string())
	} else {
		Ok(None)
	}
}
