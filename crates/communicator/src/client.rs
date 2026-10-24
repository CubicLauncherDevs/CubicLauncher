use crate::error::DiscordRpcError;
use crate::models::Activity;
use serde_json::{Value, json};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::oneshot::error::TryRecvError;
use tokio::sync::{mpsc, oneshot};
use tokio::time;
use tracing::{debug, error, info, instrument, warn};

#[cfg(unix)]
use tokio::net::UnixStream;

#[cfg(windows)]
use tokio::net::windows::named_pipe::ClientOptions;

static NONCE: AtomicU64 = AtomicU64::new(1);

#[inline]
fn next_nonce() -> String {
    NONCE.fetch_add(1, Ordering::Relaxed).to_string()
}

// ── Protocol constants ────────────────────────────────────────────────────────

/// Opcode of the initial handshake frame.
const OP_HANDSHAKE: u32 = 0;
/// Opcode of a regular command/response frame.
const OP_FRAME: u32 = 1;
/// Opcode Discord uses to close the connection.
const OP_CLOSE: u32 = 2;
/// Opcode of a keepalive ping.
const OP_PING: u32 = 3;
/// Opcode of the reply to a ping.
const OP_PONG: u32 = 4;

/// Largest frame we are willing to buffer; Discord payloads are far smaller.
const MAX_FRAME_BYTES: u32 = 64 * 1024;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(3);
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);
/// Upper bound for queueing a command, so a stalled connection never blocks
/// the caller (e.g. the game launch flow) indefinitely.
const COMMAND_TIMEOUT: Duration = Duration::from_secs(2);
/// Grace period to flush pending frames on shutdown.
const SHUTDOWN_FLUSH_TIMEOUT: Duration = Duration::from_millis(500);

const RECONNECT_MIN: Duration = Duration::from_secs(1);
const RECONNECT_MAX: Duration = Duration::from_secs(60);

const MIN_HEARTBEAT_MS: u64 = 5_000;
const MAX_HEARTBEAT_MS: u64 = 120_000;
const DEFAULT_HEARTBEAT_MS: u64 = 45_000;

/// Adds up to 20% of the delay as jitter so clients do not reconnect in
/// lockstep. Avoids pulling in an RNG dependency.
fn with_jitter(delay: Duration) -> Duration {
    let spread = (delay.as_millis() as u64 / 5).max(1);
    let entropy = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|t| u64::from(t.subsec_nanos()))
        .unwrap_or(0);
    delay + Duration::from_millis(entropy % spread)
}

/// Exponential reconnect backoff capped at RECONNECT_MAX.
struct Backoff {
    current: Duration,
}

impl Backoff {
    fn new() -> Self {
        Self {
            current: RECONNECT_MIN,
        }
    }

    fn reset(&mut self) {
        self.current = RECONNECT_MIN;
    }

    /// Delay before the next attempt; the following one doubles.
    fn next_delay(&mut self) -> Duration {
        let delay = with_jitter(self.current);
        self.current = (self.current * 2).min(RECONNECT_MAX);
        delay
    }
}

enum PlatformStream {
    #[cfg(unix)]
    Unix(UnixStream),
    #[cfg(windows)]
    Windows(tokio::net::windows::named_pipe::NamedPipeClient),
}

impl AsyncRead for PlatformStream {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match self.get_mut() {
            #[cfg(unix)]
            PlatformStream::Unix(s) => std::pin::Pin::new(s).poll_read(cx, buf),
            #[cfg(windows)]
            PlatformStream::Windows(s) => std::pin::Pin::new(s).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for PlatformStream {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        match self.get_mut() {
            #[cfg(unix)]
            PlatformStream::Unix(s) => std::pin::Pin::new(s).poll_write(cx, buf),
            #[cfg(windows)]
            PlatformStream::Windows(s) => std::pin::Pin::new(s).poll_write(cx, buf),
        }
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match self.get_mut() {
            #[cfg(unix)]
            PlatformStream::Unix(s) => std::pin::Pin::new(s).poll_flush(cx),
            #[cfg(windows)]
            PlatformStream::Windows(s) => std::pin::Pin::new(s).poll_flush(cx),
        }
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match self.get_mut() {
            #[cfg(unix)]
            PlatformStream::Unix(s) => std::pin::Pin::new(s).poll_shutdown(cx),
            #[cfg(windows)]
            PlatformStream::Windows(s) => std::pin::Pin::new(s).poll_shutdown(cx),
        }
    }
}

// ── Commands ──────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum ClientCommand {
    SetActivity(Box<Activity>),
    ClearActivity,
    /// Graceful shutdown: closes the connection and stops the background task permanently.
    Shutdown,
}

// ── Client ────────────────────────────────────────────────────────────────────

pub struct DiscordRpcClient {
    #[allow(dead_code)]
    client_id: String,
    command_tx: mpsc::Sender<ClientCommand>,
    /// Fires once the first handshake completes. Consumed by `connect()`.
    ready_rx: Option<oneshot::Receiver<()>>,
    /// Signals the background loop to stop permanently (not just disconnect).
    _stop_tx: oneshot::Sender<()>,
}

impl DiscordRpcClient {
    pub fn new(client_id: &str) -> Self {
        let (command_tx, command_rx) = mpsc::channel(4);
        let (stop_tx, stop_rx) = oneshot::channel();
        let (ready_tx, ready_rx) = oneshot::channel();

        tokio::spawn(Self::connection_loop(
            client_id.to_string(),
            command_rx,
            stop_rx,
            ready_tx,
        ));

        Self {
            client_id: client_id.to_string(),
            command_tx,
            ready_rx: Some(ready_rx),
            _stop_tx: stop_tx,
        }
    }

    /// Waits until the handshake with Discord completes.
    /// Always call this before `set_activity`.
    ///
    /// ```rust,no_run
    /// use communicator::{Activity, DiscordRpcClient};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let mut client = DiscordRpcClient::new("app_id");
    ///     client.connect().await?;
    ///     let activity = Activity::builder()
    ///         .details("In game")
    ///         .build();
    ///     client.set_activity(activity).await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn connect(&mut self) -> Result<(), DiscordRpcError> {
        if let Some(rx) = self.ready_rx.take() {
            rx.await.map_err(|_| DiscordRpcError::NotConnected)?;
        }
        Ok(())
    }

    pub async fn set_activity(&self, activity: Activity) -> Result<(), DiscordRpcError> {
        self.send(ClientCommand::SetActivity(Box::new(activity)))
            .await
    }

    pub async fn clear_activity(&self) -> Result<(), DiscordRpcError> {
        self.send(ClientCommand::ClearActivity).await
    }

    /// Queues a command with a bounded wait. While the background loop is
    /// reconnecting it stops draining the channel, so an unbounded
    /// send().await could block the caller for the whole backoff window.
    async fn send(&self, cmd: ClientCommand) -> Result<(), DiscordRpcError> {
        match time::timeout(COMMAND_TIMEOUT, self.command_tx.send(cmd)).await {
            Ok(Ok(())) => Ok(()),
            Ok(Err(_)) => Err(DiscordRpcError::NotConnected),
            Err(_) => Err(DiscordRpcError::CommandTimeout),
        }
    }

    pub async fn shutdown(self) {
        // Clear first so Discord does not keep a stale "Playing" card around
        // after the launcher is gone.
        let _ = self.clear_activity().await;
        let _ = time::timeout(
            COMMAND_TIMEOUT,
            self.command_tx.send(ClientCommand::Shutdown),
        )
        .await;
        drop(self);
    }
}

impl DiscordRpcClient {
    #[instrument(skip(command_rx, stop_rx, ready_tx))]
    async fn connection_loop(
        client_id: String,
        mut command_rx: mpsc::Receiver<ClientCommand>,
        mut stop_rx: oneshot::Receiver<()>,
        ready_tx: oneshot::Sender<()>,
    ) {
        debug!("Discord RPC connection loop started");

        let mut ready_tx = Some(ready_tx);
        let mut last_activity: Option<Activity> = None;
        let mut backoff = Backoff::new();

        loop {
            match stop_rx.try_recv() {
                Ok(()) | Err(TryRecvError::Closed) => {
                    info!("Stop signal received, exiting connection loop");
                    return;
                }
                Err(TryRecvError::Empty) => {}
            }

            let mut connected = false;
            match Self::session(
                &client_id,
                &mut command_rx,
                &mut stop_rx,
                &mut ready_tx,
                &mut last_activity,
                &mut connected,
            )
            .await
            {
                Ok(true) => {
                    info!("Shutdown requested, exiting");
                    return;
                }
                Ok(false) => debug!("Session ended"),
                Err(e) => warn!("Session error: {e}"),
            }

            // A session that never reached READY means Discord is unavailable
            // (not running, no socket, handshake refused). Back off so we do
            // not hammer the IPC socket for the whole run; after a session
            // that did connect, retry quickly.
            let delay = if connected {
                backoff.reset();
                RECONNECT_MIN
            } else {
                backoff.next_delay()
            };
            debug!("Reconnecting in {:?}", delay);

            tokio::select! {
                _ = time::sleep(delay) => {}
                _ = &mut stop_rx => {
                    info!("Stop signal received during reconnect wait");
                    return;
                }
            }
        }
    }

    #[instrument(skip(command_rx, stop_rx, ready_tx, last_activity, connected))]
    async fn session(
        client_id: &str,
        command_rx: &mut mpsc::Receiver<ClientCommand>,
        stop_rx: &mut oneshot::Receiver<()>,
        ready_tx: &mut Option<oneshot::Sender<()>>,
        last_activity: &mut Option<Activity>,
        connected: &mut bool,
    ) -> Result<bool, DiscordRpcError> {
        let mut stream = time::timeout(CONNECT_TIMEOUT, Self::open_socket())
            .await
            .map_err(|_| DiscordRpcError::ConnectionFailed("connect timed out".into()))??;

        let heartbeat_ms =
            time::timeout(HANDSHAKE_TIMEOUT, Self::handshake(&mut stream, client_id))
                .await
                .map_err(|_| DiscordRpcError::HandshakeError("handshake timed out".into()))??;

        *connected = true;

        if let Some(tx) = ready_tx.take() {
            tx.send(()).ok();
        }

        let (read_half, write_half) = tokio::io::split(stream);
        let (write_tx, write_rx) = mpsc::channel::<WriteCmd>(4);

        if let Some(activity) = last_activity.clone() {
            write_tx
                .send(WriteCmd::SetActivity(Box::new(activity)))
                .await
                .ok();
        }

        let mut read_handle = tokio::spawn(Self::read_task(read_half, write_tx.clone()));
        let mut write_handle = tokio::spawn(Self::write_task(
            write_half,
            write_rx,
            client_id.to_string(),
            heartbeat_ms,
        ));

        let mut shutdown = false;

        loop {
            tokio::select! {
                Some(cmd) = command_rx.recv() => {
                    match cmd {
                        ClientCommand::SetActivity(a) => {
                            *last_activity = Some((*a).clone());
                            write_tx.send(WriteCmd::SetActivity(a)).await.ok();
                        }
                        ClientCommand::ClearActivity => {
                            *last_activity = None;
                            write_tx.send(WriteCmd::ClearActivity).await.ok();
                        }
                        ClientCommand::Shutdown => {
                            shutdown = true;
                            break;
                        }
                    }
                }
                _ = &mut *stop_rx => {
                    shutdown = true;
                    break;
                }
                _ = &mut read_handle => break,
                _ = &mut write_handle => break,
            }
        }

        // Let the writer flush anything still queued (typically ClearActivity)
        // before the socket is dropped.
        if shutdown && write_tx.send(WriteCmd::Shutdown).await.is_ok() {
            let _ = time::timeout(SHUTDOWN_FLUSH_TIMEOUT, &mut write_handle).await;
        }

        read_handle.abort();
        write_handle.abort();

        Ok(shutdown)
    }

    // ── Connection ────────────────────────────────────────────────────────────

    async fn open_socket() -> Result<PlatformStream, DiscordRpcError> {
        #[cfg(unix)]
        return Self::connect_unix().await;
        #[cfg(windows)]
        return Self::connect_windows().await;
    }

    #[cfg(unix)]
    async fn connect_unix() -> Result<PlatformStream, DiscordRpcError> {
        // Collect candidate base paths, deduplicating empties.
        let bases: Vec<String> = [
            std::env::var("XDG_RUNTIME_DIR").ok(),
            std::env::var("TMPDIR").ok(),
            Some("/tmp".into()),
            // Flatpak / Snap sandboxes often put the socket here:
            std::env::var("XDG_RUNTIME_DIR")
                .ok()
                .map(|d| format!("{d}/app/com.discordapp.Discord")),
        ]
        .into_iter()
        .flatten()
        .collect();

        for base in &bases {
            for i in 0..10u8 {
                let path = format!("{base}/discord-ipc-{i}");
                match UnixStream::connect(&path).await {
                    Ok(s) => {
                        info!("Connected: {path}");
                        return Ok(PlatformStream::Unix(s));
                    }
                    Err(e) => debug!("Skip {path}: {e}"),
                }
            }
        }

        Err(DiscordRpcError::ConnectionFailed(
            "No Discord IPC socket found".into(),
        ))
    }

    #[cfg(windows)]
    async fn connect_windows() -> Result<PlatformStream, DiscordRpcError> {
        for i in 0..10u8 {
            let name = format!(r"\\.\pipe\discord-ipc-{i}");
            match ClientOptions::new().open(&name) {
                Ok(p) => {
                    info!("Connected: {name}");
                    return Ok(PlatformStream::Windows(p));
                }
                Err(e) => debug!("Skip {name}: {e}"),
            }
        }
        Err(DiscordRpcError::ConnectionFailed(
            "No Discord named pipe found".into(),
        ))
    }

    // ── Handshake ─────────────────────────────────────────────────────────────

    async fn handshake(
        stream: &mut PlatformStream,
        client_id: &str,
    ) -> Result<u64, DiscordRpcError> {
        let payload = json!({ "v": 1, "client_id": client_id });
        write_frame(stream, OP_HANDSHAKE, &payload).await?;

        let (_, resp) = read_frame(stream).await?;

        let is_ready = resp.get("evt").and_then(Value::as_str) == Some("READY")
            || (resp.get("cmd").and_then(Value::as_str) == Some("DISPATCH")
                && resp.pointer("/data/evt").and_then(Value::as_str) == Some("READY"));

        if !is_ready {
            if resp.get("evt").and_then(Value::as_str) == Some("ERROR") {
                let msg = resp
                    .pointer("/data/message")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown");
                return Err(DiscordRpcError::HandshakeError(msg.to_string()));
            }
            return Err(DiscordRpcError::HandshakeError(format!(
                "Unexpected response: {resp}"
            )));
        }

        let interval = resp
            .pointer("/data/heartbeat_interval")
            .or_else(|| resp.get("heartbeat_interval"))
            .and_then(Value::as_u64)
            .unwrap_or_else(|| {
                warn!("heartbeat_interval missing, defaulting to {DEFAULT_HEARTBEAT_MS} ms");
                DEFAULT_HEARTBEAT_MS
            })
            .clamp(MIN_HEARTBEAT_MS, MAX_HEARTBEAT_MS);

        info!("Handshake OK, heartbeat interval: {interval}ms");
        Ok(interval)
    }

    // ── Read task ─────────────────────────────────────────────────────────────
    // Runs on its own task; only job is forwarding ACKs and detecting disconnects.

    async fn read_task(
        mut stream: tokio::io::ReadHalf<PlatformStream>,
        write_tx: mpsc::Sender<WriteCmd>,
    ) {
        loop {
            match read_frame(&mut stream).await {
                Ok((opcode, msg)) => match opcode {
                    OP_PING => {
                        debug!("PING received, replying PONG");
                        if write_tx.send(WriteCmd::Pong(msg)).await.is_err() {
                            break;
                        }
                    }
                    OP_PONG => debug!("Heartbeat acknowledged"),
                    OP_CLOSE => {
                        warn!("Discord closed the connection: {msg}");
                        break;
                    }
                    _ => {
                        // Errors are otherwise silent: a rejected SET_ACTIVITY
                        // only shows up here.
                        if msg.get("evt").and_then(Value::as_str) == Some("ERROR") {
                            let code = msg.pointer("/data/code").and_then(Value::as_u64);
                            let message = msg
                                .pointer("/data/message")
                                .and_then(Value::as_str)
                                .unwrap_or("unknown error");
                            warn!("Discord rejected a command (code {code:?}): {message}");
                        } else {
                            debug!("Discord response: {msg}");
                        }
                    }
                },
                Err(e) => {
                    error!("Read error: {e}");
                    // Let the session loop detect the task exited and reconnect.
                    break;
                }
            }
        }
    }

    // ── Write task ────────────────────────────────────────────────────────────
    // Drives heartbeats internally via tokio::time::interval — no extra task needed.

    async fn write_task(
        mut stream: tokio::io::WriteHalf<PlatformStream>,
        mut rx: mpsc::Receiver<WriteCmd>,
        client_id: String,
        heartbeat_ms: u64,
    ) {
        let mut heartbeat = time::interval(Duration::from_millis(heartbeat_ms));
        heartbeat.set_missed_tick_behavior(time::MissedTickBehavior::Delay);
        // interval() fires immediately; consume that tick so the first PING
        // goes out one full interval after the handshake.
        heartbeat.tick().await;

        loop {
            let cmd = tokio::select! {
                _ = heartbeat.tick() => {
                    if let Err(e) = write_frame(&mut stream, OP_PING, &json!({})).await {
                        error!("Heartbeat write error: {e}");
                        break;
                    }
                    continue;
                }
                cmd = rx.recv() => match cmd {
                    Some(cmd) => cmd,
                    None => break,
                },
            };

            let result = match &cmd {
                WriteCmd::SetActivity(activity) => {
                    let (activity_json, metadata) = build_activity_payload(activity);
                    let mut args = json!({
                        "pid": std::process::id(),
                        "activity": activity_json,
                        "client_id": &client_id,
                    });
                    if let Some(meta) = metadata {
                        args["metadata"] = meta;
                    }
                    let msg = json!({
                        "cmd": "SET_ACTIVITY",
                        "nonce": next_nonce(),
                        "args": args,
                    });
                    debug!(
                        "SET_ACTIVITY payload: {}",
                        serde_json::to_string(&msg).unwrap_or_default()
                    );
                    write_frame(&mut stream, OP_FRAME, &msg).await
                }
                WriteCmd::ClearActivity => {
                    let msg = json!({
                        "cmd": "SET_ACTIVITY",
                        "nonce": next_nonce(),
                        "args": {
                            "pid": std::process::id(),
                            "activity": null,
                            "client_id": &client_id,
                        }
                    });
                    write_frame(&mut stream, OP_FRAME, &msg).await
                }
                WriteCmd::Pong(payload) => write_frame(&mut stream, OP_PONG, payload).await,
                WriteCmd::Shutdown => {
                    debug!("Write task shutting down");
                    break;
                }
            };

            if let Err(e) = result {
                error!("Write error ({cmd:?}): {e}");
                break;
            }
        }
    }
}

// ── Wire protocol helpers (free functions, not methods) ───────────────────────

async fn write_frame(
    w: &mut (impl AsyncWrite + Unpin),
    opcode: u32,
    data: &Value,
) -> Result<(), DiscordRpcError> {
    let json = serde_json::to_string(data)?;
    let len = json.len() as u32;

    let mut buf = Vec::with_capacity(8 + json.len());
    buf.extend_from_slice(&opcode.to_le_bytes());
    buf.extend_from_slice(&len.to_le_bytes());
    buf.extend_from_slice(json.as_bytes());

    w.write_all(&buf).await?;
    w.flush().await?;
    Ok(())
}

async fn read_frame(r: &mut (impl AsyncRead + Unpin)) -> Result<(u32, Value), DiscordRpcError> {
    let mut header = [0u8; 8];
    r.read_exact(&mut header).await?;

    let opcode = u32::from_le_bytes(header[0..4].try_into().unwrap());
    let length = u32::from_le_bytes(header[4..8].try_into().unwrap());

    if length > MAX_FRAME_BYTES {
        return Err(DiscordRpcError::Protocol(format!(
            "frame too large: {length} bytes"
        )));
    }

    let mut body = vec![0u8; length as usize];
    r.read_exact(&mut body).await?;

    // PING/PONG frames carry no payload.
    if body.is_empty() {
        return Ok((opcode, Value::Null));
    }

    let value: Value = serde_json::from_slice(&body)?;
    Ok((opcode, value))
}

// ── Internal write commands ───────────────────────────────────────────────────

#[derive(Debug)]
enum WriteCmd {
    SetActivity(Box<Activity>),
    ClearActivity,
    /// Reply to an incoming PING, echoing its payload.
    Pong(Value),
    /// Flush and stop the writer.
    Shutdown,
}

// ── Activity wire payload builder ─────────────────────────────────────────────
// Returns (activity_value, Option<metadata_value>).
// Discord IPC requires buttons to be split: labels stay inside `activity`,
// URLs go in the sibling `metadata.button_urls` field.

fn build_activity_payload(activity: &Activity) -> (Value, Option<Value>) {
    // Discord IPC wants buttons as {label, url} objects directly inside activity.
    // The metadata split was incorrect — revert to plain serialization.
    let val = serde_json::to_value(activity).unwrap_or(json!({}));
    (val, None)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[path = "tests/client.rs"]
mod tests;
