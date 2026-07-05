use serde::Deserialize;
use std::collections::HashMap;
use zellkern::{GameVersion, Loader};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackFormat {
    pub game: String,
    #[allow(dead_code)]
    pub format_version: i32,
    pub version_id: String,
    pub name: String,
    pub summary: Option<String>,
    pub files: Vec<PackFile>,
    pub dependencies: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackFile {
    pub path: String,
    #[allow(dead_code)]
    pub hashes: HashMap<String, String>,
    pub env: Option<HashMap<String, String>>,
    pub downloads: Vec<String>,
    #[allow(dead_code)]
    pub file_size: u32,
}

#[derive(Debug, Clone)]
pub struct MrpackMetadata {
    pub name: String,
    pub version_id: String,
    pub summary: Option<String>,
    pub game_version: Option<GameVersion>,
    pub file_count: usize,
}

impl PackFormat {
    pub fn extract_metadata(&self) -> MrpackMetadata {
        let mc_version = self.dependencies.get("minecraft").cloned();

        let game_version = mc_version.map(|mc_ver| {
            let loader = self
                .dependencies
                .get("quilt-loader")
                .map(|v| Loader::Quilt(v.clone()))
                .or_else(|| {
                    self.dependencies
                        .get("fabric-loader")
                        .map(|v| Loader::Fabric(v.clone()))
                })
                .or_else(|| {
                    self.dependencies
                        .get("forge")
                        .map(|v| Loader::Forge(v.clone()))
                })
                .or_else(|| {
                    self.dependencies
                        .get("neoforge")
                        .map(|v| Loader::NeoForge(v.clone()))
                })
                .unwrap_or(Loader::Vanilla);

            GameVersion {
                mc_version: mc_ver,
                loader,
            }
        });

        MrpackMetadata {
            name: self.name.clone(),
            version_id: self.version_id.clone(),
            summary: self.summary.clone(),
            game_version,
            file_count: self.files.len(),
        }
    }
}
