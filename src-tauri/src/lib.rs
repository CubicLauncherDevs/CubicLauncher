mod commands;
mod core;
mod services;
pub(crate) mod theme_watcher;

pub use services::InstanceManager;
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            commands::instance::get_instances,
            commands::instance::create_instance,
            commands::instance::launch,
            commands::instance::kill_instance,
            commands::instance::export::export_instance_zip,
            commands::instance::delete_instance,
            commands::instance::open_instance_dir,
            commands::instance::rename_instance,
            commands::instance::update_instance,
            commands::instance::pin_instance,
            commands::instance::get_installed_versions,
            commands::instance::get_installed_versions_with_status,
            commands::instance::check_version_integrity,
            commands::instance::get_instance_mods,
            commands::instance::toggle_instance_mod,
            commands::instance::get_instance_screenshot,
            commands::instance::get_instance_banner,
            commands::instance::get_all_instance_screenshots,
            commands::instance::set_instance_cover_image,
            commands::instance::reset_instance_cover_image,
            commands::instance::get_instance_resourcepacks,
            commands::instance::get_instance_shaderpacks,
            commands::instance::delete_instance_file,
            commands::instance::add_instance_file,
            commands::instance::reinstall_version,
            commands::instance::instance_import::detect_instance_zip,
            commands::instance::instance_import::import_instance_zip,
            commands::instance::instance_import::cancel_instance_import,
            commands::instance::upload_custom_icon,
            commands::instance::reset_instance_icon,
            commands::dependencies::resolve_mod_dependencies,
            commands::download::add_to_queue,
            commands::download::get_available_versions,
            commands::download::get_fabric_versions,
            commands::download::get_fabric_loader_versions,
            commands::download::download_fabric,
            commands::download::get_download_queue,
            commands::download::refresh_versions,
            commands::download::get_forge_versions,
            commands::download::refresh_forge_versions,
            commands::download::get_neoforge_versions,
            commands::download::refresh_neoforge_versions,
            commands::download::download_neoforge,
            commands::download::get_quilt_versions,
            commands::download::get_quilt_loader_versions,
            commands::download::refresh_quilt_versions,
            commands::download::download_quilt,
            commands::others::open_url,
            commands::settings::get_settings,
            commands::settings::update_settings,
            commands::settings::detect_java_paths,
            commands::settings::get_recommended_ram,
            commands::i18n::save_locale,
            commands::i18n::load_locales,
            commands::auth::get_device_code,
            commands::auth::authenticate_with_device_code,
            commands::auth::start_webview_auth,
            commands::auth::get_current_user,
            commands::auth::logout,
            commands::auth::switch_user,
            commands::auth::remove_user,
            commands::auth::get_user_list,
            commands::auth::get_yggdrasil_server_info,
            commands::auth::yggdrasil_authenticate,
            commands::auth::yggdrasil_refresh_token,
            commands::avatar::get_avatar_svg,
            commands::themes::list_themes,
            commands::themes::get_user_theme,
            commands::themes::set_theme,
            commands::themes::get_current_theme,
            commands::themes::import_theme,
            commands::themes::import_theme_zip,
            commands::themes::import_theme_cbth,
            commands::themes::get_themes_dir_path,
            commands::themes::remove_theme,
            commands::themes::export_theme,
            commands::market::search_modrinth,
            commands::market::get_modrinth_project,
            commands::market::get_modrinth_project_versions,
            commands::market::get_modrinth_version,
            commands::market::get_modrinth_latest_versions,
            commands::market::search_curseforge,
            commands::market::search_curseforge_modpacks,
            commands::market::get_curseforge_project,
            commands::market::get_curseforge_project_description,
            commands::market::get_curseforge_project_files,
            commands::market::get_curseforge_file_download_url,
            commands::modrinth::download_mods,
            commands::modrinth::download_resourcepacks,
            commands::modrinth::download_shaderpacks,
            commands::modrinth::download_mrpack,
            commands::pack::parse_mrpack,
            commands::pack::install_mrpack,
            commands::pack::download_curseforge_modpack,
            commands::pack::parse_curseforge_modpack,
            commands::pack::install_curseforge_modpack,
            commands::profile::get_minecraft_profile,
            commands::profile::read_skin_preview_data,
            commands::profile::upload_skin_file,
            commands::profile::upload_skin_url,
            commands::profile::equip_cape,
            commands::profile::unequip_cape,
            commands::skin_closet::get_skin_closet,
            commands::skin_closet::sync_skin_closet,
            commands::skin_closet::remove_skin_from_closet,
            commands::skin_closet::rename_skin_in_closet,
            commands::skin_closet::equip_skin_from_closet,
            commands::forge::install_forge,
            commands::forge::download_forge,
            commands::neoforge::install_neoforge,
            commands::discord::init_discord_presence,
            commands::discord::shutdown_discord_presence,
            commands::java::get_jre_status,
            commands::java::install_jre,
            commands::java::uninstall_jre,
            commands::java::get_jre_versions,
            commands::log_window::open_log_window,
            commands::log_window::get_log_history_cmd,
            commands::log_window::upload_log_to_mclogs,
        ])
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .setup(|app| {
            if let Err(errors) = core::PathManager::ensure_dirs() {
                use tauri_plugin_dialog::DialogExt;
                let handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    handle
                        .dialog()
                        .message(format!(
                            "No se pudieron crear los directorios necesarios:\n{}",
                            errors.join("\n")
                        ))
                        .title("Error de inicialización")
                        .kind(tauri_plugin_dialog::MessageDialogKind::Error)
                        .show(|_| std::process::exit(1));
                });
                return Err("Error de inicialización: no se pudieron crear los directorios".into());
            }

            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                tokio::join!(
                    services::DownloadQueue::init(Some(handle.clone())),
                    async {
                        services::Launcher::init().set_handle(handle.clone());
                    },
                    InstanceManager::init(),
                    async {
                        core::init(handle.clone());
                    },
                    theme_watcher::ThemeWatcher::start(),
                );
                services::settings_manager::init_auto_save();
                let theme = services::SettingsManager::read().theme.clone();
                if let Some(dir) = theme.strip_prefix("user:") {
                    theme_watcher::ThemeWatcher::watch(Some(dir.to_string()));
                }
            });
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app_handle, event| {
            if let tauri::RunEvent::ExitRequested { api, .. } = event
                && services::launcher::should_keep_alive()
            {
                api.prevent_exit();
            }
        });
}
