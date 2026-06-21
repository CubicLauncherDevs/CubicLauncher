pub use zellkern::{
    Argument, ArgumentValue, AssetIndex, DownloadEntry, JavaVersion, Library, LibraryArtifact,
    LibraryDownloads, Loader, MCVersion, Natives, OsRule, QuickPlay, Rule, RuleAction,
    VersionArgType, VersionDownloads, VersionManifest, parse_version,
};

use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub const MOJANG_MANIFEST_URL: &str =
    "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
pub const RESOURCES_BASE_URL: &str = "https://resources.download.minecraft.net/";

// ─── Normalized types (resolved for downloader) ───────────────────────────────

#[derive(Debug, Clone)]
pub struct NormalizedVersion {
    pub id: String,
    pub parsed_version: MCVersion,
    pub release_time: String,
    pub java_version: u8,
    pub main_class: String,
    pub client_jar: Downloadable,
    pub server_jar: Option<Downloadable>,
    pub asset_index: AssetMeta,
    pub libraries: Vec<LibraryFile>,
    pub natives: Vec<LibraryFile>,
    pub arguments: NormalizedArguments,
}

#[derive(Debug, Clone)]
pub struct Downloadable {
    pub url: String,
    pub sha1: String,
    pub size: u64,
}

#[derive(Debug, Clone)]
pub struct AssetMeta {
    pub id: String,
    pub url: String,
    pub sha1: String,
    pub size: u64,
}

#[derive(Debug, Clone)]
pub struct LibraryFile {
    pub name: String,
    pub url: String,
    pub sha1: String,
    pub size: u64,
    pub path: String,
}

#[derive(Debug, Clone)]
pub struct NormalizedArguments {
    pub game: Vec<String>,
    pub jvm: Vec<String>,
}

// ─── Asset index ──────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct VersionAssets {
    pub objects: std::collections::HashMap<String, Asset>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Asset {
    pub hash: String,
    pub size: usize,
}

impl VersionAssets {
    pub fn into_vec(mut self) -> Vec<(String, Asset)> {
        self.objects.drain().collect()
    }

    pub fn len(&self) -> usize {
        self.objects.len()
    }

    pub fn is_empty(&self) -> bool {
        self.objects.is_empty()
    }
}

// ─── Download progress ────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct DownloadProgress {
    pub current: usize,
    pub total: usize,
    pub info: DownloadProgressInfo,
    pub download_type: DownloadProgressType,
}

#[derive(Debug, Clone)]
pub struct DownloadProgressInfo {
    pub name: String,
    pub version: Arc<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DownloadProgressType {
    Library,
    Asset,
    Native,
    Client,
    Verifying,
    Generic,
    Processing,
    Jre,
}
