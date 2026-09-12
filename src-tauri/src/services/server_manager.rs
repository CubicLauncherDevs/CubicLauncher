//! Minecraft's uncompressed servers.dat, edited without discarding unknown NBT tags.
use base64::{Engine, engine::general_purpose::STANDARD};
use fastnbt::Value;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::{
    borrow::Cow,
    collections::HashMap,
    fs,
    io::{Read, Write},
    path::Path,
};

type Compound = HashMap<String, Value>;
pub type Result<T> = std::result::Result<T, String>;
const MAX_FILE_BYTES: u64 = 16 * 1024 * 1024;

#[cfg(test)]
pub(super) mod metrics {
    use std::cell::Cell;
    thread_local! {
        pub static READS: Cell<usize> = const { Cell::new(0) };
        pub static DECODES: Cell<usize> = const { Cell::new(0) };
    }
    pub fn reset() {
        READS.set(0);
        DECODES.set(0);
    }
    pub fn counts() -> (usize, usize) {
        (READS.get(), DECODES.get())
    }
}

pub fn error(code: &str, detail: impl std::fmt::Display) -> String {
    serde_json::json!({ "code": code, "params": { "detail": detail.to_string() } }).to_string()
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ResourcePolicy {
    #[default]
    Prompt,
    Enabled,
    Disabled,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerDto {
    pub index: usize,
    pub name: String,
    pub address: String,
    pub resource_policy: ResourcePolicy,
    pub has_icon: bool,
}

#[derive(Debug, Serialize)]
pub struct ServerIcon {
    pub index: usize,
    pub icon: Option<String>,
}

// Browsing only borrows the fields we display. Unknown mod data is skipped;
// edits still use the lossless Value tree below to preserve every NBT tag.
#[derive(Deserialize)]
struct ReadServers<'a> {
    #[serde(borrow)]
    servers: Vec<ReadServer<'a>>,
}

#[derive(Deserialize)]
struct ReadServer<'a> {
    #[serde(borrow)]
    name: Cow<'a, str>,
    #[serde(borrow)]
    ip: Cow<'a, str>,
    #[serde(default, borrow)]
    icon: Cow<'a, str>,
    #[serde(default, rename = "acceptTextures", deserialize_with = "read_policy")]
    resource_policy: ResourcePolicy,
}

fn read_policy<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<ResourcePolicy, D::Error> {
    Ok(match Value::deserialize(deserializer)? {
        Value::Byte(0) => ResourcePolicy::Disabled,
        Value::Byte(_) => ResourcePolicy::Enabled,
        _ => ResourcePolicy::Prompt,
    })
}

fn read_metadata(bytes: Option<&[u8]>) -> Result<ReadServers<'_>> {
    let Some(bytes) = bytes else {
        return Ok(ReadServers {
            servers: Vec::new(),
        });
    };
    validate_header(bytes)?;
    fastnbt::from_bytes(bytes).map_err(|e| error("SERVERS_READ", e))
}

fn validate_header(bytes: &[u8]) -> Result<()> {
    if !bytes.starts_with(&[10, 0, 0]) {
        return Err(error(
            "SERVERS_READ",
            "Expected an unnamed, uncompressed NBT compound",
        ));
    }
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct ServerList {
    pub revision: String,
    pub servers: Vec<ServerDto>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerInput {
    pub name: String,
    pub address: String,
    pub resource_policy: ResourcePolicy,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ServerAction {
    Add {
        server: ServerInput,
    },
    Edit {
        index: usize,
        server: ServerInput,
    },
    Delete {
        index: usize,
    },
    Move {
        index: usize,
        direction: MoveDirection,
    },
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MoveDirection {
    Up,
    Down,
}

fn read_bytes(path: &Path) -> Result<Option<Vec<u8>>> {
    let size = match fs::symlink_metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(error("SERVERS_READ", e)),
        Ok(meta) if !meta.is_file() => return Err(error("SERVERS_READ", "Not a regular file")),
        Ok(meta) => meta.len(),
    };
    if size > MAX_FILE_BYTES {
        return Err(error("SERVERS_READ", "servers.dat exceeds 16 MiB"));
    }
    let mut bytes = Vec::with_capacity(size as usize + 1);
    #[cfg(test)]
    metrics::READS.set(metrics::READS.get() + 1);
    fs::File::open(path)
        .and_then(|f| f.take(MAX_FILE_BYTES + 1).read_to_end(&mut bytes))
        .map_err(|e| error("SERVERS_READ", e))?;
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return Err(error("SERVERS_READ", "servers.dat exceeds 16 MiB"));
    }
    Ok(Some(bytes))
}

fn revision(bytes: Option<&[u8]>) -> String {
    bytes
        .map(|b| STANDARD.encode(Sha1::digest(b)))
        .unwrap_or_else(|| "missing".into())
}

// Conflict checking must not allocate a second full file alongside the NBT
// tree, original backup bytes and newly serialized output.
fn current_revision(path: &Path) -> Result<String> {
    match fs::symlink_metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok("missing".into()),
        Err(e) => return Err(error("SERVERS_READ", e)),
        Ok(meta) if !meta.is_file() || meta.len() > MAX_FILE_BYTES => {
            return Err(error("SERVERS_READ", "Invalid servers.dat"));
        }
        Ok(_) => {}
    }
    let mut input = fs::File::open(path)
        .map_err(|e| error("SERVERS_READ", e))?
        .take(MAX_FILE_BYTES + 1);
    #[cfg(test)]
    metrics::READS.set(metrics::READS.get() + 1);
    let mut hash = Sha1::new();
    let mut buffer = [0; 32 * 1024];
    let mut total = 0u64;
    loop {
        let read = input
            .read(&mut buffer)
            .map_err(|e| error("SERVERS_READ", e))?;
        if read == 0 {
            break;
        }
        total += read as u64;
        if total > MAX_FILE_BYTES {
            return Err(error("SERVERS_READ", "servers.dat exceeds 16 MiB"));
        }
        hash.update(&buffer[..read]);
    }
    Ok(STANDARD.encode(hash.finalize()))
}

fn parse(bytes: Option<&[u8]>) -> Result<Compound> {
    let Some(bytes) = bytes else {
        return Ok(HashMap::from([("servers".into(), Value::List(Vec::new()))]));
    };
    validate_header(bytes)?;
    let root: Compound = fastnbt::from_bytes(bytes).map_err(|e| error("SERVERS_READ", e))?;
    let Some(Value::List(servers)) = root.get("servers") else {
        return Err(error("SERVERS_READ", "Missing servers list"));
    };
    for entry in servers {
        let Value::Compound(entry) = entry else {
            return Err(error("SERVERS_READ", "Invalid server entry"));
        };
        for key in ["name", "ip"] {
            if !matches!(entry.get(key), Some(Value::String(_))) {
                return Err(error("SERVERS_READ", format!("Invalid server {key}")));
            }
        }
    }
    Ok(root)
}

/// Only expose small, decoded PNGs to the WebView; retain the original NBT value.
pub fn icon_url(encoded: &str) -> Option<String> {
    let encoded = encoded
        .strip_prefix("data:image/png;base64,")
        .unwrap_or(encoded);
    if encoded.len() > 88_000 {
        return None;
    }
    #[cfg(test)]
    metrics::DECODES.set(metrics::DECODES.get() + 1);
    let bytes = STANDARD.decode(encoded).ok()?;
    if bytes.len() > 64 * 1024 {
        return None;
    }
    let reader =
        image::ImageReader::with_format(std::io::Cursor::new(&bytes), image::ImageFormat::Png);
    let (w, h) = reader.into_dimensions().ok()?;
    if w == 0 || h == 0 || w > 64 || h > 64 {
        return None;
    }
    image::load_from_memory_with_format(&bytes, image::ImageFormat::Png).ok()?;
    Some(format!("data:image/png;base64,{encoded}"))
}

fn snapshot(root: &Compound, revision: String) -> ServerList {
    let Some(Value::List(entries)) = root.get("servers") else {
        unreachable!("validated NBT")
    };
    let servers = entries
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let Value::Compound(entry) = value else {
                unreachable!("validated NBT")
            };
            let text = |key: &str| match entry.get(key) {
                Some(Value::String(s)) => s.as_str(),
                _ => "",
            };
            ServerDto {
                index,
                name: text("name").into(),
                address: text("ip").into(),
                resource_policy: match entry.get("acceptTextures") {
                    Some(Value::Byte(0)) => ResourcePolicy::Disabled,
                    Some(Value::Byte(_)) => ResourcePolicy::Enabled,
                    _ => ResourcePolicy::Prompt,
                },
                has_icon: !text("icon").is_empty(),
            }
        })
        .collect();
    ServerList { revision, servers }
}

pub fn list(instance: &Path) -> Result<ServerList> {
    let bytes = read_bytes(&instance.join("servers.dat"))?;
    let metadata = read_metadata(bytes.as_deref())?;
    let servers = metadata
        .servers
        .into_iter()
        .enumerate()
        .map(|(index, entry)| ServerDto {
            index,
            name: entry.name.into_owned(),
            address: entry.ip.into_owned(),
            resource_policy: entry.resource_policy,
            has_icon: !entry.icon.is_empty(),
        })
        .collect();
    Ok(ServerList {
        revision: revision(bytes.as_deref()),
        servers,
    })
}

/// One page plus the selected row. The file buffer is released after this call.
pub fn icons(
    instance: &Path,
    expected_revision: &str,
    mut indices: Vec<usize>,
) -> Result<Vec<ServerIcon>> {
    if indices.len() > 51 {
        return Err(error("SERVERS_INVALID", "Too many icons requested"));
    }
    indices.sort_unstable();
    indices.dedup();
    if indices.is_empty() {
        return Ok(Vec::new());
    }
    let bytes = read_bytes(&instance.join("servers.dat"))?;
    if revision(bytes.as_deref()) != expected_revision {
        return Err(error("SERVERS_CONFLICT", ""));
    }
    let metadata = read_metadata(bytes.as_deref())?;
    indices
        .into_iter()
        .map(|index| {
            let server = metadata
                .servers
                .get(index)
                .ok_or_else(|| error("SERVERS_CONFLICT", ""))?;
            Ok(ServerIcon {
                index,
                icon: icon_url(&server.icon),
            })
        })
        .collect()
}

fn set_fields(entry: &mut Compound, input: ServerInput) -> Result<()> {
    let name = input.name.trim();
    if name.is_empty() || name.len() > 1024 || name.chars().any(char::is_control) {
        return Err(error("SERVERS_INVALID", "name"));
    }
    let address = input.address.trim();
    super::server_status::ServerAddress::parse(address)?;
    if !matches!(entry.get("ip"), Some(Value::String(old)) if old == address) {
        entry.remove("icon");
    }
    entry.insert("name".into(), Value::String(name.into()));
    entry.insert("ip".into(), Value::String(address.into()));
    match input.resource_policy {
        ResourcePolicy::Prompt => {
            entry.remove("acceptTextures");
        }
        ResourcePolicy::Enabled => {
            entry.insert("acceptTextures".into(), Value::Byte(1));
        }
        ResourcePolicy::Disabled => {
            entry.insert("acceptTextures".into(), Value::Byte(0));
        }
    }
    Ok(())
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| error("SERVERS_WRITE", "Invalid path"))?;
    let mut file =
        tempfile::NamedTempFile::new_in(parent).map_err(|e| error("SERVERS_WRITE", e))?;
    file.write_all(bytes)
        .and_then(|()| file.as_file().sync_all())
        .map_err(|e| error("SERVERS_WRITE", e))?;
    file.persist(path).map_err(|e| error("SERVERS_WRITE", e))?;
    Ok(())
}

/// Caller holds the instance file lock and has checked that Minecraft is stopped.
pub fn apply(instance: &Path, expected_revision: &str, action: ServerAction) -> Result<ServerList> {
    let path = instance.join("servers.dat");
    let bytes = read_bytes(&path)?;
    if revision(bytes.as_deref()) != expected_revision {
        return Err(error("SERVERS_CONFLICT", ""));
    }
    let mut root = parse(bytes.as_deref())?;
    let Some(Value::List(entries)) = root.get_mut("servers") else {
        unreachable!("validated NBT")
    };
    match action {
        ServerAction::Add { server } => {
            let mut entry = Compound::new();
            set_fields(&mut entry, server)?;
            entries.push(Value::Compound(entry));
        }
        ServerAction::Edit { index, server } => {
            let Some(Value::Compound(entry)) = entries.get_mut(index) else {
                return Err(error("SERVERS_CONFLICT", ""));
            };
            set_fields(entry, server)?;
        }
        ServerAction::Delete { index } => {
            if index >= entries.len() {
                return Err(error("SERVERS_CONFLICT", ""));
            }
            entries.remove(index);
        }
        ServerAction::Move { index, direction } => {
            let other = match direction {
                MoveDirection::Up => index.checked_sub(1),
                MoveDirection::Down => index.checked_add(1),
            };
            let Some(other) = other.filter(|&other| other < entries.len() && index < entries.len())
            else {
                return Err(error("SERVERS_CONFLICT", ""));
            };
            entries.swap(index, other);
        }
    }
    let out = fastnbt::to_bytes(&root).map_err(|e| error("SERVERS_WRITE", e))?;
    if out.len() as u64 > MAX_FILE_BYTES {
        return Err(error("SERVERS_WRITE", "servers.dat exceeds 16 MiB"));
    }
    // Detect edits made by another program since the read, before replacing files.
    if current_revision(&path)? != expected_revision {
        return Err(error("SERVERS_CONFLICT", ""));
    }
    if let Some(bytes) = bytes {
        atomic_write(&instance.join("servers.dat_old"), &bytes)?;
    }
    atomic_write(&path, &out)?;
    Ok(snapshot(&root, revision(Some(&out))))
}

#[cfg(test)]
#[path = "server_manager_tests.rs"]
mod tests;
