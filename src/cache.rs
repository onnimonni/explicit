//! On-disk cache of file-local diagnostics.
//!
//! File-local rules (structure, diagrams, code blocks, grammar, style, slop, prose) depend only
//! on the file's bytes and the effective config, so their diagnostics are stored under a key
//! hashed from the root-relative path, the content and the config. Cross-file rules (links,
//! docs) are always recomputed.
//!
//! The cache lives in `<root>/.explicit_cache/results.json` by default. Nothing in it depends on
//! where the project lives or on file metadata, so a copied or cloned checkout keeps its hits,
//! and several checkouts can share one directory: saves merge with what is on disk and replace
//! the file by rename, never writing in place.

use std::collections::BTreeMap;
use std::hash::{DefaultHasher, Hasher};
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::diagnostic::Diagnostic;

/// Bump when the stored format or the meaning of a key changes.
pub const CACHE_SCHEMA: u32 = 2;
/// Default cache directory name, inside the config root.
pub const DEFAULT_DIR: &str = ".explicit_cache";
const RESULTS: &str = "results.json";
const CACHEDIR_TAG: &str = "Signature: 8a477f597d28d172789f06886806bc55\n\
# This file is a cache directory tag created by explicit.\n\
# For information about cache directory tags see https://bford.info/cachedir/\n";
const DAY: u64 = 24 * 60 * 60;
/// Entries unused for this long are dropped on full-root runs.
const MAX_AGE: u64 = 30 * DAY;
/// Full-root runs keep at most this many entries per checked file (worktrees, old versions).
const ENTRIES_PER_FILE: usize = 4;
const MIN_ENTRIES: usize = 256;

/// Package version plus size and mtime of the running executable, so a rebuilt binary with
/// changed rules (same version) does not reuse results.
pub fn build_id() -> &'static str {
    static ID: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    ID.get_or_init(|| {
        let exe = std::env::current_exe()
            .and_then(std::fs::metadata)
            .map(|m| {
                let mtime = m
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map_or(0, |d| d.as_nanos());
                format!("{}-{mtime}", m.len())
            })
            .unwrap_or_default();
        format!("{}+{exe}", env!("CARGO_PKG_VERSION"))
    })
}

/// 128-bit hash of `parts` (two differently seeded SipHash runs).
pub fn hash128(parts: &[&[u8]]) -> u128 {
    let run = |seed: u64| {
        let mut h = DefaultHasher::new();
        h.write_u64(seed);
        for p in parts {
            h.write_usize(p.len());
            h.write(p);
        }
        h.finish()
    };
    (u128::from(run(0x9e37_79b9_7f4a_7c15)) << 64) | u128::from(run(0xc2b2_ae3d_27d4_eb4f))
}

/// Everything in `config` that can change file-local diagnostics, including the contents of
/// referenced vocab and slop catalog files. Location-independent: the root is left out and any
/// path under it is written relative, so a byte-identical copy elsewhere gets the same key.
pub fn config_key(config: &Config) -> u128 {
    let mut c = config.clone();
    c.root = PathBuf::new();
    c.cache = Default::default();
    c.general.cache = true;
    c.general.cache_dir = None;
    let mut debug = format!("{c:?}");
    // Fields resolved to absolute paths under the root (if any) must not pin the key.
    let root = config.root.to_string_lossy();
    if root.len() > 1 {
        debug = debug.replace(&*root, "<root>");
    }
    let mut files = Vec::new();
    for p in config.prose.vocab_files.iter().chain(&config.slop.extra) {
        let full = config.root.join(p);
        let rel = full.strip_prefix(&config.root).unwrap_or(&full);
        files.extend(rel.to_string_lossy().as_bytes());
        files.push(0);
        files.extend(std::fs::read(&full).unwrap_or_default());
        files.push(0);
    }
    hash128(&[
        &CACHE_SCHEMA.to_le_bytes(),
        build_id().as_bytes(),
        debug.as_bytes(),
        &files,
    ])
}

/// Cache key of one file: its root-relative path, contents and config key. Content-based only;
/// mtimes and inodes (which change on copies and clones) are never looked at.
pub fn file_key(rel: &Path, text: &str, config_key: u128) -> u128 {
    hash128(&[
        rel_key(rel).as_bytes(),
        text.as_bytes(),
        &config_key.to_le_bytes(),
    ])
}

#[derive(Serialize, Deserialize, Default)]
struct Stored {
    schema: u32,
    version: String,
    /// Hex [`file_key`] -> entry.
    entries: BTreeMap<String, Entry>,
}

#[derive(Serialize, Deserialize, Clone)]
struct Entry {
    /// Unix time of the last run that produced or used this entry (updated at most daily).
    last_used: u64,
    diagnostics: Vec<Diagnostic>,
}

/// Hit and miss counts of one run.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Stats {
    pub hits: usize,
    pub misses: usize,
}

/// Loaded cache; lookups are read-only so it can be shared across threads.
pub struct Cache {
    dir: PathBuf,
    stored: Stored,
    now: u64,
}

fn rel_key(rel: &Path) -> String {
    rel.to_string_lossy().replace('\\', "/")
}

fn hex(key: u128) -> String {
    format!("{key:032x}")
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// Stored cache in `dir`, or an empty one when missing, corrupt or from another build.
fn read_stored(dir: &Path) -> Stored {
    std::fs::read(dir.join(RESULTS))
        .ok()
        .and_then(|b| serde_json::from_slice::<Stored>(&b).ok())
        .filter(|s| s.schema == CACHE_SCHEMA && s.version == build_id())
        .unwrap_or_default()
}

impl Cache {
    /// Load the cache in `dir`. A missing, corrupt or outdated file gives an empty cache.
    pub fn open(dir: &Path) -> Cache {
        Cache {
            dir: dir.to_path_buf(),
            stored: read_stored(dir),
            now: now(),
        }
    }

    /// Cached diagnostics for [`file_key`] `key`, with paths set to `rel`.
    pub fn get(&self, rel: &Path, key: u128) -> Option<Vec<Diagnostic>> {
        let mut d = self.stored.entries.get(&hex(key))?.diagnostics.clone();
        for x in &mut d {
            x.path = rel.to_path_buf();
        }
        Some(d)
    }

    /// Store `fresh` results, mark `hits` as used, and write the cache when anything changed.
    /// The file on disk is re-read and merged first, so concurrent runs keep each other's
    /// entries. With `prune` (full-root runs), old entries go and the total is capped relative
    /// to `files`, the number of files in this run.
    pub fn save(
        mut self,
        fresh: Vec<(u128, Vec<Diagnostic>)>,
        hits: &[u128],
        files: usize,
        prune: bool,
    ) -> std::io::Result<()> {
        let now = self.now;
        let mut changed = !fresh.is_empty();
        for (key, diagnostics) in fresh {
            self.stored.entries.insert(
                hex(key),
                Entry {
                    last_used: now,
                    diagnostics,
                },
            );
        }
        for k in hits {
            if let Some(e) = self.stored.entries.get_mut(&hex(*k))
                && e.last_used + DAY <= now
            {
                e.last_used = now;
                changed = true;
            }
        }
        let cap = (files * ENTRIES_PER_FILE).max(MIN_ENTRIES);
        let stale = |e: &Entry| e.last_used + MAX_AGE < now;
        if prune && (self.stored.entries.len() > cap || self.stored.entries.values().any(stale)) {
            changed = true;
        }
        if !changed {
            return Ok(());
        }
        // Merge whatever other runs saved since we loaded.
        let mut merged = read_stored(&self.dir).entries;
        for (k, e) in std::mem::take(&mut self.stored.entries) {
            match merged.get(&k) {
                Some(old) if old.last_used >= e.last_used => {}
                _ => {
                    merged.insert(k, e);
                }
            }
        }
        if prune {
            merged.retain(|_, e| !stale(e));
            if merged.len() > cap {
                let mut ages: Vec<u64> = merged.values().map(|e| e.last_used).collect();
                ages.sort_unstable_by(|a, b| b.cmp(a));
                let cutoff = ages[cap - 1];
                let mut kept_at_cutoff = ages[..cap].iter().filter(|&&a| a == cutoff).count();
                merged.retain(|_, e| {
                    e.last_used > cutoff
                        || (e.last_used == cutoff && kept_at_cutoff > 0 && {
                            kept_at_cutoff -= 1;
                            true
                        })
                });
            }
        }
        let stored = Stored {
            schema: CACHE_SCHEMA,
            version: build_id().to_string(),
            entries: merged,
        };
        prepare_dir(&self.dir)?;
        let data = serde_json::to_vec(&stored).map_err(std::io::Error::other)?;
        write_atomic(&self.dir.join(RESULTS), &data)
    }
}

/// Number of stored entries in the cache at `dir` (for tests).
#[doc(hidden)]
pub fn entry_count(dir: &Path) -> usize {
    read_stored(dir).entries.len()
}

/// Create `dir` with a `.gitignore` that ignores everything and a `CACHEDIR.TAG`.
fn prepare_dir(dir: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let gi = dir.join(".gitignore");
    if !gi.exists() {
        write_atomic(&gi, b"# Created by explicit.\n*\n")?;
    }
    let tag = dir.join("CACHEDIR.TAG");
    if !tag.exists() {
        write_atomic(&tag, CACHEDIR_TAG.as_bytes())?;
    }
    Ok(())
}

/// Write `data` to a fresh, exclusively created temp file beside `path`, then rename it over.
/// The old file is never modified, so blocks shared with clones stay untouched.
fn write_atomic(path: &Path, data: &[u8]) -> std::io::Result<()> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let dir = path.parent().unwrap_or(Path::new("."));
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let tmp = dir.join(format!(
        ".{name}.{}.{nanos}.{}.tmp",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let res = (|| {
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)?;
        f.write_all(data)?;
        drop(f);
        std::fs::rename(&tmp, path)
    })();
    if res.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    res
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::Severity;

    fn diag(rel: &str) -> Diagnostic {
        Diagnostic {
            path: PathBuf::from(rel),
            rule: "slop/x".into(),
            severity: Severity::Warning,
            range: 0..3,
            line: 1,
            column: 1,
            end_line: 1,
            end_column: 4,
            text: "abc".into(),
            message: "m".into(),
            help: None,
            suggestions: Vec::new(),
            fix: None,
        }
    }

    #[test]
    fn keys_change_with_content_path_and_config() {
        let base = Config::default();
        let ck = config_key(&base);
        let a = Path::new("a.md");
        assert_eq!(ck, config_key(&base.clone()), "stable");
        assert_eq!(file_key(a, "x", ck), file_key(a, "x", ck));
        assert_ne!(file_key(a, "x", ck), file_key(a, "y", ck));
        assert_ne!(file_key(a, "x", ck), file_key(Path::new("b.md"), "x", ck));

        let mut c = base.clone();
        c.prose.accept.push("frobnicate".into());
        assert_ne!(config_key(&c), ck);
        let mut c = base.clone();
        c.prose.max_sentence_words += 1;
        assert_ne!(config_key(&c), ck);
        // Root, cache settings and derived state do not matter.
        let mut c = base.clone();
        c.root = PathBuf::from("/elsewhere");
        c.general.cache = false;
        c.general.cache_dir = Some(PathBuf::from("x"));
        let _ = c.accepted_words();
        assert_eq!(config_key(&c), ck);
    }

    fn vocab_config(root: &Path) -> Config {
        let mut c = Config {
            root: root.to_path_buf(),
            ..Config::default()
        };
        c.prose.vocab_files.push(PathBuf::from("vocab.txt"));
        c
    }

    #[test]
    fn vocab_contents_count_but_location_does_not() {
        let d1 = tempfile::tempdir().unwrap();
        let d2 = tempfile::tempdir().unwrap();
        std::fs::write(d1.path().join("vocab.txt"), "alpha\n").unwrap();
        std::fs::write(d2.path().join("vocab.txt"), "alpha\n").unwrap();
        let k1 = config_key(&vocab_config(d1.path()));
        assert_eq!(
            config_key(&vocab_config(d2.path())),
            k1,
            "same bytes elsewhere"
        );
        std::fs::write(d2.path().join("vocab.txt"), "beta\n").unwrap();
        assert_ne!(config_key(&vocab_config(d2.path())), k1);
    }

    #[test]
    fn roundtrip_and_version_mismatch() {
        let dir = tempfile::tempdir().unwrap();
        let cdir = dir.path().join(DEFAULT_DIR);
        let c = Cache::open(&cdir);
        assert!(c.get(Path::new("a.md"), 1).is_none());
        c.save(vec![(1, vec![diag("a.md")]), (2, vec![])], &[], 2, false)
            .unwrap();
        assert_eq!(
            std::fs::read_to_string(cdir.join(".gitignore")).unwrap(),
            "# Created by explicit.\n*\n"
        );
        assert!(
            std::fs::read_to_string(cdir.join("CACHEDIR.TAG"))
                .unwrap()
                .starts_with("Signature: 8a477f597d28d172789f06886806bc55")
        );
        let c = Cache::open(&cdir);
        assert_eq!(
            c.get(Path::new("x/a.md"), 1).unwrap()[0].path,
            Path::new("x/a.md")
        );
        assert!(c.get(Path::new("a.md"), 9).is_none(), "key mismatch");
        assert!(c.get(Path::new("b.md"), 2).unwrap().is_empty());
        // No absolute paths are stored.
        let raw = std::fs::read_to_string(cdir.join(RESULTS)).unwrap();
        assert!(!raw.contains(&*dir.path().to_string_lossy()));
        // Another build's cache is ignored.
        let raw = raw.replace(build_id(), "0.0.0-other");
        std::fs::write(cdir.join(RESULTS), raw).unwrap();
        assert!(Cache::open(&cdir).get(Path::new("a.md"), 1).is_none());
    }

    #[test]
    fn prune_drops_old_and_caps_entries() {
        let dir = tempfile::tempdir().unwrap();
        let mut c = Cache::open(dir.path());
        for k in 0..(MIN_ENTRIES as u128 + 10) {
            c.stored.entries.insert(
                hex(k),
                Entry {
                    last_used: c.now - (k as u64) * 60,
                    diagnostics: vec![],
                },
            );
        }
        c.stored.entries.insert(
            hex(9999),
            Entry {
                last_used: c.now - MAX_AGE - DAY,
                diagnostics: vec![],
            },
        );
        c.save(vec![], &[], 1, true).unwrap();
        assert_eq!(entry_count(dir.path()), MIN_ENTRIES);
        let c = Cache::open(dir.path());
        assert!(c.get(Path::new("a"), 0).is_some(), "newest kept");
        assert!(c.get(Path::new("a"), 9999).is_none(), "stale dropped");
        assert!(
            c.get(Path::new("a"), MIN_ENTRIES as u128 + 5).is_none(),
            "oldest dropped"
        );
    }

    #[test]
    fn concurrent_saves_merge() {
        let dir = tempfile::tempdir().unwrap();
        let a = Cache::open(dir.path());
        let b = Cache::open(dir.path());
        a.save(vec![(1, vec![diag("a.md")])], &[], 1, false)
            .unwrap();
        b.save(vec![(2, vec![diag("b.md")])], &[], 1, false)
            .unwrap();
        let c = Cache::open(dir.path());
        assert!(c.get(Path::new("a.md"), 1).is_some());
        assert!(c.get(Path::new("b.md"), 2).is_some());
    }

    #[test]
    fn corrupt_cache_is_ignored_and_rewritten() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(RESULTS), b"{not json").unwrap();
        let c = Cache::open(dir.path());
        assert!(c.get(Path::new("a.md"), 1).is_none());
        c.save(vec![(1, vec![])], &[], 1, false).unwrap();
        assert!(Cache::open(dir.path()).get(Path::new("a.md"), 1).is_some());
    }
}
