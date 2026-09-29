//! Persistent remote link status cache (`~/.cache/explicit/links.json`).

use std::collections::HashMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use super::remote::RemoteStatus;

/// Entries older than this are dropped on save, even for offline use.
const MAX_AGE_SECS: u64 = 30 * 24 * 3600;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Entry {
    pub status: RemoteStatus,
    pub checked_at_unix: u64,
}

#[derive(Debug, Default)]
pub struct Cache {
    path: Option<PathBuf>,
    entries: HashMap<String, Entry>,
    /// Entries written in this run, merged into the file on save.
    dirty: HashMap<String, Entry>,
}

pub fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// `$XDG_CACHE_HOME/explicit/links.json`, else `~/.cache/explicit/links.json`.
pub fn default_path() -> Option<PathBuf> {
    path_from(std::env::var_os("XDG_CACHE_HOME"), std::env::var_os("HOME"))
}

fn path_from(xdg: Option<OsString>, home: Option<OsString>) -> Option<PathBuf> {
    let base = match xdg.map(PathBuf::from).filter(|p| p.is_absolute()) {
        Some(x) => x,
        None => PathBuf::from(home.filter(|h| !h.is_empty())?).join(".cache"),
    };
    Some(base.join("explicit").join("links.json"))
}

fn read(path: &Path) -> HashMap<String, Entry> {
    std::fs::read(path)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

impl Cache {
    /// Load the cache; a missing or corrupt file yields an empty cache.
    pub fn load(path: Option<PathBuf>) -> Cache {
        let entries = path.as_deref().map(read).unwrap_or_default();
        Cache {
            path,
            entries,
            dirty: HashMap::new(),
        }
    }

    pub fn get(&self, url: &str) -> Option<&Entry> {
        self.entries.get(url)
    }

    /// Cached status if it is younger than its TTL (success and failure TTLs differ).
    pub fn fresh(
        &self,
        url: &str,
        now: u64,
        ok_ttl_hours: u64,
        fail_ttl_hours: u64,
    ) -> Option<&RemoteStatus> {
        let e = self.entries.get(url)?;
        let ttl = if matches!(e.status, RemoteStatus::Ok | RemoteStatus::Redirect(_)) {
            ok_ttl_hours
        } else {
            fail_ttl_hours
        };
        (now.saturating_sub(e.checked_at_unix) < ttl * 3600).then_some(&e.status)
    }

    pub fn insert(&mut self, url: String, status: RemoteStatus, now: u64) {
        if status == RemoteStatus::Skipped {
            return;
        }
        let e = Entry {
            status,
            checked_at_unix: now,
        };
        self.entries.insert(url.clone(), e.clone());
        self.dirty.insert(url, e);
    }

    /// Merge new entries into the on-disk file (re-read to keep other runs' results) and write atomically.
    pub fn save(&mut self) -> std::io::Result<()> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        if self.dirty.is_empty() {
            return Ok(());
        }
        let mut merged = read(path);
        for (k, v) in self.dirty.drain() {
            merged.insert(k, v);
        }
        let cutoff = now_unix().saturating_sub(MAX_AGE_SECS);
        merged.retain(|_, e| e.checked_at_unix >= cutoff);
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension(format!("json.tmp.{}", std::process::id()));
        let data = serde_json::to_vec(&merged).map_err(std::io::Error::other)?;
        std::fs::write(&tmp, data)?;
        std::fs::rename(&tmp, path).inspect_err(|_| {
            let _ = std::fs::remove_file(&tmp);
        })?;
        self.entries = merged;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths() {
        assert_eq!(
            path_from(Some("/x".into()), Some("/h".into())),
            Some(PathBuf::from("/x/explicit/links.json"))
        );
        assert_eq!(
            path_from(Some("rel".into()), Some("/h".into())),
            Some(PathBuf::from("/h/.cache/explicit/links.json"))
        );
        assert_eq!(path_from(None, None), None);
    }

    #[test]
    fn roundtrip_ttl_and_corruption() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sub/links.json");
        let now = now_unix();
        let mut c = Cache::load(Some(path.clone()));
        c.insert("https://ok".into(), RemoteStatus::Ok, now - 2 * 3600);
        c.insert(
            "https://bad".into(),
            RemoteStatus::HttpError(404),
            now - 2 * 3600,
        );
        c.insert("https://skip".into(), RemoteStatus::Skipped, now);
        c.save().unwrap();
        let c = Cache::load(Some(path.clone()));
        assert_eq!(c.fresh("https://ok", now, 24, 1), Some(&RemoteStatus::Ok));
        assert_eq!(c.fresh("https://bad", now, 24, 1), None);
        assert!(c.get("https://bad").is_some());
        assert!(c.get("https://skip").is_none());
        std::fs::write(&path, "{not json").unwrap();
        assert!(Cache::load(Some(path)).get("https://ok").is_none());
    }
}
