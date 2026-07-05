mod addon_manager;
pub(crate) mod discord_presence;
pub(crate) mod download_queue;
mod instance_manager;
pub(crate) mod java_manager;
pub(crate) mod launcher;
pub(crate) mod settings_manager;

pub use addon_manager::*;
pub use download_queue::{DownloadQueue, DownloadState};
pub use instance_manager::{
    InstOverrides, InstanceDto, InstanceManager, InstanceStatus, signal_kill,
};
pub use launcher::Launcher;
pub use settings_manager::{SettingsManager, SettingsSnapshot};
