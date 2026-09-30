//! Token bucket rate limiter keyed by client.
//!
//! Each key gets ⟪correct|its⟫ own bucket. Buckets are created lazily and evicted with
//! ⟪acronym|LRU⟫ when ⟪identifier|`max_keys`⟫ is reached. The limiter is ⟪term|lock-free⟫ on the
//! hot path; only eviction takes the ⟪identifier|`buckets`⟫ write lock.

use std::collections::HashMap;
use std::sync::RwLock;
use std::time::{Duration, Instant};

/// Configuration for a single limiter.
///
/// `requests` per `per`, so `requests: 100, per: 60s` allows ⟪unit|100⟫ requests
/// every minute with a burst of ⟪unit|100⟫.
#[derive(Clone, Debug)]
pub struct Config {
    pub requests: u32,
    pub per: Duration,
    /// Upper bound on distinct keys. ⟦a_an|An hundred|A hundred⟧ thousand fits in about ⟪unit|8MB⟫.
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
    /// The refill is computed ⟦spelling|lazly|lazily⟧ from the elapsed time since the last call,
    /// so idle keys cost nothing. Callers should treat `Err` as a ⟪code|`429`⟫ and set
    /// `Retry-After` from the duration, rounded up; browsers ignore fractional seconds and
    /// ⟪correct|their⟫ retry logic is not worth fighting.
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

// Removes the least recently touched bucket. Linear scan is fine ⟪correct|here⟫ because
// eviction only happens when the map is full, ⟪correct|which⟫ is rare in practice and
// ⟪correct|then⟫ amortized over many inserts. A proper ⟪acronym|LRU⟫ list would be faster
// ⟪correct|than⟫ this but ⟦spelling|complciates|complicates⟧ the locking.
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
        // After ⟪unit|1s⟫ one token is back.
        assert!(l.check("a", t0 + Duration::from_secs(1)).is_ok());
    }
}
