//! Java server-list status protocol (1.7+), with bounded and cancellable queries.
use super::server_manager::{error, icon_url};
use futures::{
    StreamExt,
    future::{AbortHandle, Abortable},
    stream,
};
use hickory_resolver::TokioResolver;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    net::IpAddr,
    sync::{Arc, LazyLock, Mutex, OnceLock, Weak},
    time::{Duration, Instant},
};
use tauri::ipc::Channel;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::Semaphore,
};

const MAX_PACKET: usize = 1024 * 1024;
static PING_SLOTS: Semaphore = Semaphore::const_new(4);
type Requests = HashMap<(String, String), AbortHandle>;
static REQUESTS: LazyLock<Mutex<Requests>> = LazyLock::new(|| Mutex::new(HashMap::new()));
static WINDOW_SLOTS: LazyLock<Mutex<HashMap<String, Weak<Semaphore>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn window_slots(window: &str) -> Arc<Semaphore> {
    let mut windows = WINDOW_SLOTS.lock().unwrap_or_else(|e| e.into_inner());
    windows.retain(|_, slots| slots.strong_count() > 0);
    if let Some(slots) = windows.get(window).and_then(Weak::upgrade) {
        return slots;
    }
    let slots = Arc::new(Semaphore::new(2));
    windows.insert(window.into(), Arc::downgrade(&slots));
    slots
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ServerAddress {
    pub host: String,
    pub port: u16,
}

impl ServerAddress {
    pub fn parse(address: &str) -> Result<Self, String> {
        let invalid = || error("SERVERS_INVALID", "address");
        let address = address.trim();
        if address.is_empty()
            || address.len() > 1024
            || address.chars().any(|c| c.is_whitespace() || c.is_control())
        {
            return Err(invalid());
        }
        let (host, port) = if let Some(rest) = address.strip_prefix('[') {
            let (host, suffix) = rest.split_once(']').ok_or_else(invalid)?;
            let ip = host.parse::<std::net::Ipv6Addr>().map_err(|_| invalid())?;
            let port = if suffix.is_empty() {
                25565
            } else {
                suffix
                    .strip_prefix(':')
                    .ok_or_else(invalid)?
                    .parse::<u16>()
                    .map_err(|_| invalid())?
            };
            (ip.to_string(), port)
        } else if let Ok(ip) = address.parse::<IpAddr>() {
            (ip.to_string(), 25565)
        } else {
            let (host, port) = match address.split_once(':') {
                Some((host, port)) => (host, port.parse::<u16>().map_err(|_| invalid())?),
                None => (address, 25565),
            };
            if host.contains(['/', '\\', '?', '#', '@', '%', '[', ']']) || host.is_empty() {
                return Err(invalid());
            }
            let host = url::Host::parse(host).map_err(|_| invalid())?.to_string();
            (host, port)
        };
        if port == 0 || host.len() > 255 {
            return Err(invalid());
        }
        Ok(Self { host, port })
    }

    pub fn authority(&self) -> String {
        if self.host.contains(':') {
            format!("[{}]:{}", self.host, self.port)
        } else {
            format!("{}:{}", self.host, self.port)
        }
    }
}

/// Preserve the hostname used in the handshake; SRV only changes the TCP target.
pub async fn resolve_target(address: &ServerAddress) -> ServerAddress {
    if address.port != 25565 || address.host.parse::<IpAddr>().is_ok() {
        return address.clone();
    }
    static RESOLVER: OnceLock<Option<TokioResolver>> = OnceLock::new();
    let resolver = RESOLVER.get_or_init(|| {
        TokioResolver::builder_tokio().ok().map(|mut b| {
            // Use one upstream at a time and a small TTL-aware cache.
            b.options_mut().cache_size = 32;
            b.options_mut().num_concurrent_reqs = 1;
            b.options_mut().preserve_intermediates = false;
            b.build()
        })
    });
    if let Some(resolver) = resolver
        && let Ok(Ok(records)) = tokio::time::timeout(
            Duration::from_secs(2),
            resolver.srv_lookup(format!("_minecraft._tcp.{}", address.host)),
        )
        .await
    {
        let min_priority = records.iter().map(|r| r.priority()).min();
        let records: Vec<_> = records
            .iter()
            .filter(|r| {
                Some(r.priority()) == min_priority && r.port() != 0 && !r.target().is_root()
            })
            .collect();
        let sum: u64 = records.iter().map(|r| u64::from(r.weight())).sum();
        let random = uuid::Uuid::new_v4().as_u128() as u64;
        let mut choice = if sum > 0 { random % sum } else { 0 };
        let selected = if sum == 0 {
            records
                .get((random as usize) % records.len().max(1))
                .copied()
        } else {
            records.iter().copied().find(|r| {
                let weight = u64::from(r.weight());
                if choice < weight {
                    true
                } else {
                    choice -= weight;
                    false
                }
            })
        };
        if let Some(record) = selected {
            return ServerAddress {
                host: record.target().to_utf8().trim_end_matches('.').into(),
                port: record.port(),
            };
        }
    }
    address.clone()
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerStatus {
    pub online: bool,
    pub players: Option<u32>,
    pub max_players: Option<u32>,
    pub ping: Option<u64>,
    pub motd: String,
    pub version: Option<String>,
    pub icon: Option<String>,
}

impl ServerStatus {
    fn unavailable() -> Self {
        Self {
            online: false,
            players: None,
            max_players: None,
            ping: None,
            motd: String::new(),
            version: None,
            icon: None,
        }
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerStatusEvent {
    pub indices: Vec<usize>,
    pub status: Option<ServerStatus>,
    pub done: bool,
}

pub fn cancel(window: &str, request_id: &str) {
    let handle = REQUESTS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .remove(&(window.into(), request_id.into()));
    if let Some(handle) = handle {
        handle.abort();
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct PingTarget {
    pub index: usize,
    pub address: String,
}

struct QueryGroup {
    address: Option<ServerAddress>,
    indices: Vec<usize>,
}

fn group_targets(targets: Vec<PingTarget>) -> Result<Vec<QueryGroup>, String> {
    if targets.len() > 51 {
        return Err(error("SERVERS_INVALID", "Too many ping targets"));
    }
    let mut indices = HashSet::new();
    let mut addresses: HashMap<ServerAddress, usize> = HashMap::new();
    let mut groups: Vec<QueryGroup> = Vec::new();
    for target in targets {
        if target.address.len() > 1024 || !indices.insert(target.index) {
            return Err(error("SERVERS_INVALID", "Invalid ping targets"));
        }
        let address = ServerAddress::parse(&target.address).ok();
        if let Some(address) = &address {
            if let Some(&group) = addresses.get(address) {
                groups[group].indices.push(target.index);
                continue;
            }
            addresses.insert(address.clone(), groups.len());
        }
        groups.push(QueryGroup {
            address,
            indices: vec![target.index],
        });
    }
    Ok(groups)
}

pub fn start(
    window: &str,
    targets: Vec<PingTarget>,
    channel: Channel<ServerStatusEvent>,
) -> Result<String, String> {
    let groups = group_targets(targets)?;
    let slots = window_slots(window);
    let id = uuid::Uuid::new_v4().to_string();
    let (abort, registration) = AbortHandle::new_pair();
    REQUESTS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert((window.into(), id.clone()), abort);
    let owner = window.to_string();
    let request_id = id.clone();
    tokio::spawn(async move {
        let task = async {
            let mut queries = stream::iter(groups)
                .map(|group| {
                    let slots = slots.clone();
                    async move {
                        let _slot = slots
                            .acquire_owned()
                            .await
                            .expect("window semaphore is never closed");
                        let status = match group.address {
                            Some(address) => query(&address).await,
                            None => ServerStatus::unavailable(),
                        };
                        ServerStatusEvent {
                            indices: group.indices,
                            status: Some(status),
                            done: false,
                        }
                    }
                })
                .buffer_unordered(2);
            while let Some(event) = queries.next().await {
                if channel.send(event).is_err() {
                    return;
                }
            }
            let _ = channel.send(ServerStatusEvent {
                indices: Vec::new(),
                status: None,
                done: true,
            });
        };
        let _ = Abortable::new(task, registration).await;
        REQUESTS
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&(owner, request_id));
        drop(slots);
        WINDOW_SLOTS
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .retain(|_, slots| slots.strong_count() > 0);
    });
    Ok(id)
}

async fn query(address: &ServerAddress) -> ServerStatus {
    let Ok(_slot) = PING_SLOTS.acquire().await else {
        return ServerStatus::unavailable();
    };
    tokio::time::timeout(Duration::from_secs(5), async {
        let target = resolve_target(address).await;
        let mut socket = TcpStream::connect((target.host.as_str(), target.port)).await?;
        socket.set_nodelay(true)?;
        exchange(&mut socket, address).await
    })
    .await
    .ok()
    .and_then(Result::ok)
    .unwrap_or_else(ServerStatus::unavailable)
}

fn invalid_packet() -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        "Invalid Minecraft status packet",
    )
}

fn put_varint(out: &mut Vec<u8>, mut value: u32) {
    loop {
        let mut byte = (value & 127) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 128;
        }
        out.push(byte);
        if value == 0 {
            break;
        }
    }
}

async fn read_varint(input: &mut (impl AsyncReadExt + Unpin)) -> std::io::Result<u32> {
    let mut result = 0;
    for shift in (0..35).step_by(7) {
        let byte = input.read_u8().await?;
        if shift == 28 && byte > 7 {
            return Err(invalid_packet());
        }
        result |= u32::from(byte & 127) << shift;
        if byte & 128 == 0 {
            return Ok(result);
        }
    }
    Err(invalid_packet())
}

async fn read_packet(input: &mut TcpStream) -> std::io::Result<Vec<u8>> {
    let length = read_varint(input).await? as usize;
    if length == 0 || length > MAX_PACKET {
        return Err(invalid_packet());
    }
    let mut bytes = vec![0; length];
    input.read_exact(&mut bytes).await?;
    Ok(bytes)
}

async fn send_packet(socket: &mut TcpStream, data: &[u8]) -> std::io::Result<()> {
    // Production writes here are small handshakes. Keep their frame on stack.
    if data.len() <= 507 {
        let mut frame = [0u8; 512];
        let mut length = data.len();
        let mut offset = 0;
        loop {
            frame[offset] = (length & 127) as u8;
            length >>= 7;
            if length != 0 {
                frame[offset] |= 128;
            }
            offset += 1;
            if length == 0 {
                break;
            }
        }
        frame[offset..offset + data.len()].copy_from_slice(data);
        return socket.write_all(&frame[..offset + data.len()]).await;
    }
    let mut frame = Vec::with_capacity(data.len() + 5);
    put_varint(&mut frame, data.len() as u32);
    frame.extend_from_slice(data);
    socket.write_all(&frame).await
}

fn description(value: &Value) -> String {
    fn walk(value: &Value, out: &mut String, depth: usize) {
        if depth > 32 || out.len() >= 4096 {
            return;
        }
        match value {
            Value::String(s) => {
                for c in s.chars() {
                    if out.len() + c.len_utf8() > 4096 {
                        break;
                    }
                    out.push(c);
                }
            }
            Value::Array(a) => {
                for item in a {
                    if out.len() >= 4096 {
                        break;
                    }
                    walk(item, out, depth + 1);
                }
            }
            Value::Object(o) => {
                if let Some(text) = o.get("text").or_else(|| o.get("translate")) {
                    walk(text, out, depth + 1);
                }
                if let Some(extra) = o.get("extra") {
                    walk(extra, out, depth + 1);
                }
            }
            _ => {}
        }
    }
    let mut out = String::new();
    walk(value, &mut out, 0);
    let mut skip = false;
    out.retain(|c| {
        if skip {
            skip = false;
            return false;
        }
        if c == '§' {
            skip = true;
            return false;
        }
        !c.is_control() || c == '\n'
    });
    out
}

#[derive(Default, Deserialize)]
struct Players {
    #[serde(default, deserialize_with = "optional_count")]
    online: Option<u32>,
    #[serde(default, deserialize_with = "optional_count")]
    max: Option<u32>,
}

fn optional_count<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<u32>, D::Error> {
    Ok(Value::deserialize(deserializer)?
        .as_u64()
        .and_then(|n| u32::try_from(n).ok()))
}

#[derive(Deserialize)]
#[serde(transparent)]
struct BorrowedText<'a>(#[serde(borrow)] Cow<'a, str>);

#[derive(Default, Deserialize)]
struct StatusVersion<'a> {
    #[serde(default, borrow)]
    name: Option<BorrowedText<'a>>,
}

#[derive(Deserialize)]
struct StatusResponse<'a> {
    #[serde(default)]
    description: Value,
    #[serde(default)]
    players: Option<Players>,
    #[serde(default, borrow)]
    version: Option<StatusVersion<'a>>,
    #[serde(default, borrow)]
    favicon: Option<BorrowedText<'a>>,
}

fn parse_status(input: &[u8]) -> std::io::Result<ServerStatus> {
    // Unknown fields (notably the player sample and mod metadata) are skipped.
    let json: StatusResponse<'_> = serde_json::from_slice(input).map_err(|_| invalid_packet())?;
    let players = json.players.unwrap_or_default();
    Ok(ServerStatus {
        online: true,
        players: players.online,
        max_players: players.max,
        ping: None,
        motd: description(&json.description),
        version: json
            .version
            .and_then(|v| v.name)
            .map(|name| name.0.chars().take(256).collect()),
        icon: json.favicon.and_then(|icon| {
            icon.0
                .strip_prefix("data:image/png;base64,")
                .and_then(icon_url)
        }),
    })
}

async fn exchange(
    socket: &mut TcpStream,
    address: &ServerAddress,
) -> std::io::Result<ServerStatus> {
    let mut handshake = Vec::with_capacity(address.host.len() + 10);
    handshake.push(0);
    put_varint(&mut handshake, 47);
    put_varint(&mut handshake, address.host.len() as u32);
    handshake.extend_from_slice(address.host.as_bytes());
    handshake.extend_from_slice(&address.port.to_be_bytes());
    handshake.push(1);
    send_packet(socket, &handshake).await?;
    drop(handshake);
    socket.write_all(&[1, 0]).await?;
    let mut status = {
        let packet = read_packet(socket).await?;
        let mut input = packet.as_slice();
        if read_varint(&mut input).await? != 0 {
            return Err(invalid_packet());
        }
        let length = read_varint(&mut input).await? as usize;
        if length != input.len() {
            return Err(invalid_packet());
        }
        parse_status(input)?
    }; // Release the potentially 1 MiB response before waiting for the pong.
    let nonce = (uuid::Uuid::new_v4().as_u128() as u64).to_be_bytes();
    let mut ping = [0u8; 10];
    ping[0] = 9;
    ping[1] = 1;
    ping[2..].copy_from_slice(&nonce);
    let start = Instant::now();
    socket.write_all(&ping).await?;
    if read_varint(socket).await? != 9 {
        return Err(invalid_packet());
    }
    let mut pong = [0u8; 9];
    socket.read_exact(&mut pong).await?;
    if pong != ping[1..] {
        return Err(invalid_packet());
    }
    status.ping = Some(start.elapsed().as_millis() as u64);
    Ok(status)
}

#[cfg(test)]
#[path = "server_status_tests.rs"]
mod tests;
