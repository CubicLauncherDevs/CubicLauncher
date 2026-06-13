use crate::error::DiscordRpcError;
use crate::models::Activity;
use serde_json::{Value, json};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
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
    /// ```rust
    /// let mut client = DiscordRpcClient::new("app_id");
    /// client.connect().await?;
    /// client.set_activity(activity).await?;
    /// ```
    pub async fn connect(&mut self) -> Result<(), DiscordRpcError> {
        if let Some(rx) = self.ready_rx.take() {
            rx.await.map_err(|_| DiscordRpcError::NotConnected)?;
        }
        Ok(())
    }

    pub async fn set_activity(&self, activity: Activity) -> Result<(), DiscordRpcError> {
        self.command_tx
            .send(ClientCommand::SetActivity(Box::new(activity)))
            .await
            .map_err(|_| DiscordRpcError::NotConnected)
    }

    pub async fn clear_activity(&self) -> Result<(), DiscordRpcError> {
        self.command_tx
            .send(ClientCommand::ClearActivity)
            .await
            .map_err(|_| DiscordRpcError::NotConnected)
    }

    pub async fn shutdown(self) {
        self.command_tx.try_send(ClientCommand::Shutdown).ok();
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

        loop {
            match stop_rx.try_recv() {
                Ok(()) | Err(TryRecvError::Closed) => {
                    info!("Stop signal received, exiting connection loop");
                    return;
                }
                Err(TryRecvError::Empty) => {}
            }

            match Self::session(
                &client_id,
                &mut command_rx,
                &mut stop_rx,
                &mut ready_tx,
                &mut last_activity,
            )
            .await
            {
                Ok(true) => {
                    info!("Shutdown requested, exiting");
                    return;
                }
                Ok(false) => debug!("Session ended, reconnecting in 5s"),
                Err(e) => warn!("Session error: {e}, reconnecting in 5s"),
            }

            tokio::select! {
                _ = time::sleep(Duration::from_secs(5)) => {}
                _ = &mut stop_rx => {
                    info!("Stop signal received during reconnect wait");
                    return;
                }
            }
        }
    }

    #[instrument(skip(command_rx, stop_rx, ready_tx, last_activity))]
    async fn session(
        client_id: &str,
        command_rx: &mut mpsc::Receiver<ClientCommand>,
        stop_rx: &mut oneshot::Receiver<()>,
        ready_tx: &mut Option<oneshot::Sender<()>>,
        last_activity: &mut Option<Activity>,
    ) -> Result<bool, DiscordRpcError> {
        let mut stream = Self::open_socket().await?;
        let _heartbeat_ms = Self::handshake(&mut stream, client_id).await?;

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

        let mut read_handle = tokio::spawn(Self::read_task(read_half));
        let mut write_handle = tokio::spawn(Self::write_task(
            write_half,
            write_rx,
            client_id.to_string(),
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
        write_frame(stream, 0, &payload).await?;

        let resp = read_frame(stream).await?;

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
                warn!("heartbeat_interval missing, defaulting to 45 000 ms");
                45_000
            });

        info!("Handshake OK, heartbeat interval: {interval}ms");
        Ok(interval)
    }

    // ── Read task ─────────────────────────────────────────────────────────────
    // Runs on its own task; only job is forwarding ACKs and detecting disconnects.

    async fn read_task(mut stream: tokio::io::ReadHalf<PlatformStream>) {
        loop {
            match read_frame(&mut stream).await {
                Ok(msg) => {
                    // Opcode 1 = heartbeat ACK — silently ignore
                    if msg.get("op").and_then(Value::as_u64) == Some(1) {
                        continue;
                    }
                    debug!("Discord response: {msg}");
                }
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
    ) {
        loop {
            tokio::select! {
                Some(cmd) = rx.recv() => {
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
                            debug!("SET_ACTIVITY payload: {}", serde_json::to_string(&msg).unwrap_or_default());
                            write_frame(&mut stream, 1, &msg).await
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
                            write_frame(&mut stream, 1, &msg).await
                        }
                    };

                    if let Err(e) = result {
                        error!("Write error ({cmd:?}): {e}");
                        break;
                    }
                }
                else => break,
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

async fn read_frame(r: &mut (impl AsyncRead + Unpin)) -> Result<Value, DiscordRpcError> {
    let mut header = [0u8; 8];
    r.read_exact(&mut header).await?;

    let _opcode = u32::from_le_bytes(header[0..4].try_into().unwrap());
    let length = u32::from_le_bytes(header[4..8].try_into().unwrap());

    let mut body = vec![0u8; length as usize];
    r.read_exact(&mut body).await?;

    let value: Value = serde_json::from_slice(&body)?;
    Ok(value)
}

// ── Internal write commands ───────────────────────────────────────────────────

#[derive(Debug)]
enum WriteCmd {
    SetActivity(Box<Activity>),
    ClearActivity,
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
mod tests {
    use super::*;

    #[test]
    fn nonce_increments() {
        let a = next_nonce().parse::<u64>().unwrap();
        let b = next_nonce().parse::<u64>().unwrap();
        assert_eq!(b, a + 1);
    }

    #[test]
    fn buttons_split_into_labels_and_metadata() {
        use crate::models::Button;

        let activity = Activity::builder()
            .button("GitHub", "https://github.com")
            .button("Docs", "https://docs.rs")
            .build();

        let (val, meta) = build_activity_payload(&activity);

        // Labels stay in activity
        let labels = val["buttons"].as_array().unwrap();
        assert_eq!(labels[0]["label"], "GitHub");
        assert_eq!(labels[0].get("url"), None); // url must NOT be here

        // URLs go in metadata
        let meta = meta.unwrap();
        assert_eq!(meta["button_urls"][0], "https://github.com");
        assert_eq!(meta["button_urls"][1], "https://docs.rs");
    }

    #[tokio::test]
    async fn client_stores_id() {
        let client = DiscordRpcClient::new("123456789");
        assert_eq!(client.client_id, "123456789");
    }
}
