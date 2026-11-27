use super::*;

const HASH: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const MISSING_HASH: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

struct Fixture {
    _temp: tempfile::TempDir,
    versions: PathBuf,
    shared: PathBuf,
    instance: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let shared = temp.path().join("shared");
        let instance = temp.path().join("instance");
        std::fs::create_dir_all(&instance).unwrap();
        let object = shared.join("assets").join("objects").join(&HASH[..2]);
        std::fs::create_dir_all(&object).unwrap();
        std::fs::write(object.join(HASH), b"asset").unwrap();
        Self {
            _temp: temp,
            versions: shared.join("versions"),
            shared,
            instance,
        }
    }

    fn write_index(&self, id: &str, objects: serde_json::Value, flags: serde_json::Value) {
        let mut index = serde_json::json!({"objects": objects});
        for (key, flag) in flags.as_object().unwrap() {
            index[key] = flag.clone();
        }
        let dir = self.shared.join("assets").join("indexes");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(format!("{id}.json")), index.to_string()).unwrap();
    }

    fn write_version(&self, id: &str, value: serde_json::Value) {
        let dir = self.shared.join("versions").join(id);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(format!("{id}.json")), value.to_string()).unwrap();
    }

    fn prepare(&self, manifest: serde_json::Value) -> Result<Option<PathBuf>, String> {
        prepare(&self.disk_manifest(manifest), &self.shared, &self.instance)
    }

    /// Write a version file the way the game manifest stores it and parse it
    /// back, so `prepare` receives the same shape as the launcher.
    fn disk_manifest(&self, mut value: serde_json::Value) -> VersionManifest {
        let id = value["id"].as_str().unwrap().to_string();
        let mut file = serde_json::Map::new();
        for (key, entry) in value.as_object_mut().unwrap().iter_mut() {
            let entry = std::mem::take(entry);
            file.insert(camel_case(key), entry);
        }
        let path = self.versions.join(&id).join(format!("{id}.json"));
        self.write_version(&id, serde_json::Value::Object(file));
        VersionManifest::from_file(&path).unwrap()
    }
}

fn camel_case(key: &str) -> String {
    match key {
        "asset_index" => "assetIndex".into(),
        "minecraft_arguments" => "minecraftArguments".into(),
        "inherits_from" => "inheritsFrom".into(),
        other => other.to_string(),
    }
}

fn single_object(path: &str) -> serde_json::Value {
    serde_json::json!({ path: {"hash": HASH, "size": 5} })
}

#[test]
fn virtual_index_is_unpacked_under_the_shared_virtual_root() {
    let fixture = Fixture::new();
    fixture.write_index(
        "legacy",
        single_object("minecraft/sounds/step.ogg"),
        serde_json::json!({"virtual": true}),
    );

    let assets = fixture
        .prepare(serde_json::json!({
            "id": "1.7.2",
            "assetIndex": {"id": "legacy"},
            "minecraftArguments": "--username ${auth_player_name}"
        }))
        .unwrap()
        .unwrap();

    assert_eq!(
        assets,
        fixture.shared.join("assets").join("virtual").join("legacy")
    );
    assert_eq!(
        std::fs::read(assets.join("minecraft/sounds/step.ogg")).unwrap(),
        b"asset"
    );
    assert!(!fixture.instance.join("resources").exists());
}

#[test]
fn pre_1_6_versions_map_assets_to_instance_resources() {
    let fixture = Fixture::new();
    fixture.write_index(
        "pre-1.6",
        single_object("sound/step.ogg"),
        serde_json::json!({}),
    );

    let assets = fixture
        .prepare(serde_json::json!({
            "id": "1.5.2",
            "assetIndex": {"id": "pre-1.6"},
            "minecraftArguments": "--username ${auth_player_name}"
        }))
        .unwrap()
        .unwrap();

    assert_eq!(assets, fixture.instance.join("resources"));
    assert_eq!(
        std::fs::read(assets.join("sound/step.ogg")).unwrap(),
        b"asset"
    );
    assert!(!fixture.shared.join("assets/virtual").exists());
}

#[test]
fn map_to_resources_flag_wins_for_newer_versions() {
    let fixture = Fixture::new();
    fixture.write_index(
        "pre-1.6",
        single_object("sound/step.ogg"),
        serde_json::json!({"map_to_resources": true}),
    );

    let assets = fixture
        .prepare(serde_json::json!({
            "id": "1.7.2",
            "assetIndex": {"id": "pre-1.6"},
            "minecraftArguments": "x"
        }))
        .unwrap()
        .unwrap();

    assert_eq!(assets, fixture.instance.join("resources"));
}

#[test]
fn inheritance_is_resolved_before_choosing_the_layout() {
    let fixture = Fixture::new();
    fixture.write_index(
        "pre-1.6",
        single_object("sound/step.ogg"),
        serde_json::json!({}),
    );
    fixture.write_version(
        "1.5.2",
        serde_json::json!({
            "id": "1.5.2",
            "assetIndex": {"id": "pre-1.6"},
            "minecraftArguments": "--username ${auth_player_name}"
        }),
    );

    let assets = fixture
        .prepare(serde_json::json!({
            "id": "1.5.2-forge-7.8.1.738",
            "inheritsFrom": "1.5.2"
        }))
        .unwrap()
        .unwrap();

    assert_eq!(assets, fixture.instance.join("resources"));
    assert!(assets.join("sound/step.ogg").is_file());
}

#[test]
fn marker_makes_repeated_preparation_a_no_op() {
    let fixture = Fixture::new();
    fixture.write_index(
        "pre-1.6",
        single_object("sound/step.ogg"),
        serde_json::json!({}),
    );
    let manifest = serde_json::json!({
        "id": "1.5.2",
        "assetIndex": {"id": "pre-1.6"},
        "minecraftArguments": "x"
    });

    let assets = fixture.prepare(manifest.clone()).unwrap().unwrap();
    // Files written after the extraction must survive the second pass.
    std::fs::remove_file(assets.join("sound/step.ogg")).unwrap();
    std::fs::write(assets.join("local.txt"), b"local").unwrap();

    assert_eq!(fixture.prepare(manifest).unwrap().unwrap(), assets);
    assert!(!assets.join("sound/step.ogg").exists());
    assert!(assets.join("local.txt").is_file());
}

#[test]
fn missing_object_reports_the_asset_and_leaves_no_marker() {
    let fixture = Fixture::new();
    fixture.write_index(
        "pre-1.6",
        serde_json::json!({"sound/missing.ogg": {"hash": MISSING_HASH, "size": 5}}),
        serde_json::json!({}),
    );

    let error = fixture
        .prepare(serde_json::json!({
            "id": "1.5.2",
            "assetIndex": {"id": "pre-1.6"},
            "minecraftArguments": "x"
        }))
        .unwrap_err();

    assert!(error.contains(MISSING_HASH), "{error}");
    let entries = std::fs::read_dir(fixture.instance.join("resources"))
        .expect("resources directory must exist")
        .count();
    assert_eq!(entries, 0);
}

#[test]
fn marker_stays_out_of_the_directory_the_game_scans() {
    let fixture = Fixture::new();
    fixture.write_index(
        "pre-1.6",
        single_object("pack.mcmeta"),
        serde_json::json!({}),
    );
    let manifest = serde_json::json!({
        "id": "1.5.2",
        "assetIndex": {"id": "pre-1.6"},
        "minecraftArguments": "x"
    });

    let assets = fixture.prepare(manifest).unwrap().unwrap();

    // El juego recorre `resources` como resource pack: solo debe haber assets.
    let entries: Vec<String> = std::fs::read_dir(&assets)
        .unwrap()
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(entries, ["pack.mcmeta"], "{entries:?}");
    let markers: Vec<String> = std::fs::read_dir(fixture.shared.join("assets"))
        .unwrap()
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.starts_with(".cubic-legacy-"))
        .collect();
    assert_eq!(markers.len(), 1, "{markers:?}");
}

#[test]
fn modern_versions_need_no_extraction() {
    let fixture = Fixture::new();
    fixture.write_index("test", single_object("a/b"), serde_json::json!({}));

    let manifest = fixture.disk_manifest(serde_json::json!({
        "id": "1.20.1",
        "assetIndex": {"id": "test"},
        "arguments": {"game": []}
    }));
    let result = prepare(&manifest, &fixture.shared, &fixture.instance);

    assert_eq!(result.unwrap(), None);
    assert!(!fixture.instance.join("resources").exists());
    assert!(!fixture.shared.join("assets/virtual").exists());
}
