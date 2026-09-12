use super::*;

fn input(name: &str, address: &str, resource_policy: ResourcePolicy) -> ServerInput {
    ServerInput {
        name: name.into(),
        address: address.into(),
        resource_policy,
    }
}

fn add(dir: &Path, name: &str) -> ServerList {
    let list = list(dir).unwrap();
    apply(
        dir,
        &list.revision,
        ServerAction::Add {
            server: input(name, "localhost:25565", ResourcePolicy::Prompt),
        },
    )
    .unwrap()
}

#[test]
fn server_list_roundtrip_order_duplicates_and_resource_policy() {
    let dir = tempfile::tempdir().unwrap();
    assert!(list(dir.path()).unwrap().servers.is_empty());
    assert!(!dir.path().join("servers.dat").exists());
    add(dir.path(), "Servidor 🧊");
    let current = add(dir.path(), "Otro");
    let before = fs::read(dir.path().join("servers.dat")).unwrap();
    assert!(before.starts_with(&[10, 0, 0]));
    let moved = apply(
        dir.path(),
        &current.revision,
        ServerAction::Move {
            index: 1,
            direction: MoveDirection::Up,
        },
    )
    .unwrap();
    assert_eq!(moved.servers[0].name, "Otro");
    assert_eq!(moved.servers[1].name, "Servidor 🧊");
    assert_eq!(
        fs::read(dir.path().join("servers.dat_old")).unwrap(),
        before
    );
    for policy in [
        ResourcePolicy::Enabled,
        ResourcePolicy::Disabled,
        ResourcePolicy::Prompt,
    ] {
        let current = list(dir.path()).unwrap();
        apply(
            dir.path(),
            &current.revision,
            ServerAction::Edit {
                index: 0,
                server: input("Editado", "localhost:25565", policy),
            },
        )
        .unwrap();
        assert_eq!(list(dir.path()).unwrap().servers[0].resource_policy, policy);
    }
    let current = list(dir.path()).unwrap();
    let deleted = apply(
        dir.path(),
        &current.revision,
        ServerAction::Delete { index: 0 },
    )
    .unwrap();
    assert_eq!(deleted.servers.len(), 1);
    assert_eq!(deleted.servers[0].name, "Servidor 🧊");
}

#[test]
fn edits_preserve_unknown_root_and_entry_tags_and_same_address_icon() {
    let dir = tempfile::tempdir().unwrap();
    add(dir.path(), "Original");
    let path = dir.path().join("servers.dat");
    let mut root = parse(Some(&fs::read(&path).unwrap())).unwrap();
    root.insert("mod-data".into(), Value::Long(9223372036854775800));
    let Some(Value::List(entries)) = root.get_mut("servers") else {
        panic!()
    };
    let Value::Compound(entry) = &mut entries[0] else {
        panic!()
    };
    entry.insert("hidden".into(), Value::Byte(1));
    entry.insert("icon".into(), Value::String("invalid-but-preserved".into()));
    fs::write(&path, fastnbt::to_bytes(&root).unwrap()).unwrap();
    let current = list(dir.path()).unwrap();
    assert!(current.servers[0].has_icon);
    assert!(
        icons(dir.path(), &current.revision, vec![0]).unwrap()[0]
            .icon
            .is_none()
    );
    apply(
        dir.path(),
        &current.revision,
        ServerAction::Edit {
            index: 0,
            server: input("Renamed", "localhost:25565", ResourcePolicy::Prompt),
        },
    )
    .unwrap();
    let out = parse(Some(&fs::read(&path).unwrap())).unwrap();
    assert_eq!(out["mod-data"], root["mod-data"]);
    let Value::List(entries) = &out["servers"] else {
        panic!()
    };
    let Value::Compound(entry) = &entries[0] else {
        panic!()
    };
    assert_eq!(entry["hidden"], Value::Byte(1));
    assert_eq!(entry["icon"], Value::String("invalid-but-preserved".into()));
    let current = list(dir.path()).unwrap();
    apply(
        dir.path(),
        &current.revision,
        ServerAction::Edit {
            index: 0,
            server: input("Renamed", "other.example", ResourcePolicy::Prompt),
        },
    )
    .unwrap();
    let out = parse(Some(&fs::read(&path).unwrap())).unwrap();
    let Value::List(entries) = &out["servers"] else {
        panic!()
    };
    let Value::Compound(entry) = &entries[0] else {
        panic!()
    };
    assert!(!entry.contains_key("icon"));
}

#[test]
fn stale_revision_and_invalid_mutations_never_replace_the_list() {
    let dir = tempfile::tempdir().unwrap();
    let stale = add(dir.path(), "First");
    let current = add(dir.path(), "Second");
    let path = dir.path().join("servers.dat");
    let bytes = fs::read(&path).unwrap();
    assert!(
        apply(
            dir.path(),
            &stale.revision,
            ServerAction::Delete { index: 0 }
        )
        .unwrap_err()
        .contains("SERVERS_CONFLICT")
    );
    for action in [
        ServerAction::Delete { index: 99 },
        ServerAction::Move {
            index: 0,
            direction: MoveDirection::Up,
        },
        ServerAction::Move {
            index: usize::MAX,
            direction: MoveDirection::Down,
        },
        ServerAction::Add {
            server: input("", "localhost", ResourcePolicy::Prompt),
        },
        ServerAction::Add {
            server: input("test", "http://example.com", ResourcePolicy::Prompt),
        },
    ] {
        assert!(apply(dir.path(), &current.revision, action).is_err());
    }
    assert_eq!(fs::read(path).unwrap(), bytes);
}

#[test]
fn corrupt_or_compressed_file_is_not_treated_as_an_empty_list() {
    let dir = tempfile::tempdir().unwrap();
    for bytes in [
        b"corrupt".to_vec(),
        vec![31, 139, 8],
        fastnbt::to_bytes(&HashMap::from([(
            "servers",
            Value::String("wrong type".into()),
        )]))
        .unwrap(),
    ] {
        let path = dir.path().join("servers.dat");
        fs::write(&path, &bytes).unwrap();
        assert!(list(dir.path()).is_err());
        assert!(
            apply(
                dir.path(),
                &revision(Some(&bytes)),
                ServerAction::Delete { index: 0 }
            )
            .is_err()
        );
        assert_eq!(fs::read(path).unwrap(), bytes);
    }
}

#[test]
fn file_size_and_file_type_are_checked() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("servers.dat");
    fs::File::create(&path)
        .unwrap()
        .set_len(MAX_FILE_BYTES + 1)
        .unwrap();
    assert!(list(dir.path()).is_err());
    fs::remove_file(&path).unwrap();
    fs::create_dir(&path).unwrap();
    assert!(list(dir.path()).is_err());
}

#[cfg(unix)]
#[test]
fn refuses_symlinked_server_list() {
    let dir = tempfile::tempdir().unwrap();
    let external = tempfile::NamedTempFile::new().unwrap();
    std::os::unix::fs::symlink(external.path(), dir.path().join("servers.dat")).unwrap();
    assert!(list(dir.path()).is_err());
}

#[test]
fn validates_png_bytes_and_dimensions() {
    let png = |size| {
        let mut out = std::io::Cursor::new(Vec::new());
        image::DynamicImage::new_rgba8(size, size)
            .write_to(&mut out, image::ImageFormat::Png)
            .unwrap();
        STANDARD.encode(out.into_inner())
    };
    assert!(
        icon_url(&png(64))
            .unwrap()
            .starts_with("data:image/png;base64,")
    );
    assert!(icon_url(&png(65)).is_none());
    assert!(icon_url(&STANDARD.encode("not png")).is_none());
    assert!(icon_url("data:image/svg+xml;base64,PHN2Zz4=").is_none());
}

#[test]
fn metadata_borrows_icons_and_only_requested_icons_are_decoded() {
    let dir = tempfile::tempdir().unwrap();
    let mut png = std::io::Cursor::new(Vec::new());
    image::DynamicImage::new_rgba8(32, 32)
        .write_to(&mut png, image::ImageFormat::Png)
        .unwrap();
    let encoded = STANDARD.encode(png.into_inner());
    let entries = (0..100)
        .map(|index| {
            Value::Compound(HashMap::from([
                ("name".into(), Value::String(format!("Server {index}"))),
                ("ip".into(), Value::String("localhost".into())),
                ("icon".into(), Value::String(encoded.clone())),
            ]))
        })
        .collect();
    let bytes = fastnbt::to_bytes(&HashMap::from([("servers", Value::List(entries))])).unwrap();
    fs::write(dir.path().join("servers.dat"), &bytes).unwrap();
    let borrowed = read_metadata(Some(&bytes)).unwrap();
    assert!(matches!(borrowed.servers[0].icon, Cow::Borrowed(_)));
    metrics::reset();
    let list = list(dir.path()).unwrap();
    assert_eq!(metrics::counts(), (1, 0));
    assert!(list.servers.iter().all(|s| s.has_icon));
    let requested = icons(dir.path(), &list.revision, vec![99, 5, 5]).unwrap();
    assert_eq!(
        requested.iter().map(|i| i.index).collect::<Vec<_>>(),
        [5, 99]
    );
    assert!(
        requested
            .iter()
            .all(|i| i.icon.as_ref().unwrap().ends_with(&encoded))
    );
    assert_eq!(metrics::counts(), (2, 2));
    assert!(
        icons(dir.path(), "stale", vec![0])
            .unwrap_err()
            .contains("SERVERS_CONFLICT")
    );
    assert!(icons(dir.path(), &list.revision, vec![100]).is_err());
    metrics::reset();
    assert!(icons(dir.path(), &list.revision, (0..52).collect()).is_err());
    assert_eq!(metrics::counts(), (0, 0));
}

#[test]
fn streaming_revision_matches_full_buffer_and_missing_files() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("servers.dat");
    assert_eq!(current_revision(&path).unwrap(), "missing");
    for length in [0, 32 * 1024 - 1, 32 * 1024, 32 * 1024 + 1, 200_000] {
        let bytes = vec![42; length];
        fs::write(&path, &bytes).unwrap();
        assert_eq!(current_revision(&path).unwrap(), revision(Some(&bytes)));
    }
}
