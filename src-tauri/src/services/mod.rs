mod addon_manager;
pub(crate) mod discord_presence;
mod instance_manager;
pub(crate) mod java_manager;
mod launcher;
pub(crate) mod settings_manager;

pub use addon_manager::*;
pub use instance_manager::{InstanceDto, InstanceManager, InstanceStatus, signal_kill};
pub use launcher::{DownloadQueue, DownloadState, Launcher};
pub use settings_manager::{SettingsManager, SettingsSnapshot};
