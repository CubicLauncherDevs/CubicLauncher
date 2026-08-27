mod downloaders;
pub(crate) mod errors;
pub(crate) mod jre;
pub(crate) mod manifest;
pub mod path_security;
pub mod progress;
pub(crate) mod types;
pub(crate) mod utilities;

pub use downloaders::{
    DownloadBatch, DownloadHandle, DownloadItemSpec, DownloadManager, FabricBatch, ForgeBatch,
    ForgeVersionInfo, GenericBatch, JreBatch, MinecraftBatch, NeoForgeBatch, NeoForgeVersionInfo,
    QuiltBatch,
};
pub use errors::AquaError;
pub use jre::{ArchiveFormat, JrePackage, JreProviderChain, JreStatus, JreVendor};
pub use manifest::resolve_version_data;
pub use progress::{DownloadProgress, DownloadReporter, DownloadStage, ProgressSender};
pub use types::*;
pub use utilities::{infer_java_version, java_runtime_preferences, parse_java_major_version};
