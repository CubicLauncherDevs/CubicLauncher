pub mod error;
pub mod forge;
pub mod launch_config;
pub mod loader;
pub mod manifest;
pub mod resolvers;
pub mod version;

pub use error::Error;
pub use forge::{
    DataEntry, InstallProfile, LegacyInstallProfile, LegacyLibrary, LegacyVersionInfo, Processor,
    ProfileLibrary, maven_to_path, parse_maven_coord,
};
pub use launch_config::{LaunchConfig, LaunchConfigBuilder, QuickPlay};
pub use loader::Loader;
pub use manifest::{
    Argument, ArgumentValue, AssetIndex, DownloadEntry, JavaVersion, Library, LibraryArtifact,
    LibraryDownloads, Natives, OsRule, Rule, RuleAction, VersionArgType, VersionDownloads,
    VersionManifest,
};
pub use resolvers::{
    ClasspathResolver, CommandBuilder, extract_jar, extract_natives, is_native_file,
};
pub use version::{GameVersion, MCVersion, ResolvedVersion, parse_version, resolve_dependencies};
