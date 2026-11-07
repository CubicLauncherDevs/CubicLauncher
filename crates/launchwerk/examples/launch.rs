// Copyright (C) 2025 Santiagolxx, CubicLauncher contributors
// SPDX-License-Identifier: GPL-3.0-only
//
// Uso:
//   BASE_DIR=/home/user/.cubic VERSION=fabric-loader-0.19.2-1.20.1 JAVA=/usr/bin/java cargo run --example launch

use launchwerk::models::VersionManifest;
use launchwerk::{LaunchConfig, Launchwerk};
use std::env;
use std::path::PathBuf;
use tokio::sync::broadcast::error::RecvError;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    let base_dir = env::var("BASE_DIR").unwrap_or_else(|_| "/home/santiagolxx/.cubic/".to_string());
    let version = env::var("VERSION").unwrap_or_else(|_| "26.2-snapshot-5".to_string());
    let java_path =
        env::var("JAVA").unwrap_or_else(|_| "/usr/lib/jvm/java-25-openjdk/bin/java".to_string());
    let username = env::var("USERNAME").unwrap_or_else(|_| "Player".to_string());

    let shared_dir = PathBuf::from(&base_dir).join("shared");
    let instance_dir = PathBuf::from(&base_dir).join("instances").join(&version);
    let version_json = shared_dir
        .join("versions")
        .join(&version)
        .join(format!("{version}.json"));

    println!("=== CLaunch v2 ===");
    println!("  shared_dir:   {}", shared_dir.display());
    println!("  instance_dir: {}", instance_dir.display());
    println!("  version_json: {}", version_json.display());
    println!("  java:         {java_path}");
    println!("  username:     {username}");
    println!();

    let manifest = VersionManifest::from_file(&version_json)?;
    println!(
        "Manifest cargado: {} (Java {})",
        manifest.id,
        manifest.java_major_version()
    );

    let config = LaunchConfig::builder()
        .java_path(&java_path)
        .username(&username)
        .ram("512M", "2G")
        .resolution(854, 480)
        .cracked(true)
        .build();

    let lw = Launchwerk::new(shared_dir);
    let handle = lw.prepare(manifest, config, instance_dir);

    println!("Instancia preparada: {}", handle.id());
    println!("Loader: {}", handle.loader());

    let mut stdout_rx = handle.subscribe_stdout();
    let mut stderr_rx = handle.subscribe_stderr();

    tokio::spawn(async move {
        loop {
            match stdout_rx.recv().await {
                Ok(line) => println!("[MC] {line}"),
                Err(RecvError::Closed) => break,
                Err(RecvError::Lagged(n)) => eprintln!("[stdout] {n} mensajes perdidos"),
            }
        }
    });

    tokio::spawn(async move {
        loop {
            match stderr_rx.recv().await {
                Ok(line) => eprintln!("[MC:err] {line}"),
                Err(RecvError::Closed) => break,
                Err(RecvError::Lagged(n)) => eprintln!("[stderr] {n} mensajes perdidos"),
            }
        }
    });

    handle.launch().await?;
    println!("Juego lanzado, esperando...");

    match handle.wait().await {
        Some(0) => println!("Juego cerrado correctamente."),
        Some(code) => eprintln!("Juego cerrado con código {code}."),
        None => eprintln!("El proceso no estaba corriendo."),
    }

    Ok(())
}
