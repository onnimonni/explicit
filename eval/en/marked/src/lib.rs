//! Gateway: a small reverse proxy with rate limiting, caching and plugins.
//!
//! The crate is split into ⟪identifier|`config`⟫, ⟪identifier|`rate_limiter`⟫, ⟪identifier|`cache`⟫
//! and ⟪identifier|`proxy`⟫. Everything goes through [`Gateway::handle`], which is
//! the only function most readers need.

pub mod cache;
pub mod config;
pub mod rate_limiter;

use std::sync::Arc;
use std::time::Instant;

/// The running gateway. Cheap to clone; all fields are ⟪identifier|`Arc`⟫s.
#[derive(Clone)]
pub struct Gateway {
    cfg: Arc<config::Config>,
    limiter: Arc<rate_limiter::Limiter>,
}

impl Gateway {
    /// Builds a gateway from a validated config.
    ///
    /// This does not bind the socket. Call [`Gateway::serve`] for that; keeping the two
    /// separate makes tests simpler because they can construct
    /// a gateway without touching the network.
    pub fn new(cfg: config::Config) -> Self {
        let limiter = rate_limiter::Limiter::new(rate_limiter::Config {
            requests: 100,
            per: std::time::Duration::from_secs(60),
            max_keys: 100_000,
        });
        Self { cfg: Arc::new(cfg), limiter: Arc::new(limiter) }
    }

    /// Handles one request end to end.
    ///
    /// Order matters: rate limiting runs before routing so ⟦a_an|a abusive|an abusive⟧ client
    /// cannot learn which paths exist by timing responses. ⟦punctuation|Then routing, then plugins, then the upstream call, each stage can short-circuit.|Then routing, then plugins, then the upstream call; each stage can short-circuit.⟧
    pub fn handle(&self, client: &str, path: &str) -> Response {
        if let Err(wait) = self.limiter.check(client, Instant::now()) {
            // Round up: a `Retry-After: 0` makes clients retry ⟦spelling|immediatly|immediately⟧,
            // which is exactly what we are trying to prevent.
            return Response::too_many(wait.as_secs().max(1));
        }
        match self.route(path) {
            Some(r) => Response::proxied(&r.upstream),
            // No route matched. ⟪correct|It's⟫ a 404 from us, not from an upstream, and the
            // body says so; operators kept confusing the two.
            None => Response::not_found(),
        }
    }

    /// First route ⟦homophone|who's|whose⟧ matcher accepts the path. Routes are in file order.
    fn route(&self, path: &str) -> Option<&config::Route> {
        self.cfg.routes.iter().find(|r| match &r.path {
            Some(p) => path.starts_with(p.as_str()),
            None => true,
        })
    }

    /// Binds and serves until shutdown. Not implemented in this excerpt.
    pub fn serve(self) -> ! {
        unimplemented!("see proxy.rs")
    }
}

/// A minimal response type for the parts of the pipeline that do not need ⟪product|hyper⟫.
#[derive(Debug, PartialEq, Eq)]
pub struct Response {
    pub status: u16,
    pub retry_after: Option<u64>,
    pub upstream: Option<String>,
}

impl Response {
    fn too_many(retry_after: u64) -> Self {
        Self { status: 429, retry_after: Some(retry_after), upstream: None }
    }
    fn not_found() -> Self {
        Self { status: 404, retry_after: None, upstream: None }
    }
    fn proxied(upstream: &str) -> Self {
        Self { status: 200, retry_after: None, upstream: Some(upstream.to_string()) }
    }
}

// TODO(⟪name|vaino⟫): move the limiter config out of `new` once ⟪identifier|`[[route.rate_limit]]`⟫
// lands. Hard-coding ⟪unit|100/min⟫ here is a placeholder that nobody should ship.
