use uuid::Uuid;

/// Minecraft's offline identity: Java's UUID.nameUUIDFromBytes applied to the
/// UTF-8 bytes of "OfflinePlayer:<name>" (case-sensitive, without a namespace).
pub fn offline_uuid(username: &str) -> Uuid {
    let digest = md5::compute(format!("OfflinePlayer:{username}").as_bytes());
    uuid::Builder::from_md5_bytes(digest.0).into_uuid()
}

#[cfg(test)]
#[path = "tests/identity.rs"]
mod tests;
