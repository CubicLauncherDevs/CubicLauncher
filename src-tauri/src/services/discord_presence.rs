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
                others,
                started_at,
            } => {
                let mut state = format!("{version} · {loader}");
                if *others > 0 {
                    state.push_str(&format!(" · +{others}"));
                }
                Activity::builder()
                    .details(format!("Jugando a {name}"))
                    .state(state)
                    .start_timestamp(*started_at)
                    .large_image("mc", "Minecraft")
                    .build()
            }
        }
    }
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

pub async fn on_instance_start(name: String, version: String, loader: String) {
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
            started_at,
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
    fn idle_activity_keeps_the_launcher_start() {
        let activity = PresenceState::Idle.to_activity(99);

        assert_eq!(activity.state.as_deref(), Some("CubicLauncher"));
        assert_eq!(activity.timestamps.and_then(|t| t.start), Some(99));
    }
}
