use super::crash::{self, CrashSource};
use super::rules;
use super::*;
use std::time::{Duration, SystemTime};

fn codes(analysis: &crash::CrashAnalysis) -> Vec<&str> {
    analysis
        .findings
        .iter()
        .map(|finding| finding.code.as_str())
        .collect()
}

// ── Reglas ──────────────────────────────────────────────────────────────────

#[test]
fn rule_needles_are_lowercase() {
    assert!(
        rules::needles_are_lowercase(),
        "las needles se comparan contra líneas ya normalizadas"
    );
}

#[test]
fn each_rule_detects_its_signal() {
    let cases: &[(&str, &str)] = &[
        (
            "java_unsupported_class",
            "java.lang.UnsupportedClassVersionError: net/minecraft/client/main/Main has been compiled by a more recent version",
        ),
        (
            "memory_out_of_memory",
            "java.lang.OutOfMemoryError: Java heap space",
        ),
        (
            "java_missing_main",
            "Error: Could not find or load main class net.minecraft.client.main.Main",
        ),
        (
            "mods_mixin_failed",
            "[ERROR] Mixin apply failed mixins.foo.json:bar -> baz",
        ),
        (
            "mods_duplicate",
            "Found duplicate mods: example-1.jar, example-2.jar",
        ),
        (
            "mods_missing_dependency",
            "Missing or unsupported mandatory dependencies: Mod ID: 'foo', Requested by: 'bar'",
        ),
        (
            "mods_incompatible",
            "java.lang.NoSuchMethodError: 'void net.minecraft.Foo.bar()'",
        ),
        (
            "mods_outdated",
            "[main/WARN]: Mixin apply failed mixin.foo.json:bar -> baz: NoSuchMethodError",
        ),
        (
            "mods_registry",
            "Failed to create registry entry minecraft:foo (item)",
        ),
        (
            "graphics_opengl",
            "[LWJGL] GLFW error 65542: WGL: The driver does not appear to support OpenGL",
        ),
        (
            "network_download",
            "java.net.UnknownHostException: libraries.minecraft.net",
        ),
        (
            "world_corrupt",
            "Chunk file at chunk file provided is corrupted (libdeflate)",
        ),
        ("auth_session", "Failed to verify username! Invalid session"),
        ("auth_skin", "[Render thread/WARN]: Failed to get profile"),
        (
            "graphics_shader_pack",
            "[Render thread/ERROR]: Failed to compile shader: shaders/water.vsh",
        ),
        (
            "java_heap_too_big",
            "Error occurred during initialization of VM\nCould not reserve enough space for 8388608KB object heap",
        ),
        (
            "mods_sodium_graphics",
            "Exception in thread \"Render thread\" java.lang.NoSuchMethodError: me.jellysquid.mods.sodium.Thing()",
        ),
        (
            "content_pack_invalid",
            "WARN Couldn't read pack at resourcepacks/broken.zip",
        ),
        (
            "java_missing_native_lib",
            "Exception in thread \"main\" java.lang.UnsatisfiedLinkError: /home/user/.cubic/shared/runtimes/jre8/lib/amd64/libawt_xawt.so: libXtst.so.6: no se puede abrir el fichero del objeto compartido: No existe el fichero o el directorio",
        ),
    ];

    for (code, line) in cases {
        // Un caso puede necesitar varias líneas para que la causa sea real.
        let lines: Vec<&str> = line.lines().collect();
        let findings = rules::analyze_lines(&lines);
        assert!(
            findings.iter().any(|finding| finding.code == *code),
            "la regla {code} no detectó: {line}"
        );
    }
}

#[test]
fn mod_native_link_errors_are_not_reported_as_missing_desktop_libraries() {
    let lines = [
        "java.lang.UnsatisfiedLinkError: no lwjgl in java.library.path",
        "java.lang.UnsatisfiedLinkError: org/example/Native.foo()V",
    ];
    let findings = rules::analyze_lines(&lines);
    assert!(
        findings
            .iter()
            .all(|finding| finding.code != "java_missing_native_lib")
    );
}

#[test]
fn normal_logs_produce_no_findings() {
    let lines = [
        "[12:00:00] [main/INFO]: Setting user: Steve",
        "[12:00:01] [Render thread/INFO]: Backend library: LWJGL 3.3.3",
        "[12:00:02] [Render thread/INFO]: OpenAL initialized on device",
        "[12:00:03] [Render thread/INFO]: Stopping!",
    ];
    assert!(rules::analyze_lines(&lines).is_empty());
}

fn detected_codes(lines: &[&str]) -> Vec<String> {
    rules::analyze_lines(lines)
        .into_iter()
        .map(|finding| finding.code)
        .collect()
}

#[test]
fn a_vanilla_world_does_not_crash_because_a_resource_pack_is_broken() {
    // El crash es de otro archivo: el pack roto solo genera un aviso.
    let lines = [
        "---- Minecraft Crash Report ----",
        "// I let you down. Sorry :(",
        "Description: Rendering overlay",
        "java.lang.NullPointerException: Cannot invoke \"Object.hashCode()\"",
        "[Render thread/WARN]: Couldn't read resourcepacks/broken.zip: not a valid pack",
        "Chunk file at minecraft:overworld is corrupt",
    ];
    let codes = detected_codes(&lines);
    assert!(codes.contains(&"content_pack_invalid".to_string()));
    assert!(
        !codes.contains(&"world_corrupt".to_string()),
        "la lectura del pack no corrompe el mundo: {codes:?}"
    );
}

#[test]
fn forge_error_handler_noise_is_not_read_as_mod_incompatibility() {
    let lines = [
        "[main/ERROR] [ModernFix/]: Detected possible mod incompatibility",
        "java.lang.NoSuchMethodError: net.minecraft.client.Foo.bar()",
    ];
    let codes = detected_codes(&lines);
    assert!(
        codes.contains(&"mods_incompatible".to_string()),
        "{codes:?}"
    );

    // Un fix de mods de Forge imprime la misma pila con su `[ErrorHandler]`.
    let lines_with_fix = [
        "Mixin apply failed mixins.core.json:core -> mixin.core",
        "[main/ERROR] [ErrorHandler]: ------------------------------------------------------------",
        "[main/ERROR] [ErrorHandler]: Mixin apply failed mixins.core.json:core -> mixin.core",
        "[main/ERROR] [ErrorHandler]: java.lang.NoClassDefFoundError: org/spongepowered/asm/launch/MixinLaunchPlugin",
    ];
    let codes = detected_codes(&lines_with_fix);
    assert!(codes.contains(&"mods_mixin_failed".to_string()));
    assert!(
        !codes.contains(&"mods_incompatible".to_string()),
        "el fix de mods captura el fallo; la causa concreta ya está: {codes:?}"
    );
}

#[test]
fn shader_stack_traces_do_not_blame_desktop_libraries() {
    let lines = [
        "Exception in thread \"Render thread\" java.lang.UnsatisfiedLinkError: Error looking up function 'glShaderSource': /usr/lib/libGL.so.1",
        "java.lang.ExceptionInInitializerError",
        "Caused by: unable to load native lwjgl library",
        "Caused by: org.lwjgl.LWJGLException: Stack trace unavailable (stack parse error)",
    ];
    let codes = detected_codes(&lines);
    assert!(
        !codes.contains(&"java_missing_native_lib".to_string()),
        "un StackParseError de un fix de shaders no es un JDK incompleto: {codes:?}"
    );
}

#[test]
fn vanilla_crashes_do_not_blame_mods() {
    let lines = [
        "[12:00:00] [main/INFO]: Setting user: Steve",
        "java.lang.NoSuchMethodError: net.minecraft.client.Foo.bar()",
        "java.util.concurrent.CompletionException: Mixin apply failed",
    ];
    let codes = detected_codes(&lines);
    for code in ["mods_incompatible", "mods_outdated"] {
        assert!(
            !codes.contains(&code.to_string()),
            "{code} no puede venir de un arranque vanilla: {codes:?}"
        );
    }
}

#[test]
fn fabric_reports_the_outdated_mod_instead_of_a_generic_incompatibility() {
    let lines = [
        "[12:00:00] [main/INFO]: Loading Minecraft 1.20.1 with Fabric Loader 0.15.0",
        "[12:00:01] [main/ERROR]: Mod resolution failed",
        "[12:00:01] [main/ERROR]: Mod 'sodium' (sodium) 0.5.0 requires version 0.5.2 or later of fabric-api",
        "[12:00:01] [main/ERROR]: java.lang.NoClassDefFoundError: net/fabricmc/api/EnvType",
    ];
    let codes = detected_codes(&lines);
    assert!(codes.contains(&"mods_outdated".to_string()), "{codes:?}");
    assert!(
        !codes.contains(&"mods_incompatible".to_string()),
        "{codes:?}"
    );
}

#[test]
fn sodium_removal_explains_vanilla_graphics_crashes() {
    let lines = [
        "- Minecraft 1.20.1 (com.mojang.authlib)",
        "[Render thread/WARN]: Failed to create window: GLFW error 65542",
    ];
    let codes = detected_codes(&lines);
    assert!(codes.contains(&"graphics_opengl".to_string()));
    assert!(!codes.contains(&"mods_sodium_graphics".to_string()));

    let with_sodium = [
        "[main/INFO]: Loading 87 mods:",
        "\tsodium 0.5.2",
        "[main/ERROR]: Mixins for sodium failed to apply",
    ];
    let codes = detected_codes(&with_sodium);
    assert!(
        codes.contains(&"mods_sodium_graphics".to_string()),
        "{codes:?}"
    );
}

#[test]
fn generic_mod_incompatibility_is_dropped_when_a_specific_cause_exists() {
    let lines = [
        "Mixin apply failed mixins.core.json:core -> mixin.core",
        "java.lang.NoClassDefFoundError: org/spongepowered/asm/launch/MixinLaunchPlugin",
    ];
    let codes: Vec<_> = rules::analyze_lines(&lines)
        .iter()
        .map(|finding| finding.code.clone())
        .collect();
    assert!(codes.contains(&"mods_mixin_failed".to_string()));
    assert!(!codes.contains(&"mods_incompatible".to_string()));
}

#[test]
fn evidence_is_limited_and_numbered() {
    let mut lines = vec!["[main/INFO]: Starting".to_string()];
    lines.extend((0..5).map(|_| "java.lang.OutOfMemoryError: Java heap space".to_string()));
    let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
    let findings = rules::analyze_lines(&refs);
    let oom = findings
        .iter()
        .find(|finding| finding.code == "memory_out_of_memory")
        .unwrap();
    assert_eq!(oom.evidence.len(), 3);
    assert_eq!(oom.evidence[0].line, 2);
}

#[test]
fn long_evidence_is_truncated() {
    let line = format!("java.lang.OutOfMemoryError: {}", "x".repeat(1000));
    let findings = rules::analyze_lines(&[line.as_str()]);
    let text = &findings[0].evidence[0].text;
    assert!(text.chars().count() <= 241);
    assert!(text.ends_with('…'));
}

// ── Fuentes y saneado ───────────────────────────────────────────────────────

fn instance_dir() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}

#[test]
fn analyzed_evidence_never_contains_credentials_or_private_data() {
    let dir = instance_dir();
    std::fs::create_dir_all(dir.path().join("crash-reports")).unwrap();
    std::fs::write(
        dir.path().join("crash-reports/crash-1.txt"),
        "Description: Unexpected error token=description-secret\n\
         java.lang.OutOfMemoryError: Java heap space token=super-secret-token\n\
         contact steve@example.com from 192.168.1.20\n\
         Authorization: Bearer abcdefghijklmnop\n",
    )
    .unwrap();
    let analysis = crash::analyze_report(dir.path(), "crash-1.txt").unwrap();
    let rendered = serde_json::to_string(&analysis).unwrap();
    assert!(!rendered.contains("description-secret"));
    assert!(!rendered.contains("super-secret-token"));
    assert!(!rendered.contains("steve@example.com"));
    assert!(!rendered.contains("192.168.1.20"));
    assert!(!rendered.contains("abcdefghijklmnop"));
    assert!(rendered.contains("***") || rendered.contains("<email>"));
}

#[test]
fn observed_exit_code_is_reported_when_the_source_has_none() {
    let dir = instance_dir();
    std::fs::create_dir_all(dir.path().join("logs")).unwrap();
    std::fs::write(
        dir.path().join("logs/latest.log"),
        "[12:00:00] [main/INFO]: Setting user: Steve\n\
         java.lang.OutOfMemoryError: Java heap space\n",
    )
    .unwrap();
    let analysis = crash::analysis_from_text(
        "[12:00:00] [main/INFO]: Setting user: Steve\n\
         java.lang.OutOfMemoryError: Java heap space\n",
        false,
        CrashSource::LatestLog,
        None,
        Some(1),
    );
    assert_eq!(analysis.exit_code, Some(1));
    assert_eq!(analysis.context.exit_code, Some(1));
}

#[test]
fn crash_report_exit_code_wins_over_the_observed_one() {
    let analysis = crash::analysis_from_text(
        "Description: Rendering overlay\nExit Code: 6\n",
        false,
        CrashSource::CrashReport,
        Some("crash-2.txt".into()),
        Some(1),
    );
    assert_eq!(analysis.exit_code, Some(6));
}

#[test]
fn loader_is_read_below_the_crash_report_header() {
    let analysis = crash::analysis_from_text(
        "Description: Rendering overlay\n\
         -- System Details --\n\
         Minecraft Version: 1.20.1\n\
         Launched Version: 1.20.1-forge-47.2.0\n\
         --launchTarget forgeclient --fml.forgeVersion 47.2.0\n",
        false,
        CrashSource::CrashReport,
        Some("crash-3.txt".into()),
        None,
    );
    assert_eq!(analysis.context.loader.as_deref(), Some("Forge"));
}

#[test]
fn crash_report_context_is_extracted() {
    let dir = instance_dir();
    std::fs::create_dir_all(dir.path().join("crash-reports")).unwrap();
    std::fs::write(
        dir.path().join("crash-reports/crash-context.txt"),
        "---- Minecraft Crash Report ----\n\
         // Why did you do that?\n\n\
         Time: 2026-01-01 00:00:00\n\
         Description: Rendering overlay\n\n\
         java.lang.OutOfMemoryError: Java heap space\n\n\
         -- System Details --\n\
         Minecraft Version: 1.20.1\n\
         Java Version: 8, vendor: Oracle Corporation\n\
         Exit Code: 1\n",
    )
    .unwrap();
    let analysis = crash::analyze_report(dir.path(), "crash-context.txt").unwrap();
    assert_eq!(analysis.source, CrashSource::CrashReport);
    assert_eq!(
        analysis.context.description.as_deref(),
        Some("Rendering overlay")
    );
    assert_eq!(
        analysis.context.minecraft_version.as_deref(),
        Some("1.20.1")
    );
    assert_eq!(analysis.context.exit_code, Some(1));
    assert_eq!(codes(&analysis), vec!["memory_out_of_memory"]);
}

#[test]
fn unknown_crash_report_returns_a_generic_finding() {
    let dir = instance_dir();
    std::fs::create_dir_all(dir.path().join("crash-reports")).unwrap();
    std::fs::write(
        dir.path().join("crash-reports/crash-unknown.txt"),
        "Description: Something strange happened\njava.lang.Exception: boom\n",
    )
    .unwrap();
    let analysis = crash::analyze_report(dir.path(), "crash-unknown.txt").unwrap();
    assert_eq!(codes(&analysis), vec!["unknown"]);
    assert_eq!(
        analysis.findings[0]
            .params
            .get("description")
            .map(String::as_str),
        Some("Something strange happened")
    );
}

#[test]
fn report_names_must_be_plain_txt_files() {
    let dir = instance_dir();
    std::fs::create_dir_all(dir.path().join("crash-reports")).unwrap();
    for name in ["../crash-1.txt", "sub/crash-1.txt", "notes.log", ".."] {
        assert!(
            crash::analyze_report(dir.path(), name).is_err(),
            "{name} debería rechazarse"
        );
    }
}

#[cfg(unix)]
#[test]
fn symlinked_reports_are_rejected() {
    let dir = instance_dir();
    let reports = dir.path().join("crash-reports");
    std::fs::create_dir_all(&reports).unwrap();
    let outside = dir.path().join("outside.txt");
    std::fs::write(&outside, "java.lang.OutOfMemoryError: Java heap space\n").unwrap();
    std::os::unix::fs::symlink(&outside, reports.join("crash-link.txt")).unwrap();
    assert!(crash::analyze_report(dir.path(), "crash-link.txt").is_err());
    assert!(crash::list_crash_reports(dir.path()).is_empty());
}

#[test]
fn reports_older_than_the_crash_are_ignored() {
    let dir = instance_dir();
    let reports = dir.path().join("crash-reports");
    std::fs::create_dir_all(&reports).unwrap();
    std::fs::write(reports.join("crash-old.txt"), "Description: old\n").unwrap();
    let crashed_at = SystemTime::now() + Duration::from_secs(60);
    assert!(crash::newest_report_since(dir.path(), Some(crashed_at)).is_none());
    assert!(crash::newest_report_since(dir.path(), None).is_some());
}

#[test]
fn large_logs_are_read_from_the_tail_without_partial_lines() {
    let dir = instance_dir();
    let logs = dir.path().join("logs");
    std::fs::create_dir_all(&logs).unwrap();
    let mut content = String::from("first line\n");
    // Suficiente para superar la ventana de 512 KiB de la cola.
    for index in 0..40_000 {
        content.push_str(&format!(
            "[main/INFO]: line {index} padding padding padding\n"
        ));
    }
    content.push_str("java.lang.OutOfMemoryError: Java heap space\n");
    std::fs::write(logs.join("latest.log"), &content).unwrap();

    let tail = crash::log_tail(dir.path());
    assert!(!tail.is_empty());
    assert!(tail.len() <= crash::LOG_TAIL_LINES);
    assert_eq!(
        tail.last().unwrap(),
        "java.lang.OutOfMemoryError: Java heap space"
    );
    assert!(tail[0].starts_with("[main/INFO]: line "));
    assert!(
        !tail[0].starts_with("ine "),
        "no debe empezar a mitad de línea"
    );
}

#[test]
fn invalid_utf8_does_not_drop_the_whole_log() {
    let dir = instance_dir();
    let logs = dir.path().join("logs");
    std::fs::create_dir_all(&logs).unwrap();
    let mut bytes = b"ok\n".to_vec();
    bytes.extend_from_slice(&[0xff, 0xfe]);
    bytes.extend_from_slice(b" bad utf8\njava.lang.OutOfMemoryError: Java heap space\n");
    std::fs::write(logs.join("latest.log"), bytes).unwrap();

    let tail = crash::log_tail(dir.path());
    assert_eq!(tail.len(), 3);
    assert!(tail[2].contains("OutOfMemory"));
}

#[cfg(unix)]
#[test]
fn log_reader_ignores_symlinks_to_outside_files() {
    let dir = instance_dir();
    std::fs::create_dir_all(dir.path().join("logs")).unwrap();
    let outside = dir.path().join("outside.log");
    std::fs::write(&outside, "java.lang.OutOfMemoryError: Java heap space\n").unwrap();
    std::os::unix::fs::symlink(&outside, dir.path().join("logs/latest.log")).unwrap();
    assert!(crash::log_tail(dir.path()).is_empty());
}

#[test]
fn health_reports_low_memory_and_corrupt_worlds() {
    let dir = instance_dir();
    std::fs::create_dir_all(dir.path().join("saves/Broken")).unwrap();
    std::fs::write(dir.path().join("saves/Broken/level.dat"), b"not gzip").unwrap();

    let context = health::InstanceContext {
        version: "1.21".into(),
        instance_dir: dir.path().to_path_buf(),
        min_memory: 2048,
        max_memory: 1024,
        java_override: None,
        shared_dir: dir.path().to_path_buf(),
        launch_ms: 0,
    };
    let checks = health::check_files(&context);
    assert!(
        checks
            .iter()
            .any(|check| check.id == "worlds_metadata_error")
    );

    let mut all = checks;
    health::check_memory_for_tests(&context, &mut all);
    assert!(all.iter().any(|check| check.id == "memory_low"));
}

#[test]
fn duplicate_mods_are_reported_only_from_cached_entries() {
    let dir = instance_dir();
    std::fs::create_dir_all(dir.path().join("mods")).unwrap();
    let context = health::InstanceContext {
        version: "1.21".into(),
        instance_dir: dir.path().to_path_buf(),
        min_memory: 1024,
        max_memory: 2048,
        java_override: None,
        shared_dir: dir.path().to_path_buf(),
        launch_ms: 0,
    };
    // Sin caché no hay duplicados conocidos: no se hashea nada.
    assert!(health::check_files(&context).is_empty());

    let report = DiagnosticReport::from_checks(vec![
        DiagnosticCheck::new("memory_low", DiagnosticSeverity::Warning),
        DiagnosticCheck::new("version_missing", DiagnosticSeverity::Error),
    ]);
    assert_eq!(report.errors, 1);
    assert_eq!(report.warnings, 1);
    assert!(!report.launch_ready);
    assert_eq!(report.checks[0].id, "version_missing");
}

#[test]
fn report_is_launch_ready_without_errors() {
    let report = DiagnosticReport::from_checks(vec![DiagnosticCheck::new(
        "instance_busy",
        DiagnosticSeverity::Warning,
    )]);
    assert!(report.launch_ready);
    assert_eq!(report.errors, 0);
}

#[test]
fn actions_are_serialized_for_the_frontend() {
    let check = DiagnosticCheck::new("java_missing", DiagnosticSeverity::Error)
        .param("version", "21")
        .action(DiagnosticAction::InstallJava { version: 21 });
    let json = serde_json::to_value(&check).unwrap();
    assert_eq!(json["action"]["kind"], "installJava");
    assert_eq!(json["action"]["version"], 21);
    assert_eq!(json["params"]["version"], "21");
}

#[test]
fn report_names_are_validated() {
    assert!(crash::validate_report_name("crash-1.txt").is_ok());
    assert!(crash::validate_report_name("../x.txt").is_err());
}

// ── Último crash ────────────────────────────────────────────────────────────

#[test]
fn recent_launch_error_needs_the_current_launch_stamp() {
    let dir = instance_dir();
    let now = now_millis();
    let mut context = health::InstanceContext {
        version: "1.21".into(),
        instance_dir: dir.path().to_path_buf(),
        min_memory: 1024,
        max_memory: 2048,
        java_override: None,
        shared_dir: dir.path().to_path_buf(),
        launch_ms: now,
    };
    // Se comprueba por el mensaje marcado por el launcher, no por su texto.
    let reason = |stamp: u64| format!("[launch {stamp}] Java no encontrado");
    assert!(health::recent_launch_error_for_tests(
        &context,
        &reason(now)
    ));
    context.launch_ms = now + 60_000;
    assert!(
        !health::recent_launch_error_for_tests(&context, &reason(now)),
        "un error de un arranque anterior no explica el estado actual"
    );
    assert!(!health::recent_launch_error_for_tests(
        &context,
        "mensaje viejo"
    ));
}
