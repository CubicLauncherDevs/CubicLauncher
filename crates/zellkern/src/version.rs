use serde::{Deserialize, Deserializer, Serialize};

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
        Self {
            major,
            minor,
            patch,
        }
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
    Some(MCVersion {
        major,
        minor,
        patch,
    })
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
            Loader::Fabric(_)
            | Loader::Forge(_)
            | Loader::NeoForge(_)
            | Loader::Quilt(_)
            | Loader::OptiFine(_) => extract_mc_version(id),
        };
        Self { mc_version, loader }
    }

    pub fn to_version_id(&self) -> String {
        match &self.loader {
            Loader::Vanilla => self.mc_version.clone(),
            Loader::Fabric(v) => format!("fabric-loader-{}-{}", v, self.mc_version),
            Loader::Forge(v) => format!("{}-forge-{}", self.mc_version, v),
            Loader::NeoForge(v) => format!("{}-neoforge-{}", self.mc_version, v),
            Loader::Quilt(v) => format!("quilt-loader-{}-{}", v, self.mc_version),
            Loader::OptiFine(v) => format!("{}-OptiFine_{}", self.mc_version, v),
        }
    }

    pub fn display_name(&self) -> String {
        match &self.loader {
            Loader::Vanilla => self.mc_version.clone(),
            Loader::Fabric(v) => format!("{} (Fabric {})", self.mc_version, v),
            Loader::Forge(v) => format!("{} (Forge {})", self.mc_version, v),
            Loader::NeoForge(v) => format!("{} (NeoForge {})", self.mc_version, v),
            Loader::Quilt(v) => format!("{} (Quilt {})", self.mc_version, v),
            Loader::OptiFine(v) => format!("{} (OptiFine {})", self.mc_version, v),
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
///   "quilt-loader-0.25.0-1.20.1"     → "1.20.1"
fn extract_mc_version(full_id: &str) -> String {
    // Fabric/Quilt loader: "fabric-loader-{loader_version}-{mc_version}"
    // We split at the *first* dash after the prefix because the loader version
    // does not contain dashes, but the Minecraft version can (e.g. snapshots).
    if let Some(rest) = full_id.strip_prefix("fabric-loader-")
        && let Some(dash) = rest.find('-')
    {
        return rest[dash + 1..].to_string();
    }
    // Quilt loader: "quilt-loader-{loader_version}-{mc_version}"
    if let Some(rest) = full_id.strip_prefix("quilt-loader-")
        && let Some(dash) = rest.find('-')
    {
        return rest[dash + 1..].to_string();
    }
    // Forge/NeoForge: "{mc_version}-{loader_name}-{loader_version}"
    for loader_name in &["-forge-", "-neoforge-", "-OptiFine_"] {
        if let Some(idx) = full_id.find(loader_name) {
            return full_id[..idx].to_string();
        }
    }
    full_id.to_string()
}

/// Whether the version predates 1.6 (snapshot `13w23a`): alpha, beta, classic,
/// `rd-`/`inf-` builds, older snapshots and releases before 1.6.
pub fn is_pre_1_6(version: &str) -> bool {
    let base = GameVersion::from_version_id(version)
        .mc_version
        .trim()
        .to_ascii_lowercase();

    let legacy_prefix = matches!(base.as_bytes().first(), Some(b'a' | b'b' | b'c'))
        && base.as_bytes().get(1).is_some_and(u8::is_ascii_digit);
    eprintln!(
        "DBG base={base:?} len={} prefix={legacy_prefix}",
        base.len()
    );
    if legacy_prefix || base.starts_with("rd-") || base.starts_with("inf-") {
        return true;
    }

    if let Some((year, week)) = snapshot_year_week(&base) {
        return (year, week) < (13, 23);
    }

    release_major_minor(&base).is_some_and(|(major, minor)| (major, minor) < (1, 6))
}

fn snapshot_year_week(version: &str) -> Option<(u32, u32)> {
    let (year, rest) = version.split_once('w')?;
    let year: u32 = year.parse().ok()?;
    if !(10..=99).contains(&year) {
        return None;
    }
    let week: u32 = rest
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>()
        .parse()
        .ok()?;
    Some((year, week))
}

fn release_major_minor(version: &str) -> Option<(u32, u32)> {
    if !version.starts_with(|c: char| c.is_ascii_digit()) {
        return None;
    }
    let mut parts = version.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts
        .next()?
        .split(|c: char| !c.is_ascii_digit())
        .next()?
        .parse()
        .ok()?;
    Some((major, minor))
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
        | Loader::Quilt(_)
        | Loader::OptiFine(_) => {
            vec![game_version.mc_version.clone(), version_id.to_string()]
        }
    }
}

#[cfg(test)]
#[path = "tests/version.rs"]
mod tests;
