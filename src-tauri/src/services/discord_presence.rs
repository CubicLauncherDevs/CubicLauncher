use communicator::{Activity, DiscordRpcClient};
use std::sync::OnceLock;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::{Mutex, watch};
use tracing::{error, info, warn};

use crate::services::SettingsManager;

const CLIENT_ID: &str = "1305247641252397059";

/// How long `init` waits for the first handshake before returning and letting
/// the background loop keep retrying.
const INIT_CONNECT_TIMEOUT: Duration = Duration::from_secs(8);

/// Minimum spacing between presence updates. Discord allows roughly five
/// updates per 20 seconds; anything closer is coalesced by the worker.
const MIN_UPDATE_INTERVAL: Duration = Duration::from_secs(4);

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// An instance whose process is currently running.
struct RunningInstance {
    name: String,
    version: String,
    loader: String,
    modpack_name: Option<String>,
    modpack_icon: Option<String>,
    started_at: u64,
}

/// What the launcher wants Discord to display. Always derived from the running
/// instances, so the presence can never drift from the real state.
#[derive(Debug, Clone, PartialEq, Eq)]
enum PresenceState {
    Idle,
    Playing {
        name: String,
        version: String,
        loader: String,
        modpack_name: Option<String>,
        modpack_icon: Option<String>,
        /// How many other instances are running at the same time.
        others: usize,
        started_at: u64,
    },
}

impl PresenceState {
    fn resolve(running: &[RunningInstance]) -> Self {
        match running.last() {
            None => Self::Idle,
            Some(current) => Self::Playing {
                name: current.name.clone(),
                version: current.version.clone(),
                loader: current.loader.clone(),
                modpack_name: current.modpack_name.clone(),
                modpack_icon: current.modpack_icon.clone(),
                others: running.len() - 1,
                started_at: current.started_at,
            },
        }
    }

    fn to_activity(&self, session_start: u64) -> Activity {
        match self {
            Self::Idle => Activity::builder()
                .details("Navegando el launcher")
                .state("CubicLauncher")
                .start_timestamp(session_start)
                .large_image("logo", "CubicLauncher")
                .build(),
            Self::Playing {
                name,
                version,
                loader,
                modpack_name,
                modpack_icon,
                others,
                started_at,
            } => {
                let mut state = format!("{version} · {loader}");
                if *others > 0 {
                    state.push_str(&format!(" · +{others}"));
                }
                let details = match modpack_name.as_deref().filter(|n| !n.trim().is_empty()) {
                    Some(pack) => fit(format!("Jugando a {pack}"), 128),
                    None => fit(format!("Jugando a {name}"), 128),
                };
                let (image, text) = match modpack_icon.as_deref() {
                    Some(icon) if valid_icon(icon) => (
                        icon.to_string(),
                        fit(modpack_name.clone().unwrap_or_default(), 128),
                    ),
                    _ => ("mc".to_string(), "Minecraft".to_string()),
                };
                Activity::builder()
                    .details(details)
                    .state(fit(state, 128))
                    .start_timestamp(*started_at)
                    .large_image(image, text)
                    .build()
            }
        }
    }
}

/// Discord rejects activity texts under 2 characters, so an empty name still
/// falls back to something displayable.
fn fit(text: String, max: usize) -> String {
    let text: String = text.chars().take(max).collect();
    if text.trim().chars().count() < 2 {
        "CubicLauncher".to_string()
    } else {
        text
    }
}

/// Only external https URLs are sent: Discord cannot render anything else.
fn valid_icon(url: &str) -> bool {
    url.starts_with("https://") && url.len() <= 300
}

struct DiscordPresence {
    client: Mutex<Option<DiscordRpcClient>>,
    /// Single task that owns the conversation with Discord. `None` until `init`.
    worker: Mutex<Option<tokio::task::JoinHandle<()>>>,
    state_tx: watch::Sender<PresenceState>,
    running: Mutex<Vec<RunningInstance>>,
    /// Fixed at startup so the idle timer reflects launcher uptime instead of
    /// resetting on every transition.
    session_start: u64,
}

impl DiscordPresence {
    fn new() -> Self {
        let (state_tx, _) = watch::channel(PresenceState::Idle);
        Self {
            client: Mutex::new(None),
            worker: Mutex::new(None),
            state_tx,
            running: Mutex::new(Vec::new()),
            session_start: now_secs(),
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
            return Ok(());
        }
    }

    info!("Initializing Discord RPC client");
    let mut client = DiscordRpcClient::new(CLIENT_ID);

    // Discord may not be running yet. Keep the client either way: its
    // connection loop retries in the background, so presence appears as soon
    // as the user opens Discord instead of requiring a launcher restart.
    match tokio::time::timeout(INIT_CONNECT_TIMEOUT, client.connect()).await {
        Ok(Ok(())) => info!("Connected to Discord"),
        Ok(Err(e)) => warn!("Discord RPC unavailable, retrying in background: {e}"),
        Err(_) => warn!("Discord RPC handshake timed out, retrying in background"),
    }

    *presence.client.lock().await = Some(client);

    let mut worker = presence.worker.lock().await;
    if worker.as_ref().is_none_or(|handle| handle.is_finished()) {
        *worker = Some(tokio::spawn(run_worker(presence.state_tx.subscribe())));
    }
    drop(worker);

    // Republish the current state so the freshly spawned worker sends it even
    // when the value itself did not change (e.g. re-enabling the setting).
    let current = presence.state_tx.borrow().clone();
    presence.state_tx.send_replace(current);

    Ok(())
}

pub async fn shutdown() {
    let Some(presence) = DISCORD.get() else {
        return;
    };

    if let Some(handle) = presence.worker.lock().await.take() {
        handle.abort();
    }

    let mut guard = presence.client.lock().await;
    if let Some(client) = guard.take() {
        info!("Shutting down Discord RPC client");
        client.shutdown().await;
    }
}

pub async fn on_instance_start(
    name: String,
    version: String,
    loader: String,
    modpack_name: Option<String>,
    modpack_icon: Option<String>,
) {
    let Some(presence) = DISCORD.get() else {
        return;
    };

    // Tracked even when presence is disabled: the worker is not running, but
    // re-enabling the setting must show the right state immediately.
    let mut running = presence.running.lock().await;
    if running.iter().any(|instance| instance.name == name) {
        return;
    }
    running.push(RunningInstance {
        name,
        version,
        loader,
        modpack_name,
        modpack_icon,
        started_at: now_secs(),
    });
    presence
        .state_tx
        .send_replace(PresenceState::resolve(&running));
}

pub async fn on_instance_stop(name: &str) {
    let Some(presence) = DISCORD.get() else {
        return;
    };

    let mut running = presence.running.lock().await;
    let before = running.len();
    running.retain(|instance| instance.name != name);
    if running.len() == before {
        return;
    }
    presence
        .state_tx
        .send_replace(PresenceState::resolve(&running));
}

/// Applies state changes to Discord, coalescing bursts so we stay inside the
/// rate-limit window. Leading edge: the first change goes out immediately.
async fn run_worker(mut rx: watch::Receiver<PresenceState>) {
    let mut last_applied: Option<PresenceState> = None;
    let mut last_sent: Option<Instant> = None;

    loop {
        if rx.changed().await.is_err() {
            return;
        }

        if let Some(sent) = last_sent {
            let elapsed = sent.elapsed();
            if elapsed < MIN_UPDATE_INTERVAL {
                tokio::time::sleep(MIN_UPDATE_INTERVAL - elapsed).await;
            }
        }

        let state = rx.borrow_and_update().clone();
        if last_applied.as_ref() == Some(&state) {
            continue;
        }

        if apply(&state).await {
            last_applied = Some(state);
            last_sent = Some(Instant::now());
        }
    }
}

async fn apply(state: &PresenceState) -> bool {
    let Some(presence) = DISCORD.get() else {
        return false;
    };
    let activity = state.to_activity(presence.session_start);

    let guard = presence.client.lock().await;
    let Some(client) = guard.as_ref() else {
        return false;
    };

    match client.set_activity(activity).await {
        Ok(()) => true,
        Err(e) => {
            error!("Failed to update Discord presence: {e}");
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn instance(name: &str, started_at: u64) -> RunningInstance {
        RunningInstance {
            name: name.to_string(),
            version: "1.21.1".to_string(),
            loader: "fabric".to_string(),
            modpack_name: None,
            modpack_icon: None,
            started_at,
        }
    }

    fn modpack_instance(name: &str, pack: &str, icon: Option<&str>) -> RunningInstance {
        RunningInstance {
            modpack_name: Some(pack.to_string()),
            modpack_icon: icon.map(str::to_owned),
            ..instance(name, 10)
        }
    }

    #[test]
    fn idle_without_instances() {
        assert_eq!(PresenceState::resolve(&[]), PresenceState::Idle);
    }

    #[test]
    fn playing_uses_the_latest_instance() {
        let running = vec![instance("a", 10), instance("b", 20)];

        match PresenceState::resolve(&running) {
            PresenceState::Playing {
                name,
                others,
                started_at,
                ..
            } => {
                assert_eq!(name, "b");
                assert_eq!(others, 1);
                assert_eq!(started_at, 20);
            }
            PresenceState::Idle => panic!("expected a playing state"),
        }
    }

    #[test]
    fn stopping_the_latest_falls_back_to_the_previous_one() {
        let mut running = vec![instance("a", 10), instance("b", 20)];
        running.retain(|instance| instance.name != "b");

        match PresenceState::resolve(&running) {
            PresenceState::Playing { name, others, .. } => {
                assert_eq!(name, "a");
                assert_eq!(others, 0);
            }
            PresenceState::Idle => panic!("expected a playing state"),
        }
    }

    #[test]
    fn playing_activity_uses_the_instance_start() {
        let activity = PresenceState::resolve(&[instance("a", 10)]).to_activity(99);

        assert_eq!(activity.details.as_deref(), Some("Jugando a a"));
        assert_eq!(activity.state.as_deref(), Some("1.21.1 · fabric"));
        assert_eq!(activity.timestamps.and_then(|t| t.start), Some(10));
    }

    #[test]
    fn extra_instances_are_reflected_in_the_state_line() {
        let running = vec![instance("a", 1), instance("b", 2)];
        let activity = PresenceState::resolve(&running).to_activity(0);

        assert_eq!(activity.state.as_deref(), Some("1.21.1 · fabric · +1"));
    }

    #[test]
    fn modpack_with_icon_uses_its_name_and_art() {
        let icon = "https://cdn.modrinth.com/data/project/icon.png";
        let activity = PresenceState::resolve(&[modpack_instance(
            "lobby",
            "Fabulously Optimized",
            Some(icon),
        )])
        .to_activity(0);

        assert_eq!(
            activity.details.as_deref(),
            Some("Jugando a Fabulously Optimized")
        );
        assert_eq!(activity.state.as_deref(), Some("1.21.1 · fabric"));
        let assets = activity.assets.unwrap();
        assert_eq!(assets.large_image.as_deref(), Some(icon));
        assert_eq!(assets.large_text.as_deref(), Some("Fabulously Optimized"));
    }

    #[test]
    fn modpack_without_a_usable_url_falls_back_to_the_minecraft_art() {
        for icon in [
            None,
            Some("http://insecure.example/icon.png"),
            Some("local/path/icon.png"),
        ] {
            let activity = PresenceState::resolve(&[modpack_instance("lobby", "Cave pack", icon)])
                .to_activity(0);
            let assets = activity.assets.as_ref().unwrap();
            assert_eq!(assets.large_image.as_deref(), Some("mc"));
            assert_eq!(assets.large_text.as_deref(), Some("Minecraft"));
            assert_eq!(activity.details.as_deref(), Some("Jugando a Cave pack"));
        }
    }

    #[test]
    fn plain_instance_keeps_the_instance_name_and_art() {
        let activity = PresenceState::resolve(&[instance("a", 10)]).to_activity(0);
        let assets = activity.assets.as_ref().unwrap();

        assert_eq!(activity.details.as_deref(), Some("Jugando a a"));
        assert_eq!(assets.large_image.as_deref(), Some("mc"));
    }

    #[test]
    fn long_modpack_names_are_truncated() {
        let pack = "p".repeat(200);
        let activity = PresenceState::resolve(&[modpack_instance("a", &pack, None)]).to_activity(0);
        let details = activity.details.unwrap();

        assert_eq!(details.chars().count(), 128);
        assert!(details.starts_with("Jugando a ppp"));
    }

    #[test]
    fn overlong_icon_urls_are_rejected() {
        let long = format!("https://example.com/{}", "a".repeat(300));
        let activity =
            PresenceState::resolve(&[modpack_instance("a", "Pack", Some(&long))]).to_activity(0);

        assert_eq!(activity.assets.unwrap().large_image.as_deref(), Some("mc"));
    }

    #[test]
    fn idle_activity_keeps_the_launcher_start() {
        let activity = PresenceState::Idle.to_activity(99);

        assert_eq!(activity.state.as_deref(), Some("CubicLauncher"));
        assert_eq!(activity.timestamps.and_then(|t| t.start), Some(99));
    }
}
