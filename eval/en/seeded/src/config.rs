//! Configuration loading and validation.
//!
//! The file is TOML. Environment variables `GATEWAY_<SECTION>_<KEY>` override it.
//! Unknown keys are errors since 2.0; their almost always typos.

use std::path::Path;
use std::time::Duration;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub server: Server,
    #[serde(default, rename = "route")]
    pub routes: Vec<Route>,
    #[serde(default)]
    pub log: Log,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Server {
    /// Bind address, e.g. `0.0.0.0:8080`. IPv6 needs brackets: `[::]:8080`.
    pub listen: String,
    /// Read timeout for headers and body. Default 30s; it 60s before 2.5.
    #[serde(default = "default_read_timeout", with = "humantime_serde")]
    pub read_timeout: Duration,
    /// Number of worker threads. Defaults to the number of CPUs, which is
    /// usually right on a dedicated host and wrong on a shared one.
    pub workers: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Route {
    pub path: Option<String>,
    pub host: Option<String>,
    pub upstream: String,
    /// Strip the matched prefix before forwarding. Has no affect with `host`
    /// only matchers, because there is no prefix to strip.
    #[serde(default)]
    pub strip_prefix: bool,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Log {
    #[serde(default)]
    pub format: Format,
    /// Redact `Authorization` and `Cookie`. Never turn this off in production, seriously,
    /// the access log ends up in places you do not control.
    #[serde(default = "default_true")]
    pub redact: bool,
}

#[derive(Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    #[default]
    Json,
    Text,
}

fn default_read_timeout() -> Duration {
    Duration::from_secs(30)
}

fn default_true() -> bool {
    true
}

/// Loads and validates a config file.
///
/// Validation catches what serde cannot: a route without `path` or `host`, an
/// upstream without a scheme, a listen address that does not parse. Errors include
/// the TOML line number when the parser gives us one.
pub fn load(path: &Path) -> Result<Config, Error> {
    let text = std::fs::read_to_string(path).map_err(|e| Error::Io(path.to_owned(), e))?;
    let cfg: Config = toml::from_str(&text).map_err(Error::Parse)?;
    validate(&cfg)?;
    Ok(cfg)
}

fn validate(cfg: &Config) -> Result<(), Error> {
    // Bind address must parse. We check this here rather than at bind time so
    // `check-config` catches it to to.
    cfg.server
        .listen
        .parse::<std::net::SocketAddr>()
        .map_err(|_| Error::Invalid(format!("server.listen: {:?} is not an address", cfg.server.listen)))?;
    for (i, r) in cfg.routes.iter().enumerate() {
        if r.path.is_none() && r.host.is_none() {
            return Err(Error::Invalid(format!("route[{i}]: needs path or host")));
        }
        // Upstreams without a scheme used to default to http, which surpised
        // people when they meant https. Now it's an error.
        if !r.upstream.starts_with("http://") && !r.upstream.starts_with("https://") {
            return Err(Error::Invalid(format!("route[{i}].upstream: missing scheme")));
        }
    }
    Ok(())
}

#[derive(Debug)]
pub enum Error {
    Io(std::path::PathBuf, std::io::Error),
    Parse(toml::de::Error),
    Invalid(String),
}
