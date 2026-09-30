//! Persistent remote link status cache (`<root>/.explicit_cache/links.json`).
//!
//! Lives in the project (next to the results cache) so each git worktree has its own.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use super::remote::RemoteStatus;
use crate::config::Config;

/// Entries older than this are dropped on save, even for offline use.
const MAX_AGE_SECS: u64 = 30 * 24 * 3600;
/// Permanent entries are dropped once no checked file has used them for this long.
const PERMANENT_UNSEEN_SECS: u64 = 90 * 24 * 3600;
/// `last_seen` is rewritten at most this often, so reads do not rewrite the file every run.
const TOUCH_EVERY_SECS: u64 = 24 * 3600;
const FILE: &str = "links.json";
const CACHEDIR_TAG: &str = "Signature: 8a477f597d28d172789f06886806bc55\n\
# This file is a cache directory tag created by explicit.\n\
# For information about cache directory tags see https://bford.info/cachedir/\n";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Entry {
    pub status: RemoteStatus,
    pub checked_at_unix: u64,
    /// Final response `Content-Type` of an image check (`""` when absent); `None` for plain link checks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    /// Positive answer that can never change (commit SHA, tag, issue number): no TTL.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub permanent: bool,
    /// Last run that used a permanent entry (`0`: `checked_at_unix`).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub last_seen_unix: u64,
}

fn is_zero(n: &u64) -> bool {
    *n == 0
}

impl Entry {
    fn is_ok(&self) -> bool {
        matches!(self.status, RemoteStatus::Ok | RemoteStatus::Redirect(_))
    }

    /// Whether this entry answers an image check (failures do; successes need a content type).
    pub fn serves_image(&self) -> bool {
        !self.is_ok() || self.content_type.is_some()
    }

    fn last_seen(&self) -> u64 {
        self.last_seen_unix.max(self.checked_at_unix)
    }

    /// Whether `save` keeps this entry at time `now`.
    fn keep(&self, now: u64) -> bool {
        if self.permanent {
            now.saturating_sub(self.last_seen()) <= PERMANENT_UNSEEN_SECS
        } else {
            now.saturating_sub(self.checked_at_unix) <= MAX_AGE_SECS
        }
    }
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

/// Project cache directory: `[general] cache_dir` (relative to the root) or `.explicit_cache`.
pub fn default_dir(config: &Config) -> PathBuf {
    config.root.join(
        config
            .general
            .cache_dir
            .as_deref()
            .unwrap_or(Path::new(crate::cache::DEFAULT_DIR)),
    )
}

/// Link cache file inside the cache directory `dir`.
pub fn path_in(dir: &Path) -> PathBuf {
    dir.join(FILE)
}

/// Create `dir` with a `.gitignore` ignoring everything and a `CACHEDIR.TAG`, if missing.
fn prepare_dir(dir: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let gi = dir.join(".gitignore");
    if !gi.exists() {
        std::fs::write(&gi, "# Created by explicit.\n*\n")?;
    }
    let tag = dir.join("CACHEDIR.TAG");
    if !tag.exists() {
        std::fs::write(&tag, CACHEDIR_TAG)?;
    }
    Ok(())
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

    /// Cached entry if it is younger than its TTL (success and failure TTLs differ).
    pub fn fresh(
        &self,
        url: &str,
        now: u64,
        ok_ttl_hours: u64,
        fail_ttl_hours: u64,
    ) -> Option<&Entry> {
        let e = self.entries.get(url)?;
        if e.permanent && e.is_ok() {
            return Some(e);
        }
        let ttl = if e.is_ok() {
            ok_ttl_hours
        } else {
            fail_ttl_hours
        };
        (now.saturating_sub(e.checked_at_unix) < ttl.saturating_mul(3600)).then_some(e)
    }

    pub fn insert(
        &mut self,
        url: String,
        status: RemoteStatus,
        content_type: Option<String>,
        now: u64,
    ) {
        self.insert_with(url, status, content_type, now, false);
    }

    /// [`Cache::insert`]; `permanent` positive answers never expire (negative ones always do).
    pub fn insert_with(
        &mut self,
        url: String,
        status: RemoteStatus,
        content_type: Option<String>,
        now: u64,
        permanent: bool,
    ) {
        if status == RemoteStatus::Skipped {
            return;
        }
        let permanent = permanent && matches!(status, RemoteStatus::Ok | RemoteStatus::Redirect(_));
        let e = Entry {
            status,
            checked_at_unix: now,
            content_type,
            permanent,
            last_seen_unix: 0,
        };
        self.entries.insert(url.clone(), e.clone());
        self.dirty.insert(url, e);
    }

    /// Merge new entries into the on-disk file (re-read to keep other runs' results) and write atomically.
    pub fn save(&mut self) -> std::io::Result<()> {
        self.save_at(now_unix())
    }

    /// Record that a checked file still uses `url` (keeps permanent entries from being pruned).
    pub fn touch(&mut self, url: &str, now: u64) {
        if let Some(e) = self.entries.get_mut(url)
            && e.permanent
            && now.saturating_sub(e.last_seen()) >= TOUCH_EVERY_SECS
        {
            e.last_seen_unix = now;
            self.dirty.insert(url.to_string(), e.clone());
        }
    }

    /// [`Cache::save`], pruning as of `now`: plain entries after 30 days, permanent ones after
    /// 90 days without use.
    pub fn save_at(&mut self, now: u64) -> std::io::Result<()> {
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
        merged.retain(|_, e| e.keep(now));
        if let Some(dir) = path.parent() {
            prepare_dir(dir)?;
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
    fn dir_prepared_and_old_entries_load() {
        let dir = tempfile::tempdir().unwrap();
        let cache_dir = dir.path().join(".explicit_cache");
        let path = path_in(&cache_dir);
        // Pre-image-check cache files have no `content_type`.
        std::fs::create_dir_all(&cache_dir).unwrap();
        std::fs::write(
            &path,
            r#"{"https://old":{"status":"Ok","checked_at_unix":5}}"#,
        )
        .unwrap();
        let mut c = Cache::load(Some(path.clone()));
        let old = c.get("https://old").unwrap();
        assert_eq!(old.content_type, None);
        assert!(!old.serves_image());
        c.insert(
            "https://img".into(),
            RemoteStatus::Ok,
            Some("image/png".into()),
            now_unix(),
        );
        c.save().unwrap();
        assert!(cache_dir.join(".gitignore").exists());
        assert!(cache_dir.join("CACHEDIR.TAG").exists());
        let c = Cache::load(Some(path));
        assert!(c.get("https://img").unwrap().serves_image());
        assert!(
            c.get("https://old").is_none(),
            "older than 30 days: dropped"
        );
    }

    #[test]
    fn roundtrip_ttl_and_corruption() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sub/links.json");
        let now = now_unix();
        let mut c = Cache::load(Some(path.clone()));
        c.insert("https://ok".into(), RemoteStatus::Ok, None, now - 2 * 3600);
        c.insert(
            "https://bad".into(),
            RemoteStatus::HttpError(404),
            None,
            now - 2 * 3600,
        );
        c.insert("https://skip".into(), RemoteStatus::Skipped, None, now);
        c.save().unwrap();
        let c = Cache::load(Some(path.clone()));
        assert_eq!(
            c.fresh("https://ok", now, 24, 1).map(|e| &e.status),
            Some(&RemoteStatus::Ok)
        );
        assert_eq!(c.fresh("https://bad", now, 24, 1).map(|e| &e.status), None);
        assert!(c.get("https://bad").is_some());
        assert!(c.get("https://skip").is_none());
        std::fs::write(&path, "{not json").unwrap();
        assert!(Cache::load(Some(path)).get("https://ok").is_none());
    }

    #[test]
    fn permanent_entries() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("links.json");
        let now = now_unix();
        let day = 24 * 3600;
        let mut c = Cache::load(Some(path.clone()));
        c.insert_with("gh:sha".into(), RemoteStatus::Ok, None, now - 8 * day, true);
        c.insert_with(
            "gh:branch".into(),
            RemoteStatus::Ok,
            None,
            now - 8 * day,
            false,
        );
        // Negative answers never become permanent.
        c.insert_with(
            "gh:gone".into(),
            RemoteStatus::HttpError(404),
            None,
            now - 2 * 3600,
            true,
        );
        c.insert_with(
            "gh:old".into(),
            RemoteStatus::Ok,
            None,
            now - 100 * day,
            true,
        );
        c.insert_with(
            "gh:used".into(),
            RemoteStatus::Ok,
            None,
            now - 100 * day,
            true,
        );
        c.touch("gh:used", now - 10 * day);
        c.save_at(now).unwrap();
        let c = Cache::load(Some(path.clone()));
        let fresh = |k: &str| c.fresh(k, now, 168, 1).is_some();
        assert!(fresh("gh:sha"), "permanent: older than 7 days, still fresh");
        assert!(!fresh("gh:branch"), "branch answers expire after 7 days");
        assert!(!fresh("gh:gone"), "negatives expire after an hour");
        assert!(!c.get("gh:gone").unwrap().permanent);
        assert!(c.get("gh:old").is_none(), "unseen for 90+ days: pruned");
        assert!(fresh("gh:used"), "seen recently: kept");
        // Old cache files without the new fields still load.
        std::fs::write(&path, r#"{"k":{"status":"Ok","checked_at_unix":1}}"#).unwrap();
        let e = Cache::load(Some(path)).get("k").cloned().unwrap();
        assert!(!e.permanent && e.last_seen_unix == 0);
    }
}
