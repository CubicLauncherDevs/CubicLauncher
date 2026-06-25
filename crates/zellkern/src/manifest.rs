use std::collections::HashMap;
use std::env::consts::{ARCH, OS};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::Error;
use crate::version::MCVersion;

// ─── OS / arch rules ──────────────────────────────────────────────────────────

trait Evaluable {
    fn rules(&self) -> Option<&Vec<Rule>>;

    fn evaluate(&self) -> bool {
        let rules = match self.rules() {
            Some(r) => r,
            None => return true,
        };
        let mut result = false;
        for rule in rules {
            if let Some(action) = rule.action_if_matches() {
                result = matches!(action, RuleAction::Allow);
            }
        }
        result
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RuleAction {
    Allow,
    Disallow,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct OsRule {
    pub name: Option<String>,
    pub arch: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Rule {
    pub action: RuleAction,
    pub os: Option<OsRule>,
}

impl Rule {
    pub fn condition_matches(&self) -> bool {
        match &self.os {
            Some(os_rule) => {
                let os_matches = match os_rule.name.as_deref() {
                    Some("osx") => OS == "macos",
                    Some(name) => OS == name,
                    None => true,
                };
                let arch_matches = match os_rule.arch.as_deref() {
                    Some("x86") => ARCH == "x86" || ARCH == "x86_64",
                    Some(arch) => ARCH == arch,
                    None => true,
                };
                os_matches && arch_matches
            }
            None => true,
        }
    }

    pub fn evaluate(&self) -> bool {
        matches!(self.action_if_matches(), Some(RuleAction::Allow))
    }

    pub fn action_if_matches(&self) -> Option<RuleAction> {
        if self.condition_matches() {
            Some(self.action.clone())
        } else {
            None
        }
    }
}

// ─── Arguments ────────────────────────────────────────────────────────────────

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(untagged)]
pub enum ArgumentValue {
    Single(String),
    Many(Vec<String>),
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(untagged)]
pub enum Argument {
    WithRule {
        rules: Vec<Rule>,
        value: ArgumentValue,
    },
    Plain(String),
}

impl Argument {
    pub fn get_if_applies(&self) -> Vec<String> {
        match self {
            Argument::WithRule { rules, value } => {
                if rules.iter().all(|r| r.evaluate()) {
                    match value {
                        ArgumentValue::Single(s) => vec![s.clone()],
                        ArgumentValue::Many(v) => v.clone(),
                    }
                } else {
                    vec![]
                }
            }
            Argument::Plain(s) => vec![s.clone()],
        }
    }
}

impl Evaluable for Argument {
    fn rules(&self) -> Option<&Vec<Rule>> {
        match self {
            Argument::WithRule { rules, .. } => Some(rules),
            Argument::Plain(_) => None,
        }
    }
}

// ─── Libraries ────────────────────────────────────────────────────────────────

#[derive(Deserialize, Debug, Clone, Default)]
pub struct LibraryArtifact {
    pub path: String,
    pub url: Option<String>,
    pub sha1: Option<String>,
    pub size: Option<u64>,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct LibraryDownloads {
    pub artifact: Option<LibraryArtifact>,
    #[serde(default)]
    pub classifiers: Option<HashMap<String, LibraryArtifact>>,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct Natives {
    pub linux: Option<String>,
    pub windows: Option<String>,
    pub osx: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Library {
    pub name: String,
    pub downloads: Option<LibraryDownloads>,
    pub rules: Option<Vec<Rule>>,
    #[serde(default)]
    pub natives: Option<Natives>,
}

impl Evaluable for Library {
    fn rules(&self) -> Option<&Vec<Rule>> {
        self.rules.as_ref()
    }
}

impl Library {
    pub fn should_include(&self) -> bool {
        self.evaluate()
    }

    pub fn is_correct_arch(&self) -> bool {
        let name = self.name.to_lowercase();
        let has_x86 = name.contains("natives-windows-x86") || name.contains("natives-linux-x86");
        let has_arm64 = name.contains("natives-windows-arm64")
            || name.contains("natives-macos-arm64")
            || name.contains("natives-linux-aarch_64")
            || name.contains("natives-linux-arm64");
        let has_arm32 = name.contains("natives-linux-arm32");

        if has_x86 {
            return ARCH == "x86";
        }
        if has_arm64 {
            return ARCH == "aarch64";
        }
        if has_arm32 {
            return ARCH == "arm";
        }
        true
    }

    pub fn get_path(&self) -> PathBuf {
        if let Some(path) = self
            .downloads
            .as_ref()
            .and_then(|d| d.artifact.as_ref())
            .map(|a| a.path.as_str())
        {
            return PathBuf::from(path);
        }

        let parts: Vec<&str> = self.name.splitn(4, ':').collect();
        if parts.len() < 3 {
            return PathBuf::from(&self.name);
        }
        let group_path = parts[0].replace('.', "/");
        let artifact = parts[1];
        let version = parts[2];

        match parts.get(3) {
            Some(classifier) => PathBuf::new()
                .join(&group_path)
                .join(artifact)
                .join(version)
                .join(format!("{artifact}-{version}-{classifier}.jar")),
            None => PathBuf::new()
                .join(&group_path)
                .join(artifact)
                .join(version)
                .join(format!("{artifact}-{version}.jar")),
        }
    }

    pub fn native_artifact(&self) -> Option<&LibraryArtifact> {
        let natives = self.natives.as_ref()?;
        let classifier = match OS {
            "linux" => natives.linux.as_deref(),
            "windows" => natives.windows.as_deref(),
            "macos" => natives.osx.as_deref(),
            _ => return None,
        }?;
        self.downloads
            .as_ref()
            .and_then(|d| d.classifiers.as_ref())
            .and_then(|c| c.get(classifier))
    }

    pub fn is_native(&self) -> bool {
        if self.native_artifact().is_some() {
            return true;
        }
        if self.rules.is_some() {
            let parts: Vec<&str> = self.name.splitn(4, ':').collect();
            if parts.len() == 4 && parts[3].starts_with("natives-") {
                return true;
            }
        }
        let path_str = self
            .downloads
            .as_ref()
            .and_then(|d| d.artifact.as_ref())
            .map(|a| a.path.as_str())
            .unwrap_or(&self.name)
            .to_lowercase();
        path_str.contains("natives-")
            || path_str.contains("/natives/")
            || path_str.ends_with("-natives.jar")
    }

    pub fn url(&self) -> Option<&str> {
        self.downloads
            .as_ref()
            .and_then(|d| d.artifact.as_ref())
            .and_then(|a| a.url.as_deref())
    }

    pub fn sha1(&self) -> Option<&str> {
        self.downloads
            .as_ref()
            .and_then(|d| d.artifact.as_ref())
            .and_then(|a| a.sha1.as_deref())
    }
}

// ─── Misc manifest types ─────────────────────────────────────────────────────

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct JavaVersion {
    pub component: String,
    pub major_version: u8,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AssetIndex {
    pub id: String,
    pub url: Option<String>,
    pub sha1: Option<String>,
    pub size: Option<u64>,
    pub total_size: Option<u64>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct VersionArgType {
    pub game: Option<Vec<Argument>>,
    pub jvm: Option<Vec<Argument>>,
    #[serde(default)]
    pub default_user_jvm: Option<Vec<Argument>>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct VersionDownloads {
    pub client: DownloadEntry,
    pub server: Option<DownloadEntry>,
    pub client_mappings: Option<DownloadEntry>,
    pub server_mappings: Option<DownloadEntry>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DownloadEntry {
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

// ─── VersionManifest ──────────────────────────────────────────────────────────

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct VersionManifest {
    pub id: MCVersion,
    pub id_raw: String,
    pub main_class: Option<String>,
    pub arguments: Option<VersionArgType>,
    pub minecraft_arguments: Option<String>,
    pub downloads: Option<VersionDownloads>,
    pub libraries: Option<Vec<Library>>,
    pub asset_index: Option<AssetIndex>,
    pub java_version: Option<JavaVersion>,
    pub inherits_from: Option<String>,
}

impl VersionManifest {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        let raw: serde_json::Value = serde_json::from_slice(bytes)?;
        let id_raw = raw["id"].as_str().unwrap_or("0.0").to_string();
        let id: MCVersion = serde_json::from_value(raw["id"].clone())?;

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Inner {
            main_class: Option<String>,
            arguments: Option<VersionArgType>,
            minecraft_arguments: Option<String>,
            downloads: Option<VersionDownloads>,
            libraries: Option<Vec<Library>>,
            asset_index: Option<AssetIndex>,
            java_version: Option<JavaVersion>,
            inherits_from: Option<String>,
        }

        let inner: Inner = serde_json::from_value(raw)?;
        Ok(Self {
            id,
            id_raw,
            main_class: inner.main_class,
            arguments: inner.arguments,
            minecraft_arguments: inner.minecraft_arguments,
            downloads: inner.downloads,
            libraries: inner.libraries,
            asset_index: inner.asset_index,
            java_version: inner.java_version,
            inherits_from: inner.inherits_from,
        })
    }

    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, Error> {
        let bytes = std::fs::read(path.as_ref())?;
        Self::from_bytes(&bytes)
    }

    pub fn resolve(&self, parent: &VersionManifest) -> VersionManifest {
        VersionManifest {
            id: self.id,
            id_raw: self.id_raw.clone(),
            main_class: self.main_class.clone().or(parent.main_class.clone()),
            arguments: {
                let p_args = parent.arguments.as_ref();
                let c_args = self.arguments.as_ref();
                match (p_args, c_args) {
                    (None, None) => None,
                    (Some(p), None) => Some(p.clone()),
                    (None, Some(c)) => Some(c.clone()),
                    (Some(p), Some(c)) => {
                        let mut jvm = p.jvm.clone().unwrap_or_default();
                        if let Some(cj) = &c.jvm {
                            jvm.extend(cj.clone());
                        }
                        let mut game = p.game.clone().unwrap_or_default();
                        if let Some(cg) = &c.game {
                            game.extend(cg.clone());
                        }
                        Some(VersionArgType {
                            jvm: Some(jvm),
                            game: Some(game),
                            default_user_jvm: c
                                .default_user_jvm
                                .clone()
                                .or(p.default_user_jvm.clone()),
                        })
                    }
                }
            },
            minecraft_arguments: self
                .minecraft_arguments
                .clone()
                .or(parent.minecraft_arguments.clone()),
            downloads: self.downloads.clone().or(parent.downloads.clone()),
            libraries: {
                let mut libs = parent.libraries.clone().unwrap_or_default();
                if let Some(child_libs) = &self.libraries {
                    libs.extend(child_libs.clone());
                }
                Some(libs)
            },
            asset_index: self.asset_index.clone().or(parent.asset_index.clone()),
            java_version: self.java_version.clone().or(parent.java_version.clone()),
            inherits_from: parent.inherits_from.clone(),
        }
    }

    pub fn java_major_version(&self) -> u8 {
        self.java_version
            .as_ref()
            .map(|j| j.major_version)
            .unwrap_or(8)
    }

    pub fn get_classpath(&self, lib_dir: &Path) -> String {
        let vec = vec![];
        let libs = self.libraries.as_ref().unwrap_or(&vec);
        let paths: Vec<String> = libs
            .iter()
            .filter(|lib| lib.should_include() && !lib.is_native())
            .map(|lib| lib_dir.join(lib.get_path()).to_string_lossy().to_string())
            .collect();
        #[cfg(target_os = "windows")]
        return paths.join(";");
        #[cfg(not(target_os = "windows"))]
        return paths.join(":");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rule_without_os_allows() {
        let r: Rule = serde_json::from_str(r#"{"action":"allow"}"#).unwrap();
        assert!(r.action_if_matches().is_some());
    }

    #[test]
    fn native_detection_by_path() {
        let lib: Library = serde_json::from_str(
            r#"{
            "name": "org.lwjgl:lwjgl:3.4.1:natives-linux",
            "downloads": {
                "artifact": {
                    "path": "org/lwjgl/lwjgl/3.4.1/lwjgl-3.4.1-natives-linux.jar",
                    "url": "https://libraries.minecraft.net/org/lwjgl/lwjgl/3.4.1/lwjgl-3.4.1-natives-linux.jar",
                    "sha1": "abc123",
                    "size": 12345
                }
            },
            "rules": [{"action":"allow","os":{"name":"linux"}}]
        }"#,
        )
        .unwrap();
        assert!(lib.is_native());
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn allow_linux() {
        let r: Rule = serde_json::from_str(r#"{"action":"allow","os":{"name":"linux"}}"#).unwrap();
        assert!(r.evaluate());
    }

    #[test]
    #[cfg(not(target_os = "linux"))]
    fn disallow_linux_on_non_linux() {
        let r: Rule = serde_json::from_str(r#"{"action":"allow","os":{"name":"linux"}}"#).unwrap();
        assert!(!r.evaluate());
    }
}
