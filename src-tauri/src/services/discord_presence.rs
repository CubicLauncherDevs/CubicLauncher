use communicator::{Activity, DiscordRpcClient};
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;
use tracing::{error, info, warn};

use crate::services::SettingsManager;

const CLIENT_ID: &str = "1305247641252397059";

struct DiscordPresence {
    client: Mutex<Option<DiscordRpcClient>>,
    first_instance: Mutex<Option<String>>,
}

impl DiscordPresence {
    fn new() -> Self {
        Self {
            client: Mutex::new(None),
            first_instance: Mutex::new(None),
        }
    }
}

static DISCORD: OnceLock<DiscordPresence> = OnceLock::new();

fn is_enabled() -> bool {
    SettingsManager::read().discord_presence
}

pub async fn init() -> Result<(), &'static str> {
    if !is_enabled() {
        info!("Discord presence disabled in settings, skipping init");
        return Err("Discord presence disabled in settings");
    }

    let presence = DISCORD.get_or_init(DiscordPresence::new);
    {
        let guard = presence.client.lock().await;
        if guard.is_some() {
            return Err("Discord client already running");
        }
    }

    info!("Initializing Discord RPC client");
    let mut client = DiscordRpcClient::new(CLIENT_ID);
    client.connect().await.map_err(|e| {
        warn!("Failed to connect to Discord: {e}");
        "Failed to connect to Discord"
    })?;

    info!("Connected to Discord");
    *presence.client.lock().await = Some(client);
    set_idle().await;
    Ok(())
}

async fn set_idle() {
    let presence = match DISCORD.get() {
        Some(p) => p,
        None => return,
    };

    if !is_enabled() {
        return;
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let activity = Activity::builder()
        .details("Navegando el launcher")
        .state("CubicLauncher")
        .start_timestamp(now)
        .large_image("logo", "CubicLauncher")
        .build();

    let guard = presence.client.lock().await;
    if let Some(ref client) = *guard
        && let Err(e) = client.set_activity(activity).await
    {
        error!("Failed to set idle presence: {e}");
    }
}

async fn set_playing(name: String, version: String, loader: String) {
    let presence = match DISCORD.get() {
        Some(p) => p,
        None => return,
    };

    if !is_enabled() {
        return;
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let state = format!("{version} · {loader}");
    let activity = Activity::builder()
        .details(format!("Jugando a {name}"))
        .state(state)
        .start_timestamp(now)
        .large_image("mc", "Minecraft")
        .build();

    let guard = presence.client.lock().await;
    if let Some(ref client) = *guard
        && let Err(e) = client.set_activity(activity).await
    {
        error!("Failed to set playing presence: {e}");
    }
}

pub async fn shutdown() {
    let presence = match DISCORD.get() {
        Some(p) => p,
        None => return,
    };

    {
        let mut guard = presence.first_instance.lock().await;
        *guard = None;
    }

    let mut guard = presence.client.lock().await;
    if let Some(client) = guard.take() {
        info!("Shutting down Discord RPC client");
        client.shutdown().await;
    }
}

pub async fn on_instance_start(name: String, version: String, loader: String) {
    let presence = match DISCORD.get() {
        Some(p) => p,
        None => return,
    };

    let mut guard = presence.first_instance.lock().await;
    if guard.is_some() {
        return;
    }
    *guard = Some(name.clone());
    drop(guard);

    set_playing(name, version, loader).await;
}

pub async fn on_instance_stop(name: &str) {
    let presence = match DISCORD.get() {
        Some(p) => p,
        None => return,
    };

    let mut guard = presence.first_instance.lock().await;
    if guard.as_deref() != Some(name) {
        return;
    }
    *guard = None;
    drop(guard);

    set_idle().await;
}
