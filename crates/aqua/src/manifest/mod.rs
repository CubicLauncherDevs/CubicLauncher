use crate::AquaError;
use crate::types::{
    AssetMeta, Downloadable, LibraryFile, MOJANG_MANIFEST_URL, NormalizedArguments,
    NormalizedVersion, VersionAssets,
};
use crate::utilities::HTTP_CLIENT;
use reqwest::Client;
use serde::Deserialize;
use zellkern::VersionManifest;

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

async fn fetch_manifest_v2(client: &Client) -> Result<ManifestV2, AquaError> {
    let resp = crate::utilities::get_with_fallback(client, MOJANG_MANIFEST_URL)
        .await?
        .json::<ManifestV2>()
        .await?;
    Ok(resp)
}

async fn fetch_version_json(
    client: &Client,
    url: &str,
) -> Result<(VersionManifest, Vec<u8>), AquaError> {
    let bytes = crate::utilities::get_with_fallback(client, url)
        .await?
        .bytes()
        .await?;
    let manifest = VersionManifest::from_bytes(&bytes)?;
    Ok((manifest, bytes.to_vec()))
}

pub async fn resolve_version_data(
    version_id: &str,
) -> Result<(NormalizedVersion, Vec<u8>), AquaError> {
    let manifest = fetch_manifest_v2(&HTTP_CLIENT).await?;

    let entry = manifest
        .versions
        .into_iter()
        .find(|v| v.id == version_id)
        .ok_or_else(|| AquaError::VersionNotFound(version_id.to_string()))?;

    let (version, raw_json) = fetch_version_json(&HTTP_CLIENT, &entry.url).await?;
    let normalized = resolve_normalized(version)?;
    Ok((normalized, raw_json))
}

fn resolve_normalized(version: VersionManifest) -> Result<NormalizedVersion, AquaError> {
    let main_class = version
        .main_class
        .clone()
        .ok_or(AquaError::MainClassNotFound)?;

    let asset_index = version
        .asset_index
        .as_ref()
        .ok_or_else(|| AquaError::Other("No asset index in manifest".into()))?;

    let asset_index_url = asset_index
        .url
        .as_ref()
        .ok_or_else(|| AquaError::Other("No asset index URL".into()))?;

    let asset_index_sha1 = asset_index
        .sha1
        .as_ref()
        .ok_or_else(|| AquaError::Other("No asset index SHA-1".into()))?;

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
    client: &Client,
    version: &NormalizedVersion,
) -> Result<(VersionAssets, Vec<u8>), AquaError> {
    let res = crate::utilities::get_with_fallback(client, &version.asset_index.url).await?;
    let bytes = res.bytes().await?;
    let assets: VersionAssets = serde_json::from_slice(&bytes)?;
    Ok((assets, bytes.to_vec()))
}

#[cfg(test)]
#[path = "../tests/manifest.rs"]
mod tests;
