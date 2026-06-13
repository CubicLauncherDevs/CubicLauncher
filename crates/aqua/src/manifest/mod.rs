use crate::types::{
    AssetMeta, Downloadable, LibraryFile, MOJANG_MANIFEST_URL, NormalizedArguments,
    NormalizedVersion, VersionAssets,
};
use crate::utilities::HTTP_CLIENT;
use crate::ProtonError;
use zellkern::VersionManifest;
use serde::Deserialize;

#[derive(Deserialize)]
struct ManifestV2 {
    versions: Vec<ManifestEntry>,
}

#[derive(Debug, Deserialize)]
struct ManifestEntry {
    id: String,
    url: String,
    #[serde(rename = "type")]
    #[allow(dead_code)]
    version_type: String,
}

async fn fetch_manifest_v2() -> Result<ManifestV2, ProtonError> {
    let resp = HTTP_CLIENT
        .get(MOJANG_MANIFEST_URL)
        .send()
        .await?
        .json::<ManifestV2>()
        .await?;
    Ok(resp)
}

async fn fetch_version_json(url: &str) -> Result<VersionManifest, ProtonError> {
    let bytes = HTTP_CLIENT.get(url).send().await?.bytes().await?;
    Ok(VersionManifest::from_bytes(&bytes)?)
}

pub async fn resolve_version_data(version_id: &str) -> Result<NormalizedVersion, ProtonError> {
    let manifest = fetch_manifest_v2().await?;

    let entry = manifest
        .versions
        .into_iter()
        .find(|v| v.id == version_id)
        .ok_or_else(|| ProtonError::VersionNotFound(version_id.to_string()))?;

    let version = fetch_version_json(&entry.url).await?;
    resolve_normalized(version)
}

fn resolve_normalized(version: VersionManifest) -> Result<NormalizedVersion, ProtonError> {
    let main_class = version
        .main_class
        .clone()
        .ok_or(ProtonError::MainClassNotFound)?;

    let asset_index = version
        .asset_index
        .as_ref()
        .ok_or_else(|| ProtonError::Other("No asset index in manifest".into()))?;

    let asset_index_url = asset_index
        .url
        .as_ref()
        .ok_or_else(|| ProtonError::Other("No asset index URL".into()))?;

    let asset_index_sha1 = asset_index
        .sha1
        .as_ref()
        .ok_or_else(|| ProtonError::Other("No asset index SHA-1".into()))?;

    // Resolve client/server JARs from downloads field
    let client_jar = version
        .downloads
        .as_ref()
        .map(|d| Downloadable {
            url: d.client.url.clone(),
            sha1: d.client.sha1.clone(),
            size: d.client.size,
        })
        .unwrap_or_else(|| Downloadable {
            url: String::new(),
            sha1: String::new(),
            size: 0,
        });

    let server_jar = version.downloads.as_ref().and_then(|d| {
        d.server.as_ref().map(|s| Downloadable {
            url: s.url.clone(),
            sha1: s.sha1.clone(),
            size: s.size,
        })
    });

    let mut libraries: Vec<LibraryFile> = Vec::new();
    let mut natives: Vec<LibraryFile> = Vec::new();
    let mut arguments = NormalizedArguments {
        game: Vec::new(),
        jvm: Vec::new(),
    };

    // Process libraries
    if let Some(lib_list) = &version.libraries {
        for lib in lib_list {
            if !lib.should_include() {
                continue;
            }

            // Check for legacy native (natives field + classifiers)
            if let Some(native_artifact) = lib.native_artifact() {
                if !lib.is_correct_arch() {
                    continue;
                }
                let path = native_artifact.path.clone();
                let url = native_artifact.url.clone().unwrap_or_default();
                let sha1 = native_artifact.sha1.clone().unwrap_or_default();
                let size = native_artifact.size.unwrap_or(0);
                natives.push(LibraryFile {
                    name: lib.name.clone(),
                    url,
                    sha1,
                    size,
                    path,
                });
                continue;
            }

            // Check for modern native (Maven coordinate has :natives-)
            if lib.is_native() {
                if !lib.is_correct_arch() {
                    continue;
                }
                if let Some(artifact) = lib.downloads.as_ref().and_then(|d| d.artifact.as_ref()) {
                    let path = artifact.path.clone();
                    let url = artifact.url.clone().unwrap_or_default();
                    let sha1 = artifact.sha1.clone().unwrap_or_default();
                    let size = artifact.size.unwrap_or(0);
                    natives.push(LibraryFile {
                        name: lib.name.clone(),
                        url,
                        sha1,
                        size,
                        path,
                    });
                }
                continue;
            }

            // Regular library
            if let Some(artifact) = lib.downloads.as_ref().and_then(|d| d.artifact.as_ref()) {
                libraries.push(LibraryFile {
                    name: lib.name.clone(),
                    url: artifact.url.clone().unwrap_or_default(),
                    sha1: artifact.sha1.clone().unwrap_or_default(),
                    size: artifact.size.unwrap_or(0),
                    path: artifact.path.clone(),
                });
            }
        }
    }

    // Process arguments
    if let Some(args) = &version.arguments {
        if let Some(game_args) = &args.game {
            for arg in game_args {
                arguments.game.extend(arg.get_if_applies());
            }
        }

        if let Some(jvm_args) = &args.jvm {
            for arg in jvm_args {
                let tokens = arg.get_if_applies();
                if tokens.iter().any(|t| t.contains("${classpath}")) {
                    continue;
                }
                arguments.jvm.extend(tokens);
            }
        }

        if let Some(default_args) = &args.default_user_jvm {
            for arg in default_args {
                let tokens = arg.get_if_applies();
                arguments.jvm.extend(tokens);
            }
        }
    } else if let Some(legacy) = &version.minecraft_arguments {
        for token in legacy.split_whitespace() {
            arguments.game.push(token.to_string());
        }
        arguments
            .jvm
            .push("-Djava.library.path=${natives_directory}".into());
        arguments.jvm.push("-cp".into());
        arguments.jvm.push("${classpath}".into());
    }

    Ok(NormalizedVersion {
        id: version.id_raw.clone(),
        parsed_version: version.id,
        release_time: String::new(),
        java_version: version.java_major_version(),
        main_class,
        client_jar,
        server_jar,
        asset_index: AssetMeta {
            id: asset_index.id.clone(),
            url: asset_index_url.clone(),
            sha1: asset_index_sha1.clone(),
            size: asset_index.size.unwrap_or(0),
        },
        libraries,
        natives,
        arguments,
    })
}

pub async fn resolve_asset_index(
    version: &NormalizedVersion,
) -> Result<VersionAssets, ProtonError> {
    let res = HTTP_CLIENT
        .get(&version.asset_index.url)
        .send()
        .await?
        .json::<VersionAssets>()
        .await?;
    Ok(res)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{HashMap, HashSet};

    async fn fetch_manifest_for_test() -> ManifestV2 {
        fetch_manifest_v2().await.unwrap()
    }

    async fn fetch_and_parse_version(url: &str) -> VersionManifest {
        let bytes = HTTP_CLIENT
            .get(url)
            .send()
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap();
        VersionManifest::from_bytes(&bytes).unwrap()
    }

    #[tokio::test]
    async fn download_and_parse_manifests_from_all_release_types() {
        let manifest = fetch_manifest_for_test().await;

        let mut by_type: HashMap<String, Vec<&ManifestEntry>> = HashMap::new();
        for entry in &manifest.versions {
            by_type
                .entry(entry.version_type.clone())
                .or_default()
                .push(entry);
        }

        let interesting_types = ["release", "snapshot", "old_alpha", "old_beta"];
        let mut tested: HashSet<&str> = HashSet::new();

        for entry in &manifest.versions {
            if !interesting_types.contains(&&entry.version_type.as_str()) {
                continue;
            }
            if tested.contains(entry.version_type.as_str()) {
                continue;
            }

            let version = fetch_and_parse_version(&entry.url).await;

            assert!(
                !version.id_raw.is_empty(),
                "id_raw empty for type {}",
                entry.version_type
            );
            assert!(
                version.main_class.is_some(),
                "main_class missing for {} ({})",
                entry.id,
                entry.version_type
            );
            assert!(
                version.downloads.is_some(),
                "downloads missing for {} ({})",
                entry.id,
                entry.version_type
            );
            if let Some(dl) = &version.downloads {
                assert!(
                    !dl.client.url.is_empty(),
                    "client url empty for {}",
                    entry.id
                );
            }
            assert!(
                version.asset_index.is_some(),
                "asset_index missing for {} ({})",
                entry.id,
                entry.version_type
            );

            tested.insert(entry.version_type.as_str());
        }

        assert_eq!(
            tested.len(),
            4,
            "Not all release types were tested: {:?}",
            tested
        );
    }

    #[tokio::test]
    async fn resolve_normalized_from_each_release_type() {
        let manifest = fetch_manifest_for_test().await;

        let interesting_types = ["release", "snapshot", "old_alpha", "old_beta"];
        let mut tested: HashSet<&str> = HashSet::new();

        for entry in &manifest.versions {
            if !interesting_types.contains(&&entry.version_type.as_str()) {
                continue;
            }
            if tested.contains(entry.version_type.as_str()) {
                continue;
            }

            let version = fetch_and_parse_version(&entry.url).await;
            let normalized = resolve_normalized(version).unwrap();

            assert!(!normalized.id.is_empty(), "normalized id empty");
            assert!(!normalized.main_class.is_empty(), "main_class empty");
            assert!(
                normalized.java_version >= 6,
                "java_version too low: {}",
                normalized.java_version
            );

            tested.insert(entry.version_type.as_str());
        }

        assert_eq!(
            tested.len(),
            4,
            "Not all release types were tested: {:?}",
            tested
        );
    }
}
