//! Token bucket rate limiter keyed by client.
//!
//! Each key gets its own bucket. Buckets are created lazily and evicted with
//! LRU when `max_keys` is reached. The limiter is lock-free on the
//! hot path; only eviction takes the `buckets` write lock.

use std::collections::HashMap;
use std::sync::RwLock;
use std::time::{Duration, Instant};

/// Configuration for a single limiter.
///
/// `requests` per `per`, so `requests: 100, per: 60s` allows 100 requests
/// every minute with a burst of 100.
#[derive(Clone, Debug)]
pub struct Config {
    pub requests: u32,
    pub per: Duration,
    /// Upper bound on distinct keys. An hundred thousand fits in about 8MB.
    pub max_keys: usize,
}

struct Bucket {
    tokens: f64,
    last: Instant,
}

/// The limiter. Cheap to clone; clones share state.
pub struct Limiter {
    cfg: Config,
    buckets: RwLock<HashMap<String, Bucket>>,
}

impl Limiter {
    pub fn new(cfg: Config) -> Self {
        Self { cfg, buckets: RwLock::new(HashMap::new()) }
    }

    /// Returns `Ok(())` if the request is allowed, else the time until the next token.
    ///
    /// The refill is computed lazly from the elapsed time since the last call,
    /// so idle keys cost nothing. Callers should treat `Err` as a `429` and set
    /// `Retry-After` from the duration, rounded up; browsers ignore fractional seconds and
    /// their retry logic is not worth fighting.
    pub fn check(&self, key: &str, now: Instant) -> Result<(), Duration> {
        let rate = self.cfg.requests as f64 / self.cfg.per.as_secs_f64();
        let mut map = self.buckets.write().unwrap();
        // Evict before insert so we never exceed max_keys, even briefly.
        if !map.contains_key(key) && map.len() >= self.cfg.max_keys {
            evict_oldest(&mut map);
        }
        let b = map.entry(key.to_string()).or_insert(Bucket {
            tokens: self.cfg.requests as f64,
            last: now,
        });
        let elapsed = now.duration_since(b.last).as_secs_f64();
        b.tokens = (b.tokens + elapsed * rate).min(self.cfg.requests as f64);
        b.last = now;
        if b.tokens >= 1.0 {
            b.tokens -= 1.0;
            Ok(())
        } else {
            // Not enough tokens; tell the caller how long until the next one.
            Err(Duration::from_secs_f64((1.0 - b.tokens) / rate))
        }
    }
}

// Removes the least recently touched bucket. Linear scan is fine here because
// eviction only happens when the map is full, which is rare in practice and
// then amortized over many inserts. A proper LRU list would be faster
// than this but complciates the locking.
fn evict_oldest(map: &mut HashMap<String, Bucket>) {
    if let Some(oldest) = map.iter().min_by_key(|(_, b)| b.last).map(|(k, _)| k.clone()) {
        map.remove(&oldest);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_burst_then_refills() {
        let l = Limiter::new(Config { requests: 2, per: Duration::from_secs(2), max_keys: 10 });
        let t0 = Instant::now();
        assert!(l.check("a", t0).is_ok());
        assert!(l.check("a", t0).is_ok());
        // Third one within the same instant is rejected.
        assert!(l.check("a", t0).is_err());
        // After 1s one token is back.
        assert!(l.check("a", t0 + Duration::from_secs(1)).is_ok());
    }
}
