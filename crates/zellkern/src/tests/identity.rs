use super::*;

#[test]
fn offline_uuid_matches_minecraft_java_vectors() {
    for (name, expected) in [
        ("Steve", "5627dd98-e6be-3c21-b8a8-e92344183641"),
        ("Alex", "36532b5e-c442-3dbb-a24c-c7e55d0f979a"),
        ("Notch", "b50ad385-829d-3141-a216-7e7d7539ba7f"),
    ] {
        assert_eq!(offline_uuid(name).to_string(), expected);
        assert_eq!(offline_uuid(name), offline_uuid(name));
    }
    assert_ne!(offline_uuid("Steve"), offline_uuid("steve"));
    assert_ne!(offline_uuid("Steve"), offline_uuid("Alex"));
}
