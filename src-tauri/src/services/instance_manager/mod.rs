mod manager;
mod status;

pub(crate) mod data;
pub(crate) mod handle;

pub use data::{InstOverrides, InstanceDto};
pub use handle::InstanceHandle;
pub use manager::{InstanceManager, register_kill_sender, signal_kill, unregister_kill_sender};
pub use status::InstanceStatus;
