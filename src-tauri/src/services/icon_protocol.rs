//! `cubic-icon://` serves addon icons as raw bytes so they never travel as
//! base64 inside an IPC payload. The WebView caches each icon by URL; the
//! revision query parameter only exists to invalidate that cache when the
//! underlying file changes, and is never trusted by the handler.

use super::addon_manager::AddonManager;
use super::instance_manager::InstanceManager;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use tauri::http::{Request, Response, StatusCode, Uri, header};

pub(crate) const SCHEME: &str = "cubic-icon";

// Mirrors Tauri's own custom-protocol origins: Windows and Android serve
// custom schemes through `http://<scheme>.localhost`.
#[cfg(any(windows, target_os = "android"))]
const ORIGIN: &str = "http://cubic-icon.localhost";
#[cfg(not(any(windows, target_os = "android")))]
const ORIGIN: &str = "cubic-icon://localhost";

const KINDS: [&str; 3] = ["mods", "resourcepacks", "shaderpacks"];

/// URL for one addon icon. The filename travels in the query string so it can
/// be percent-encoded without touching the path segments.
pub(crate) fn url(instance_id: &str, kind: &str, filename: &str, revision: Option<&str>) -> String {
    let mut url = format!(
        "{}/{}/{}?file={}",
        ORIGIN,
        instance_id,
        kind,
        urlencoding::encode(filename)
    );
    if let Some(revision) = revision {
        url.push_str("&rev=");
        url.push_str(&urlencoding::encode(revision));
    }
    url
}

/// Resolves one protocol request. Icons are read with the same parsers used by
/// the listings, so the on-disk cache is shared instead of duplicated.
pub(crate) async fn respond(request: Request<Vec<u8>>) -> Response<Vec<u8>> {
    let Some((instance_id, kind, filename)) = parse(request.uri()) else {
        return not_found();
    };
    let Some(handle) = InstanceManager::get().get_handle(&instance_id).await else {
        return not_found();
    };
    let path = handle.get_instance_dir().await.join(kind).join(&filename);
    let icon = tokio::task::spawn_blocking(move || match kind {
        "mods" => AddonManager::get_mod_icon(&path),
        "resourcepacks" => AddonManager::get_resourcepack_info_full(&path).1,
        _ => AddonManager::get_shaderpack_info_full(&path).1,
    })
    .await
    .ok()
    .flatten();
    let Some(icon) = icon else {
        return not_found();
    };
    let Some((mime, bytes)) = decode_data_url(&icon) else {
        return not_found();
    };
    ok(mime, bytes)
}

fn parse(uri: &Uri) -> Option<(String, &'static str, String)> {
    let mut segments = uri.path().trim_start_matches('/').split('/');
    let instance_id = segments.next()?.to_string();
    let kind = segments.next()?;
    if segments.next().is_some() {
        return None;
    }
    let kind = KINDS.iter().copied().find(|candidate| *candidate == kind)?;
    if uuid::Uuid::parse_str(&instance_id).is_err() {
        return None;
    }
    let query = uri.query()?;
    let filename = url::form_urlencoded::parse(query.as_bytes())
        .find(|(key, _)| key == "file")
        .map(|(_, value)| value.into_owned())?;
    crate::core::validate_filename(&filename).ok()?;
    Some((instance_id, kind, filename))
}

/// The addon cache stores `data:<mime>;base64,<payload>` so listings keep a
/// single representation. Only the transport changes here.
fn decode_data_url(data_url: &str) -> Option<(&str, Vec<u8>)> {
    let rest = data_url.strip_prefix("data:")?;
    let (mime, encoded) = rest.split_once(";base64,")?;
    Some((mime, STANDARD.decode(encoded).ok()?))
}

fn ok(mime: &str, bytes: Vec<u8>) -> Response<Vec<u8>> {
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime)
        .header(header::CACHE_CONTROL, "public, max-age=31536000, immutable")
        .header(header::CONTENT_LENGTH, bytes.len().to_string())
        .body(bytes)
        .unwrap_or_else(|_| not_found())
}

fn not_found() -> Response<Vec<u8>> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .header(header::CACHE_CONTROL, "no-store")
        .body(Vec::new())
        .unwrap_or_else(|_| Response::new(Vec::new()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls_round_trip_through_the_request_parser() {
        let url = url(
            "550e8400-e29b-41d4-a716-446655440000",
            "mods",
            "Mi mod #1.jar",
            Some("42"),
        );
        let uri: Uri = url.parse().unwrap();
        let parsed = parse(&uri).unwrap();
        assert_eq!(parsed.0, "550e8400-e29b-41d4-a716-446655440000");
        assert_eq!(parsed.1, "mods");
        assert_eq!(parsed.2, "Mi mod #1.jar");
    }

    #[test]
    fn rejects_unknown_kinds_traversal_and_non_uuid_instances() {
        let traversal: Uri =
            "cubic-icon://localhost/550e8400-e29b-41d4-a716-446655440000/mods?file=..%2Fsecret"
                .parse()
                .unwrap();
        assert!(parse(&traversal).is_none());

        let kind: Uri =
            "cubic-icon://localhost/550e8400-e29b-41d4-a716-446655440000/saves?file=a.jar"
                .parse()
                .unwrap();
        assert!(parse(&kind).is_none());

        let instance: Uri = "cubic-icon://localhost/not-a-uuid/mods?file=a.jar"
            .parse()
            .unwrap();
        assert!(parse(&instance).is_none());
    }

    #[test]
    fn decodes_the_cached_data_url() {
        assert_eq!(
            decode_data_url("data:image/png;base64,aGVsbG8="),
            Some(("image/png", b"hello".to_vec()))
        );
        assert!(decode_data_url("data:image/png,not-base64").is_none());
    }
}
