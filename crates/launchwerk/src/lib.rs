pub mod error;
pub mod handle;
pub mod launchwerk;
pub mod models;

pub use error::Error;
pub use handle::InstanceHandle;
pub use launchwerk::Launchwerk;
pub use zellkern::{LaunchConfig, QuickPlay};

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(feature = "auth")]
pub mod auth;

#[cfg(feature = "auth")]
pub use auth::MinecraftUser;
