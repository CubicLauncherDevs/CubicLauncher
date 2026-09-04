use super::profile::{get_minecraft_profile_impl, upload_skin_file_impl, upload_skin_url_impl};
use crate::services::skin_closet_manager::{
    SkinClosetEntry, SkinClosetManager, SkinImageSource, now_ts,
};
use std::path::Path;
use tauri::command;

#[command]
pub async fn get_skin_closet(uuid: String) -> Result<Vec<SkinClosetEntry>, String> {
    Ok(SkinClosetManager::get_entries(&uuid).await)
}

#[command]
pub async fn sync_skin_closet(uuid: String) -> Result<Vec<SkinClosetEntry>, String> {
    let profile = get_minecraft_profile_impl(&uuid).await?;

    if let Some(active) = profile.skins.into_iter().find(|s| s.state == "ACTIVE") {
        let entries = SkinClosetManager::get_entries(&uuid).await;
        if !entries.iter().any(|e| e.id == active.id) {
            let local_path = SkinClosetManager::store_image(
                &uuid,
                &active.id,
                SkinImageSource::Url(active.url.clone()),
            )
            .await?;

            let entry = SkinClosetEntry {
                id: active.id,
                url: active.url,
                local_path: local_path.to_string_lossy().to_string(),
                variant: active.variant,
                alias: String::new(),
                saved_at: now_ts(),
            };
            SkinClosetManager::sync_entry(&uuid, entry).await?;
        }
    }

    Ok(SkinClosetManager::get_entries(&uuid).await)
}

#[command]
pub async fn remove_skin_from_closet(uuid: String, entry_id: String) -> Result<(), String> {
    SkinClosetManager::remove_entry(&uuid, &entry_id).await
}

#[command]
pub async fn rename_skin_in_closet(
    uuid: String,
    entry_id: String,
    alias: String,
) -> Result<(), String> {
    SkinClosetManager::update_alias(&uuid, &entry_id, alias).await
}

#[command]
pub async fn equip_skin_from_closet(uuid: String, entry_id: String) -> Result<(), String> {
    let entries = SkinClosetManager::get_entries(&uuid).await;
    let entry = entries
        .into_iter()
        .find(|e| e.id == entry_id)
        .ok_or_else(|| "Skin no encontrada en el closet".to_string())?;

    let variant = if entry.variant.to_lowercase() == "slim" {
        "slim"
    } else {
        "classic"
    };

    if Path::new(&entry.local_path).exists() {
        upload_skin_file_impl(&uuid, &entry.local_path, variant).await?;
    } else {
        upload_skin_url_impl(&uuid, &entry.url, variant).await?;
    }

    Ok(())
}
