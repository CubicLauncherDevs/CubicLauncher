use aqua::{DownloadManager, DownloadProgress, DownloadStage, FabricBatch};
use std::env;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let base_dir = env::var("BASE_DIR").unwrap_or_else(|_| "/tmp/xd".to_string());
    let version = env::var("VERSION").unwrap_or_else(|_| "1.12.2".to_string());
    let max_handles: usize = env::var("MAX_HANDLES")
        .unwrap_or_else(|_| "1".to_string())
        .parse()
        .unwrap_or(1);

    let download_type = env::var("DOWNLOAD_TYPE").unwrap_or_else(|_| "minecraft".to_string());

    let (tx, mut rx) = tokio::sync::watch::channel(DownloadProgress::empty(0));
    let progress_handle = tokio::spawn(async move {
        let mut last = None;
        loop {
            if rx.changed().await.is_err() {
                break;
            }
            let prog = rx.borrow().clone();
            if last.as_ref() == Some(&prog) {
                continue;
            }
            last = Some(prog.clone());
            let label = match prog.stage {
                DownloadStage::Client => "CLIENT",
                DownloadStage::Library => "LIB",
                DownloadStage::Asset => "ASSET",
                DownloadStage::Native => "NATIVE",
                DownloadStage::Verifying => "VERIFY",
                DownloadStage::Generic => "GENERIC",
                DownloadStage::Processing => "PROC",
                DownloadStage::Jre => "JRE",
                DownloadStage::Resolving => "RESOLV",
                DownloadStage::Extracting => "EXTRACT",
            };
            println!(
                "items [{}/{}] bytes [{}/{}] [{label:7}] {}",
                prog.item_current,
                prog.item_total,
                prog.bytes_current,
                prog.bytes_total,
                prog.current_item.as_deref().unwrap_or("-"),
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
