use super::*;

#[test]
fn test_file_hasher_selects_algorithm_by_length() {
    // SHA-256 (64 caracteres) para los paquetes de JRE.
    let mut hasher = FileHasher::for_hash(&"0".repeat(64));
    hasher.update(b"abc");
    assert_eq!(
        hasher.finalize_hex(),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );

    // SHA-1 (40 caracteres) para los assets de Minecraft.
    let mut hasher = FileHasher::for_hash(&"0".repeat(40));
    hasher.update(b"abc");
    assert_eq!(
        hasher.finalize_hex(),
        "a9993e364706816aba3e25717850c26c9cd0d89d"
    );
}

#[test]
fn test_hex_encode() {
    assert_eq!(hex_encode(&[0xab, 0xcd]), "abcd");
    assert_eq!(hex_encode(&[0x00, 0xff]), "00ff");
}

#[cfg(feature = "extract-natives")]
#[test]
fn test_is_native_file() {
    assert!(is_native_file("liblwjgl.so"));
    assert!(is_native_file("opengl32.dll"));
    assert!(is_native_file("libglfw.dylib"));
    assert!(!is_native_file("META-INF/MANIFEST.MF"));
    assert!(!is_native_file("some/path/"));
}

#[test]
fn test_parse_java_major_version() {
    assert_eq!(
        parse_java_major_version("openjdk version \"21.0.11\" 2025-04-15"),
        Some(21)
    );
    assert_eq!(
        parse_java_major_version("openjdk version \"17.0.12\" 2024-07-16"),
        Some(17)
    );
    assert_eq!(
        parse_java_major_version("openjdk version \"1.8.0_412\" 2024-05-16"),
        Some(8)
    );
    assert_eq!(
        parse_java_major_version("java version \"1.7.0_80\" "),
        Some(7)
    );
    assert_eq!(parse_java_major_version("not a version"), None);
}

#[test]
fn test_java_runtime_preferences() {
    assert_eq!(java_runtime_preferences("1.16.5"), &[8, 17, 21]);
    assert_eq!(java_runtime_preferences("1.17.1"), &[17, 21, 8]);
    assert_eq!(java_runtime_preferences("1.20.4"), &[17, 21, 8]);
    assert_eq!(java_runtime_preferences("1.21"), &[21, 17, 8]);
    assert_eq!(java_runtime_preferences("1.21.4"), &[21, 17, 8]);
    assert_eq!(java_runtime_preferences("26.3-snapshot-2"), &[21, 17, 8]);
}

#[test]
fn test_infer_java_version() {
    assert_eq!(infer_java_version("1.16.5"), 8);
    assert_eq!(infer_java_version("1.17.1"), 17);
    assert_eq!(infer_java_version("1.21.4"), 21);
    assert_eq!(infer_java_version("26.3-snapshot-2"), 21);
}
