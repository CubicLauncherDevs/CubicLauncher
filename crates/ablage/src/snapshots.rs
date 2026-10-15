//! Bounded, immutable read snapshots. Disk metadata is checked on every open;
//! writes remain explicit and each Repo uses copy-on-write isolation.

use super::{Entry, read_all};
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::SystemTime;

const MAX_ENTRIES: usize = 16;
const MAX_BYTES: usize = 16 * 1024 * 1024;
type Entries = Arc<HashMap<String, Entry>>;

#[derive(Clone, PartialEq, Eq)]
struct Stamp {
    modified: SystemTime,
    created: Option<SystemTime>,
    len: u64,
    #[cfg(unix)]
    identity: (u64, u64, i64, i64),
}

fn stamp(path: &Path) -> Option<Stamp> {
    let metadata = std::fs::metadata(path).ok()?;
    if !metadata.is_file() {
        return None;
    }
    Some(Stamp {
        modified: metadata.modified().ok()?,
        created: metadata.created().ok(),
        len: metadata.len(),
        #[cfg(unix)]
        identity: {
            use std::os::unix::fs::MetadataExt;
            (
                metadata.dev(),
                metadata.ino(),
                metadata.ctime(),
                metadata.ctime_nsec(),
            )
        },
    })
}

struct Snapshot {
    path: PathBuf,
    stamp: Stamp,
    entries: Entries,
    bytes: usize,
}

#[derive(Default)]
struct Cache {
    entries: VecDeque<Snapshot>,
    bytes: usize,
}

impl Cache {
    fn remove(&mut self, path: &Path) -> Option<Snapshot> {
        let index = self.entries.iter().position(|entry| entry.path == path)?;
        let entry = self.entries.remove(index)?;
        self.bytes -= entry.bytes;
        Some(entry)
    }

    fn get(&mut self, path: &Path, stamp: &Stamp) -> Option<Entries> {
        let entry = self.remove(path)?;
        if entry.stamp != *stamp {
            return None;
        }
        let entries = entry.entries.clone();
        self.bytes += entry.bytes;
        self.entries.push_back(entry);
        Some(entries)
    }

    fn insert(&mut self, path: PathBuf, stamp: Stamp, entries: Entries) {
        self.remove(&path);
        // Include retained string/vector capacity and a conservative hash-table
        // allowance, not just the serialized file size.
        let bytes = std::mem::size_of::<Snapshot>()
            + path.as_os_str().len()
            + entries.capacity() * (std::mem::size_of::<(String, Entry)>() + 16)
            + entries
                .iter()
                .map(|(key, entry)| key.capacity() + entry.data.capacity())
                .sum::<usize>();
        if bytes > MAX_BYTES {
            return;
        }
        while self.entries.len() >= MAX_ENTRIES || self.bytes + bytes > MAX_BYTES {
            if let Some(entry) = self.entries.pop_front() {
                self.bytes -= entry.bytes;
            }
        }
        self.bytes += bytes;
        self.entries.push_back(Snapshot {
            path,
            stamp,
            entries,
            bytes,
        });
    }
}

static CACHE: LazyLock<Mutex<Cache>> = LazyLock::new(|| Mutex::new(Cache::default()));

fn absolute(path: &Path) -> Option<PathBuf> {
    if path.is_absolute() {
        Some(path.to_owned())
    } else {
        Some(std::env::current_dir().ok()?.join(path))
    }
}

pub(super) fn open(path: &Path) -> Entries {
    let Some(key) = absolute(path) else {
        return Arc::new(read_all(path).unwrap_or_default());
    };
    let before = stamp(&key);
    if let Some(ref before) = before
        && let Some(entries) = CACHE
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&key, before)
    {
        return entries;
    }
    // Parsing and filesystem I/O must not hold the global cache lock.
    let Ok(entries) = read_all(&key) else {
        invalidate(&key);
        return Arc::default();
    };
    let entries = Arc::new(entries);
    if let Some(before) = before
        && stamp(&key).as_ref() == Some(&before)
    {
        CACHE
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(key, before, entries.clone());
    }
    entries
}

pub(super) fn invalidate(path: &Path) {
    if let Some(key) = absolute(path) {
        CACHE.lock().unwrap_or_else(|e| e.into_inner()).remove(&key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_stamp() -> Stamp {
        Stamp {
            modified: SystemTime::UNIX_EPOCH,
            created: None,
            len: 0,
            #[cfg(unix)]
            identity: (0, 0, 0, 0),
        }
    }

    #[test]
    fn eviction_is_bounded_and_reads_refresh_recency() {
        let mut cache = Cache::default();
        let entries = Arc::new(HashMap::new());
        for i in 0..MAX_ENTRIES {
            cache.insert(PathBuf::from(i.to_string()), test_stamp(), entries.clone());
        }
        assert!(cache.get(Path::new("0"), &test_stamp()).is_some());
        cache.insert(PathBuf::from("next"), test_stamp(), entries.clone());
        assert!(cache.get(Path::new("1"), &test_stamp()).is_none());
        assert!(cache.get(Path::new("0"), &test_stamp()).is_some());
        assert_eq!(cache.entries.len(), MAX_ENTRIES);
        let mut changed = test_stamp();
        changed.len = 1;
        assert!(cache.get(Path::new("0"), &changed).is_none());
        assert_eq!(cache.entries.len(), MAX_ENTRIES - 1);

        let large = Arc::new(HashMap::from([(
            "mod".into(),
            Entry {
                version: 1,
                fingerprint: 0,
                data: vec![0; MAX_BYTES / 2],
            },
        )]));
        for name in ["large-a", "large-b", "large-c"] {
            cache.insert(PathBuf::from(name), test_stamp(), large.clone());
            assert!(cache.bytes <= MAX_BYTES);
        }
        assert!(cache.get(Path::new("large-a"), &test_stamp()).is_none());
        assert!(cache.get(Path::new("large-c"), &test_stamp()).is_some());
    }
}
