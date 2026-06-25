use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{AssetIndex, LibraryDownloads, VersionArgType};

// ── Legacy Forge (pre-1.13, spec 0) ──────────────────────────────────────

/// Old Forge installer format (e.g., 1.9.4-12.17.0.2317).
/// The install_profile.json has `install` + `versionInfo` instead of
/// `spec` + `processors` + `data` + `libraries`.
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct LegacyInstallProfile {
    pub install: LegacyInstallSection,
    #[serde(rename = "versionInfo")]
    pub version_info: LegacyVersionInfo,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct LegacyInstallSection {
    #[serde(rename = "profileName")]
    pub profile_name: Option<String>,
    pub target: String,
    pub path: String,
    pub version: Option<String>,
    #[serde(rename = "filePath")]
    pub file_path: Option<String>,
    pub minecraft: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct LegacyVersionInfo {
    pub id: String,
    #[serde(rename = "mainClass")]
    pub main_class: Option<String>,
    #[serde(rename = "inheritsFrom")]
    pub inherits_from: Option<String>,
    #[serde(default)]
    pub libraries: Vec<LegacyLibrary>,
    #[serde(rename = "minecraftArguments")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minecraft_arguments: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<VersionArgType>,
    #[serde(rename = "assetIndex")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub asset_index: Option<AssetIndex>,
}

/// Old-format library entry: `url` is a base Maven URL, not a full download URL.
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct LegacyLibrary {
    pub name: String,
    pub url: Option<String>,
    pub checksums: Option<Vec<String>>,
    #[serde(default)]
    pub clientreq: Option<bool>,
    #[serde(default)]
    pub serverreq: Option<bool>,
}

impl LegacyLibrary {
    /// Construct full download URL from base Maven URL + maven path.
    pub fn download_url(&self) -> Option<String> {
        let base = self.url.as_deref()?;
        let path = maven_to_path(&self.name).to_string_lossy().into_owned();
        let base = base.trim_end_matches('/');
        Some(format!("{base}/{path}"))
    }

    /// Client-side library path (for dedup in staging/libraries/).
    pub fn lib_path(&self) -> PathBuf {
        maven_to_path(&self.name)
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct InstallProfile {
    #[serde(default)]
    pub spec: u8,
    #[serde(default)]
    pub profile: String,
    pub version: String,
    pub path: Option<String>,
    pub minecraft: String,
    #[serde(default)]
    pub server_jar_path: Option<String>,
    #[serde(default)]
    pub data: HashMap<String, DataEntry>,
    #[serde(default)]
    pub processors: Vec<Processor>,
    #[serde(default)]
    pub libraries: Vec<ProfileLibrary>,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct DataEntry {
    pub client: Option<serde_json::Value>,
    pub server: Option<serde_json::Value>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Processor {
    #[serde(default = "default_sides")]
    pub sides: Vec<String>,
    pub jar: String,
    #[serde(default)]
    pub classpath: Vec<String>,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub outputs: HashMap<String, String>,
}

fn default_sides() -> Vec<String> {
    vec!["client".into(), "server".into()]
}

#[derive(Deserialize, Debug, Clone)]
pub struct ProfileLibrary {
    pub name: String,
    pub downloads: Option<LibraryDownloads>,
    /// Fallback Maven base URL (legacy format / some modern libs without downloads).
    pub url: Option<String>,
}

/// Convert a Maven coordinate to a filesystem path.
///
/// Examples:
///   "org.ow2.asm:asm:9.9.1" → "org/ow2/asm/asm/9.9.1/asm-9.9.1.jar"
///   "net.minecraftforge:forge:1.12.2-14.23.5.2859" → "net/minecraftforge/forge/1.12.2-14.23.5.2859/forge-1.12.2-14.23.5.2859.jar"
///   "net.minecraftforge:forge:26.1.2-64.0.8:client" → "net/minecraftforge/forge/26.1.2-64.0.8/forge-26.1.2-64.0.8-client.jar"
///   "de.oceanlabs.mcp:mcp_config:1.12.2-20200226.224830@zip" → "de/oceanlabs/mcp/mcp_config/1.12.2-20200226.224830/mcp_config-1.12.2-20200226.224830.zip"
pub fn maven_to_path(coord: &str) -> PathBuf {
    let (group, artifact, version, classifier, extension) = parse_maven_coord(coord);
    let group_path = group.replace('.', "/");
    let mut filename = format!("{artifact}-{version}");

    if let Some(cls) = classifier {
        filename.push('-');
        filename.push_str(&cls);
    }

    filename.push('.');
    filename.push_str(&extension);

    PathBuf::new()
        .join(group_path)
        .join(&artifact)
        .join(&version)
        .join(filename)
}

/// Parse a Maven coordinate into its parts.
///
/// Returns `(group, artifact, version, classifier, extension)`.
pub fn parse_maven_coord(coord: &str) -> (String, String, String, Option<String>, String) {
    let mut coord = coord;

    let extension = if let Some(idx) = coord.rfind('@') {
        let ext = coord[idx + 1..].to_string();
        coord = &coord[..idx];
        ext
    } else {
        "jar".to_string()
    };

    let parts: Vec<&str> = coord.splitn(4, ':').collect();
    let group = parts.first().unwrap_or(&"").to_string();
    let artifact = parts.get(1).unwrap_or(&"").to_string();
    let version = parts.get(2).unwrap_or(&"").to_string();
    let classifier = parts.get(3).map(|s| s.to_string());

    (group, artifact, version, classifier, extension)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_maven_coord() {
        let path = maven_to_path("org.ow2.asm:asm:9.9.1");
        assert_eq!(path, PathBuf::from("org/ow2/asm/asm/9.9.1/asm-9.9.1.jar"));
    }

    #[test]
    fn maven_coord_with_classifier() {
        let path = maven_to_path("net.minecraftforge:forge:26.1.2-64.0.8:client");
        assert_eq!(
            path,
            PathBuf::from("net/minecraftforge/forge/26.1.2-64.0.8/forge-26.1.2-64.0.8-client.jar")
        );
    }

    #[test]
    fn maven_coord_with_extension_override() {
        let path = maven_to_path("de.oceanlabs.mcp:mcp_config:1.12.2-20200226.224830@zip");
        assert_eq!(
            path,
            PathBuf::from(
                "de/oceanlabs/mcp/mcp_config/1.12.2-20200226.224830/mcp_config-1.12.2-20200226.224830.zip"
            )
        );
    }

    #[test]
    fn parse_simple_coord() {
        let (g, a, v, c, e) = parse_maven_coord("org.ow2.asm:asm:9.9.1");
        assert_eq!(g, "org.ow2.asm");
        assert_eq!(a, "asm");
        assert_eq!(v, "9.9.1");
        assert_eq!(c, None);
        assert_eq!(e, "jar");
    }

    #[test]
    fn parse_coord_with_classifier() {
        let (g, a, v, c, e) = parse_maven_coord("net.minecraftforge:forge:26.1.2-64.0.8:universal");
        assert_eq!(g, "net.minecraftforge");
        assert_eq!(a, "forge");
        assert_eq!(v, "26.1.2-64.0.8");
        assert_eq!(c, Some("universal".into()));
        assert_eq!(e, "jar");
    }

    #[test]
    fn parse_coord_with_extension() {
        let (_g, _a, _v, c, e) = parse_maven_coord("de.oceanlabs.mcp:mcp_config:1.12.2@zip");
        assert_eq!(e, "zip");
        assert_eq!(c, None);
    }
}
