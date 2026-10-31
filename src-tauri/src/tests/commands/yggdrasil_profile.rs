use super::*;

fn profile_json(textures: &str) -> Vec<u8> {
    let encoded = general_purpose::STANDARD.encode(textures);
    serde_json::json!({
        "id": "069a79f444e94726a5befca90e38aaf5",
        "name": "Notch",
        "properties": [{ "name": "textures", "value": encoded }],
    })
    .to_string()
    .into_bytes()
}

fn png(width: u32, height: u32) -> Vec<u8> {
    let image = image::RgbaImage::from_pixel(width, height, image::Rgba([30, 80, 120, 255]));
    let mut bytes = Cursor::new(Vec::new());
    image.write_to(&mut bytes, image::ImageFormat::Png).unwrap();
    bytes.into_inner()
}

#[test]
fn reads_skin_url_and_model_from_textures() {
    let classic = profile_json(
        r#"{"textures":{"SKIN":{"url":"https://auth.cubiclauncher.org/textures/abc"}}}"#,
    );
    assert_eq!(
        parse_skin(&classic).unwrap(),
        Some((
            "https://auth.cubiclauncher.org/textures/abc".to_string(),
            "classic"
        ))
    );

    let slim = profile_json(
        r#"{"textures":{"SKIN":{"url":"https://auth.cubiclauncher.org/textures/def","metadata":{"model":"slim"}}}}"#,
    );
    assert_eq!(
        parse_skin(&slim).unwrap(),
        Some((
            "https://auth.cubiclauncher.org/textures/def".to_string(),
            "slim"
        ))
    );
}

#[test]
fn profiles_without_skin_are_not_an_error() {
    let empty = profile_json(r#"{"textures":{}}"#);
    assert_eq!(parse_skin(&empty).unwrap(), None);

    let no_properties = br#"{"id":"069a79f444e94726a5befca90e38aaf5","name":"Notch"}"#;
    assert_eq!(parse_skin(no_properties).unwrap(), None);

    let cape_only = profile_json(
        r#"{"textures":{"CAPE":{"url":"https://auth.cubiclauncher.org/textures/cape"}}}"#,
    );
    assert_eq!(parse_skin(&cape_only).unwrap(), None);
}

#[test]
fn rejects_malformed_profiles() {
    assert!(parse_skin(b"<html>Not found</html>").is_err());

    let invalid_base64 = serde_json::json!({
        "properties": [{ "name": "textures", "value": "no-es-base64!" }],
    })
    .to_string();
    assert!(parse_skin(invalid_base64.as_bytes()).is_err());

    let invalid_json = profile_json("no es json");
    assert!(parse_skin(&invalid_json).is_err());
}

#[test]
fn accepts_legacy_and_modern_skins_without_changing_pixels() {
    for height in [32, 64] {
        let bytes = png(64, height);
        let data = skin_data_url(&bytes).unwrap();
        let encoded = data.strip_prefix("data:image/png;base64,").unwrap();
        assert_eq!(general_purpose::STANDARD.decode(encoded).unwrap(), bytes);
    }
}

#[test]
fn classic_model_is_sent_empty_like_the_web() {
    assert_eq!(model_field("slim"), "slim");
    assert_eq!(model_field("classic"), "");
    assert_eq!(model_field(""), "");
}

#[test]
fn only_mosaicary_pngs_come_from_the_catalog() {
    for url in [
        "https://mosaicary.com/skins/abc.png",
        "https://mosaicary.com/skins/abc.png?for=view",
        "https://www.mosaicary.com/skins/abc.png",
    ] {
        assert!(is_catalog_url(url), "{url}");
    }
    for url in [
        "",
        "http://mosaicary.com/skins/abc.png",
        "https://mosaicary.com.example.com/skins/abc.png",
        "https://mosaicary.com.evil.com/skins/abc.png",
        "https://mosaicary.com:8443/skins/abc.png",
        "https://mosaicary.com@evil.com/skins/abc.png",
        "file:///etc/passwd",
        "https://auth.cubiclauncher.org/textures/abc",
    ] {
        assert!(!is_catalog_url(url), "{url}");
    }
}

#[test]
fn rejects_skins_the_viewer_cannot_draw() {
    for (width, height) in [(32, 32), (64, 48), (128, 128), (64, 128)] {
        assert!(
            skin_data_url(&png(width, height)).is_err(),
            "{width}x{height}"
        );
    }
    assert!(skin_data_url(b"<html>Service unavailable</html>").is_err());
    let bytes = png(64, 64);
    assert!(skin_data_url(&bytes[..bytes.len() / 2]).is_err());
}
