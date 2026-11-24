use super::*;

fn modrinth_version(
    id: &str,
    _date: &str,
    deps: Vec<(&str, &str, Option<&str>)>,
) -> ModrinthVersionBrief {
    ModrinthVersionBrief {
        id: id.to_string(),
        game_versions: vec!["1.20.1".into()],
        loaders: vec!["fabric".into()],
        files: vec![ModrinthFileBrief {
            url: format!("https://cdn.test/{id}.jar"),
            filename: format!("{id}.jar"),
            primary: true,
        }],
        dependencies: deps
            .into_iter()
            .map(|(kind, project, version)| ModrinthDependencyBrief {
                dependency_type: kind.to_string(),
                project_id: Some(project.to_string()),
                version_id: version.map(str::to_string),
            })
            .collect(),
    }
}

fn candidates(versions: Vec<ModrinthVersionBrief>) -> ModrinthCandidates {
    ModrinthCandidates {
        compatible: versions,
        referenced: HashMap::new(),
    }
}

#[test]
fn dependency_kind_from_modrinth() {
    assert_eq!(
        DependencyKind::from_modrinth("required"),
        DependencyKind::Required
    );
    assert_eq!(
        DependencyKind::from_modrinth("optional"),
        DependencyKind::Optional
    );
    assert_eq!(
        DependencyKind::from_modrinth("embedded"),
        DependencyKind::Embedded
    );
    assert_eq!(
        DependencyKind::from_modrinth("incompatible"),
        DependencyKind::Incompatible
    );
    assert_eq!(
        DependencyKind::from_modrinth("unknown"),
        DependencyKind::Required
    );
    assert_eq!(
        DependencyKind::from_modrinth("OPTIONAL"),
        DependencyKind::Optional
    );
}

#[test]
fn dependency_kind_from_curseforge_relation() {
    assert_eq!(
        DependencyKind::from_curseforge_relation(1),
        Some(DependencyKind::Embedded)
    );
    assert_eq!(
        DependencyKind::from_curseforge_relation(2),
        Some(DependencyKind::Optional)
    );
    assert_eq!(
        DependencyKind::from_curseforge_relation(3),
        Some(DependencyKind::Required)
    );
    assert_eq!(
        DependencyKind::from_curseforge_relation(5),
        Some(DependencyKind::Incompatible)
    );
    assert_eq!(DependencyKind::from_curseforge_relation(99), None);
}

#[test]
fn select_modrinth_without_constraints_picks_the_newest_compatible() {
    let candidates = candidates(vec![
        modrinth_version("v2", "2026-02-01", vec![]),
        modrinth_version("v1", "2026-01-01", vec![]),
    ]);
    let selected = select_modrinth(&[], &candidates).expect("version");
    assert_eq!(selected.id, "v2");
}

#[test]
fn select_modrinth_honours_a_single_pinned_version() {
    let mut candidates = candidates(vec![
        modrinth_version("v2", "2026-02-01", vec![]),
        modrinth_version("v1", "2026-01-01", vec![]),
    ]);
    candidates
        .referenced
        .insert("v0".into(), modrinth_version("v0", "2025-12-01", vec![]));
    let constraints = vec![(0, "v1".to_string())];
    let selected = select_modrinth(&constraints, &candidates).expect("version");
    assert_eq!(selected.id, "v1");
}

#[test]
fn select_modrinth_returns_the_pinned_referenced_version_when_incompatible() {
    let mut candidates = candidates(vec![modrinth_version("v2", "2026-02-01", vec![])]);
    candidates
        .referenced
        .insert("v0".into(), modrinth_version("v0", "2025-12-01", vec![]));
    let constraints = vec![(0, "v0".to_string())];
    let selected = select_modrinth(&constraints, &candidates).expect("version");
    assert_eq!(selected.id, "v0");
    assert!(!candidates.is_compatible(selected));
}

#[test]
fn select_modrinth_with_agreeing_parents_prefers_that_version() {
    let candidates = candidates(vec![
        modrinth_version("v2", "2026-02-01", vec![]),
        modrinth_version("v1", "2026-01-01", vec![]),
    ]);
    let constraints = vec![(0, "v1".to_string()), (1, "v1".to_string())];
    let selected = select_modrinth(&constraints, &candidates).expect("version");
    assert_eq!(selected.id, "v1");
}

#[test]
fn select_modrinth_with_conflicting_parents_falls_back_to_newest() {
    let candidates = candidates(vec![
        modrinth_version("v2", "2026-02-01", vec![]),
        modrinth_version("v1", "2026-01-01", vec![]),
    ]);
    let constraints = vec![(0, "v1".to_string()), (1, "v2".to_string())];
    let selected = select_modrinth(&constraints, &candidates).expect("version");
    assert_eq!(selected.id, "v2");
}

#[test]
fn unique_constraint_ids_ignores_duplicates_and_empty_pins() {
    let constraints = vec![
        (0, "v1".to_string()),
        (1, "v1".to_string()),
        (2, String::new()),
    ];
    assert_eq!(unique_constraint_ids(&constraints), vec!["v1".to_string()]);
}

#[test]
fn minecraft_version_labels_are_separated_from_loaders() {
    assert!(is_minecraft_version_label("1.20.1"));
    assert!(is_minecraft_version_label("1.7.10"));
    assert!(!is_minecraft_version_label("fabric"));
    assert!(!is_minecraft_version_label("1.20.1-forge"));
}

#[test]
fn curseforge_loaders_and_versions_are_split() {
    let file = CurseForgeFile {
        id: 1,
        game_id: None,
        mod_id: Some(7),
        is_available: true,
        display_name: "Mod".into(),
        file_name: "mod.jar".into(),
        release_type: 1,
        file_status: 4,
        hashes: vec![],
        file_date: "2026-01-01T00:00:00Z".into(),
        file_length: 10,
        download_count: 0,
        download_url: None,
        game_versions: vec!["1.20.1".into(), "Fabric".into()],
        sortable_game_versions: None,
        dependencies: vec![],
        alternate_file_id: 0,
        is_server_pack: false,
        file_fingerprint: 0,
        modules: None,
    };
    assert_eq!(
        curseforge_player_versions(&file),
        vec!["1.20.1".to_string()]
    );
    assert_eq!(curseforge_file_loaders(&file), vec!["fabric".to_string()]);
}

#[test]
fn collector_merges_repeated_dependencies() {
    let mut collector = Collector::default();
    let (first, new) = collector.remember(
        DependencySource::Modrinth,
        "fabric-api",
        NodeRole::Optional,
        2,
    );
    assert!(new);
    let (second, new) =
        collector.remember(DependencySource::Modrinth, "fabric-api", NodeRole::Hard, 1);
    assert!(!new);
    assert_eq!(first, second);
    assert_eq!(collector.nodes[second].role, NodeRole::Hard);
    assert_eq!(collector.nodes[second].depth, 1);
    assert_eq!(collector.nodes.len(), 1);
}

#[test]
fn collector_flags_parents_requesting_different_versions() {
    let mut collector = Collector::default();
    collector.modrinth.insert(
        "lib".into(),
        candidates(vec![
            modrinth_version("v2", "2026-02-01", vec![]),
            modrinth_version("v1", "2026-01-01", vec![]),
        ]),
    );
    let (parent_a, _) = collector.remember(DependencySource::Modrinth, "a", NodeRole::Hard, 0);
    let (parent_b, _) = collector.remember(DependencySource::Modrinth, "b", NodeRole::Hard, 0);
    let (lib, _) = collector.remember(DependencySource::Modrinth, "lib", NodeRole::Hard, 1);
    collector.add_constraint(lib, parent_a, Some("v1"));
    collector.add_constraint(lib, parent_b, Some("v2"));

    collector.solve(lib);

    assert_eq!(collector.states[lib].selected.as_deref(), Some("v2"));
    assert_eq!(collector.conflicts.len(), 1);
    let conflict = &collector.conflicts[0];
    assert_eq!(conflict.kind, ConflictType::VersionMismatch);
    assert_eq!(conflict.project_id, "lib");
    assert_eq!(conflict.requested_by.len(), 2);
    assert_eq!(conflict.requested_versions.len(), 2);
}

#[test]
fn collector_reuses_the_conflict_when_solved_again() {
    let mut collector = Collector::default();
    collector.modrinth.insert(
        "lib".into(),
        candidates(vec![modrinth_version("v1", "2026-01-01", vec![])]),
    );
    let (parent_a, _) = collector.remember(DependencySource::Modrinth, "a", NodeRole::Hard, 0);
    let (parent_b, _) = collector.remember(DependencySource::Modrinth, "b", NodeRole::Hard, 0);
    let (lib, _) = collector.remember(DependencySource::Modrinth, "lib", NodeRole::Hard, 1);
    collector.add_constraint(lib, parent_a, Some("v1"));
    collector.add_constraint(lib, parent_b, Some("v9"));

    collector.solve(lib);
    collector.solve(lib);

    assert_eq!(collector.conflicts.len(), 1);
}

#[test]
fn collector_clears_the_conflict_when_parents_agree() {
    let mut collector = Collector::default();
    collector.modrinth.insert(
        "lib".into(),
        candidates(vec![modrinth_version("v1", "2026-01-01", vec![])]),
    );
    let (parent_a, _) = collector.remember(DependencySource::Modrinth, "a", NodeRole::Hard, 0);
    let (parent_b, _) = collector.remember(DependencySource::Modrinth, "b", NodeRole::Hard, 0);
    let (lib, _) = collector.remember(DependencySource::Modrinth, "lib", NodeRole::Hard, 1);
    collector.add_constraint(lib, parent_a, Some("v1"));
    collector.add_constraint(lib, parent_b, Some("v2"));
    collector.solve(lib);
    assert_eq!(collector.conflicts.len(), 1);

    collector.states[lib]
        .constraints
        .retain(|(parent, _)| *parent != parent_b);
    collector.add_constraint(lib, parent_b, Some("v1"));
    collector.solve(lib);

    assert!(collector.conflicts.is_empty());
}

#[test]
fn collector_marks_required_project_without_compatible_version_as_unresolved() {
    let mut collector = Collector::default();
    collector
        .modrinth
        .insert("lib".into(), candidates(Vec::new()));
    let (parent, _) = collector.remember(DependencySource::Modrinth, "a", NodeRole::Hard, 0);
    let (lib, _) = collector.remember(DependencySource::Modrinth, "lib", NodeRole::Hard, 1);
    collector.add_constraint(lib, parent, None);

    collector.solve(lib);

    assert!(collector.states[lib].selected.is_none());
    assert!(collector.states[lib].unresolved_reason.is_some());
    assert_eq!(collector.conflicts[0].kind, ConflictType::Unresolved);
}

#[test]
fn collector_does_not_flag_optional_nodes() {
    let mut collector = Collector::default();
    let (optional, _) =
        collector.remember(DependencySource::Modrinth, "extras", NodeRole::Optional, 1);
    collector.solve(optional);
    assert!(collector.conflicts.is_empty());
}

#[test]
fn build_node_carries_version_metadata_and_children() {
    let mut collector = Collector::default();
    collector.modrinth.insert(
        "lib".into(),
        candidates(vec![modrinth_version("v1", "2026-01-01", vec![])]),
    );
    let (lib, _) = collector.remember(DependencySource::Modrinth, "lib", NodeRole::Hard, 1);
    collector.states[lib].selected = Some("v1".into());
    collector.modrinth_projects.insert(
        "lib".into(),
        ModrinthProjectBrief {
            title: "Library".into(),
            icon_url: None,
        },
    );
    collector.nodes[lib].title = Some("Library".into());
    let (child, _) = collector.remember(DependencySource::Modrinth, "child", NodeRole::Optional, 2);
    collector.push_child(lib, child);

    let node = collector.build_node(lib);

    assert_eq!(node.version_id.as_deref(), Some("v1"));
    assert_eq!(node.filename, "v1.jar");
    assert_eq!(node.game_versions, vec!["1.20.1".to_string()]);
    assert_eq!(node.loaders, vec!["fabric".to_string()]);
    assert_eq!(node.children.len(), 1);
    assert_eq!(node.children[0].kind, DependencyKind::Optional);
}

#[test]
fn listed_nodes_use_the_resolved_project_title_instead_of_the_id() {
    let mut collector = Collector::default();
    let (parent, _) = collector.remember(DependencySource::Modrinth, "root", NodeRole::Hard, 0);
    let (optional, _) = collector.remember(
        DependencySource::Modrinth,
        "EE4xvUHj",
        NodeRole::Optional,
        1,
    );
    collector.add_constraint(optional, parent, None);
    collector.modrinth_projects.insert(
        "EE4xvUHj".into(),
        ModrinthProjectBrief {
            title: "Fabric API".into(),
            icon_url: Some("https://cdn.test/fabric.png".into()),
        },
    );
    // describe_node() copies both fields from the cached project.
    collector.nodes[optional].title = Some("Fabric API".into());
    collector.nodes[optional].icon_url = Some("https://cdn.test/fabric.png".into());

    let node = collector.build_node(optional);

    assert_eq!(node.title, "Fabric API");
    assert_eq!(node.project_id, "EE4xvUHj");
    assert_eq!(
        node.icon_url.as_deref(),
        Some("https://cdn.test/fabric.png")
    );
}

#[test]
fn incompatibles_are_reported_separately_from_conflicts() {
    let mut collector = Collector::default();
    let (parent, _) = collector.remember(DependencySource::Modrinth, "a", NodeRole::Hard, 0);
    let (bad, _) = collector.remember(
        DependencySource::Modrinth,
        "optifine",
        NodeRole::Incompatible,
        1,
    );
    collector.add_constraint(bad, parent, None);

    let incompatible = collector.incompatible_projects();

    assert_eq!(incompatible.len(), 1);
    assert_eq!(incompatible[0].project_id, "optifine");
    assert!(collector.conflicts.is_empty());
}
