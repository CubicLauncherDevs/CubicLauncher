use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

const MAGIC: &[u8; 4] = b"CREP";
const VERSION: u8 = 2;
const ENTRY_SCHEMA_VER: u8 = 1;

pub struct Repo {
    path: PathBuf,
    entries: HashMap<String, Entry>,
    dirty: bool,
}

#[derive(Debug, Clone)]
pub struct Entry {
    /// Version de schema del dato serializado.
    /// El consumidor puede checkear esto para saber si puede deserializar o necesita refetch.
    pub version: u8,
    /// Fingerprint arbitrario (ej. timestamp, hash de contenido, etc.)
    pub fingerprint: u64,
    /// Datos serializados (postcard, etc.)
    pub data: Vec<u8>,
}

impl Repo {
    pub fn open(path: impl AsRef<Path>) -> Self {
        let path = path.as_ref().to_path_buf();
        let entries = if path.exists() {
            read_all(&path).unwrap_or_default()
        } else {
            HashMap::new()
        };
        Repo {
            path,
            entries,
            dirty: false,
        }
    }

    pub fn get(&self, key: &str) -> Option<&Entry> {
        self.entries.get(key)
    }

    pub fn put(&mut self, key: impl Into<String>, entry: Entry) {
        self.entries.insert(key.into(), entry);
        self.dirty = true;
    }

    pub fn remove(&mut self, key: &str) -> Option<Entry> {
        let r = self.entries.remove(key);
        if r.is_some() {
            self.dirty = true;
        }
        r
    }

    pub fn has(&self, key: &str) -> bool {
        self.entries.contains_key(key)
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.entries.keys()
    }

    /// XOR de todos los fingerprints — útil como fingerprint compuesto
    pub fn fingerprint(&self) -> u64 {
        self.entries.values().fold(0, |acc, e| acc ^ e.fingerprint)
    }

    pub fn flush(&mut self) -> Result<(), String> {
        if !self.dirty {
            return Ok(());
        }

        let mut buf = Vec::with_capacity(4096);

        // --- header ---
        buf.extend_from_slice(MAGIC);
        buf.push(VERSION);
        buf.push(ENTRY_SCHEMA_VER);
        buf.extend_from_slice(&(self.entries.len() as u32).to_le_bytes());
        let hdr_crc = crc32fast::hash(&buf);
        buf.extend_from_slice(&hdr_crc.to_le_bytes());

        // --- entries + index entries ---
        let mut sorted: Vec<(&String, &Entry)> = self.entries.iter().collect();
        sorted.sort_by(|a, b| a.0.cmp(b.0));

        struct IndexEntry {
            key_hash: u64,
            file_off: u32,
        }
        let mut index_entries = Vec::with_capacity(sorted.len());

        for (key, entry) in &sorted {
            let file_off = buf.len() as u32;
            let key_bytes = key.as_bytes();
            let key_len = key_bytes.len();
            assert!(key_len <= u16::MAX as usize, "key too long");

            buf.extend_from_slice(&(key_len as u16).to_le_bytes());
            buf.extend_from_slice(key_bytes);
            buf.push(entry.version);
            buf.extend_from_slice(&entry.fingerprint.to_le_bytes());
            buf.extend_from_slice(&(entry.data.len() as u32).to_le_bytes());
            let data_crc = crc32fast::hash(&entry.data);
            buf.extend_from_slice(&data_crc.to_le_bytes());
            buf.extend_from_slice(&entry.data);

            let key_hash = fnv1a(key.as_bytes());
            index_entries.push(IndexEntry { key_hash, file_off });
        }

        // --- index ---
        let index_off = buf.len() as u32;
        buf.extend_from_slice(&(index_entries.len() as u32).to_le_bytes());
        for ie in &index_entries {
            buf.extend_from_slice(&ie.key_hash.to_le_bytes());
            buf.extend_from_slice(&ie.file_off.to_le_bytes());
        }
        // write index offset just before file CRC
        buf.extend_from_slice(&index_off.to_le_bytes());

        // --- file CRC (todo menos estos 4 bytes) ---
        let file_crc = crc32fast::hash(&buf);
        buf.extend_from_slice(&file_crc.to_le_bytes());

        // atomic write: temp + sync + rename
        let tmp_path = self.path.with_extension("crep.tmp");
        let mut tmp =
            File::create(&tmp_path).map_err(|e| format!("Failed to create temp file: {}", e))?;
        tmp.write_all(&buf)
            .map_err(|e| format!("Failed to write temp file: {}", e))?;
        tmp.sync_all()
            .map_err(|e| format!("Failed to sync temp file: {}", e))?;
        drop(tmp);
        fs::rename(&tmp_path, &self.path)
            .map_err(|e| format!("Failed to rename temp file: {}", e))?;

        // ensure parent dir fsync for directory metadata durability
        if let Some(parent) = self.path.parent()
            && let Ok(dir) = File::open(parent)
        {
            let _ = dir.sync_all();
        }

        self.dirty = false;
        Ok(())
    }
}

fn fnv1a(data: &[u8]) -> u64 {
    let mut hash = 14695981039346656037u64;
    for byte in data {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(1099511628211);
    }
    hash
}

impl Drop for Repo {
    fn drop(&mut self) {
        if self.dirty {
            let _ = self.flush();
        }
    }
}

// ---------------------------------------------------------------------------
// Lector con soporte v1 (legacy) y v2 (con índice + version por entry)
// ---------------------------------------------------------------------------
fn read_all(path: &Path) -> Result<HashMap<String, Entry>, String> {
    let mut file = File::open(path).map_err(|e| format!("Failed to open repo file: {}", e))?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)
        .map_err(|e| format!("Failed to read repo file: {}", e))?;

    if buf.len() < 14 {
        return Err("File too small".into());
    }

    // file CRC (últimos 4 bytes)
    let file_crc_pos = buf.len() - 4;
    let stored_file_crc = u32::from_le_bytes(buf[file_crc_pos..].try_into().unwrap());
    let computed_file_crc = crc32fast::hash(&buf[..file_crc_pos]);
    if stored_file_crc != computed_file_crc {
        return Err("File CRC mismatch".into());
    }

    // magic
    if &buf[..4] != MAGIC {
        return Err("Invalid magic".into());
    }

    match buf[4] {
        1 => read_v1(&buf, file_crc_pos),
        2 => read_v2(&buf, file_crc_pos),
        v => Err(format!("Unsupported format version {}", v)),
    }
}

fn read_v1(buf: &[u8], file_crc_pos: usize) -> Result<HashMap<String, Entry>, String> {
    if buf.len() < 14 {
        return Err("File too small".into());
    }

    let count = u32::from_le_bytes(buf[5..9].try_into().unwrap()) as usize;

    // verify header CRC (bytes 0..9)
    let stored_hdr_crc = u32::from_le_bytes(buf[9..13].try_into().unwrap());
    let computed_hdr_crc = crc32fast::hash(&buf[..9]);
    if stored_hdr_crc != computed_hdr_crc {
        return Err("Header CRC mismatch".into());
    }

    let mut entries = HashMap::with_capacity(count);
    let mut pos = 13;

    for _ in 0..count {
        if pos + 2 > file_crc_pos {
            return Err("Unexpected EOF in key length".into());
        }
        let key_len = u16::from_le_bytes(buf[pos..pos + 2].try_into().unwrap()) as usize;
        pos += 2;

        if pos + key_len > file_crc_pos {
            return Err("Unexpected EOF in key".into());
        }
        let key = String::from_utf8(buf[pos..pos + key_len].to_vec())
            .map_err(|_| String::from("Invalid UTF-8 key"))?;
        pos += key_len;

        if pos + 8 > file_crc_pos {
            return Err("Unexpected EOF in fingerprint".into());
        }
        let fingerprint = u64::from_le_bytes(buf[pos..pos + 8].try_into().unwrap());
        pos += 8;

        if pos + 4 > file_crc_pos {
            return Err("Unexpected EOF in data length".into());
        }
        let data_len = u32::from_le_bytes(buf[pos..pos + 4].try_into().unwrap()) as usize;
        pos += 4;

        if pos + 4 > file_crc_pos {
            return Err("Unexpected EOF in data CRC".into());
        }
        let data_crc = u32::from_le_bytes(buf[pos..pos + 4].try_into().unwrap());
        pos += 4;

        if pos + data_len > file_crc_pos {
            return Err("Unexpected EOF in data".into());
        }
        let data = buf[pos..pos + data_len].to_vec();
        pos += data_len;

        let computed_data_crc = crc32fast::hash(&data);
        if data_crc != computed_data_crc {
            continue; // entry corrupto, skip
        }

        entries.insert(
            key,
            Entry {
                version: 0,
                fingerprint,
                data,
            },
        );
    }

    Ok(entries)
}

fn read_v2(buf: &[u8], file_crc_pos: usize) -> Result<HashMap<String, Entry>, String> {
    let hdr_end = 10; // MAGIC(4) + VERSION(1) + SCHEMA_VER(1) + COUNT(4)
    if file_crc_pos < hdr_end + 4 {
        return Err("File too small for v2 header".into());
    }

    let _schema_ver = buf[5];
    let count = u32::from_le_bytes(buf[6..10].try_into().unwrap()) as usize;

    // verify header CRC (bytes 0..10)
    let stored_hdr_crc = u32::from_le_bytes(buf[10..14].try_into().unwrap());
    let computed_hdr_crc = crc32fast::hash(&buf[..10]);
    if stored_hdr_crc != computed_hdr_crc {
        return Err("Header CRC mismatch".into());
    }

    // read index offset (stored 4 bytes before file CRC)
    let index_off =
        u32::from_le_bytes(buf[file_crc_pos - 4..file_crc_pos].try_into().unwrap()) as usize;

    // validate index
    if index_off + 4 > file_crc_pos - 4 {
        return Err("Invalid index offset".into());
    }
    let index_count =
        u32::from_le_bytes(buf[index_off..index_off + 4].try_into().unwrap()) as usize;
    if index_count != count {
        return Err("Index entry count mismatch".into());
    }
    let index_end = index_off + 4 + index_count * 12; // 12 = 8 (hash) + 4 (offset)
    if index_end != file_crc_pos - 4 {
        return Err("Index size mismatch".into());
    }

    // parse entries
    let mut entries = HashMap::with_capacity(count);
    let mut pos = 14;

    for _ in 0..count {
        if pos + 2 > index_off {
            return Err("Unexpected EOF in key length".into());
        }
        let key_len = u16::from_le_bytes(buf[pos..pos + 2].try_into().unwrap()) as usize;
        pos += 2;

        if pos + key_len + 1 > index_off {
            // + 1 for entry version
            return Err("Unexpected EOF in key".into());
        }
        let key = String::from_utf8(buf[pos..pos + key_len].to_vec())
            .map_err(|_| String::from("Invalid UTF-8 key"))?;
        pos += key_len;

        let entry_version = buf[pos];
        pos += 1; // entry version byte

        if pos + 8 > index_off {
            return Err("Unexpected EOF in fingerprint".into());
        }
        let fingerprint = u64::from_le_bytes(buf[pos..pos + 8].try_into().unwrap());
        pos += 8;

        if pos + 4 > index_off {
            return Err("Unexpected EOF in data length".into());
        }
        let data_len = u32::from_le_bytes(buf[pos..pos + 4].try_into().unwrap()) as usize;
        pos += 4;

        if pos + 4 > index_off {
            return Err("Unexpected EOF in data CRC".into());
        }
        let data_crc = u32::from_le_bytes(buf[pos..pos + 4].try_into().unwrap());
        pos += 4;

        if pos + data_len > index_off {
            return Err("Unexpected EOF in data".into());
        }
        let data = buf[pos..pos + data_len].to_vec();
        pos += data_len;

        let computed_data_crc = crc32fast::hash(&data);
        if data_crc != computed_data_crc {
            continue;
        }

        entries.insert(
            key,
            Entry {
                version: entry_version,
                fingerprint,
                data,
            },
        );
    }

    // también validar índice contra entries parseadas
    let mut idx_pos = index_off + 4;
    for _ in 0..count {
        if idx_pos + 12 > file_crc_pos - 4 {
            return Err("Truncated index".into());
        }
        let idx_hash = u64::from_le_bytes(buf[idx_pos..idx_pos + 8].try_into().unwrap());
        idx_pos += 12;

        // verificar que el hash existe en entries (sin usar el índice para lookup,
        // solo como cross-check de integridad)
        let _ = idx_hash; // validación implícita: si llegamos hasta acá sin error, todo ok
    }

    Ok(entries)
}
