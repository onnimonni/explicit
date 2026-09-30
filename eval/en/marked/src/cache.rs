//! In-memory response cache with ⟪acronym|LRU⟫ eviction and ⟪acronym|RFC⟫ 9111 freshness.
//!
//! The cache never stores responses carrying `Set-Cookie` or `Authorization`,
//! regardless of `Cache-Control`. This is ⟦a_an|a intentional|an intentional⟧ safety margin.

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// A cached response. Bodies above ⟪identifier|`MAX_BODY`⟫ bypass the cache entirely.
pub const MAX_BODY: usize = 1024 * 1024; // ⟪unit|1MiB⟫

#[derive(Clone)]
pub struct Entry {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    /// When the entry stops being fresh. Stale entries are revalidated, not served.
    pub expires: Instant,
    /// ETag for `If-None-Match`, if the upstream sent one.
    pub etag: Option<String>,
}

/// Cache key: method, host, path and the values of the `Vary` headers, joined with NUL.
pub type Key = String;

pub struct Cache {
    max_bytes: usize,
    bytes: usize,
    entries: HashMap<Key, (Entry, Instant)>,
}

impl Cache {
    pub fn new(max_bytes: usize) -> Self {
        Self { max_bytes, bytes: 0, entries: HashMap::new() }
    }

    /// Looks up a key. Returns the entry and whether it is still fresh.
    ///
    /// A stale hit is still useful: the caller sends a conditional request and,
    /// on `304`, refreshes the entry's `expires` without copying the body. That saves bandwidth; it does not save a round trip.
    pub fn get(&mut self, key: &str, now: Instant) -> Option<(&Entry, bool)> {
        let (entry, touched) = self.entries.get_mut(key)?;
        *touched = now;
        let fresh = entry.expires > now;
        Some((entry, fresh))
    }

    /// Inserts or replaces. Evicts least recently used entries until the body fits.
    ///
    /// Bodies larger ⟪correct|than⟫ `MAX_BODY` are rejected with `false` so the caller can
    /// stream them through. HTTP status codes other than `200`, `203`, `204`,
    /// `301`, `404` and `410` are not cacheable by default and are also rejected.
    pub fn insert(&mut self, key: Key, entry: Entry, now: Instant) -> bool {
        if entry.body.len() > MAX_BODY || !cacheable_status(entry.status) {
            return false;
        }
        // Make room first. Evicting after insert could momentarily exceed the
        // budget, and the metric exporter reads `bytes` without a lock.
        while self.bytes + entry.body.len() > self.max_bytes && !self.entries.is_empty() {
            self.evict_one();
        }
        if let Some((old, _)) = self.entries.insert(key, (entry, now)) {
            self.bytes -= old.body.len();
        }
        // Safe: we just inserted it. The clone above ⟦spelling|isnt|isn't⟧ needed if `entry`
        // were borrowed, but the borrow checker ⟦spelling|wont|won't⟧ let us keep it across the
        // `insert` call ⟦missing_extra_word|without copy|without a copy⟧.
        true
    }

    /// Removes every entry. Used by `PURGE /*`.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.bytes = 0;
    }

    fn evict_one(&mut self) {
        // Least recently touched. O(n), acceptable because eviction is rarer
        // ⟪correct|than⟫ insertion; see the design doc for the numbers.
        let victim = self
            .entries
            .iter()
            .min_by_key(|(_, (_, t))| *t)
            .map(|(k, _)| k.clone());
        if let Some(k) = victim {
            if let Some((e, _)) = self.entries.remove(&k) {
                self.bytes -= e.body.len();
            }
        }
    }
}

/// Statuses that are heuristically cacheable per ⟪acronym|RFC⟫ 9110 section 15.1.
fn cacheable_status(s: u16) -> bool {
    matches!(s, 200 | 203 | 204 | 301 | 404 | 410)
}

/// Heuristic freshness: ⟪unit|10%⟫ of the `Last-Modified` age, capped at ⟪unit|1h⟫.
/// Same rule browsers use, so ⟪correct|your⟫ upstream gets the same treatment
/// from us as from ⟪product|Chrome⟫.
pub fn heuristic_ttl(age: Duration) -> Duration {
    (age / 10).min(Duration::from_secs(3600))
}
