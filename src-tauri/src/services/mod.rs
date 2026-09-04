mod addon_manager;
pub(crate) mod curseforge_api;
pub(crate) mod curseforge_modpack;
pub mod dependency_resolver;
pub(crate) mod discord_presence;
pub(crate) mod download_queue;
pub mod instance_export;
pub mod instance_import;
mod instance_manager;
pub(crate) mod java_manager;
pub(crate) mod launcher;
pub(crate) mod settings_manager;
pub mod skin_closet_manager;

pub use addon_manager::*;
pub use download_queue::{DownloadQueue, DownloadState};
pub use instance_import::{
    InstanceImportPlan, cancel_preview, detect_instance_zip, import_instance_zip,
};
pub use instance_manager::{
    InstOverrides, InstanceDto, InstanceManager, InstanceStatus, signal_kill,
};
pub use launcher::Launcher;
pub use settings_manager::{SettingsManager, SettingsSnapshot};
