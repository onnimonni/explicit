//! Remote (http/https) link checking with a persistent cache and per-host politeness.

use std::collections::{HashMap, HashSet, VecDeque};
use std::ops::Range;
use std::path::PathBuf;
use std::sync::{Condvar, Mutex};
use std::time::{Duration, Instant};

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
}

const MAX_REDIRECTS: usize = 5;
const MAX_PER_HOST: usize = 2;
const MAX_RETRY_AFTER: Duration = Duration::from_secs(10);
/// Statuses sites use to turn away bots or anonymous clients (999 is LinkedIn's).
const BLOCKED_STATUSES: &[u16] = &[401, 403, 999];
/// Many sites reject non-browser clients outright, so look like one while staying identifiable.
const USER_AGENT: &str = concat!(
    "Mozilla/5.0 (compatible; explicit-link-checker/",
    env!("CARGO_PKG_VERSION"),
    ")"
);
const ACCEPT: &str = "text/html,application/xhtml+xml,*/*;q=0.8";

/// Tunables for `check_all_with` (tests use a temp cache and no pacing).
#[derive(Debug, Clone)]
pub struct CheckOpts {
    pub cache_path: Option<PathBuf>,
    /// Minimum time between starting requests to the same host.
    pub min_interval: Duration,
}

impl Default for CheckOpts {
    fn default() -> Self {
        CheckOpts {
            cache_path: super::cache::default_path(),
            min_interval: Duration::from_millis(250),
        }
    }
}

/// Check all URLs, using and updating the on-disk cache.
pub fn check_all(urls: &[String], config: &Config) -> HashMap<String, RemoteStatus> {
    check_all_with(urls, config, &CheckOpts::default())
}

pub fn check_all_with(
    urls: &[String],
    config: &Config,
    opts: &CheckOpts,
) -> HashMap<String, RemoteStatus> {
    let lc = &config.links;
    let mut cache = Cache::load(opts.cache_path.clone());
    let now = now_unix();
    let mut result = HashMap::new();
    let mut todo: VecDeque<String> = VecDeque::new();
    let mut seen = HashSet::new();
    for url in urls {
        let key = strip_fragment(url).to_string();
        if !seen.insert(key.clone()) {
            continue;
        }
        let status = if !is_http(&key) || is_excluded(config, &key) {
            Some(RemoteStatus::Skipped)
        } else if let Some(s) =
            cache.fresh(&key, now, lc.cache_ttl_hours, lc.cache_failed_ttl_hours)
        {
            Some(s.clone())
        } else if lc.offline {
            Some(
                cache
                    .get(&key)
                    .map_or(RemoteStatus::Skipped, |e| e.status.clone()),
            )
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
    let checked: Mutex<Vec<(String, RemoteStatus)>> = Mutex::new(Vec::new());
    let workers = lc.concurrency.max(1).min(sched.len());
    std::thread::scope(|s| {
        for _ in 0..workers {
            s.spawn(|| {
                while let Some((url, host)) = sched.next() {
                    let status = check_url(&agent, &url, config);
                    sched.done(&host);
                    checked
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .push((url, status));
                }
            });
        }
    });
    let now = now_unix();
    for (url, status) in checked.into_inner().unwrap_or_else(|e| e.into_inner()) {
        cache.insert(url.clone(), status.clone(), now);
        result.insert(url, status);
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
}

fn send(agent: &ureq::Agent, get: bool, url: &str) -> Result<Resp, String> {
    let r = if get {
        agent.get(url).header("Accept", ACCEPT).call()
    } else {
        agent.head(url).header("Accept", ACCEPT).call()
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
    })
}

fn send_retry(agent: &ureq::Agent, get: bool, url: &str) -> Result<Resp, String> {
    let r = send(agent, get, url)?;
    if r.status != 429 {
        return Ok(r);
    }
    let wait = r
        .retry_after
        .as_deref()
        .and_then(|s| s.trim().parse::<u64>().ok())
        .map_or(Duration::from_secs(1), Duration::from_secs);
    std::thread::sleep(wait.min(MAX_RETRY_AFTER));
    send(agent, get, url)
}

/// HEAD, falling back to GET when the server rejects HEAD.
fn fetch(agent: &ureq::Agent, url: &str) -> Result<Resp, String> {
    let r = send_retry(agent, false, url)?;
    if ((400..500).contains(&r.status) && r.status != 429) || r.status == 501 {
        return send_retry(agent, true, url);
    }
    Ok(r)
}

fn check_url(agent: &ureq::Agent, url: &str, config: &Config) -> RemoteStatus {
    let accept = &config.links.accept_status;
    let mut current = url.to_string();
    let mut permanent = true;
    // Resolved here (in a worker) rather than while planning, so DNS lookups run in parallel.
    if !config.links.allow_private && is_private_url(url) {
        return RemoteStatus::Skipped;
    }
    for _ in 0..=MAX_REDIRECTS {
        if current != url
            && let Some(refused) = refuse_hop(config, &current)
        {
            return refused;
        }
        let r = match fetch(agent, &current) {
            Ok(r) => r,
            Err(e) => return RemoteStatus::Unreachable(e),
        };
        if (200..300).contains(&r.status) || accept.contains(&r.status) {
            return if current != url && permanent {
                RemoteStatus::Redirect(current)
            } else {
                RemoteStatus::Ok
            };
        }
        match (r.status, r.location) {
            (301 | 302 | 303 | 307 | 308, Some(loc)) => {
                permanent &= matches!(r.status, 301 | 308);
                current = join_url(&current, loc.trim());
            }
            (s, _) if BLOCKED_STATUSES.contains(&s) => return RemoteStatus::Blocked(s),
            (s, _) => return RemoteStatus::HttpError(s),
        }
    }
    RemoteStatus::Unreachable(format!("more than {MAX_REDIRECTS} redirects"))
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
struct Occurrence {
    url: String,
    range: Range<usize>,
}

fn occurrences(ctx: &FileCtx) -> Vec<Occurrence> {
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
            });
        }
        for d in md.ref_defs.iter().filter(|d| is_http(&d.dest)) {
            let exact = src
                .get(d.range.clone())
                .and_then(|raw| raw.find(d.dest.as_str()))
                .map(|i| d.range.start + i..d.range.start + i + d.dest.len());
            v.push(Occurrence {
                url: d.dest.clone(),
                range: exact.unwrap_or(d.range.clone()),
            });
        }
    } else {
        for (url, range) in super::local::comment_urls(ctx.a) {
            v.push(Occurrence { url, range });
        }
    }
    v
}

/// Report statuses for the remote links of one file.
pub fn report(ctx: &FileCtx, statuses: &HashMap<String, RemoteStatus>, out: &mut Out) {
    for o in occurrences(ctx) {
        let url = &o.url;
        match statuses.get(strip_fragment(url)) {
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
            Some(RemoteStatus::Ok | RemoteStatus::Skipped) | None => {}
        }
    }
}

/// Remote URLs linked from a file, without fragments.
pub fn urls(ctx: &FileCtx) -> Vec<String> {
    let mut v: Vec<String> = occurrences(ctx)
        .into_iter()
        .map(|o| strip_fragment(&o.url).to_string())
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

    fn opts(dir: &tempfile::TempDir) -> CheckOpts {
        CheckOpts {
            cache_path: Some(dir.path().join("links.json")),
            min_interval: Duration::ZERO,
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
        let r = check_all_with(&urls, &cfg(), &opts(&dir));
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
        let r = check_all_with(&urls, &cfg(), &opts(&dir));
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
        let r = check_all_with(std::slice::from_ref(&url), &c, &opts(&dir));
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
        let r = check_all_with(std::slice::from_ref(&url), &c, &opts(&dir));
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
        let r = check_all_with(std::slice::from_ref(&dead), &cfg(), &opts(&dir));
        assert!(matches!(r[&dead], RemoteStatus::Unreachable(_)), "{r:?}");
        let r = check_all_with(std::slice::from_ref(&dead), &Config::default(), &opts(&dir));
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
        // Stale failure (1h TTL): re-checked. Fresh success (24h TTL) on a 404 path: not re-checked.
        c.insert(format!("{base}/ok"), RemoteStatus::HttpError(500), old);
        c.insert(format!("{base}/missing"), RemoteStatus::Ok, old);
        c.save().unwrap();
        let urls = vec![format!("{base}/ok"), format!("{base}/missing")];
        let r = check_all_with(&urls, &cfg(), &o);
        assert_eq!(r[&urls[0]], RemoteStatus::Ok);
        assert_eq!(r[&urls[1]], RemoteStatus::Ok);
        assert_eq!(hits.lock().unwrap().get("/missing"), None);
        assert_eq!(hits.lock().unwrap()["/ok"], 1);
        // Second run: everything cached.
        let r = check_all_with(&urls, &cfg(), &o);
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
            now_unix() - 48 * 3600,
        );
        c.save().unwrap();
        let mut config = cfg();
        config.links.offline = true;
        let urls = vec![format!("{base}/moved"), format!("{base}/ok")];
        let r = check_all_with(&urls, &config, &o);
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
        let r = check_all_with(&urls, &cfg(), &o);
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
            vec![
                "https://a.org/old",
                "https://b.org/gone",
                "https://c.org/down",
                "https://d.org/private"
            ]
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
}
