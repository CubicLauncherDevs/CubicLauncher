use super::*;

#[test]
fn recognizes_only_official_ely_servers() {
    for server in [
        "ely.by",
        "https://ely.by/",
        "https://www.ely.by/",
        "https://authserver.ely.by/",
        "http://authserver.ely.by/",
        "https://AUTHSERVER.ELY.BY/",
        "https://account.ely.by/api/authlib-injector",
    ] {
        assert!(is_ely_server(server), "{server}");
    }
    for server in [
        "",
        "https://littlesk.in/",
        "https://authserver.ely.by.example.com/",
        "https://ely.by@example.com/",
        "https://example.com/ely.by",
        "https://authserver.ely.by:8080/",
        "ftp://ely.by/",
    ] {
        assert!(!is_ely_server(server), "{server}");
    }
}

fn png(width: u32, height: u32) -> Vec<u8> {
    let image = image::RgbaImage::from_pixel(width, height, image::Rgba([30, 80, 120, 255]));
    let mut bytes = Cursor::new(Vec::new());
    image.write_to(&mut bytes, image::ImageFormat::Png).unwrap();
    bytes.into_inner()
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
fn rejects_invalid_images_before_preview() {
    for (width, height) in [(32, 32), (64, 48), (128, 128), (64, 128)] {
        assert!(skin_data_url(&png(width, height)).is_err());
    }
    assert!(skin_data_url(b"<html>Service unavailable</html>").is_err());
    let bytes = png(64, 64);
    assert!(skin_data_url(&bytes[..bytes.len() / 2]).is_err());
}

#[test]
fn reads_optional_model_from_ely_response() {
    let classic: Textures =
        serde_json::from_str(r#"{"SKIN":{"url":"http://ely.by/storage/skins/test.png"}}"#).unwrap();
    assert!(classic.skin.unwrap().metadata.model.is_none());
    let slim: Textures = serde_json::from_str(
        r#"{"SKIN":{"url":"http://ely.by/storage/skins/test.png","metadata":{"model":"slim"}}}"#,
    )
    .unwrap();
    assert_eq!(slim.skin.unwrap().metadata.model.as_deref(), Some("slim"));
    assert!(
        serde_json::from_str::<Textures>("{}")
            .unwrap()
            .skin
            .is_none()
    );
}
