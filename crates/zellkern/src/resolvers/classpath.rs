use std::collections::HashMap;
use std::path::{Path, PathBuf};

use log::{debug, warn};

use crate::{Loader, VersionManifest};

pub struct ClasspathResolver<'a> {
    manifest: &'a VersionManifest,
    base_id: &'a str,
    lib_dir: PathBuf,
}

impl<'a> ClasspathResolver<'a> {
    pub fn new(manifest: &'a VersionManifest, base_id: &'a str, lib_dir: &Path) -> Self {
        Self {
            manifest,
            base_id,
            lib_dir: lib_dir.to_path_buf(),
        }
    }

    pub fn build(&self) -> String {
        let mut paths: Vec<String> = Vec::new();
        let mut seen: HashMap<String, String> = HashMap::new();

        self.collect_libraries(&mut paths, &mut seen);
        self.add_version_jars(&mut paths);

        #[cfg(target_os = "windows")]
        return paths.join(";");
        #[cfg(not(target_os = "windows"))]
        return paths.join(":");
    }

    fn collect_libraries(&self, paths: &mut Vec<String>, seen: &mut HashMap<String, String>) {
        let libs = match &self.manifest.libraries {
            Some(libs) => libs,
            None => return,
        };

        for lib in libs {
            if !lib.should_include() {
                continue;
            }
            if lib.is_native() {
                debug!("Skipping native JAR: {}", lib.name);
                continue;
            }

            let rel_path = lib.get_path();
            let full_path = self.lib_dir.join(&rel_path);

            if !full_path.exists() {
                warn!("Library not found: {}", full_path.display());
                continue;
            }

            let path_str = full_path.to_string_lossy().to_string();
            let key = maven_key(&lib.name);

            if let Some(existing) = seen.get(&key) {
                debug!("Conflict resolved (last wins): {key}");
                paths.retain(|p| p != existing);
                paths.push(path_str.clone());
                seen.insert(key, path_str);
            } else {
                seen.insert(key, path_str.clone());
                paths.push(path_str);
            }
        }
    }

    fn add_version_jars(&self, paths: &mut Vec<String>) {
        let loader = Loader::from_version_id(&self.manifest.id_raw);
        match loader {
            Loader::Forge(_) | Loader::NeoForge(_) => {
                if let Some(forge_jar) = self.find_forge_universal() {
                    self.push_if_exists(paths, &forge_jar);
                }
                let shared = self.lib_dir.parent().unwrap_or(Path::new("."));
                let vanilla_jar = shared
                    .join("versions")
                    .join(self.base_id)
                    .join(format!("{}.jar", self.base_id));
                self.push_if_exists(paths, &vanilla_jar);
                let version_jar = shared
                    .join("versions")
                    .join(&*self.manifest.id_raw)
                    .join(format!("{}.jar", &*self.manifest.id_raw));
                self.push_if_exists(paths, &version_jar);
            }
            _ => {
                let version_jar = self
                    .lib_dir
                    .parent()
                    .unwrap_or(Path::new("."))
                    .join("versions")
                    .join(self.base_id)
                    .join(format!("{}.jar", self.base_id));
                self.push_if_exists(paths, &version_jar);
            }
        }
    }

    fn push_if_exists(&self, paths: &mut Vec<String>, p: &Path) {
        if p.exists() {
            let s = p.to_string_lossy().to_string();
            if !paths.contains(&s) {
                debug!("Adding JAR: {s}");
                paths.push(s);
            }
        } else {
            debug!("Version JAR not found: {}", p.display());
        }
    }

    fn find_forge_universal(&self) -> Option<PathBuf> {
        let libs = match &self.manifest.libraries {
            Some(libs) => libs,
            None => return None,
        };
        for lib in libs {
            let name = &lib.name;
            if name.contains("net.minecraftforge:forge:")
                || name.contains("net.minecraftforge:minecraftforge:")
            {
                let parts: Vec<&str> = name.splitn(4, ':').collect();
                if parts.len() < 3 {
                    continue;
                }
                let group_path = parts[0].replace('.', "/");
                let artifact = parts[1];
                let version = parts[2];
                let classifier = parts.get(3);

                let base = self.lib_dir.join(&group_path).join(artifact).join(version);

                if let Some(cls) = classifier {
                    let jar = base.join(format!("{artifact}-{version}-{cls}.jar"));
                    if jar.exists() {
                        return Some(jar);
                    }
                }

                let universal = base.join(format!("{artifact}-{version}-universal.jar"));
                if universal.exists() {
                    return Some(universal);
                }
                let normal = base.join(format!("{artifact}-{version}.jar"));
                if normal.exists() {
                    return Some(normal);
                }
            }
        }
        None
    }
}

fn maven_key(name: &str) -> String {
    let parts: Vec<&str> = name.splitn(4, ':').collect();
    match parts.as_slice() {
        [group, artifact, _, classifier] => format!("{group}:{artifact}:{classifier}"),
        [group, artifact, ..] => format!("{group}:{artifact}"),
        _ => name.to_string(),
    }
}
