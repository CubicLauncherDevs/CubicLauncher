use super::*;
use std::fs;

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("cubic_jre_test_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&path).unwrap();
        // macOS exposes the temp dir through the /var -> /private/var symlink;
        // canonicalize once so read_dir entries and canonicalized expectations match.
        Self(path.canonicalize().unwrap())
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn test_find_runtime_root_top_level() {
    let tmp = TempDir::new();
    let root = tmp.path().join("jre21");
    fs::create_dir_all(root.join("bin")).unwrap();
    fs::write(root.join("bin").join("java"), "").unwrap();

    assert_eq!(
        find_runtime_root(tmp.path(), "java"),
        Some(root.canonicalize().unwrap())
    );
}

#[test]
fn test_find_runtime_root_nested() {
    let tmp = TempDir::new();
    // Simulates archive layout where the runtime is nested one level deep.
    let root = tmp.path().join("zulu21").join("jre");
    fs::create_dir_all(root.join("bin")).unwrap();
    fs::write(root.join("bin").join("java"), "").unwrap();

    assert_eq!(
        find_runtime_root(tmp.path(), "java"),
        Some(root.canonicalize().unwrap())
    );
}

#[test]
fn test_find_runtime_root_shallowest_wins() {
    let tmp = TempDir::new();
    let shallow = tmp.path().join("shallow");
    let deep = tmp.path().join("deep").join("nested");
    fs::create_dir_all(shallow.join("bin")).unwrap();
    fs::create_dir_all(deep.join("bin")).unwrap();
    fs::write(shallow.join("bin").join("java"), "").unwrap();
    fs::write(deep.join("bin").join("java"), "").unwrap();

    assert_eq!(
        find_runtime_root(tmp.path(), "java"),
        Some(shallow.canonicalize().unwrap())
    );
}

#[test]
fn test_find_runtime_root_missing_binary() {
    let tmp = TempDir::new();
    fs::create_dir_all(tmp.path().join("empty")).unwrap();
    assert_eq!(find_runtime_root(tmp.path(), "java"), None);
}
