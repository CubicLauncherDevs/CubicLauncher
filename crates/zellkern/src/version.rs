use serde::{Deserialize, Serialize, Deserializer};

use crate::Loader;

/// Parsed numeric Minecraft version (e.g. "1.21.4" → major=1, minor=21, patch=Some(4)).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MCVersion {
    pub major: u8,
    pub minor: u8,
    pub patch: Option<u8>,
}

impl MCVersion {
    pub fn new(major: u8, minor: u8, patch: Option<u8>) -> Self {
        Self { major, minor, patch }
    }
}

impl std::fmt::Display for MCVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.patch {
            Some(p) => write!(f, "{}.{}.{}", self.major, self.minor, p),
            None => write!(f, "{}.{}", self.major, self.minor),
        }
    }
}

impl<'de> Deserialize<'de> for MCVersion {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(parse_version(&s).unwrap_or_default())
    }
}

/// Parse a Minecraft version string like "1.21", "1.20.6", "26.2-snapshot-5".
pub fn parse_version(s: &str) -> Option<MCVersion> {
    let s = s.trim();
    let digits: Vec<&str> = s
        .split(|c: char| !c.is_ascii_digit() && c != '.')
        .next()
        .map(|part| part.split('.').collect())
        .unwrap_or_default();

    if digits.len() < 2 {
        return None;
    }
    let major = digits[0].parse::<u8>().ok()?;
    let minor = digits[1].parse::<u8>().ok()?;
    let patch = digits.get(2).and_then(|p| p.parse().ok());
    Some(MCVersion { major, minor, patch })
}

/// A structured Minecraft version with loader information.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameVersion {
    pub mc_version: String,
    pub loader: Loader,
}

impl GameVersion {
    pub fn from_version_id(id: &str) -> Self {
        let loader = Loader::from_version_id(id);
        let mc_version = match &loader {
            Loader::Vanilla => id.to_string(),
            Loader::Fabric(full_id)
            | Loader::Forge(full_id)
            | Loader::NeoForge(full_id)
            | Loader::Quilt(full_id) => extract_mc_version(full_id),
        };
        Self { mc_version, loader }
    }

    pub fn to_version_id(&self) -> String {
        match &self.loader {
            Loader::Vanilla => self.mc_version.clone(),
            Loader::Fabric(v) => format!("fabric-loader-{}-{}", v, self.mc_version),
            Loader::Forge(v) => format!("{}-forge-{}", self.mc_version, v),
            Loader::NeoForge(v) => format!("{}-neoforge-{}", self.mc_version, v),
            Loader::Quilt(v) => format!("{}-quilt-{}", self.mc_version, v),
        }
    }

    pub fn display_name(&self) -> String {
        match &self.loader {
            Loader::Vanilla => self.mc_version.clone(),
            Loader::Fabric(v) => format!("{} (Fabric {})", self.mc_version, v),
            Loader::Forge(v) => format!("{} (Forge {})", self.mc_version, v),
            Loader::NeoForge(v) => format!("{} (NeoForge {})", self.mc_version, v),
            Loader::Quilt(v) => format!("{} (Quilt {})", self.mc_version, v),
        }
    }
}

impl std::fmt::Display for GameVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Fully resolved version with all metadata needed for launching.
#[derive(Debug, Clone)]
pub struct ResolvedVersion {
    pub mc_version: String,
    pub loader: Loader,
    pub java_version: u8,
}

/// Extract the Minecraft version from a full version ID with loader.
///
/// Formats:
///   "fabric-loader-0.15.11-1.20.1"   → "1.20.1"
///   "1.20.1-forge-47.2.0"            → "1.20.1"
///   "1.21-neoforge-21.0.0"           → "1.21"
///   "1.20.1-quilt-0.25.0"            → "1.20.1"
fn extract_mc_version(full_id: &str) -> String {
    // Fabric/Quilt loader: "fabric-loader-{loader_version}-{mc_version}"
    if let Some(rest) = full_id.strip_prefix("fabric-loader-")
        && let Some(last_dash) = rest.rfind('-')
    {
        return rest[last_dash + 1..].to_string();
    }
    // Forge/NeoForge/Quilt: "{mc_version}-{loader_name}-{loader_version}"
    for loader_name in &["-forge-", "-neoforge-", "-quilt-"] {
        if let Some(idx) = full_id.find(loader_name) {
            return full_id[..idx].to_string();
        }
    }
    full_id.to_string()
}

/// Resolve all version IDs that a given version depends on.
///
/// Vanilla versions have no dependencies (return only themselves).
/// Fabric/Forge/NeoForge/Quilt depend on their base MC version.
///
/// Example: `"1.20.1-forge-47.2.0"` → `["1.20.1", "1.20.1-forge-47.2.0"]`
pub fn resolve_dependencies(version_id: &str) -> Vec<String> {
    let game_version = GameVersion::from_version_id(version_id);
    match &game_version.loader {
        Loader::Vanilla => vec![version_id.to_string()],
        Loader::Fabric(_)
        | Loader::Forge(_)
        | Loader::NeoForge(_)
        | Loader::Quilt(_) => {
            vec![game_version.mc_version.clone(), version_id.to_string()]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_standard() {
        assert_eq!(parse_version("1.21").unwrap(), MCVersion::new(1, 21, None));
        assert_eq!(
            parse_version("1.20.6").unwrap(),
            MCVersion::new(1, 20, Some(6))
        );
    }

    #[test]
    fn parses_snapshot() {
        let v = parse_version("26.2-snapshot-5").unwrap();
        assert_eq!(v, MCVersion::new(26, 2, None));
    }

    #[test]
    fn extract_fabric_version() {
        let gv = GameVersion::from_version_id("fabric-loader-0.15.11-1.20.1");
        assert_eq!(gv.mc_version, "1.20.1");
        assert_eq!(gv.loader, Loader::Fabric("fabric-loader-0.15.11-1.20.1".into()));
    }

    #[test]
    fn extract_forge_version() {
        let gv = GameVersion::from_version_id("1.20.1-forge-47.2.0");
        assert_eq!(gv.mc_version, "1.20.1");
    }

    #[test]
    fn extract_neoforge_version() {
        let gv = GameVersion::from_version_id("1.21-neoforge-21.0.0");
        assert_eq!(gv.mc_version, "1.21");
    }

    #[test]
    fn extract_quilt_version() {
        let gv = GameVersion::from_version_id("1.20.1-quilt-0.25.0");
        assert_eq!(gv.mc_version, "1.20.1");
    }

    #[test]
    fn roundtrip_fabric() {
        let original = "fabric-loader-0.15.11-1.20.1";
        let gv = GameVersion::from_version_id(original);
        assert_eq!(gv.to_version_id(), original);
    }

    #[test]
    fn roundtrip_forge() {
        let original = "1.20.1-forge-47.2.0";
        let gv = GameVersion::from_version_id(original);
        assert_eq!(gv.to_version_id(), original);
    }

    #[test]
    fn dependencies_vanilla() {
        let deps = resolve_dependencies("1.20.1");
        assert_eq!(deps, vec!["1.20.1"]);
    }

    #[test]
    fn dependencies_forge() {
        let deps = resolve_dependencies("1.20.1-forge-47.2.0");
        assert_eq!(deps, vec!["1.20.1", "1.20.1-forge-47.2.0"]);
    }

    #[test]
    fn dependencies_fabric() {
        let deps = resolve_dependencies("fabric-loader-0.15.11-1.20.1");
        assert_eq!(deps, vec!["1.20.1", "fabric-loader-0.15.11-1.20.1"]);
    }

    #[test]
    fn dependencies_neoforge() {
        let deps = resolve_dependencies("1.21-neoforge-21.0.0");
        assert_eq!(deps, vec!["1.21", "1.21-neoforge-21.0.0"]);
    }

    #[test]
    fn dependencies_quilt() {
        let deps = resolve_dependencies("1.20.1-quilt-0.25.0");
        assert_eq!(deps, vec!["1.20.1", "1.20.1-quilt-0.25.0"]);
    }
}
