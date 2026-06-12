use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

const MAGIC: &[u8; 4] = b"CREP";
const VERSION: u8 = 1;

pub struct Repo {
    path: PathBuf,
    entries: HashMap<String, Entry>,
    dirty: bool,
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub fingerprint: u64,
    pub data: Vec<u8>,
}

impl Repo {
    pub fn open(path: impl AsRef<Path>) -> Self {
        let path = path.as_ref().to_path_buf();
        let entries = if path.exists() {
            match read_all(&path) {
                Ok(e) => e,
                Err(_) => HashMap::new(),
            }
        } else {
            HashMap::new()
        };
        Repo { path, entries, dirty: false }
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

    /// XOR de todos los fingerprints — útil como fingerprint compuesto
    pub fn fingerprint(&self) -> u64 {
        self.entries.values().fold(0, |acc, e| acc ^ e.fingerprint)
    }

    pub fn flush(&mut self) -> Result<(), String> {
        if !self.dirty {
            return Ok(());
        }

        let mut buf = Vec::with_capacity(4096);

        // header
        buf.extend_from_slice(MAGIC);
        buf.push(VERSION);
        buf.extend_from_slice(&(self.entries.len() as u32).to_le_bytes());
        let hdr_crc = crc32fast::hash(&buf);
        buf.extend_from_slice(&hdr_crc.to_le_bytes());

        // entries
        let mut sorted: Vec<(&String, &Entry)> = self.entries.iter().collect();
        sorted.sort_by(|a, b| a.0.cmp(b.0));

        for (key, entry) in &sorted {
            let key_bytes = key.as_bytes();
            let key_len = key_bytes.len();
            assert!(key_len <= u16::MAX as usize, "key too long");

            buf.extend_from_slice(&(key_len as u16).to_le_bytes());
            buf.extend_from_slice(key_bytes);
            buf.extend_from_slice(&entry.fingerprint.to_le_bytes());
            buf.extend_from_slice(&(entry.data.len() as u32).to_le_bytes());
            let data_crc = crc32fast::hash(&entry.data);
            buf.extend_from_slice(&data_crc.to_le_bytes());
            buf.extend_from_slice(&entry.data);
        }

        // file crc (todo excepto estos 4 bytes)
        let file_crc = crc32fast::hash(&buf);
        buf.extend_from_slice(&file_crc.to_le_bytes());

        // atomic write: temp + sync + rename
        let tmp_path = self.path.with_extension("crep.tmp");
        let mut tmp = File::create(&tmp_path)
            .map_err(|e| format!("Failed to create temp file: {}", e))?;
        tmp.write_all(&buf)
            .map_err(|e| format!("Failed to write temp file: {}", e))?;
        tmp.sync_all()
            .map_err(|e| format!("Failed to sync temp file: {}", e))?;
        drop(tmp);
        fs::rename(&tmp_path, &self.path)
            .map_err(|e| format!("Failed to rename temp file: {}", e))?;

        // ensure parent dir fsync for directory metadata durability
        if let Some(parent) = self.path.parent() {
            if let Ok(dir) = File::open(parent) {
                let _ = dir.sync_all();
            }
        }

        self.dirty = false;
        Ok(())
    }
}

impl Drop for Repo {
    fn drop(&mut self) {
        if self.dirty {
            let _ = self.flush();
        }
    }
}

fn read_all(path: &Path) -> Result<HashMap<String, Entry>, String> {
    let mut file = File::open(path)
        .map_err(|e| format!("Failed to open repo file: {}", e))?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)
        .map_err(|e| format!("Failed to read repo file: {}", e))?;

    if buf.len() < 14 {
        return Err("File too small".into());
    }

    let file_crc_pos = buf.len() - 4;
    let stored_file_crc = u32::from_le_bytes(
        buf[file_crc_pos..].try_into().unwrap(),
    );
    let computed_file_crc = crc32fast::hash(&buf[..file_crc_pos]);
    if stored_file_crc != computed_file_crc {
        return Err("File CRC mismatch".into());
    }

    if &buf[..4] != MAGIC {
        return Err("Invalid magic".into());
    }
    if buf[4] != VERSION {
        return Err("Unsupported version".into());
    }

    let count = u32::from_le_bytes(buf[5..9].try_into().unwrap()) as usize;
    // verify header CRC
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
        let key_len = u16::from_le_bytes(buf[pos..pos+2].try_into().unwrap()) as usize;
        pos += 2;

        if pos + key_len > file_crc_pos {
            return Err("Unexpected EOF in key".into());
        }
        let key = String::from_utf8(buf[pos..pos+key_len].to_vec())
            .map_err(|_| String::from("Invalid UTF-8 key"))?;
        pos += key_len;

        if pos + 8 > file_crc_pos {
            return Err("Unexpected EOF in fingerprint".into());
        }
        let fingerprint = u64::from_le_bytes(buf[pos..pos+8].try_into().unwrap());
        pos += 8;

        if pos + 4 > file_crc_pos {
            return Err("Unexpected EOF in data length".into());
        }
        let data_len = u32::from_le_bytes(buf[pos..pos+4].try_into().unwrap()) as usize;
        pos += 4;

        if pos + 4 > file_crc_pos {
            return Err("Unexpected EOF in data CRC".into());
        }
        let data_crc = u32::from_le_bytes(buf[pos..pos+4].try_into().unwrap());
        pos += 4;

        if pos + data_len > file_crc_pos {
            return Err("Unexpected EOF in data".into());
        }
        let data = buf[pos..pos+data_len].to_vec();
        pos += data_len;

        let computed_data_crc = crc32fast::hash(&data);
        if data_crc != computed_data_crc {
            // data corrupt, skip this entry
            continue;
        }

        entries.insert(key, Entry { fingerprint, data });
    }

    Ok(entries)
}
