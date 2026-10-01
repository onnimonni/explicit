//! Remote (http/https) link checking with a persistent cache and per-host politeness.

use std::collections::{HashMap, HashSet, VecDeque};
use std::ops::Range;
use std::path::PathBuf;
use std::sync::{Arc, Condvar, LazyLock, Mutex};
use std::time::{Duration, Instant};

use regex::Regex;
use serde::{Deserialize, Serialize};

use super::cache::{Cache, now_unix};
use super::{dest_range, has_scheme, is_excluded, is_http, report_range, sev, strip_fragment};
use crate::config::Config;
use crate::diagnostic::Finding;
use crate::extract::markdown::LinkKind;
use crate::rules::{FileCtx, Out};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RemoteStatus {
    Ok,
    /// Permanent redirect to the given URL.
    Redirect(String),
    HttpError(u16),
    /// 401, 403 or 999: bot protection or login wall, not proof the page is gone.
    Blocked(u16),
    Unreachable(String),
    /// Not checked (offline without cache entry, excluded).
    Skipped,
    /// Image URL answers 2xx with an HTML page (login wall, soft 404); holds the Content-Type.
    /// Derived per run for image URLs, never cached.
    NotImage(String),
}

const MAX_REDIRECTS: usize = 5;
const MAX_PER_HOST: usize = 2;
const MAX_RETRY_AFTER: Duration = Duration::from_secs(10);
/// Statuses sites use to turn away bots or anonymous clients (999 is LinkedIn's).
const BLOCKED_STATUSES: &[u16] = &[401, 403, 999];
/// Many sites reject non-browser clients outright, so look like one while staying identifiable.
pub(crate) const USER_AGENT: &str = concat!(
    "Mozilla/5.0 (compatible; explicit-link-checker/",
    env!("CARGO_PKG_VERSION"),
    ")"
);
const ACCEPT: &str = "text/html,application/xhtml+xml,*/*;q=0.8";
/// What browsers send for `<img>`; asking for HTML would invite content negotiation to an HTML viewer page.
const ACCEPT_IMAGE: &str = "image/avif,image/webp,image/apng,image/svg+xml,image/*,*/*;q=0.8";

/// Tunables for `check_all_with` (tests use a temp cache and no pacing).
#[derive(Debug, Clone)]
pub struct CheckOpts {
    /// Link cache file; `None` disables the cache.
    pub cache_path: Option<PathBuf>,
    /// Minimum time between starting requests to the same host.
    pub min_interval: Duration,
    /// Progress bar on stderr; disabled by default.
    pub progress: Arc<crate::progress::Progress>,
}

impl Default for CheckOpts {
    fn default() -> Self {
        CheckOpts {
            cache_path: None,
            min_interval: Duration::from_millis(250),
            progress: Arc::default(),
        }
    }
}

/// A remote URL to check; `image` URLs must also serve something other than an HTML page.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Target {
    pub url: String,
    pub image: bool,
}

impl Target {
    pub fn link(url: impl Into<String>) -> Target {
        Target {
            url: url.into(),
            image: false,
        }
    }

    pub fn image(url: impl Into<String>) -> Target {
        Target {
            url: url.into(),
            image: true,
        }
    }
}

/// Check all URLs, using and updating the link cache at `cache_path` (`None`: no cache).
pub fn check_all(
    targets: &[Target],
    config: &Config,
    cache_path: Option<PathBuf>,
) -> HashMap<String, RemoteStatus> {
    check_all_with(
        targets,
        config,
        &CheckOpts {
            cache_path,
            ..CheckOpts::default()
        },
    )
}

/// Final Content-Type is an HTML page.
fn is_html(content_type: &str) -> bool {
    let mime = content_type.split(';').next().unwrap_or("").trim();
    mime.eq_ignore_ascii_case("text/html") || mime.eq_ignore_ascii_case("application/xhtml+xml")
}

/// Status reported for a URL: image URLs that turn out to be HTML pages become `NotImage`.
fn verdict(status: &RemoteStatus, content_type: Option<&str>, image: bool) -> RemoteStatus {
    match (status, content_type) {
        (RemoteStatus::Ok | RemoteStatus::Redirect(_), Some(ct)) if image && is_html(ct) => {
            RemoteStatus::NotImage(ct.to_string())
        }
        _ => status.clone(),
    }
}

pub fn check_all_with(
    targets: &[Target],
    config: &Config,
    opts: &CheckOpts,
) -> HashMap<String, RemoteStatus> {
    let lc = &config.links;
    let mut cache = Cache::load(opts.cache_path.clone());
    let now = now_unix();
    let mut result = HashMap::new();
    // A URL used both as a link and as an image is checked as an image.
    let mut wanted: HashMap<String, bool> = HashMap::new();
    let mut order = Vec::new();
    for t in targets {
        let key = strip_fragment(&t.url).to_string();
        match wanted.get_mut(&key) {
            Some(img) => *img |= t.image,
            None => {
                wanted.insert(key.clone(), t.image);
                order.push(key);
            }
        }
    }
    let mut todo: VecDeque<String> = VecDeque::new();
    for key in order {
        let image = wanted[&key];
        let status = if !is_http(&key) || is_excluded(config, &key) {
            Some(RemoteStatus::Skipped)
        } else if let Some(e) = cache
            .fresh(&key, now, lc.cache_ttl_hours, lc.cache_failed_ttl_hours)
            .filter(|e| !image || e.serves_image())
        {
            Some(verdict(&e.status, e.content_type.as_deref(), image))
        } else if lc.offline {
            Some(cache.get(&key).map_or(RemoteStatus::Skipped, |e| {
                verdict(&e.status, e.content_type.as_deref(), image)
            }))
        } else {
            None
        };
        match status {
            Some(s) => {
                result.insert(key, s);
            }
            None => todo.push_back(key),
        }
    }
    if todo.is_empty() {
        return result;
    }

    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(lc.timeout_secs.max(1))))
        .http_status_as_error(false)
        .max_redirects(0)
        .user_agent(USER_AGENT)
        .build()
        .into();
    let sched = Scheduler {
        state: Mutex::new(SchedState {
            queue: todo,
            hosts: HashMap::new(),
        }),
        cv: Condvar::new(),
        interval: opts.min_interval,
    };
    type Checked = (String, RemoteStatus, Option<String>);
    let checked: Mutex<Vec<Checked>> = Mutex::new(Vec::new());
    let workers = lc.concurrency.max(1).min(sched.len());
    opts.progress.start("Checking links", sched.len());
    std::thread::scope(|s| {
        for _ in 0..workers {
            s.spawn(|| {
                while let Some((url, host)) = sched.next() {
                    let image = wanted.get(&url).copied().unwrap_or(false);
                    let (status, content_type) = check_url(&agent, &url, config, image);
                    sched.done(&host);
                    opts.progress.tick(&url);
                    checked.lock().unwrap_or_else(|e| e.into_inner()).push((
                        url,
                        status,
                        content_type,
                    ));
                }
            });
        }
    });
    opts.progress.finish();
    let now = now_unix();
    for (url, status, content_type) in checked.into_inner().unwrap_or_else(|e| e.into_inner()) {
        let image = wanted.get(&url).copied().unwrap_or(false);
        let shown = verdict(&status, content_type.as_deref(), image);
        cache.insert(url.clone(), status, content_type, now);
        result.insert(url, shown);
    }
    if let Err(e) = cache.save() {
        eprintln!("explicit: could not write link cache: {e}");
    }
    result
}

struct Host {
    in_flight: usize,
    next_at: Instant,
}

struct SchedState {
    queue: VecDeque<String>,
    hosts: HashMap<String, Host>,
}

/// Hands out URLs so that each host gets at most `MAX_PER_HOST` requests in flight, spaced by `interval`.
struct Scheduler {
    state: Mutex<SchedState>,
    cv: Condvar,
    interval: Duration,
}

impl Scheduler {
    fn len(&self) -> usize {
        self.state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .queue
            .len()
    }

    fn next(&self) -> Option<(String, String)> {
        let mut g = self.state.lock().unwrap_or_else(|e| e.into_inner());
        loop {
            if g.queue.is_empty() {
                return None;
            }
            let now = Instant::now();
            let st = &mut *g;
            let ready = st.queue.iter().position(|u| {
                st.hosts
                    .get(host_of(u))
                    .is_none_or(|h| h.in_flight < MAX_PER_HOST && h.next_at <= now)
            });
            if let Some(i) = ready {
                let url = st.queue.remove(i).unwrap_or_default();
                let host = host_of(&url).to_string();
                let h = st.hosts.entry(host.clone()).or_insert(Host {
                    in_flight: 0,
                    next_at: now,
                });
                h.in_flight += 1;
                h.next_at = now + self.interval;
                return Some((url, host));
            }
            let wait = st
                .queue
                .iter()
                .filter_map(|u| st.hosts.get(host_of(u)))
                .filter(|h| h.in_flight < MAX_PER_HOST)
                .map(|h| h.next_at.saturating_duration_since(now))
                .min()
                .unwrap_or(Duration::from_millis(100))
                .max(Duration::from_millis(1));
            g = self
                .cv
                .wait_timeout(g, wait)
                .unwrap_or_else(|e| e.into_inner())
                .0;
        }
    }

    fn done(&self, host: &str) {
        let mut g = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(h) = g.hosts.get_mut(host) {
            h.in_flight = h.in_flight.saturating_sub(1);
        }
        self.cv.notify_all();
    }
}

/// Authority (`host[:port]`) of a URL, used as the politeness key.
fn host_of(url: &str) -> &str {
    let rest = url.split_once("://").map_or(url, |(_, r)| r);
    let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let auth = &rest[..end];
    auth.rsplit_once('@').map_or(auth, |(_, h)| h)
}

fn origin(url: &str) -> &str {
    let Some(i) = url.find("://") else { return url };
    let end = url[i + 3..]
        .find(['/', '?', '#'])
        .map_or(url.len(), |j| i + 3 + j);
    &url[..end]
}

/// Resolve a `Location` header against the request URL.
fn join_url(base: &str, loc: &str) -> String {
    if has_scheme(loc) {
        loc.to_string()
    } else if loc.starts_with("//") {
        format!("{}:{loc}", base.split(':').next().unwrap_or("https"))
    } else if loc.starts_with('/') {
        format!("{}{loc}", origin(base))
    } else {
        let o = origin(base);
        let path = &base[o.len()..];
        let path = &path[..path.find(['?', '#']).unwrap_or(path.len())];
        let dir = path.rfind('/').map_or("/", |i| &path[..=i]);
        format!("{o}{dir}{loc}")
    }
}

struct Resp {
    status: u16,
    location: Option<String>,
    retry_after: Option<String>,
    content_type: Option<String>,
}

fn send(agent: &ureq::Agent, get: bool, url: &str, image: bool) -> Result<Resp, String> {
    let accept = if image { ACCEPT_IMAGE } else { ACCEPT };
    let r = if get {
        agent.get(url).header("Accept", accept).call()
    } else {
        agent.head(url).header("Accept", accept).call()
    };
    // The body is dropped unread: only the status line and headers matter.
    let resp = r.map_err(|e| e.to_string())?;
    let header = |n: &str| {
        resp.headers()
            .get(n)
            .and_then(|v| v.to_str().ok())
            .map(String::from)
    };
    Ok(Resp {
        status: resp.status().as_u16(),
        location: header("location"),
        retry_after: header("retry-after"),
        content_type: header("content-type"),
    })
}

fn send_retry(agent: &ureq::Agent, get: bool, url: &str, image: bool) -> Result<Resp, String> {
    let r = send(agent, get, url, image)?;
    if r.status != 429 {
        return Ok(r);
    }
    let wait = r
        .retry_after
        .as_deref()
        .and_then(|s| s.trim().parse::<u64>().ok())
        .map_or(Duration::from_secs(1), Duration::from_secs);
    std::thread::sleep(wait.min(MAX_RETRY_AFTER));
    send(agent, get, url, image)
}

/// HEAD, falling back to GET when the server rejects HEAD.
fn fetch(agent: &ureq::Agent, url: &str, image: bool) -> Result<Resp, String> {
    let r = send_retry(agent, false, url, image)?;
    if ((400..500).contains(&r.status) && r.status != 429) || r.status == 501 {
        return send_retry(agent, true, url, image);
    }
    Ok(r)
}

/// Status of `url` and, for image checks, the final Content-Type (`""` when absent).
fn check_url(
    agent: &ureq::Agent,
    url: &str,
    config: &Config,
    image: bool,
) -> (RemoteStatus, Option<String>) {
    let accept = &config.links.accept_status;
    let mut current = url.to_string();
    let mut permanent = true;
    // Resolved here (in a worker) rather than while planning, so DNS lookups run in parallel.
    if !config.links.allow_private && is_private_url(url) {
        return (RemoteStatus::Skipped, None);
    }
    for _ in 0..=MAX_REDIRECTS {
        if current != url
            && let Some(refused) = refuse_hop(config, &current)
        {
            return (refused, None);
        }
        let r = match fetch(agent, &current, image) {
            Ok(r) => r,
            Err(e) => return (RemoteStatus::Unreachable(e), None),
        };
        if (200..300).contains(&r.status) || accept.contains(&r.status) {
            let status = if current != url && permanent {
                RemoteStatus::Redirect(current)
            } else {
                RemoteStatus::Ok
            };
            // Accepted non-2xx statuses (e.g. 429) carry an error page, not the resource.
            let ct = (200..300)
                .contains(&r.status)
                .then_some(r.content_type)
                .flatten();
            return (status, image.then(|| ct.unwrap_or_default()));
        }
        match (r.status, r.location) {
            (301 | 302 | 303 | 307 | 308, Some(loc)) => {
                permanent &= matches!(r.status, 301 | 308);
                current = join_url(&current, loc.trim());
            }
            (s, _) if BLOCKED_STATUSES.contains(&s) => return (RemoteStatus::Blocked(s), None),
            (s, _) => return (RemoteStatus::HttpError(s), None),
        }
    }
    (
        RemoteStatus::Unreachable(format!("more than {MAX_REDIRECTS} redirects")),
        None,
    )
}

/// Why a redirect target must not be requested: excluded, not http(s), or a private address.
fn refuse_hop(config: &Config, url: &str) -> Option<RemoteStatus> {
    if is_excluded(config, url) {
        return Some(RemoteStatus::Skipped);
    }
    if !is_http(url) {
        return Some(RemoteStatus::Unreachable(format!(
            "redirects to non-http URL `{url}`"
        )));
    }
    (!config.links.allow_private && is_private_url(url)).then(|| {
        RemoteStatus::Unreachable(format!(
            "redirects to private address `{url}` (set links.allow_private = true to follow)"
        ))
    })
}

/// Loopback, private, link-local, unique-local, CGNAT or unspecified address.
fn is_private_ip(ip: std::net::IpAddr) -> bool {
    use std::net::IpAddr;
    match ip {
        IpAddr::V4(v4) => {
            let o = v4.octets();
            v4.is_loopback()
                || v4.is_private()
                || v4.is_link_local()
                || v4.is_unspecified()
                || v4.is_broadcast()
                || (o[0] == 100 && (o[1] & 0xc0) == 64)
        }
        IpAddr::V6(v6) => {
            if let Some(v4) = v6.to_ipv4_mapped() {
                return is_private_ip(IpAddr::V4(v4));
            }
            let seg = v6.segments();
            v6.is_loopback()
                || v6.is_unspecified()
                || (seg[0] & 0xfe00) == 0xfc00
                || (seg[0] & 0xffc0) == 0xfe80
        }
    }
}

/// Host and port of a web URL (`[::1]:8080` -> `::1`, 8080).
fn host_port(url: &str) -> Option<(String, u16)> {
    let auth = host_of(url);
    let default = if url
        .get(..8)
        .is_some_and(|s| s.eq_ignore_ascii_case("https://"))
    {
        443
    } else {
        80
    };
    let (host, port) = if let Some(rest) = auth.strip_prefix('[') {
        let (h, after) = rest.split_once(']')?;
        (h, after.strip_prefix(':'))
    } else {
        match auth.rsplit_once(':') {
            Some((h, p)) => (h, Some(p)),
            None => (auth, None),
        }
    };
    let port = match port {
        Some(p) if !p.is_empty() => p.parse().ok()?,
        _ => default,
    };
    (!host.is_empty()).then(|| (host.to_ascii_lowercase(), port))
}

/// Whether the URL's host is (or only resolves to) a private address.
fn is_private_url(url: &str) -> bool {
    use std::net::ToSocketAddrs;
    let Some((host, port)) = host_port(url) else {
        return false;
    };
    if let Ok(ip) = host.parse::<std::net::IpAddr>() {
        return is_private_ip(ip);
    }
    if host == "localhost" || host.ends_with(".localhost") {
        return true;
    }
    // Unresolvable hosts are left to the request, which reports them as unreachable.
    match (host.as_str(), port).to_socket_addrs() {
        Ok(addrs) => {
            let ips: Vec<_> = addrs.map(|a| a.ip()).collect();
            !ips.is_empty() && ips.into_iter().all(is_private_ip)
        }
        Err(_) => false,
    }
}

/// One http(s) URL occurrence in a file.
pub(crate) struct Occurrence {
    pub(crate) url: String,
    pub(crate) range: Range<usize>,
    pub(crate) image: bool,
}

/// `srcset` attributes of `<img>` / `<source>` tags.
static SRCSET_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)<(?:img|source)\b[^>]*?\ssrcset\s*=\s*["']([^"']+)["']"#)
        .expect("hardcoded regex is valid")
});

/// First URL of a `srcset` value (`a.png 1x, b.png 2x` -> `a.png`).
fn srcset_first(v: &str) -> Option<&str> {
    let first = v.trim_start().split(char::is_whitespace).next()?;
    let first = first.trim_end_matches(',');
    (!first.is_empty()).then_some(first)
}

pub(crate) fn occurrences(ctx: &FileCtx) -> Vec<Occurrence> {
    let src = ctx.src();
    let mut v = Vec::new();
    if let Some(md) = &ctx.a.md {
        for link in md
            .links
            .iter()
            .filter(|l| l.kind != LinkKind::Reference && is_http(&l.dest))
        {
            let exact = dest_range(src, link);
            v.push(Occurrence {
                url: link.dest.clone(),
                range: exact.unwrap_or_else(|| report_range(src, link)),
                image: link.is_image,
            });
        }
        let image_labels: HashSet<&str> = md
            .links
            .iter()
            .filter(|l| l.kind == LinkKind::Reference && l.is_image)
            .filter_map(|l| l.label.as_deref())
            .collect();
        for d in md.ref_defs.iter().filter(|d| is_http(&d.dest)) {
            let exact = src
                .get(d.range.clone())
                .and_then(|raw| raw.find(d.dest.as_str()))
                .map(|i| d.range.start + i..d.range.start + i + d.dest.len());
            v.push(Occurrence {
                url: d.dest.clone(),
                range: exact.unwrap_or(d.range.clone()),
                image: image_labels.contains(d.label.as_str()),
            });
        }
        for r in &md.html {
            let Some(raw) = src.get(r.clone()) else {
                continue;
            };
            for c in SRCSET_RE.captures_iter(raw) {
                let (Some(m), Some(url)) =
                    (c.get(1), c.get(1).and_then(|m| srcset_first(m.as_str())))
                else {
                    continue;
                };
                if !is_http(url) {
                    continue;
                }
                let at = r.start + m.start() + m.as_str().find(url).unwrap_or(0);
                v.push(Occurrence {
                    url: url.to_string(),
                    range: at..at + url.len(),
                    image: true,
                });
            }
        }
        v.sort_by_key(|o| o.range.start);
    } else {
        for (url, range) in super::local::comment_urls(ctx.a) {
            v.push(Occurrence {
                url,
                range,
                image: false,
            });
        }
    }
    v
}

/// Finding for an image URL that fails, or `None` when it is fine.
fn image_finding(url: &str, status: &RemoteStatus, range: Range<usize>) -> Option<Finding> {
    let rule = "links/image-url";
    let f = |msg: String| Finding::new(rule, sev(rule), range.clone(), msg);
    Some(match status {
        RemoteStatus::HttpError(code) => f(format!("Image `{url}` returns HTTP {code}")),
        RemoteStatus::Blocked(code) => f(format!(
            "Image `{url}` returns HTTP {code}: blocked or requires auth"
        ))
        .help("Readers without access see a broken image; host it in the repository or a public location"),
        RemoteStatus::Unreachable(e) => f(format!("Image `{url}` is unreachable: {e}")),
        RemoteStatus::NotImage(ct) => f(format!(
            "Image `{url}` returns an HTML page, not an image (Content-Type `{ct}`)"
        ))
        .help("Usually a login wall, a soft 404 or an image viewer page; link the raw image file"),
        RemoteStatus::Ok | RemoteStatus::Redirect(_) | RemoteStatus::Skipped => return None,
    })
}

/// Report statuses for the remote links of one file.
pub fn report(ctx: &FileCtx, statuses: &HashMap<String, RemoteStatus>, out: &mut Out) {
    let image_rule = ctx.enabled("links/image-url");
    for o in occurrences(ctx) {
        let url = &o.url;
        let status = statuses.get(strip_fragment(url));
        if o.image
            && image_rule
            && let Some(s) = status
            && let Some(f) = image_finding(url, s, o.range.clone())
        {
            out.push(f);
            continue;
        }
        match status {
            Some(RemoteStatus::HttpError(code)) => {
                out.push(Finding::new(
                    "links/http-error",
                    sev("links/http-error"),
                    o.range,
                    format!("Link `{url}` returns HTTP {code}"),
                ));
            }
            Some(RemoteStatus::Blocked(code)) => {
                out.push(
                    Finding::new(
                        "links/http-unreachable",
                        sev("links/http-unreachable"),
                        o.range,
                        format!("Link `{url}` returns HTTP {code}: blocked or requires auth"),
                    )
                    .help("The site may reject automated clients; check it in a browser or add it to links.exclude"),
                );
            }
            Some(RemoteStatus::Unreachable(e)) => {
                out.push(Finding::new(
                    "links/http-unreachable",
                    sev("links/http-unreachable"),
                    o.range,
                    format!("Link `{url}` is unreachable: {e}"),
                ));
            }
            Some(RemoteStatus::Redirect(new)) => {
                let new = match url.split_once('#') {
                    Some((_, frag)) if !new.contains('#') => format!("{new}#{frag}"),
                    _ => new.clone(),
                };
                // Suggestion only: redirects often land on consent, login or locale pages.
                out.push(
                    Finding::new(
                        "links/http-redirect",
                        sev("links/http-redirect"),
                        o.range,
                        format!("Link `{url}` permanently redirects to `{new}`"),
                    )
                    .suggest(new),
                );
            }
            // Only image occurrences care whether the response is an HTML page.
            Some(RemoteStatus::Ok | RemoteStatus::Skipped | RemoteStatus::NotImage(_)) | None => {}
        }
    }
}

/// Remote URLs of a file to check, without fragments: images when `links/image-url` is on
/// (else as plain links), other links when any `links/http-*` rule is on.
pub fn urls(ctx: &FileCtx) -> Vec<Target> {
    let image_rule = ctx.enabled("links/image-url");
    let http_rules = ctx.family_enabled("links/http");
    // URLs into this repository are never requested: anonymous requests to a private repository
    // answer 404, and `links/same-repo-url` / `links/same-repo-ref` check them locally.
    let same_repo = super::github::repo(ctx);
    let mut v: Vec<Target> = occurrences(ctx)
        .into_iter()
        .filter(|o| {
            same_repo
                .as_ref()
                .is_none_or(|r| !super::same_repo::is_same_repo(r, &o.url))
        })
        .filter_map(|o| {
            let url = strip_fragment(&o.url).to_string();
            match (o.image && image_rule, http_rules) {
                (true, _) => Some(Target::image(url)),
                (false, true) => Some(Target::link(url)),
                (false, false) => None,
            }
        })
        .collect();
    v.sort();
    v.dedup();
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Write};
    use std::net::{TcpListener, TcpStream};
    use std::sync::Arc;

    type Hits = Arc<Mutex<HashMap<String, usize>>>;

    /// Minimal HTTP/1.1 server: one request per connection.
    fn server() -> (String, Hits) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let hits: Hits = Arc::default();
        let h = hits.clone();
        let b = base.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let h = h.clone();
                let b = b.clone();
                std::thread::spawn(move || handle(stream, &h, &b));
            }
        });
        (base, hits)
    }

    fn handle(mut stream: TcpStream, hits: &Hits, base: &str) {
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut first = String::new();
        if reader.read_line(&mut first).is_err() {
            return;
        }
        let mut req_headers = String::new();
        loop {
            let mut l = String::new();
            if reader.read_line(&mut l).unwrap_or(0) == 0 || l == "\r\n" {
                break;
            }
            req_headers.push_str(&l.to_ascii_lowercase());
        }
        let browser_like = req_headers.contains("user-agent: mozilla/5.0")
            && req_headers.contains("accept: text/html");
        let mut parts = first.split_whitespace();
        let method = parts.next().unwrap_or("").to_string();
        let path = parts.next().unwrap_or("").to_string();
        let n = {
            let mut g = hits.lock().unwrap();
            *g.entry(path.clone()).or_insert(0) += 1;
            let c = g.entry(format!("{method} {path}")).or_insert(0);
            *c += 1;
            *c
        };
        let (status, headers): (u16, String) = match path.split('?').next().unwrap_or("") {
            "/ok" => (200, String::new()),
            "/moved" => (301, "Location: /ok\r\n".into()),
            "/temp" => (302, "Location: /ok\r\n".into()),
            "/chain" => (308, format!("Location: {base}/moved\r\n")),
            "/relative/a" => (301, "Location: b\r\n".into()),
            "/relative/b" => (200, String::new()),
            "/nohead" if method == "HEAD" => (405, String::new()),
            "/nohead" => (200, String::new()),
            "/limited" if n == 1 => (429, "Retry-After: 0\r\n".into()),
            "/limited" => (200, String::new()),
            "/loop" => (301, "Location: /loop\r\n".into()),
            "/forbidden" => (403, String::new()),
            "/auth" => (401, String::new()),
            "/linkedin" => (999, String::new()),
            "/bot-wall" if browser_like => (200, String::new()),
            "/bot-wall" => (403, String::new()),
            "/to-admin" => (302, format!("Location: {base}/admin\r\n")),
            "/admin" => (200, String::new()),
            "/img.png" => (200, "Content-Type: image/png\r\n".into()),
            "/login.png" => (200, "Content-Type: text/html; charset=utf-8\r\n".into()),
            "/moved.png" => (301, "Location: /login.png\r\n".into()),
            "/raw.bin" => (200, "Content-Type: application/octet-stream\r\n".into()),
            "/noct.png" => (200, String::new()),
            "/page" => (200, "Content-Type: text/html\r\n".into()),
            _ => (404, String::new()),
        };
        let body = if method == "HEAD" { "" } else { "body" };
        let _ = write!(
            stream,
            "HTTP/1.1 {status} X\r\n{headers}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
    }

    fn cfg() -> Config {
        let mut c = Config::default();
        c.links.exclude.clear();
        c.links.accept_status.clear();
        c.links.concurrency = 4;
        c.links.timeout_secs = 5;
        // The fake server listens on 127.0.0.1.
        c.links.allow_private = true;
        c
    }

    fn links(urls: &[String]) -> Vec<Target> {
        urls.iter().map(Target::link).collect()
    }

    fn opts(dir: &tempfile::TempDir) -> CheckOpts {
        CheckOpts {
            cache_path: Some(dir.path().join("links.json")),
            min_interval: Duration::ZERO,
            ..CheckOpts::default()
        }
    }

    #[test]
    fn statuses_from_fake_server() {
        let (base, hits) = server();
        let dir = tempfile::tempdir().unwrap();
        let u = |p: &str| format!("{base}{p}");
        let urls: Vec<String> = [
            "/ok",
            "/moved",
            "/temp",
            "/chain",
            "/relative/a",
            "/missing",
            "/nohead",
            "/limited",
            "/loop",
            "/ok#frag",
        ]
        .iter()
        .map(|p| u(p))
        .collect();
        let r = check_all_with(&links(&urls), &cfg(), &opts(&dir));
        assert_eq!(r[&u("/ok")], RemoteStatus::Ok);
        assert_eq!(r[&u("/moved")], RemoteStatus::Redirect(u("/ok")));
        assert_eq!(r[&u("/temp")], RemoteStatus::Ok);
        assert_eq!(r[&u("/chain")], RemoteStatus::Redirect(u("/ok")));
        assert_eq!(
            r[&u("/relative/a")],
            RemoteStatus::Redirect(u("/relative/b"))
        );
        assert_eq!(r[&u("/missing")], RemoteStatus::HttpError(404));
        assert_eq!(r[&u("/nohead")], RemoteStatus::Ok);
        assert_eq!(r[&u("/limited")], RemoteStatus::Ok);
        assert!(matches!(r[&u("/loop")], RemoteStatus::Unreachable(_)));
        assert_eq!(r.len(), 9, "fragment stripped and deduped");
        let h = hits.lock().unwrap();
        assert_eq!(h["HEAD /nohead"], 1);
        assert_eq!(h["GET /nohead"], 1);
        assert_eq!(h["HEAD /missing"], 1);
        assert_eq!(h["GET /missing"], 1);
        assert_eq!(h["HEAD /limited"], 2);
    }

    #[test]
    fn blocked_statuses_and_browser_headers() {
        let (base, _) = server();
        let dir = tempfile::tempdir().unwrap();
        let u = |p: &str| format!("{base}{p}");
        let urls: Vec<String> = ["/forbidden", "/auth", "/linkedin", "/bot-wall"]
            .iter()
            .map(|p| u(p))
            .collect();
        let r = check_all_with(&links(&urls), &cfg(), &opts(&dir));
        assert_eq!(r[&u("/forbidden")], RemoteStatus::Blocked(403));
        assert_eq!(r[&u("/auth")], RemoteStatus::Blocked(401));
        assert_eq!(r[&u("/linkedin")], RemoteStatus::Blocked(999));
        assert_eq!(
            r[&u("/bot-wall")],
            RemoteStatus::Ok,
            "browser-like UA and Accept sent"
        );
    }

    #[test]
    fn redirects_honor_exclusions() {
        let (base, hits) = server();
        let dir = tempfile::tempdir().unwrap();
        let mut c = cfg();
        c.links.exclude.push("/admin$".into());
        let url = format!("{base}/to-admin");
        let r = check_all_with(&links(std::slice::from_ref(&url)), &c, &opts(&dir));
        assert_eq!(r[&url], RemoteStatus::Skipped);
        assert_eq!(hits.lock().unwrap().get("/admin"), None);
    }

    #[test]
    fn private_addresses_refused_by_default() {
        let (base, hits) = server();
        let dir = tempfile::tempdir().unwrap();
        let mut c = cfg();
        c.links.allow_private = false;
        let url = format!("{base}/ok");
        let r = check_all_with(&links(std::slice::from_ref(&url)), &c, &opts(&dir));
        assert_eq!(r[&url], RemoteStatus::Skipped);
        assert!(hits.lock().unwrap().is_empty());
        // Redirect hops are checked too.
        assert!(matches!(
            refuse_hop(&c, "http://10.1.2.3/admin"),
            Some(RemoteStatus::Unreachable(_))
        ));
        assert!(matches!(
            refuse_hop(&c, "http://[::1]:8080/"),
            Some(RemoteStatus::Unreachable(_))
        ));
        assert_eq!(refuse_hop(&c, "https://93.184.215.14/"), None);
        c.links.allow_private = true;
        assert_eq!(refuse_hop(&c, "http://10.1.2.3/admin"), None);
    }

    #[test]
    fn private_ranges() {
        for u in [
            "http://127.0.0.1/",
            "http://localhost:3000/x",
            "http://app.localhost/",
            "http://192.168.1.1/",
            "http://10.0.0.1/",
            "http://172.16.0.1/",
            "http://169.254.169.254/latest",
            "http://100.64.0.1/",
            "http://0.0.0.0/",
            "http://[::1]/",
            "http://[fe80::1]/",
            "http://[fd00::1]:8080/",
            "http://[::ffff:127.0.0.1]/",
            "http://user@127.0.0.1:81/",
        ] {
            assert!(is_private_url(u), "{u}");
        }
        for u in [
            "https://1.1.1.1/",
            "http://[2606:4700::1111]/",
            "https://172.32.0.1/",
        ] {
            assert!(!is_private_url(u), "{u}");
        }
        assert_eq!(host_port("https://Ex.com/x"), Some(("ex.com".into(), 443)));
        assert_eq!(host_port("http://[::1]:81/"), Some(("::1".into(), 81)));
    }

    #[test]
    fn unreachable_and_excluded() {
        let port = TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let dir = tempfile::tempdir().unwrap();
        let dead = format!("http://127.0.0.1:{port}/x");
        let r = check_all_with(&links(std::slice::from_ref(&dead)), &cfg(), &opts(&dir));
        assert!(matches!(r[&dead], RemoteStatus::Unreachable(_)), "{r:?}");
        let r = check_all_with(
            &links(std::slice::from_ref(&dead)),
            &Config::default(),
            &opts(&dir),
        );
        assert_eq!(
            r[&dead],
            RemoteStatus::Skipped,
            "localhost excluded by default"
        );
    }

    #[test]
    fn cache_ttl() {
        let (base, hits) = server();
        let dir = tempfile::tempdir().unwrap();
        let o = opts(&dir);
        let old = now_unix() - 2 * 3600;
        let mut c = Cache::load(o.cache_path.clone());
        // Stale failure (1h TTL): re-checked. Fresh success (7 day TTL) on a 404 path: not re-checked.
        c.insert(
            format!("{base}/ok"),
            RemoteStatus::HttpError(500),
            None,
            old,
        );
        c.insert(format!("{base}/missing"), RemoteStatus::Ok, None, old);
        c.save().unwrap();
        let urls = vec![format!("{base}/ok"), format!("{base}/missing")];
        let r = check_all_with(&links(&urls), &cfg(), &o);
        assert_eq!(r[&urls[0]], RemoteStatus::Ok);
        assert_eq!(r[&urls[1]], RemoteStatus::Ok);
        assert_eq!(hits.lock().unwrap().get("/missing"), None);
        assert_eq!(hits.lock().unwrap()["/ok"], 1);
        // Second run: everything cached.
        let r = check_all_with(&links(&urls), &cfg(), &o);
        assert_eq!(r[&urls[0]], RemoteStatus::Ok);
        assert_eq!(hits.lock().unwrap()["/ok"], 1);
    }

    #[test]
    fn offline_uses_cache_only() {
        let (base, hits) = server();
        let dir = tempfile::tempdir().unwrap();
        let o = opts(&dir);
        let mut c = Cache::load(o.cache_path.clone());
        c.insert(
            format!("{base}/moved"),
            RemoteStatus::HttpError(410),
            None,
            now_unix() - 48 * 3600,
        );
        c.save().unwrap();
        let mut config = cfg();
        config.links.offline = true;
        let urls = vec![format!("{base}/moved"), format!("{base}/ok")];
        let r = check_all_with(&links(&urls), &config, &o);
        assert_eq!(
            r[&urls[0]],
            RemoteStatus::HttpError(410),
            "stale entry still used offline"
        );
        assert_eq!(r[&urls[1]], RemoteStatus::Skipped);
        assert!(hits.lock().unwrap().is_empty());
    }

    #[test]
    fn per_host_pacing() {
        let (base, _) = server();
        let dir = tempfile::tempdir().unwrap();
        let mut o = opts(&dir);
        o.min_interval = Duration::from_millis(60);
        let urls: Vec<String> = (0..4).map(|i| format!("{base}/ok?{i}")).collect();
        let t = Instant::now();
        let r = check_all_with(&links(&urls), &cfg(), &o);
        assert!(r.values().all(|s| *s == RemoteStatus::Ok));
        assert!(
            t.elapsed() >= Duration::from_millis(180),
            "{:?}",
            t.elapsed()
        );
    }

    #[test]
    fn url_helpers() {
        assert_eq!(
            host_of("https://user@Example.com:8080/x?y"),
            "Example.com:8080"
        );
        assert_eq!(join_url("https://a.b/x/y?q", "z"), "https://a.b/x/z");
        assert_eq!(join_url("https://a.b/x/y", "/z"), "https://a.b/z");
        assert_eq!(join_url("https://a.b/x", "//c.d/e"), "https://c.d/e");
        assert_eq!(join_url("https://a.b", "https://c.d"), "https://c.d");
        assert_eq!(join_url("https://a.b", "z"), "https://a.b/z");
    }

    #[test]
    fn report_findings() {
        use crate::rules::Analyzed;
        use crate::source::{FileKind, SourceFile};
        let src = "[a](https://a.org/old#sec) and https://b.org/gone and [r][x] <https://d.org/private>\n\n[x]: https://c.org/down\n";
        let a = Analyzed::new(SourceFile::new(
            "/t/a.md".into(),
            "a.md".into(),
            FileKind::Markdown,
            src.into(),
        ));
        let config = Config::default();
        let ctx = FileCtx {
            a: &a,
            config: &config,
        };
        assert_eq!(
            urls(&ctx),
            [
                "https://a.org/old",
                "https://b.org/gone",
                "https://c.org/down",
                "https://d.org/private"
            ]
            .map(Target::link)
        );
        let statuses: HashMap<String, RemoteStatus> = [
            (
                "https://a.org/old".to_string(),
                RemoteStatus::Redirect("https://example.org/new".into()),
            ),
            (
                "https://b.org/gone".to_string(),
                RemoteStatus::HttpError(404),
            ),
            (
                "https://c.org/down".to_string(),
                RemoteStatus::Unreachable("dns".into()),
            ),
            (
                "https://d.org/private".to_string(),
                RemoteStatus::Blocked(403),
            ),
        ]
        .into();
        let mut out = Vec::new();
        report(&ctx, &statuses, &mut out);
        let rules: Vec<&str> = out.iter().map(|f| f.rule.as_str()).collect();
        assert_eq!(
            rules,
            vec![
                "links/http-redirect",
                "links/http-error",
                "links/http-unreachable",
                "links/http-unreachable"
            ]
        );
        assert!(
            out.iter()
                .any(|f| f.message.contains("blocked or requires auth"))
        );
        // Redirects are suggestion-only, never auto-fixed.
        assert!(out[0].fix.is_none());
        assert_eq!(&src[out[0].range.clone()], "https://a.org/old#sec");
        assert_eq!(out[0].suggestions, ["https://example.org/new#sec"]);
        assert_eq!(&src[out[1].range.clone()], "https://b.org/gone");
    }

    #[test]
    fn image_content_types() {
        let (base, hits) = server();
        let dir = tempfile::tempdir().unwrap();
        let u = |p: &str| format!("{base}{p}");
        let targets = vec![
            Target::image(u("/img.png")),
            Target::image(u("/login.png")),
            Target::image(u("/gone.png")),
            Target::image(u("/raw.bin")),
            Target::image(u("/noct.png")),
            Target::image(u("/moved.png")),
            // Same URL as a link and an image: checked once, as an image.
            Target::link(u("/page")),
            Target::image(u("/page")),
            // HTML is fine for plain links.
            Target::link(u("/ok")),
        ];
        let o = opts(&dir);
        let r = check_all_with(&targets, &cfg(), &o);
        assert_eq!(r[&u("/img.png")], RemoteStatus::Ok);
        assert_eq!(
            r[&u("/login.png")],
            RemoteStatus::NotImage("text/html; charset=utf-8".into())
        );
        assert_eq!(r[&u("/gone.png")], RemoteStatus::HttpError(404));
        assert_eq!(r[&u("/raw.bin")], RemoteStatus::Ok);
        assert_eq!(r[&u("/noct.png")], RemoteStatus::Ok);
        assert!(matches!(r[&u("/moved.png")], RemoteStatus::NotImage(_)));
        assert_eq!(r[&u("/page")], RemoteStatus::NotImage("text/html".into()));
        assert_eq!(r[&u("/ok")], RemoteStatus::Ok);
        assert_eq!(hits.lock().unwrap()["HEAD /page"], 1);
        // The cache keeps the raw status plus the content type, not the derived verdict.
        let c = Cache::load(o.cache_path.clone());
        let e = c.get(&u("/login.png")).unwrap();
        assert_eq!(e.status, RemoteStatus::Ok);
        assert_eq!(e.content_type.as_deref(), Some("text/html; charset=utf-8"));
        assert_eq!(
            c.get(&u("/noct.png")).unwrap().content_type.as_deref(),
            Some("")
        );
        assert_eq!(c.get(&u("/ok")).unwrap().content_type, None);
        // Second run: served from cache, verdict re-derived.
        let r2 = check_all_with(&targets, &cfg(), &o);
        assert_eq!(r, r2);
        assert_eq!(hits.lock().unwrap()["HEAD /img.png"], 1);
        // A plain-link cache entry lacks a content type, so an image use re-checks it.
        let r3 = check_all_with(&[Target::image(u("/ok"))], &cfg(), &o);
        assert_eq!(r3[&u("/ok")], RemoteStatus::Ok);
        assert_eq!(hits.lock().unwrap()["HEAD /ok"], 2);
    }

    #[test]
    fn image_cache_ttl_and_offline() {
        let (base, hits) = server();
        let dir = tempfile::tempdir().unwrap();
        let o = opts(&dir);
        let now = now_unix();
        let u = |p: &str| format!("{base}{p}");
        let mut c = Cache::load(o.cache_path.clone());
        // OK image checked 3 days ago: still fresh (7 day TTL), though the server now says 404.
        c.insert(
            u("/gone.png"),
            RemoteStatus::Ok,
            Some("image/png".into()),
            now - 72 * 3600,
        );
        // OK image checked 8 days ago: stale.
        c.insert(
            u("/img.png"),
            RemoteStatus::Ok,
            Some("image/png".into()),
            now - 192 * 3600,
        );
        // Failure checked 2 hours ago: stale (1h TTL), re-checked and now OK.
        c.insert(
            u("/raw.bin"),
            RemoteStatus::HttpError(404),
            None,
            now - 2 * 3600,
        );
        c.save().unwrap();
        let targets = [u("/gone.png"), u("/img.png"), u("/raw.bin")].map(Target::image);
        let r = check_all_with(&targets, &cfg(), &o);
        assert_eq!(r[&u("/gone.png")], RemoteStatus::Ok);
        assert_eq!(r[&u("/img.png")], RemoteStatus::Ok);
        assert_eq!(r[&u("/raw.bin")], RemoteStatus::Ok);
        let h = hits.lock().unwrap().clone();
        assert_eq!(h.get("/gone.png"), None);
        assert_eq!(h["/img.png"], 1);
        assert_eq!(h["/raw.bin"], 1);

        // Offline: cached verdicts (even stale ones) only, no requests.
        let mut c = Cache::load(o.cache_path.clone());
        c.insert(
            u("/login.png"),
            RemoteStatus::Ok,
            Some("text/html".into()),
            now - 20 * 24 * 3600,
        );
        c.save().unwrap();
        let mut config = cfg();
        config.links.offline = true;
        let targets = [u("/login.png"), u("/noct.png")].map(Target::image);
        let r = check_all_with(&targets, &config, &o);
        assert_eq!(
            r[&u("/login.png")],
            RemoteStatus::NotImage("text/html".into())
        );
        assert_eq!(r[&u("/noct.png")], RemoteStatus::Skipped);
        assert_eq!(hits.lock().unwrap().get("/login.png"), None);
        assert_eq!(hits.lock().unwrap().get("/noct.png"), None);
    }

    #[test]
    fn image_occurrences_and_findings() {
        use crate::rules::Analyzed;
        use crate::source::{FileKind, SourceFile};
        let src = "![a](https://i.org/a.png) [l](https://i.org/page) ![r][pic]\n\n\
            <img src=\"https://i.org/b.png\"> <picture><source srcset=\"https://i.org/c.webp 1x, https://i.org/d.webp 2x\"></picture>\n\n\
            [pic]: https://i.org/ref.png\n";
        let a = Analyzed::new(SourceFile::new(
            "/t/a.md".into(),
            "a.md".into(),
            FileKind::Markdown,
            src.into(),
        ));
        let config = Config::default();
        let ctx = FileCtx {
            a: &a,
            config: &config,
        };
        let mut want = vec![
            Target::image("https://i.org/a.png"),
            Target::image("https://i.org/b.png"),
            Target::image("https://i.org/c.webp"),
            Target::link("https://i.org/page"),
            Target::image("https://i.org/ref.png"),
        ];
        want.sort();
        assert_eq!(urls(&ctx), want);
        let statuses: HashMap<String, RemoteStatus> = [
            (
                "https://i.org/a.png",
                RemoteStatus::NotImage("text/html".into()),
            ),
            ("https://i.org/b.png", RemoteStatus::HttpError(404)),
            (
                "https://i.org/c.webp",
                RemoteStatus::Unreachable("dns".into()),
            ),
            ("https://i.org/ref.png", RemoteStatus::Blocked(403)),
            (
                "https://i.org/page",
                RemoteStatus::NotImage("text/html".into()),
            ),
        ]
        .map(|(k, v)| (k.to_string(), v))
        .into();
        let mut out = Vec::new();
        report(&ctx, &statuses, &mut out);
        let got: Vec<(&str, &str)> = out
            .iter()
            .map(|f| (f.rule.as_str(), &src[f.range.clone()]))
            .collect();
        assert_eq!(
            got,
            vec![
                ("links/image-url", "https://i.org/a.png"),
                ("links/image-url", "https://i.org/b.png"),
                ("links/image-url", "https://i.org/c.webp"),
                ("links/image-url", "https://i.org/ref.png"),
            ],
            "no http-* double reports; HTML is fine for the plain link"
        );
        assert!(
            out[0]
                .message
                .contains("returns an HTML page, not an image")
        );
        assert!(out[1].message.contains("HTTP 404"));

        // With links/image-url off, images fall back to the http-* rules.
        let mut config = Config::default();
        config
            .rules
            .insert("links/image-url".into(), crate::config::Level::Off);
        let ctx = FileCtx {
            a: &a,
            config: &config,
        };
        assert!(urls(&ctx).iter().all(|t| !t.image));
        let mut out = Vec::new();
        report(&ctx, &statuses, &mut out);
        let rules: Vec<&str> = out.iter().map(|f| f.rule.as_str()).collect();
        assert_eq!(
            rules,
            vec![
                "links/http-error",
                "links/http-unreachable",
                "links/http-unreachable"
            ]
        );
    }

    #[test]
    fn engine_keeps_link_cache_in_project() {
        let (base, hits) = server();
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
        std::fs::write(
            root.join("a.md"),
            format!("# A\n\n![Logo]({base}/login.png)\n"),
        )
        .unwrap();
        let mut config = cfg();
        config.root = root.clone();
        let files = vec![root.join("a.md")];
        let ws = crate::engine::build_workspace(&files, &config);
        // `--no-cache` (no results cache dir) still keeps the link cache in the project.
        let mut eo = crate::engine::Options {
            remote: true,
            cache_dir: None,
            prune_cache: false,
            ..Default::default()
        };
        let d = crate::engine::check(&ws, &files, &config, &eo);
        assert!(d.iter().any(|d| d.rule == "links/image-url"), "{d:?}");
        let cache_dir = root.join(crate::cache::DEFAULT_DIR);
        assert!(cache_dir.join("links.json").exists());
        assert!(cache_dir.join(".gitignore").exists());
        assert!(cache_dir.join("CACHEDIR.TAG").exists());
        // `--cache-dir` moves it.
        eo.cache_dir = Some(root.join("ci-cache"));
        crate::engine::check(&ws, &files, &config, &eo);
        assert!(root.join("ci-cache/links.json").exists());
        assert_eq!(hits.lock().unwrap()["HEAD /login.png"], 2);
        // `links.cache = false` turns it off: nothing written, every run hits the network.
        config.links.cache = false;
        eo.cache_dir = Some(root.join("off"));
        crate::engine::check(&ws, &files, &config, &eo);
        crate::engine::check(&ws, &files, &config, &eo);
        assert!(!root.join("off/links.json").exists());
        assert_eq!(hits.lock().unwrap()["HEAD /login.png"], 4);
    }
}
