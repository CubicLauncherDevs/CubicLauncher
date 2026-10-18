use std::future::Future;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::pin::Pin;

use serde_json::{Value, json};

use super::batch::{DownloadBatch, DownloadItemSpec};
use crate::AquaError;
use crate::optifine::{OptiFineVersion, resolve_download_url};
use crate::progress::{DownloadProgress, DownloadStage, ProgressSender};
use crate::utilities::{compute_sha1_sync, download_file, run_java_process};

pub struct OptiFineBatch {
    version: OptiFineVersion,
    shared_dir: PathBuf,
    staging_dir: PathBuf,
    java_path: PathBuf,
    items: Vec<DownloadItemSpec>,
}

/// Elige la URL del instalador: BMCLAPI cuando está activo (evita depender del
/// servidor oficial de OptiFine), o el flujo normal de resolución en caso contrario.
async fn resolve_installer_url(version: &OptiFineVersion) -> Result<String, AquaError> {
    if crate::provider::download_provider() == crate::provider::DownloadProvider::Bmclapi
        && let Some(url) = bmclapi_optifine_url(version)
    {
        return Ok(url);
    }
    resolve_download_url(version).await
}

/// Construye la URL del espejo BMCLAPI: `/optifine/{mcversion}/{type}/{patch}`.
fn bmclapi_optifine_url(version: &OptiFineVersion) -> Option<String> {
    if version.filename.starts_with("preview_") {
        return None;
    }
    let (kind, patch) = version.optifine_version.rsplit_once('_')?;
    if kind.is_empty() || patch.is_empty() {
        return None;
    }
    Some(format!(
        "{}/optifine/{}/{}/{}",
        crate::provider::BMCLAPI_ROOT,
        version.game_version,
        kind,
        patch
    ))
}

impl OptiFineBatch {
    pub async fn new(
        shared_dir: &Path,
        version: OptiFineVersion,
        java_path: PathBuf,
    ) -> Result<Self, AquaError> {
        // Revalidate public input before constructing filesystem paths.
        let parsed = OptiFineVersion::from_filename(&version.filename)
            .ok_or_else(|| AquaError::Other("Invalid OptiFine filename".into()))?;
        if parsed.version_id != version.version_id
            || parsed.game_version != version.game_version
            || parsed.optifine_version != version.optifine_version
        {
            return Err(AquaError::Other("Inconsistent OptiFine version".into()));
        }
        let staging_dir = shared_dir
            .join("temp")
            .join(format!("optifine-{}", uuid::Uuid::new_v4()));
        let url = resolve_installer_url(&version).await?;
        let items = vec![
            DownloadItemSpec::new(url, staging_dir.join("installer.jar"), "OptiFine installer")
                .with_stage(DownloadStage::Library),
        ];
        Ok(Self {
            version,
            shared_dir: shared_dir.into(),
            staging_dir,
            java_path,
            items,
        })
    }

    async fn install(&self, progress: Option<ProgressSender>) -> Result<(), AquaError> {
        if let Some(tx) = &progress {
            let _ = tx.send(DownloadProgress {
                stage: DownloadStage::Processing,
                current_item: Some("OptiFine Patcher".into()),
                ..DownloadProgress::empty(1)
            });
        }
        let installer = self.staging_dir.join("installer.jar");
        let archive_path = installer.clone();
        let wrapper = tokio::task::spawn_blocking(move || read_wrapper(&archive_path)).await??;
        let (wrapper_name, wrapper_path) = if let Some((version, bytes)) = wrapper {
            let path =
                format!("optifine/launchwrapper-of/{version}/launchwrapper-of-{version}.jar");
            let dest = self.shared_dir.join("libraries").join(&path);
            tokio::fs::create_dir_all(dest.parent().unwrap()).await?;
            tokio::fs::write(dest, bytes).await?;
            (format!("optifine:launchwrapper-of:{version}"), path)
        } else {
            let path = "net/minecraft/launchwrapper/1.12/launchwrapper-1.12.jar".to_string();
            let dest = self.shared_dir.join("libraries").join(&path);
            tokio::fs::create_dir_all(dest.parent().unwrap()).await?;
            download_file(
                &format!("https://libraries.minecraft.net/{path}"),
                &dest,
                "111e7bea9c968cdb3d06ef4632bf7ff0824d0f36",
                None,
                None,
            )
            .await?;
            ("net.minecraft:launchwrapper:1.12".into(), path)
        };

        let mc = &self.version.game_version;
        let base_dir = self.shared_dir.join("versions").join(mc);
        let patched = self.staging_dir.join("patched.jar");
        run_java_process(
            &self.java_path,
            &installer.to_string_lossy(),
            "optifine.Patcher",
            vec![
                base_dir
                    .join(format!("{mc}.jar"))
                    .to_string_lossy()
                    .into_owned(),
                installer.to_string_lossy().into_owned(),
                patched.to_string_lossy().into_owned(),
            ],
            "OptiFine Patcher",
        )
        .await?;

        // The patcher in some releases exits successfully even when it did not produce a JAR.
        let patched_check = patched.clone();
        tokio::task::spawn_blocking(move || validate_jar(&patched_check)).await??;
        let release = format!("{mc}_{}", self.version.optifine_version);
        let lib_path = format!("optifine/OptiFine/{release}/OptiFine-{release}.jar");
        let library = self.shared_dir.join("libraries").join(&lib_path);
        tokio::fs::create_dir_all(library.parent().unwrap()).await?;
        tokio::fs::copy(&patched, &library).await?;

        let base: Value =
            serde_json::from_slice(&tokio::fs::read(base_dir.join(format!("{mc}.json"))).await?)
                .map_err(|e| AquaError::Other(e.to_string()))?;
        let shared = self.shared_dir.clone();
        let libraries = tokio::task::spawn_blocking(move || -> Result<Vec<Value>, AquaError> {
            Ok(vec![
                local_library(&shared, &format!("optifine:OptiFine:{release}"), &lib_path)?,
                local_library(&shared, &wrapper_name, &wrapper_path)?,
            ])
        })
        .await??;
        let profile = make_profile(&self.version, &base, libraries);
        let id = &self.version.version_id;
        let version_dir = self.shared_dir.join("versions").join(id);
        tokio::fs::create_dir_all(&version_dir).await?;
        let pending = version_dir.join(format!("{id}.json.tmp"));
        tokio::fs::write(
            &pending,
            serde_json::to_vec_pretty(&profile).map_err(|e| AquaError::Other(e.to_string()))?,
        )
        .await?;
        tokio::fs::rename(pending, version_dir.join(format!("{id}.json"))).await?;
        Ok(())
    }
}

fn validate_jar(path: &Path) -> Result<(), AquaError> {
    let jar = zip::ZipArchive::new(std::fs::File::open(path)?)
        .map_err(|e| AquaError::Other(format!("Invalid OptiFine JAR: {e}")))?;
    if jar.is_empty() {
        return Err(AquaError::Other("OptiFine produced an empty JAR".into()));
    }
    Ok(())
}

fn read_wrapper(path: &Path) -> Result<Option<(String, Vec<u8>)>, AquaError> {
    let mut jar = zip::ZipArchive::new(std::fs::File::open(path)?)
        .map_err(|e| AquaError::Other(format!("Invalid OptiFine installer: {e}")))?;
    jar.by_name("optifine/Patcher.class").map_err(|_| {
        AquaError::Other("This OptiFine release does not include a standalone installer".into())
    })?;
    let mut version = String::new();
    match jar.by_name("launchwrapper-of.txt") {
        Ok(mut entry) => {
            entry.read_to_string(&mut version)?;
        }
        Err(zip::result::ZipError::FileNotFound) => return Ok(None),
        Err(e) => return Err(AquaError::Other(e.to_string())),
    }
    let version = version.trim().to_string();
    if version.is_empty()
        || version.contains("..")
        || !version
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
    {
        return Err(AquaError::Other(
            "Invalid OptiFine launchwrapper version".into(),
        ));
    }
    let mut bytes = Vec::new();
    jar.by_name(&format!("launchwrapper-of-{version}.jar"))
        .map_err(|e| AquaError::Other(format!("Missing OptiFine launchwrapper: {e}")))?
        .read_to_end(&mut bytes)?;
    Ok(Some((version, bytes)))
}

fn local_library(shared: &Path, name: &str, path: &str) -> Result<Value, AquaError> {
    let file = shared.join("libraries").join(path);
    Ok(json!({ "name": name, "downloads": { "artifact": {
        "path": path, "sha1": compute_sha1_sync(&file)?, "size": std::fs::metadata(file)?.len(), "url": ""
    }}}))
}

fn make_profile(version: &OptiFineVersion, base: &Value, libraries: Vec<Value>) -> Value {
    let mut profile = json!({
        "id": version.version_id, "inheritsFrom": version.game_version,
        "mainClass": "net.minecraft.launchwrapper.Launch", "type": "release",
        "libraries": libraries,
        "arguments": { "game": ["--tweakClass", "optifine.OptiFineTweaker"], "jvm": [] }
    });
    if let Some(args) = base.get("minecraftArguments").and_then(Value::as_str) {
        // The launcher prefers modern arguments when both formats exist.
        // Keep the entire legacy command instead of shadowing it with only the tweaker.
        profile.as_object_mut().unwrap().remove("arguments");
        profile["minecraftArguments"] =
            json!(format!("{args} --tweakClass optifine.OptiFineTweaker"));
    }
    profile
}

impl DownloadBatch for OptiFineBatch {
    fn name(&self) -> String {
        self.version.version_id.clone()
    }
    fn items(&self) -> &[DownloadItemSpec] {
        &self.items
    }
    fn finalize(
        &self,
        progress: Option<ProgressSender>,
    ) -> Pin<Box<dyn Future<Output = Result<(), AquaError>> + Send + '_>> {
        Box::pin(async move {
            let result = self.install(progress).await;
            let _ = tokio::fs::remove_dir_all(&self.staging_dir).await;
            result
        })
    }
}

#[cfg(test)]
#[path = "../tests/optifine_install.rs"]
mod tests;
