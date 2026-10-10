use super::{Candidate, Provider};
use crate::services::instance_import::{launchers::multimc::parser, sanitize_instance_name};
use serde_json::Value;
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};
use zellkern::{GameVersion, Loader};

pub fn default_roots(provider: Provider) -> Vec<PathBuf> {
    let Some(base) = directories::BaseDirs::new() else {
        return Vec::new();
    };
    let home = base.home_dir();
    let mut roots = match provider {
        Provider::Official => vec![
            home.join(".minecraft"),
            base.data_dir().join(".minecraft"),
            home.join("Library/Application Support/minecraft"),
            home.join(".var/app/com.mojang.Minecraft/.minecraft"),
        ],
        Provider::Multimc => {
            let mut paths = Vec::new();
            // Known MultiMC-compatible launchers (directories vary by distro and fork)
            for name in [
                "multimc",
                "MultiMC",
                "PrismLauncher",
                "PolyMC",
                // PineconeMC / Ely Prism Launcher forks
                "ElyPrismLauncher",
                "PineconeMC",
            ] {
                paths.push(base.data_dir().join(name));
                paths.push(home.join(name));
                paths.push(home.join(".config").join(name));
            }
            // Flatpak sandboxed app data directories
            for (app, name) in [
                ("org.prismlauncher.PrismLauncher", "PrismLauncher"),
                ("org.polymc.PolyMC", "PolyMC"),
                ("org.multimc.MultiMC", "multimc"),
                // PineconeMC Flatpak ID storing data under ElyPrismLauncher or PineconeMC
                ("ru.pineconemc.launcher", "ElyPrismLauncher"),
                ("ru.pineconemc.launcher", "PineconeMC"),
            ] {
                paths.push(home.join(".var/app").join(app).join("data").join(name));
            }
            paths
        }
        Provider::Xmcl => vec![
            // Data directory (holds instances.json). Linux ~/.config/xmcl,
            // macOS ~/Library/Application Support/xmcl, Windows %APPDATA%\xmcl.
            base.data_dir().join("xmcl"),
            // Legacy data directory used by older releases.
            home.join(".xmcl"),
            // Windows AppX / WinGet installs store data inside the package sandbox.
            base.cache_dir()
                .join("Packages")
                .join("XMCL_68mcaawk44tpj")
                .join("LocalCache")
                .join("Roaming")
                .join("xmcl"),
        ],
    };
    let mut seen = HashSet::new();
    roots.retain(|p| p.is_dir() && seen.insert(fs::canonicalize(p).unwrap_or_else(|_| p.clone())));
    roots
}

pub fn scan_root(
    provider: Provider,
    root: &Path,
    latest: &[(String, String)],
) -> Result<Vec<Candidate>, String> {
    let root = fs::canonicalize(root).map_err(|e| e.to_string())?;
    match provider {
        Provider::Official => official(&root, latest),
        Provider::Multimc => multimc(&root),
        Provider::Xmcl => xmcl(&root),
    }
}

fn candidate(
    name: &str,
    source: PathBuf,
    root: &Path,
    version: Result<GameVersion, String>,
) -> Candidate {
    let (version, error) = match version {
        Ok(v) => (Some(v), None),
        Err(e) => (None, Some(e)),
    };
    Candidate {
        id: uuid::Uuid::new_v4().to_string(),
        name: name.to_string(),
        suggested_name: sanitize_instance_name(name),
        source,
        root: root.to_path_buf(),
        version,
        error,
        shared_directory: false,
        reinstalls_components: false,
        icon: None,
        memory: None,
    }
}

fn multimc(root: &Path) -> Result<Vec<Candidate>, String> {
    let mut dirs = if root.join("instance.cfg").is_file() {
        vec![root.to_path_buf()]
    } else {
        let instances = if root.join("instances").is_dir() {
            root.join("instances")
        } else {
            root.to_path_buf()
        };
        fs::read_dir(instances)
            .map_err(|e| e.to_string())?
            .map(|entry| entry.map(|e| e.path()).map_err(|e| e.to_string()))
            .collect::<Result<Vec<_>, _>>()?
    };
    dirs.sort();
    let mut result = Vec::new();
    for dir in dirs {
        if !dir.join("instance.cfg").is_file() {
            continue;
        }
        let fallback = dir.file_name().unwrap_or_default().to_string_lossy();
        let meta = parser::parse_multimc_instance(&dir);
        let mut c = match meta {
            Ok(meta) => {
                let resolved = resolve_multimc_components(&dir);
                let reinstalls_components = resolved.as_ref().is_ok_and(|(_, patched)| *patched);
                let version = resolved.map(|(version, _)| version);
                let game = if dir.join(".minecraft").is_dir() {
                    dir.join(".minecraft")
                } else {
                    dir.join("minecraft")
                };
                let mut c = candidate(&meta.original_name, game, root, version);
                c.reinstalls_components = reinstalls_components;
                // Only explicit per-instance memory overrides should replace Cubic defaults.
                let cfg =
                    fs::read_to_string(dir.join("instance.cfg")).map_err(|e| e.to_string())?;
                let cfg = parser::parse_ini(&cfg);
                if cfg
                    .general
                    .get("OverrideMemory")
                    .is_some_and(|v| v.eq_ignore_ascii_case("true"))
                {
                    let min = meta.min_memory.unwrap_or(512);
                    let max = meta.max_memory.unwrap_or(2048);
                    if min > 0 && max >= min {
                        c.memory = Some((min, max));
                    }
                }
                let mut icons = vec![dir.join("icon.png"), c.source.join("icon.png")];
                if let Some(key) = meta.icon_key.filter(|k| safe_component(k)) {
                    icons.push(dir.join(format!("{key}.png")));
                    if let Some(launcher) = dir.parent().and_then(Path::parent) {
                        icons.push(launcher.join("icons").join(format!("{key}.png")));
                    }
                }
                c.icon = icons.into_iter().find(|p| p.is_file());
                c
            }
            Err(e) => candidate(&fallback, dir.clone(), root, Err(e.to_string())),
        };
        if c.source.is_dir() {
            c.source = fs::canonicalize(&c.source).map_err(|e| e.to_string())?;
        } else if c.error.is_none() {
            // A never-launched MultiMC instance may have no game directory yet.
            c.source = dir;
        }
        result.push(c);
    }
    Ok(result)
}

/// Scan an XMCL data directory for instances.
///
/// XMCL keeps a registry (`instances.json`) in its data directory and instance
/// folders under `<game data>/instances`. Managed instances are listed relative
/// to that `instances` folder while external ones are absolute; a `root` pointer
/// file in the data directory relocates the game data folder.
fn xmcl(root: &Path) -> Result<Vec<Candidate>, String> {
    let game_root = fs::read_to_string(root.join("root"))
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .filter(|path| path.is_dir())
        .unwrap_or_else(|| root.to_path_buf());
    // Accept either the data directory (contains `instances/`) or the
    // `instances` folder itself.
    let instances = if game_root.join("instances").is_dir() {
        game_root.join("instances")
    } else {
        game_root.clone()
    };

    let mut dirs: Vec<PathBuf> = Vec::new();
    if instances.is_dir() {
        for entry in fs::read_dir(&instances).map_err(|e| e.to_string())? {
            let path = entry.map_err(|e| e.to_string())?.path();
            if path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with('.'))
            {
                continue;
            }
            dirs.push(path);
        }
    }
    for registry in [root.join("instances.json"), game_root.join("instances.json")] {
        let Ok(bytes) = fs::read(&registry) else {
            continue;
        };
        let Ok(json) = serde_json::from_slice::<Value>(&bytes) else {
            continue;
        };
        for listed in json
            .get("instances")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            let path = PathBuf::from(listed);
            dirs.push(if path.is_absolute() {
                path
            } else {
                instances.join(path)
            });
        }
    }

    let mut seen = HashSet::new();
    dirs.retain(|path| path.join("instance.json").is_file() && seen.insert(path.clone()));
    dirs.sort();

    let mut result = Vec::new();
    for dir in dirs {
        let fallback = dir
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let json = match fs::read(dir.join("instance.json"))
            .map_err(|e| e.to_string())
            .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).map_err(|e| e.to_string()))
        {
            Ok(json) => json,
            Err(e) => {
                result.push(candidate(&fallback, dir, root, Err(e)));
                continue;
            }
        };
        let name = json
            .get("name")
            .and_then(Value::as_str)
            .filter(|name| !name.trim().is_empty())
            .unwrap_or(&fallback);
        if json.get("edition").and_then(Value::as_str) == Some("bedrock") {
            result.push(candidate(
                name,
                dir,
                root,
                Err("Bedrock Edition is not supported".into()),
            ));
            continue;
        }
        let version = xmcl_version(json.get("runtime").unwrap_or(&Value::Null));
        let mut c = candidate(name, dir.clone(), root, version);
        c.reinstalls_components = c
            .version
            .as_ref()
            .is_some_and(|version| !matches!(version.loader, Loader::Vanilla));
        if xmcl_assigns_memory(&json) {
            let min = json.get("minMemory").and_then(Value::as_u64).unwrap_or(0);
            let max = json.get("maxMemory").and_then(Value::as_u64).unwrap_or(0);
            // Only explicit, launchable overrides; avoids an invalid `-Xms0M`.
            if min > 0 && max >= min && max <= u32::MAX as u64 {
                c.memory = Some((min as u32, max as u32));
            }
        }
        let mut icons = vec![dir.join("icon.png")];
        if let Some(icon) = json.get("icon").and_then(Value::as_str) {
            let icon = Path::new(icon);
            if icon.is_absolute() {
                icons.push(icon.to_path_buf());
            } else if safe_component(icon.to_string_lossy().as_ref()) {
                icons.push(dir.join(icon));
            }
        }
        c.icon = icons.into_iter().find(|path| path.is_file());
        if c.source.is_dir() {
            c.source = fs::canonicalize(&c.source).map_err(|e| e.to_string())?;
        }
        result.push(c);
    }
    Ok(result)
}

/// Build a `GameVersion` from an XMCL instance `runtime` object.
fn xmcl_version(runtime: &Value) -> Result<GameVersion, String> {
    let field = |key: &str| {
        runtime
            .get(key)
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
    };
    let mc_version = field("minecraft").ok_or("Minecraft version not found")?;
    if !safe_component(mc_version) {
        return Err("Invalid Minecraft version".into());
    }
    // Cubic cannot reinstall these client components; refuse instead of silently
    // dropping them, matching the official launcher scanner.
    if field("optifine").is_some() {
        return Err("Unsupported component: OptiFine".into());
    }
    if field("labyMod").is_some() {
        return Err("Unsupported component: LabyMod".into());
    }
    let mut loaders = Vec::new();
    if let Some(version) = field("fabricLoader") {
        if !safe_component(version) {
            return Err("Invalid Fabric version".into());
        }
        loaders.push(Loader::Fabric(version.to_string()));
    }
    if let Some(version) = field("quiltLoader") {
        if !safe_component(version) {
            return Err("Invalid Quilt version".into());
        }
        loaders.push(Loader::Quilt(version.to_string()));
    }
    if let Some(version) = field("forge") {
        let version = version.strip_prefix(&format!("{mc_version}-")).unwrap_or(version);
        if !safe_component(version) {
            return Err("Invalid Forge version".into());
        }
        loaders.push(Loader::Forge(version.to_string()));
    }
    if let Some(version) = field("neoForged") {
        if !safe_component(version) {
            return Err("Invalid NeoForge version".into());
        }
        loaders.push(Loader::NeoForge(version.to_string()));
    }
    let loader = match loaders.len() {
        0 => Loader::Vanilla,
        1 => loaders.pop().unwrap(),
        _ => return Err("Multiple loaders are not supported".into()),
    };
    Ok(GameVersion {
        mc_version: mc_version.to_string(),
        loader,
    })
}

fn xmcl_assigns_memory(json: &Value) -> bool {
    match json.get("assignMemory") {
        Some(Value::Bool(true)) => true,
        Some(Value::String(value)) => value == "auto",
        _ => false,
    }
}

fn resolve_multimc_components(dir: &Path) -> Result<(GameVersion, bool), String> {
    let mut pack = parser::read_mmc_pack(dir).ok_or("Missing or invalid mmc-pack.json")?;
    let patches = dir.join("patches");
    let mut patched = false;
    let mut required_minecraft = Vec::new();
    if patches.is_dir() {
        for entry in fs::read_dir(patches).map_err(|e| e.to_string())? {
            let path = entry.map_err(|e| e.to_string())?.path();
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            let patch: Value = serde_json::from_slice(&fs::read(&path).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
            let uid = patch
                .get("uid")
                .and_then(Value::as_str)
                .ok_or("Component patch has no UID")?;
            if !matches!(
                uid,
                "net.fabricmc.fabric-loader"
                    | "net.fabricmc.intermediary"
                    | "net.neoforged"
                    | "org.quiltmc.quilt-loader"
                    | "org.quiltmc.hashed"
                    | "net.minecraftforge"
            ) {
                return Err(format!("Unsupported component patch: {uid}"));
            }
            let component = pack
                .components
                .iter_mut()
                .find(|c| c.uid == uid)
                .ok_or_else(|| format!("Component patch missing from mmc-pack.json: {uid}"))?;
            if let Some(version) = patch.get("version").and_then(Value::as_str) {
                if !safe_component(version) {
                    return Err(format!("Invalid component version: {uid}"));
                }
                component.version = version.to_string();
            }
            for requirement in patch
                .get("requires")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                if requirement.get("uid").and_then(Value::as_str) == Some("net.minecraft")
                    && let Some(version) = requirement.get("equals").and_then(Value::as_str)
                {
                    required_minecraft.push(version.to_string());
                }
            }
            // Cubic reinstalls the published loader; local library/argument edits are not copied.
            patched = true;
        }
    }
    let mut loaders = 0;
    for component in &pack.components {
        match component.uid.as_str() {
            "net.minecraft"
            | "org.lwjgl"
            | "org.lwjgl3"
            | "net.fabricmc.intermediary"
            | "org.quiltmc.hashed" => {}
            "net.fabricmc.fabric-loader"
            | "org.quiltmc.quilt-loader"
            | "net.minecraftforge"
            | "net.neoforged" => {
                loaders += 1;
                if component.effective_version().is_none() {
                    return Err(format!("Missing loader version: {}", component.uid));
                }
            }
            _ => return Err(format!("Unsupported component: {}", component.uid)),
        }
    }
    if loaders > 1 {
        return Err("Multiple loaders are not supported".into());
    }
    let (version, _) = parser::resolve_game_version(&pack);
    let version = version.ok_or("Minecraft version not found")?;
    if required_minecraft
        .iter()
        .any(|required| required != &version.mc_version)
    {
        return Err("Component patch requires a different Minecraft version".into());
    }
    for component in &pack.components {
        if matches!(
            component.uid.as_str(),
            "net.fabricmc.intermediary" | "org.quiltmc.hashed"
        ) && component
            .effective_version()
            .is_some_and(|v| v != version.mc_version)
        {
            return Err("Mappings do not match the Minecraft version".into());
        }
    }
    Ok((version, patched))
}

fn official(root: &Path, latest: &[(String, String)]) -> Result<Vec<Candidate>, String> {
    let profiles_paths: Vec<_> = [
        "launcher_profiles.json",
        "launcher_profiles_microsoft_store.json",
    ]
    .into_iter()
    .map(|name| root.join(name))
    .filter(|p| p.is_file())
    .collect();
    if profiles_paths.is_empty() {
        return Err("launcher_profiles.json not found".into());
    }
    let mut profiles = Vec::new();
    for profiles_path in profiles_paths {
        let json: Value =
            serde_json::from_slice(&fs::read(profiles_path).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        for (id, profile) in json
            .get("profiles")
            .and_then(Value::as_object)
            .ok_or("Invalid launcher profiles")?
        {
            let entry = (id.clone(), profile.clone());
            if !profiles.contains(&entry) {
                profiles.push(entry);
            }
        }
    }
    let mut result = Vec::new();
    for (id, profile) in profiles {
        let name = profile
            .get("name")
            .and_then(Value::as_str)
            .filter(|s| !s.trim().is_empty())
            .unwrap_or(&id);
        let source = profile
            .get("gameDir")
            .and_then(Value::as_str)
            .map(PathBuf::from)
            .unwrap_or_else(|| root.to_path_buf());
        let source = if source.is_absolute() {
            source
        } else {
            root.join(source)
        };
        let version = profile
            .get("lastVersionId")
            .and_then(Value::as_str)
            .unwrap_or("latest-release");
        let version = resolve_official(root, version, latest, &mut HashSet::new());
        let mut c = candidate(name, source.clone(), root, version);
        match fs::canonicalize(source) {
            Ok(path) if path.is_dir() => c.source = path,
            _ => c.error = Some("Game directory not found".into()),
        }
        result.push(c);
    }
    for i in 0..result.len() {
        result[i].shared_directory = result
            .iter()
            .enumerate()
            .any(|(j, c)| i != j && c.source == result[i].source);
    }
    Ok(result)
}

fn safe_component(id: &str) -> bool {
    !id.is_empty()
        && id != "."
        && id != ".."
        && !id.chars().any(|c| matches!(c, '/' | '\\' | ':' | '\0'))
}

fn resolve_official(
    root: &Path,
    id: &str,
    latest: &[(String, String)],
    seen: &mut HashSet<String>,
) -> Result<GameVersion, String> {
    let id = match id {
        "latest-release" => latest
            .iter()
            .find(|(_, kind)| kind == "release")
            .map(|(id, _)| id.as_str())
            .ok_or("Cannot resolve latest release")?,
        "latest-snapshot" => latest
            .first()
            .map(|(id, _)| id.as_str())
            .ok_or("Cannot resolve latest snapshot")?,
        id => id,
    };
    if !safe_component(id) || seen.len() >= 16 || !seen.insert(id.to_string()) {
        return Err("Invalid or cyclic version inheritance".into());
    }
    let path = root.join("versions").join(id).join(format!("{id}.json"));
    if !path.exists() {
        if latest.iter().any(|(known, _)| known == id) {
            return Ok(GameVersion {
                mc_version: id.to_string(),
                loader: Loader::Vanilla,
            });
        }
        return Err(format!("Version metadata not found: {id}"));
    }
    let json: Value = serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let parent = json.get("inheritsFrom").and_then(Value::as_str);
    let arguments: Vec<&str> = json
        .pointer("/arguments/game")
        .and_then(Value::as_array)
        .map(|args| args.iter().filter_map(Value::as_str).collect())
        .unwrap_or_else(|| {
            json.get("minecraftArguments")
                .and_then(Value::as_str)
                .unwrap_or("")
                .split_whitespace()
                .collect()
        });
    let argument = |key: &str| {
        arguments
            .windows(2)
            .find(|pair| pair[0] == key)
            .map(|pair| pair[1])
    };
    let intermediary = json
        .get("libraries")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|lib| lib.get("name").and_then(Value::as_str))
        .find_map(|name| {
            name.strip_prefix("net.fabricmc:intermediary:")
                .map(|v| v.split(':').next().unwrap_or(v))
        });
    let base_hint = argument("--fml.mcVersion").or(intermediary);
    let mut version = if let Some(parent) = parent {
        if base_hint.is_some_and(|hint| hint != parent) {
            return Err("Conflicting Minecraft versions in loader metadata".into());
        }
        // Installers may create a profile before the vanilla parent has been downloaded.
        let parent_path = root
            .join("versions")
            .join(parent)
            .join(format!("{parent}.json"));
        if safe_component(parent)
            && base_hint == Some(parent)
            && !parent_path.exists()
            && !seen.contains(parent)
        {
            GameVersion {
                mc_version: parent.to_string(),
                loader: Loader::Vanilla,
            }
        } else {
            resolve_official(root, parent, latest, seen)?
        }
    } else if let Some(base) = base_hint.filter(|base| safe_component(base)) {
        GameVersion {
            mc_version: base.to_string(),
            loader: Loader::Vanilla,
        }
    } else {
        GameVersion {
            mc_version: id.to_string(),
            loader: Loader::Vanilla,
        }
    };
    let mut loader = None;
    if let Some(v) = argument("--fml.neoForgeVersion") {
        if !safe_component(v) {
            return Err("Invalid NeoForge version".into());
        }
        loader = Some(Loader::NeoForge(v.to_string()));
    }
    if let Some(libraries) = json.get("libraries").and_then(Value::as_array) {
        for library in libraries {
            let Some(name) = library.get("name").and_then(Value::as_str) else {
                continue;
            };
            let parts: Vec<_> = name.split(':').collect();
            if parts.len() < 3 {
                continue;
            }
            let v = parts[2].to_string();
            let detected = match (parts[0], parts[1]) {
                ("net.fabricmc", "fabric-loader") => Some(Loader::Fabric(v)),
                ("org.quiltmc", "quilt-loader") => Some(Loader::Quilt(v)),
                ("net.minecraftforge", "forge") => Some(Loader::Forge(
                    v.strip_prefix(&format!("{}-", version.mc_version))
                        .unwrap_or(&v)
                        .to_string(),
                )),
                ("net.neoforged", "neoforge") => Some(Loader::NeoForge(v)),
                ("optifine", _) | ("com.mumfrey", "liteloader") => {
                    return Err(format!("Unsupported component: {name}"));
                }
                _ => None,
            };
            if let Some(detected) = detected {
                if loader.as_ref().is_some_and(|current| current != &detected) {
                    return Err("Multiple loaders are not supported".into());
                }
                loader = Some(detected);
            }
        }
    }
    if let Some(loader) = loader {
        if parent.is_none() && base_hint.is_none() {
            return Err("Modded version has no base Minecraft version".into());
        }
        version.loader = loader;
    } else if parent.is_some()
        || (!latest.is_empty() && !latest.iter().any(|(known, _)| known == id))
        || json
            .get("mainClass")
            .and_then(Value::as_str)
            .is_some_and(|main| {
                main != "net.minecraft.client.main.Main" && main != "net.minecraft.client.Minecraft"
            })
    {
        return Err(format!("Unsupported custom version: {id}"));
    }
    Ok(version)
}

#[cfg(test)]
#[path = "../../../tests/services/instance_import/migration_discovery.rs"]
mod tests;
