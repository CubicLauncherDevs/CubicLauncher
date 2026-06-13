pub mod client;
pub mod error;
pub mod models;

pub use client::DiscordRpcClient;
pub use error::DiscordRpcError;
pub use models::{Activity, ActivityBuilder, Assets, Button, Party, Secrets, Timestamps};
