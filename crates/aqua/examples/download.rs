use aqua::{DownloadManager, DownloadProgress, DownloadProgressType, FabricBatch};
use std::env;
use std::path::PathBuf;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let base_dir = env::var("BASE_DIR").unwrap_or_else(|_| "/tmp/xd".to_string());
    let version = env::var("VERSION").unwrap_or_else(|_| "1.12.2".to_string());
    let max_handles: usize = env::var("MAX_HANDLES")
        .unwrap_or_else(|_| "1".to_string())
        .parse()
        .unwrap_or(1);

    let download_type = env::var("DOWNLOAD_TYPE").unwrap_or_else(|_| "minecraft".to_string());

    let (tx, mut rx) = mpsc::channel::<DownloadProgress>(100);
    let progress_handle = tokio::spawn(async move {
        while let Some(prog) = rx.recv().await {
            let label = match prog.download_type {
                DownloadProgressType::Client => "CLIENT",
                DownloadProgressType::Library => "LIB",
                DownloadProgressType::Asset => "ASSET",
                DownloadProgressType::Native => "NATIVE",
                DownloadProgressType::Verifying => "VERIFY",
                DownloadProgressType::Generic => "GENERIC",
                DownloadProgressType::Processing => "PROC",
                DownloadProgressType::Jre => "JRE",
            };
            println!(
                "[{}/{}] [{label:6}] {}",
                prog.current, prog.total, prog.info.name
            );
        }
    });

    let manager = DownloadManager::new(PathBuf::from(&base_dir))
        .with_max_handles(max_handles)
        .with_max_downloads(128);

    if download_type == "fabric" {
        let game_version = env::var("GAME_VERSION").unwrap_or_else(|_| "1.21".to_string());
        let loader_version = if let Ok(lv) = env::var("LOADER_VERSION") {
            lv
        } else {
            FabricBatch::resolve_latest_loader(&game_version).await?
        };
        let batch = FabricBatch::new(
            PathBuf::from(&base_dir).as_path(),
            &game_version,
            &loader_version,
        )
        .await?;
        let handle = manager.prepare_batch(Box::new(batch)).await?;
        println!("=== Fabric ===");
        println!("  name: {}", handle.name());
        println!("  base_dir:  {base_dir}");
        println!();
        handle.start(Some(tx)).await?;
        handle.wait().await?;
        progress_handle.await?;
        println!("\n✓ Fabric download complete: {}", handle.name());
    } else {
        let handle = manager.prepare(&version).await?;
        println!("=== Proton ===");
        let v = handle.version().expect("Minecraft version info");
        println!("  version:   {}", v.id);
        println!("  java:      {}", v.java_version);
        println!("  libraries: {}", v.libraries.len());
        println!("  natives:   {}", v.natives.len());
        println!("  base_dir:  {base_dir}");
        println!();

        let instant = std::time::Instant::now();
        handle.start(Some(tx)).await?;
        handle.wait().await?;
        progress_handle.await?;
        println!("{:#?}", instant.elapsed());
        println!("\n✓ Descarga completada: {}", handle.name());
    }

    Ok(())
}
