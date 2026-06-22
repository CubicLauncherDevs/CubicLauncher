use std::collections::HashSet;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;

use log::info;
use tokio::sync::mpsc::Sender;
use zellkern::{
    InstallProfile, LegacyInstallProfile, Processor, ProfileLibrary, VersionManifest, maven_to_path,
};

use super::batch::{DownloadBatch, DownloadItemSpec};
use crate::AquaError;
use crate::types::DownloadProgress;
use crate::utilities::{
    compute_sha1_sync, extract_zip_to_dir, read_jar_main_class, run_java_process,
};

#[derive(Debug, Clone)]
pub struct ForgeVersionInfo {
    pub game_version: String,
    pub forge_version: String,
    pub version_id: String,
    pub installer_url: String,
}

/// Internal representation — either modern or legacy Forge.
enum ProfileKind {
    Modern { profile: InstallProfile },
    Legacy { profile: LegacyInstallProfile },
}

pub struct ForgeBatch {
    version_id: String,
    game_version: String,
    #[allow(dead_code)]
    forge_version: String,
    shared_dir: PathBuf,
    staging_dir: PathBuf,
    items: Vec<DownloadItemSpec>,
    profile_kind: ProfileKind,
    version_json_text: String,
    java_path: Option<PathBuf>,
}

impl ForgeBatch {
    pub async fn new(
        shared_dir: &Path,
        game_version: &str,
        forge_version: &str,
        installer_url: &str,
        java_path: Option<PathBuf>,
    ) -> Result<Self, AquaError> {
        let version_id = format!("{game_version}-forge-{forge_version}");
        let temp_dir = shared_dir.join("temp").join(format!("forge-{version_id}"));
        let staging_dir = temp_dir.join("staging");

        if temp_dir.exists() {
            tokio::fs::remove_dir_all(&temp_dir).await?;
        }
        tokio::fs::create_dir_all(&staging_dir).await?;

        // 1. Download installer — try modern URL first, fallback with MC suffix
        let installer_path = temp_dir.join("installer.jar");
        info!("Downloading Forge installer: {installer_url}");
        let dl_result = crate::utilities::download_file(installer_url, &installer_path, "").await;
        if let Err(ref e) = dl_result {
            if e.to_string().contains("HTTP") {
                let fallback_url = format!(
                    "https://maven.minecraftforge.net/net/minecraftforge/forge/{game_version}-{forge_version}-{game_version}/forge-{game_version}-{forge_version}-{game_version}-installer.jar"
                );
                if fallback_url != installer_url {
                    info!("Installer download failed ({e}), trying legacy URL: {fallback_url}");
                    crate::utilities::download_file(&fallback_url, &installer_path, "").await?;
                }
            } else {
                dl_result?;
            }
        }

        // 2. Extract installer
        let extract_dir = temp_dir.join("extracted");
        tokio::fs::create_dir_all(&extract_dir).await?;
        let extract_dir_clone = extract_dir.clone();
        let installer_path_clone = installer_path.clone();
        tokio::task::spawn_blocking(move || {
            extract_zip_to_dir(&installer_path_clone, &extract_dir_clone)
        })
        .await??;

        // 3. Parse install_profile.json — detect modern vs legacy
        let profile_path = extract_dir.join("install_profile.json");
        let profile_bytes = tokio::fs::read(&profile_path).await?;
        let profile_kind;
        let version_json_text;

        match serde_json::from_slice::<InstallProfile>(&profile_bytes) {
            Ok(mut profile) => {
                info!(
                    "Forge profile (modern): spec={}, version={}, {} processors, {} profile libs",
                    profile.spec,
                    profile.version,
                    profile.processors.len(),
                    profile.libraries.len()
                );
                // Inject INSTALLER data entry (XMCL-style)
                let installer_maven =
                    format!("net.minecraftforge:forge:{game_version}-{forge_version}:installer");
                let installer_entry = zellkern::DataEntry {
                    client: Some(serde_json::Value::String(format!("[{installer_maven}]"))),
                    server: Some(serde_json::Value::String(format!("[{installer_maven}]"))),
                };
                profile.data.insert("INSTALLER".into(), installer_entry);

                // Modern: read separate version.json
                let version_json_path = extract_dir.join("version.json");
                version_json_text = tokio::fs::read_to_string(&version_json_path)
                    .await
                    .map_err(|e| {
                        AquaError::ForgeExtract(format!("Cannot read version.json: {e}"))
                    })?;
                profile_kind = ProfileKind::Modern { profile };
            }
            Err(e) => {
                info!("Modern parse failed ({e}), trying legacy install_profile.json format");
                let legacy: LegacyInstallProfile =
                    serde_json::from_slice(&profile_bytes).map_err(|e2| {
                        AquaError::ForgeProfileParse(format!("Not modern ({e}) or legacy ({e2})"))
                    })?;
                info!(
                    "Forge profile (legacy): target={}, minecraft={}, {} libs",
                    legacy.install.target,
                    legacy.install.minecraft,
                    legacy.version_info.libraries.len()
                );
                // Legacy: version.json content is the versionInfo
                version_json_text =
                    serde_json::to_string_pretty(&legacy.version_info).map_err(|e| {
                        AquaError::ForgeProfileParse(format!(
                            "Failed to serialize legacy versionInfo: {e}"
                        ))
                    })?;
                profile_kind = ProfileKind::Legacy { profile: legacy };
            }
        }

        // 4. Collect library download items
        let staging_libs = staging_dir.join("libraries");
        let mut items: Vec<DownloadItemSpec> = Vec::new();
        let mut seen_paths: HashSet<String> = HashSet::new();

        match &profile_kind {
            ProfileKind::Modern { profile } => {
                // Modern profile libs (from install_profile.json)
                add_modern_libs(
                    &profile.libraries,
                    &mut items,
                    &mut seen_paths,
                    &staging_libs,
                )?;

                // Also collect libs from version.json
                let vj: serde_json::Value = serde_json::from_str(&version_json_text)
                    .map_err(|e| AquaError::ForgeProfileParse(e.to_string()))?;
                if let Some(vj_libs) = vj.get("libraries").and_then(|l| l.as_array()) {
                    for vj_lib in vj_libs {
                        add_vj_lib(vj_lib, &mut items, &mut seen_paths, &staging_libs);
                    }
                }
            }
            ProfileKind::Legacy { profile } => {
                // Legacy: libs from versionInfo.libraries (old format with url at lib level)
                add_legacy_libs(
                    &profile.version_info.libraries,
                    &mut items,
                    &mut seen_paths,
                    &staging_libs,
                )?;
            }
        }

        info!("Forge batch: {} libraries to download", items.len());

        Ok(Self {
            version_id,
            game_version: game_version.to_string(),
            forge_version: forge_version.to_string(),
            shared_dir: shared_dir.to_path_buf(),
            staging_dir,
            items,
            profile_kind,
            version_json_text,
            java_path,
        })
    }

    pub async fn install(
        shared_dir: &Path,
        game_version: &str,
        forge_version: &str,
        java_path: Option<PathBuf>,
    ) -> Result<VersionManifest, AquaError> {
        let installer_url = Self::resolve_installer_url(game_version, forge_version);
        let batch = Self::new(
            shared_dir,
            game_version,
            forge_version,
            &installer_url,
            java_path,
        )
        .await?;
        batch.run(shared_dir).await
    }

    pub fn resolve_installer_url(game_version: &str, forge_version: &str) -> String {
        format!(
            "https://maven.minecraftforge.net/net/minecraftforge/forge/{game_version}-{forge_version}/forge-{game_version}-{forge_version}-installer.jar"
        )
    }

    fn extract_dir(&self) -> PathBuf {
        self.staging_dir
            .parent()
            .expect("staging must have parent")
            .join("extracted")
    }

    async fn run(self, shared_dir: &Path) -> Result<VersionManifest, AquaError> {
        let handle = super::batch::GenericBatch::new(&self.version_id, self.items.clone());
        let manager = super::DownloadManager::new(shared_dir.to_path_buf());
        let dl_handle = manager.prepare_batch(Box::new(handle)).await?;
        dl_handle.download_all(None).await?;

        self.finalize(None).await?;

        let manifest = VersionManifest::from_bytes(self.version_json_text.as_bytes())?;
        Ok(manifest)
    }
}

impl DownloadBatch for ForgeBatch {
    fn name(&self) -> String {
        self.version_id.clone()
    }

    fn items(&self) -> &[DownloadItemSpec] {
        &self.items
    }

    fn prepare(&self) -> Pin<Box<dyn Future<Output = Result<(), AquaError>> + Send + '_>> {
        let staging_dir = self.staging_dir.clone();
        let version_id = self.version_id.clone();
        let version_json_text = self.version_json_text.clone();
        let extract_dir = self.extract_dir();

        Box::pin(async move {
            // Write version.json to staging/versions/{id}/
            let staging_versions = staging_dir.join("versions").join(&version_id);
            tokio::fs::create_dir_all(&staging_versions).await?;
            let vj_path = staging_versions.join(format!("{version_id}.json"));
            tokio::fs::write(&vj_path, &version_json_text).await?;
            info!("Wrote version.json to staging: {:?}", vj_path);

            // Copy maven/ to staging/libraries/
            let maven_dir = extract_dir.join("maven");
            if maven_dir.exists() {
                let staging_libs = staging_dir.join("libraries");
                let maven = maven_dir.clone();
                let libs = staging_libs.clone();
                tokio::task::spawn_blocking(move || copy_dir_recursive(&maven, &libs)).await??;
                info!("Copied maven/ to staging/libraries/");
            }

            Ok(())
        })
    }

    fn finalize(
        &self,
        progress_tx: Option<Sender<DownloadProgress>>,
    ) -> Pin<Box<dyn Future<Output = Result<(), AquaError>> + Send + '_>> {
        let shared_dir = self.shared_dir.clone();
        let staging_dir = self.staging_dir.clone();
        let version_id = self.version_id.clone();
        let game_version = self.game_version.clone();
        let extract_dir = self.extract_dir();
        let is_legacy = matches!(&self.profile_kind, ProfileKind::Legacy { .. });
        let processors = match &self.profile_kind {
            ProfileKind::Modern { profile } => profile.processors.clone(),
            ProfileKind::Legacy { .. } => vec![],
        };
        let data = match &self.profile_kind {
            ProfileKind::Modern { profile } => profile.data.clone(),
            ProfileKind::Legacy { .. } => std::collections::HashMap::new(),
        };

        Box::pin(async move {
            if is_legacy {
                info!("Forge legacy (spec 0): no post-processors needed");
                commit_and_cleanup(&staging_dir, &shared_dir)?;
                return Ok(());
            }

            let java_path = self.java_path.as_ref().ok_or_else(|| {
                AquaError::JavaNotFound(
                    "No Java runtime path was provided for Forge installation".into(),
                )
            })?;
            let staging_libs = staging_dir.join("libraries");
            let mc_jar = shared_dir
                .join("versions")
                .join(&game_version)
                .join(format!("{game_version}.jar"));

            // Copy client.lzma to staging/data/
            let client_lzma = extract_dir.join("data").join("client.lzma");
            if client_lzma.exists() {
                let staging_data = staging_dir.join("data");
                tokio::fs::create_dir_all(&staging_data).await?;
                let dest = staging_data.join("client.lzma");
                tokio::fs::copy(&client_lzma, &dest).await?;
                info!("Copied client.lzma to staging/data/");
            }

            // Filter client-side processors
            let client_processors: Vec<&Processor> = processors
                .iter()
                .filter(|p| p.sides.contains(&"client".to_string()))
                .collect();

            if client_processors.is_empty() {
                info!("No client-side processors to run");
                commit_and_cleanup(&staging_dir, &shared_dir)?;
                return Ok(());
            }

            info!(
                "Running {} client-side post-processors",
                client_processors.len()
            );

            let total = client_processors.len();
            for (i, proc) in client_processors.iter().enumerate() {
                let proc_name = proc.jar.split(':').nth(1).unwrap_or(&proc.jar);
                info!("Processor {}/{}: {}", i + 1, total, proc_name);

                if let Some(ref tx) = progress_tx {
                    let _ = tx
                        .send(DownloadProgress {
                            current: i,
                            total,
                            info: crate::types::DownloadProgressInfo {
                                name: format!("Forge post-processor: {proc_name}"),
                                version: Arc::new(version_id.clone()),
                            },
                            download_type: crate::types::DownloadProgressType::Processing,
                        })
                        .await;
                }

                // Build classpath — processor JARs live in staging/libraries/
                let processor_jar = staging_libs.join(maven_to_path(&proc.jar));
                if !processor_jar.exists() {
                    return Err(AquaError::ForgeProcessor {
                        processor: proc.jar.clone(),
                        detail: format!("Processor JAR not found: {}", processor_jar.display()),
                    });
                }

                let mut cp_parts: Vec<String> = Vec::new();
                cp_parts.push(processor_jar.to_string_lossy().to_string());
                for dep in &proc.classpath {
                    let dep_path = staging_libs.join(maven_to_path(dep));
                    if dep_path.exists() {
                        cp_parts.push(dep_path.to_string_lossy().to_string());
                    } else {
                        return Err(AquaError::ForgeProcessor {
                            processor: proc.jar.clone(),
                            detail: format!("Classpath dependency not found: {dep}"),
                        });
                    }
                }
                let separator = if cfg!(target_os = "windows") {
                    ";"
                } else {
                    ":"
                };
                let classpath = cp_parts.join(separator);

                let main_class = read_jar_main_class(&processor_jar)?;

                // resolve_arg uses staging_dir as ROOT so {ROOT}/data/client.lzma works
                let staging_parent = staging_dir.clone();
                let resolved_args: Vec<String> = proc
                    .args
                    .iter()
                    .map(|arg| resolve_arg(arg, &data, &mc_jar, &staging_libs, &staging_parent))
                    .collect();

                run_java_process(java_path, &classpath, &main_class, resolved_args, &proc.jar)
                    .await?;

                // Verify outputs
                for (out_coord, expected_sha) in &proc.outputs {
                    let out_path =
                        resolve_arg(out_coord, &data, &mc_jar, &staging_libs, &staging_parent);
                    let expected_resolved =
                        resolve_arg(expected_sha, &data, &mc_jar, &staging_libs, &staging_parent);
                    let expected_clean = expected_resolved.trim_matches('\'');

                    let out_path = Path::new(&out_path);
                    if !out_path.exists() {
                        return Err(AquaError::ForgeOutputVerification {
                            file: out_path.display().to_string(),
                            expected: expected_clean.to_string(),
                            actual: "File not generated".into(),
                        });
                    }

                    let actual = compute_sha1_sync(out_path)?;
                    if actual != expected_clean {
                        return Err(AquaError::ForgeOutputVerification {
                            file: out_path.display().to_string(),
                            expected: expected_clean.to_string(),
                            actual,
                        });
                    }

                    info!(
                        "Output verified: {}",
                        out_path.file_name().unwrap_or_default().to_string_lossy()
                    );
                }
            }

            if let Some(ref tx) = progress_tx {
                let _ = tx
                    .send(DownloadProgress {
                        current: total,
                        total,
                        info: crate::types::DownloadProgressInfo {
                            name: "Forge installation complete".into(),
                            version: Arc::new(version_id.clone()),
                        },
                        download_type: crate::types::DownloadProgressType::Processing,
                    })
                    .await;
            }

            info!("All Forge post-processors completed successfully");

            // Copy patched MC JAR to Forge version dir so classpath matches -DignoreList
            let patched_jar = shared_dir
                .join("versions")
                .join(&game_version)
                .join(format!("{game_version}.jar"));
            let forge_jar_dest = staging_dir
                .join("versions")
                .join(&version_id)
                .join(format!("{version_id}.jar"));
            if patched_jar.exists() && !forge_jar_dest.exists() {
                tokio::fs::copy(&patched_jar, &forge_jar_dest).await?;
                info!(
                    "Copied patched MC JAR to Forge version dir: {:?}",
                    forge_jar_dest
                );
            }

            commit_and_cleanup(&staging_dir, &shared_dir)?;
            Ok(())
        })
    }
}

// ── Library collection helpers ──────────────────────────────────────────

/// Collect download items from modern `ProfileLibrary` entries (install_profile.json).
fn add_modern_libs(
    libs: &[ProfileLibrary],
    items: &mut Vec<DownloadItemSpec>,
    seen: &mut HashSet<String>,
    staging_libs: &Path,
) -> Result<(), AquaError> {
    for lib in libs {
        // Prefer downloads.artifact
        if let Some(ref downloads) = lib.downloads
            && let Some(ref artifact) = downloads.artifact
            && !artifact.path.is_empty()
            && !seen.contains(&artifact.path)
        {
            if let Some(ref url) = artifact.url
                && !url.is_empty()
            {
                let dest = staging_libs.join(&artifact.path);
                items.push(
                    DownloadItemSpec::new(url.clone(), dest, &lib.name)
                        .with_hash(artifact.sha1.clone().unwrap_or_default()),
                );
            }
            seen.insert(artifact.path.clone());
            continue;
        }
        // Fallback: lib has `url` base but no `downloads.artifact` — compute path from name
        if let Some(ref base_url) = lib.url {
            let path = maven_to_path(&lib.name).to_string_lossy().into_owned();
            if !seen.contains(&path) {
                let full_url = format!("{}/{path}", base_url.trim_end_matches('/'));
                let dest = staging_libs.join(&path);
                items.push(DownloadItemSpec::new(full_url, dest, &lib.name));
                seen.insert(path);
            }
        }
    }
    Ok(())
}

/// Collect download items from a version.json library entry (serde_json::Value).
fn add_vj_lib(
    vj_lib: &serde_json::Value,
    items: &mut Vec<DownloadItemSpec>,
    seen: &mut HashSet<String>,
    staging_libs: &Path,
) {
    let name = vj_lib
        .get("name")
        .and_then(|n| n.as_str())
        .unwrap_or("")
        .to_string();
    if let Some(artifact) = vj_lib.get("downloads").and_then(|d| d.get("artifact")) {
        let path = artifact
            .get("path")
            .and_then(|p| p.as_str())
            .unwrap_or("")
            .to_string();
        let url = artifact
            .get("url")
            .and_then(|u| u.as_str())
            .unwrap_or("")
            .to_string();
        let sha1 = artifact
            .get("sha1")
            .and_then(|s| s.as_str())
            .unwrap_or("")
            .to_string();

        if !path.is_empty() && !seen.contains(&path) {
            if !url.is_empty() {
                let dest = staging_libs.join(&path);
                items.push(DownloadItemSpec::new(url, dest, &name).with_hash(sha1));
            }
            seen.insert(path);
        }
    } else if let Some(base_url) = vj_lib.get("url").and_then(|u| u.as_str()) {
        // Old-style lib with url at lib level but found inside version.json
        let path = maven_to_path(&name).to_string_lossy().into_owned();
        if !seen.contains(&path) {
            let full_url = format!("{}/{path}", base_url.trim_end_matches('/'));
            let dest = staging_libs.join(&path);
            items.push(DownloadItemSpec::new(full_url, dest, &name));
            seen.insert(path);
        }
    }
}

/// Collect download items from legacy `LegacyLibrary` entries (old install_profile.json).
fn add_legacy_libs(
    libs: &[zellkern::LegacyLibrary],
    items: &mut Vec<DownloadItemSpec>,
    seen: &mut HashSet<String>,
    staging_libs: &Path,
) -> Result<(), AquaError> {
    for lib in libs {
        // Skip libs without a URL (e.g., "net.minecraft:launchwrapper" bundled with MC)
        let Some(url) = lib.download_url() else {
            continue;
        };
        let path = lib.lib_path().to_string_lossy().into_owned();
        if seen.contains(&path) {
            continue;
        }
        let sha1 = lib
            .checksums
            .as_ref()
            .and_then(|c| c.first().cloned())
            .unwrap_or_default();
        let dest = staging_libs.join(&path);
        items.push(DownloadItemSpec::new(url, dest, &lib.name).with_hash(sha1));
        seen.insert(path);
    }
    Ok(())
}

fn commit_and_cleanup(staging_dir: &Path, shared_dir: &Path) -> Result<(), AquaError> {
    info!("Committing Forge installation from staging to shared...");

    // Move version.json
    let staging_versions = staging_dir.join("versions");
    if staging_versions.exists() {
        let dest = shared_dir.join("versions");
        copy_dir_recursive(&staging_versions, &dest)?;
    }

    // Merge libraries
    let staging_libs = staging_dir.join("libraries");
    if staging_libs.exists() {
        let dest = shared_dir.join("libraries");
        copy_dir_recursive(&staging_libs, &dest)?;
    }

    info!("Forge installation committed successfully");

    // Cleanup temp dir (parent of staging)
    if let Some(temp_dir) = staging_dir.parent()
        && temp_dir.exists()
    {
        std::fs::remove_dir_all(temp_dir)
            .map_err(|e| AquaError::ForgeExtract(format!("Failed to cleanup temp: {e}")))?;
        info!("Cleaned up temp dir: {:?}", temp_dir);
    }

    Ok(())
}

fn resolve_arg(
    arg: &str,
    data: &std::collections::HashMap<String, zellkern::DataEntry>,
    mc_jar: &Path,
    lib_dir: &Path,
    root: &Path,
) -> String {
    if let Some(key) = arg.strip_prefix('{').and_then(|s| s.strip_suffix('}')) {
        return match key {
            "MINECRAFT_JAR" => mc_jar.to_string_lossy().to_string(),
            "ROOT" | "MC_SUPPLIED_JARS_DIR" => root.to_string_lossy().to_string(),
            "LIBRARY_DIR" => lib_dir.to_string_lossy().to_string(),
            "SIDE" => "client".to_string(),
            _ => {
                if let Some(entry) = data.get(key)
                    && let Some(ref client_val) = entry.client
                {
                    return resolve_data_value(client_val, data, mc_jar, lib_dir, root);
                }
                arg.to_string()
            }
        };
    }

    if let Some(coord) = arg.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
        return lib_dir
            .join(maven_to_path(coord))
            .to_string_lossy()
            .to_string();
    }

    if let Some(lit) = arg.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')) {
        return lit.to_string();
    }

    if let Some(rest) = arg.strip_prefix('/') {
        return root.join(rest).to_string_lossy().to_string();
    }

    arg.to_string()
}

fn resolve_data_value(
    val: &serde_json::Value,
    data: &std::collections::HashMap<String, zellkern::DataEntry>,
    mc_jar: &Path,
    lib_dir: &Path,
    root: &Path,
) -> String {
    match val {
        serde_json::Value::String(s) => {
            let resolved =
                if let Some(coord) = s.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
                    lib_dir
                        .join(maven_to_path(coord))
                        .to_string_lossy()
                        .to_string()
                } else if let Some(lit) = s.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')) {
                    lit.to_string()
                } else if let Some(rest) = s.strip_prefix('/') {
                    root.join(rest).to_string_lossy().to_string()
                } else {
                    s.clone()
                };
            resolve_var_refs(&resolved, data, mc_jar, lib_dir, root)
        }
        _ => val.to_string(),
    }
}

fn resolve_var_refs(
    s: &str,
    data: &std::collections::HashMap<String, zellkern::DataEntry>,
    mc_jar: &Path,
    lib_dir: &Path,
    root: &Path,
) -> String {
    let mut result = s.to_string();
    while let Some(start) = result.find('{') {
        if let Some(end) = result[start..].find('}') {
            let key = &result[start + 1..start + end];
            let replacement = match key {
                "MINECRAFT_JAR" => mc_jar.to_string_lossy().to_string(),
                "ROOT" | "MC_SUPPLIED_JARS_DIR" => root.to_string_lossy().to_string(),
                "LIBRARY_DIR" => lib_dir.to_string_lossy().to_string(),
                "SIDE" => "client".to_string(),
                _ => {
                    if let Some(entry) = data.get(key)
                        && let Some(ref client_val) = entry.client
                    {
                        resolve_data_value(client_val, data, mc_jar, lib_dir, root)
                    } else {
                        result[start..=start + end].to_string()
                    }
                }
            };
            result.replace_range(start..=start + end, &replacement);
        } else {
            break;
        }
    }
    result
}

fn copy_dir_recursive(from: &Path, to: &Path) -> Result<(), AquaError> {
    for entry in std::fs::read_dir(from)
        .map_err(|e| AquaError::ForgeExtract(format!("Cannot read dir: {e}")))?
    {
        let entry =
            entry.map_err(|e| AquaError::ForgeExtract(format!("Cannot read dir entry: {e}")))?;
        let src = entry.path();
        let dst = to.join(entry.file_name());

        if src.is_dir() {
            std::fs::create_dir_all(&dst)?;
            copy_dir_recursive(&src, &dst)?;
        } else {
            if let Some(parent) = dst.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(&src, &dst)?;
        }
    }
    Ok(())
}
