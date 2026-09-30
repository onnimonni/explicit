//! Existence of this repository's issues, pull requests and discussions via GitHub's GraphQL API:
//! `gh api graphql` when gh is installed and logged in (it may keep the token in the system
//! keychain), else HTTPS with `GH_TOKEN`, `GITHUB_TOKEN` or the token in gh's `hosts.yml`.
//! Results live in the link cache; the token is never logged or cached.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use super::cache::{Cache, now_unix};
use super::remote::RemoteStatus;
use crate::config::Config;

/// Aliases per GraphQL query.
pub const BATCH: usize = 50;
pub const API_URL: &str = "https://api.github.com/graphql";

/// Something to look up on GitHub; `Issue` also covers pull requests (shared number space).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Item {
    Issue(u64),
    Discussion(u64),
    /// A git object by rev expression: `<rev>` or `<rev>:<path>` (abbreviated SHAs work).
    Object(String),
    /// A tag name (`refs/tags/<name>`).
    Tag(String),
}

/// What GitHub says about an item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Answer {
    /// Exists; holds the GraphQL type (`Blob`, `Tree`, `Commit`...; empty when not asked).
    Found(String),
    Missing,
}

impl Answer {
    pub fn found(&self) -> bool {
        matches!(self, Answer::Found(_))
    }
}

impl Item {
    /// Alias of the item at position `i` of a query.
    fn alias(&self, i: usize) -> String {
        match self {
            Item::Issue(n) => format!("i{n}"),
            Item::Discussion(n) => format!("d{n}"),
            Item::Object(_) => format!("o{i}"),
            Item::Tag(_) => format!("t{i}"),
        }
    }

    /// Whether a positive answer can never change: numbers, tags and objects named by SHA
    /// (branches move, so their answers expire).
    pub fn permanent(&self) -> bool {
        match self {
            Item::Issue(_) | Item::Discussion(_) | Item::Tag(_) => true,
            Item::Object(e) => {
                let rev = e.split_once(':').map_or(e.as_str(), |(r, _)| r);
                (7..=40).contains(&rev.len()) && rev.chars().all(|c| c.is_ascii_hexdigit())
            }
        }
    }

    fn cache_key(&self, slug: &str) -> String {
        match self {
            Item::Issue(n) => format!("gh:{slug}/issue/{n}"),
            Item::Discussion(n) => format!("gh:{slug}/discussion/{n}"),
            Item::Object(e) => format!("gh:{slug}/object/{e}"),
            Item::Tag(t) => format!("gh:{slug}/tag/{t}"),
        }
    }
}

fn valid_name(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

/// GraphQL string literal (JSON escapes are valid GraphQL escapes).
fn lit(s: &str) -> String {
    serde_json::Value::String(s.to_string()).to_string()
}

/// One GraphQL query asking whether each item exists.
pub fn query(owner: &str, name: &str, items: &[Item]) -> String {
    let mut q = format!("query{{repository(owner:\"{owner}\",name:\"{name}\"){{");
    for (i, it) in items.iter().enumerate() {
        let a = it.alias(i);
        q.push_str(&match it {
            Item::Issue(n) => format!(" {a}:issueOrPullRequest(number:{n}){{__typename}}"),
            Item::Discussion(n) => format!(" {a}:discussion(number:{n}){{id}}"),
            Item::Object(e) => format!(" {a}:object(expression:{}){{__typename}}", lit(e)),
            Item::Tag(t) => format!(
                " {a}:ref(qualifiedName:{}){{__typename}}",
                lit(&format!("refs/tags/{t}"))
            ),
        });
    }
    q.push_str("}}");
    q
}

/// Items the response answers. `null` means missing: for issues only with a `NOT_FOUND` error,
/// for objects and tags unless another error (permissions, rate limits) names the alias; such
/// items are left out. `None` when the repository itself is not visible.
pub fn parse(json: &str, items: &[Item]) -> Option<Vec<(Item, Answer)>> {
    let v: serde_json::Value = serde_json::from_str(json).ok()?;
    let repo = v.get("data")?.get("repository")?.as_object()?;
    // Error type naming `alias` (`Some("")` for an error without a type).
    let error = |alias: &str| {
        v.get("errors").and_then(|e| e.as_array()).and_then(|errs| {
            errs.iter().find_map(|e| {
                let at = e
                    .get("path")
                    .and_then(|p| p.as_array())
                    .and_then(|p| p.last())
                    .and_then(|p| p.as_str())
                    == Some(alias);
                at.then(|| e.get("type").and_then(|t| t.as_str()).unwrap_or(""))
            })
        })
    };
    Some(
        items
            .iter()
            .enumerate()
            .filter_map(|(i, it)| {
                let a = it.alias(i);
                match repo.get(&a)? {
                    serde_json::Value::Object(o) => {
                        let ty = o.get("__typename").and_then(|t| t.as_str()).unwrap_or("");
                        Some((it.clone(), Answer::Found(ty.to_string())))
                    }
                    serde_json::Value::Null => {
                        let missing = matches!(
                            (it, error(&a)),
                            (_, Some("NOT_FOUND")) | (Item::Object(_) | Item::Tag(_), None)
                        );
                        missing.then(|| (it.clone(), Answer::Missing))
                    }
                    _ => None,
                }
            })
            .collect(),
    )
}

/// Token from `GH_TOKEN`, `GITHUB_TOKEN`, else gh's `hosts.yml`.
pub fn token() -> Option<String> {
    token_from(&|k| std::env::var(k).ok())
}

fn token_from(get: &dyn Fn(&str) -> Option<String>) -> Option<String> {
    for k in ["GH_TOKEN", "GITHUB_TOKEN"] {
        if let Some(t) = get(k)
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty())
        {
            return Some(t);
        }
    }
    hosts_token(&std::fs::read_to_string(hosts_path(get)?).ok()?)
}

/// gh's `hosts.yml`: `$GH_CONFIG_DIR`, `$XDG_CONFIG_HOME/gh`, `%AppData%/GitHub CLI` or `~/.config/gh`.
fn hosts_path(get: &dyn Fn(&str) -> Option<String>) -> Option<PathBuf> {
    let nonempty = |k: &str| get(k).filter(|v| !v.is_empty()).map(PathBuf::from);
    let dir = nonempty("GH_CONFIG_DIR")
        .or_else(|| nonempty("XDG_CONFIG_HOME").map(|d| d.join("gh")))
        .or_else(|| {
            cfg!(windows)
                .then(|| nonempty("AppData").map(|d| d.join("GitHub CLI")))
                .flatten()
        })
        .or_else(|| nonempty("HOME").map(|h| h.join(".config/gh")))?;
    Some(dir.join("hosts.yml"))
}

/// `oauth_token` of `github.com` in gh's `hosts.yml` (host level, else the active user's).
fn hosts_token(text: &str) -> Option<String> {
    let indent = |l: &str| l.len() - l.trim_start().len();
    let value = |l: &str, key: &str| {
        l.trim()
            .strip_prefix(key)
            .and_then(|r| r.strip_prefix(':'))
            .map(|v| v.trim().trim_matches(['"', '\'']).to_string())
    };
    let mut lines = text.lines().filter(|l| {
        let t = l.trim();
        !t.is_empty() && !t.starts_with('#')
    });
    lines.find(|l| indent(l) == 0 && value(l, "github.com").is_some())?;
    let block: Vec<&str> = lines.take_while(|l| indent(l) > 0).collect();
    let child = indent(block.first()?);
    let direct = |key: &str| {
        block
            .iter()
            .filter(|l| indent(l) == child)
            .find_map(|l| value(l, key))
            .filter(|v| !v.is_empty())
    };
    if let Some(t) = direct("oauth_token") {
        return Some(t);
    }
    let user = direct("user")?;
    let i = block
        .iter()
        .position(|l| indent(l) > child && value(l, &user).is_some())?;
    let user_indent = indent(block[i]);
    block[i + 1..]
        .iter()
        .take_while(|l| indent(l) > user_indent)
        .find_map(|l| value(l, "oauth_token"))
        .filter(|v| !v.is_empty())
}

fn via_gh(bin: &str, q: &str) -> Option<String> {
    let mut cmd = Command::new(bin);
    cmd.args(["api", "graphql", "-f"])
        .arg(format!("query={q}"))
        .env("GH_PROMPT_DISABLED", "1")
        .env("GH_NO_UPDATE_NOTIFIER", "1")
        .env("NO_COLOR", "1");
    // GraphQL errors (e.g. one missing issue) exit non-zero but still print the response.
    let (_, out) = super::git_local::run(&mut cmd, None, super::git_local::TIMEOUT)?;
    let s = String::from_utf8(out).ok()?;
    (!s.trim().is_empty()).then_some(s)
}

fn via_http(api_url: &str, token: &str, q: &str, timeout: Duration) -> Option<String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(timeout))
        .http_status_as_error(false)
        .user_agent(super::remote::USER_AGENT)
        .build()
        .into();
    let body = serde_json::json!({ "query": q }).to_string();
    let mut resp = agent
        .post(api_url)
        .header("Authorization", &format!("Bearer {token}"))
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .header("Content-Type", "application/json")
        .send(body)
        .ok()?;
    // 401/403 (bad token, SSO, rate limit): unverifiable, not missing.
    if resp.status().as_u16() != 200 {
        return None;
    }
    resp.body_mut().read_to_string().ok()
}

/// How to reach GitHub (tests swap in a fake server and no gh).
pub struct Transport<F: FnOnce() -> Option<String>> {
    /// gh binary to try first; `None` skips gh.
    pub gh: Option<String>,
    pub api_url: String,
    /// Token for the HTTPS fallback, looked up only when needed.
    pub token: F,
}

/// Existence of `items` in `slug` (`owner/repo`): cached answers first, then the network when
/// `network` is set (never in offline mode). Unverifiable items are left out.
pub fn check(
    slug: &str,
    items: &[Item],
    config: &Config,
    cache_path: Option<PathBuf>,
    network: bool,
) -> HashMap<Item, Answer> {
    let t = Transport {
        gh: Some("gh".into()),
        api_url: API_URL.into(),
        token,
    };
    check_with(slug, items, config, cache_path, network, t)
}

pub fn check_with<F: FnOnce() -> Option<String>>(
    slug: &str,
    items: &[Item],
    config: &Config,
    cache_path: Option<PathBuf>,
    network: bool,
    transport: Transport<F>,
) -> HashMap<Item, Answer> {
    let lc = &config.links;
    let mut items = items.to_vec();
    items.sort();
    items.dedup();
    let mut out = HashMap::new();
    if items.is_empty() {
        return out;
    }
    let mut cache = Cache::load(cache_path);
    let now = now_unix();
    let answer = |e: &super::cache::Entry| match e.status {
        RemoteStatus::Ok | RemoteStatus::Redirect(_) => {
            Answer::Found(e.content_type.clone().unwrap_or_default())
        }
        _ => Answer::Missing,
    };
    let network = network && !lc.offline;
    let mut todo = Vec::new();
    for it in items {
        let key = it.cache_key(slug);
        cache.touch(&key, now);
        if let Some(e) = cache.fresh(&key, now, lc.cache_ttl_hours, lc.cache_failed_ttl_hours) {
            out.insert(it, answer(e));
        } else if !network {
            if let Some(e) = cache.get(&key) {
                out.insert(it, answer(e));
            }
        } else {
            todo.push(it);
        }
    }
    let Some((owner, name)) = slug.split_once('/') else {
        return out;
    };
    if !todo.is_empty() && valid_name(owner) && valid_name(name) {
        let Transport {
            gh,
            api_url,
            token: token_fn,
        } = transport;
        let mut gh = gh;
        let mut token_fn = Some(token_fn);
        let mut token: Option<String> = None;
        let timeout = Duration::from_secs(lc.timeout_secs.max(1) * 3);
        for chunk in todo.chunks(BATCH) {
            let q = query(owner, name, chunk);
            let mut res = gh
                .as_deref()
                .and_then(|b| via_gh(b, &q))
                .and_then(|j| parse(&j, chunk));
            if res.is_none() {
                // gh missing, logged out or without access: use the token from now on.
                gh = None;
                if let Some(f) = token_fn.take() {
                    token = f();
                }
                res = token
                    .as_deref()
                    .and_then(|t| via_http(&api_url, t, &q, timeout))
                    .and_then(|j| parse(&j, chunk));
            }
            let Some(res) = res else { break };
            for (it, ans) in res {
                // The GraphQL type rides in the entry's content type.
                let (status, ty) = match &ans {
                    Answer::Found(t) => (RemoteStatus::Ok, Some(t.clone())),
                    Answer::Missing => (RemoteStatus::HttpError(404), None),
                };
                cache.insert_with(it.cache_key(slug), status, ty, now, it.permanent());
                out.insert(it, ans);
            }
        }
    }
    if let Err(e) = cache.save() {
        eprintln!("explicit: could not write link cache: {e}");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};

    #[test]
    fn builds_query() {
        assert_eq!(
            query(
                "o",
                "r",
                &[
                    Item::Issue(7),
                    Item::Discussion(9),
                    Item::Object("abc1234:docs/\"q\".md".into()),
                    Item::Tag("v1".into())
                ]
            ),
            "query{repository(owner:\"o\",name:\"r\"){ i7:issueOrPullRequest(number:7){__typename} d9:discussion(number:9){id} o2:object(expression:\"abc1234:docs/\\\"q\\\".md\"){__typename} t3:ref(qualifiedName:\"refs/tags/v1\"){__typename}}}"
        );
    }

    #[test]
    fn parses_response() {
        let items = [
            Item::Issue(1),
            Item::Issue(2),
            Item::Discussion(3),
            Item::Issue(4),
            Item::Object("abc1234:src".into()),
            Item::Object("abc1234:none.md".into()),
            Item::Tag("v1".into()),
            Item::Tag("v9".into()),
            Item::Object("secret".into()),
        ];
        // Missing objects and refs are plain nulls; missing issues come with NOT_FOUND errors.
        let json = r#"{"data":{"repository":{"i1":{"__typename":"PullRequest"},"i2":null,"d3":null,"i4":null,
            "o4":{"__typename":"Tree"},"o5":null,"t6":{"__typename":"Ref"},"t7":null,"o8":null}},
            "errors":[{"type":"NOT_FOUND","path":["repository","i2"],"message":"x"},
                      {"type":"NOT_FOUND","path":["repository","d3"]},
                      {"type":"FORBIDDEN","path":["repository","i4"]},
                      {"type":"FORBIDDEN","path":["repository","o8"]}]}"#;
        assert_eq!(
            parse(json, &items).unwrap(),
            vec![
                (Item::Issue(1), Answer::Found("PullRequest".into())),
                (Item::Issue(2), Answer::Missing),
                (Item::Discussion(3), Answer::Missing),
                (
                    Item::Object("abc1234:src".into()),
                    Answer::Found("Tree".into())
                ),
                (Item::Object("abc1234:none.md".into()), Answer::Missing),
                (Item::Tag("v1".into()), Answer::Found("Ref".into())),
                (Item::Tag("v9".into()), Answer::Missing),
            ]
        );
        let no_repo =
            r#"{"data":{"repository":null},"errors":[{"type":"NOT_FOUND","path":["repository"]}]}"#;
        assert!(parse(no_repo, &items).is_none());
        assert!(parse("not json", &items).is_none());
    }

    #[test]
    fn permanence() {
        assert!(Item::Issue(1).permanent() && Item::Tag("v1".into()).permanent());
        assert!(Item::Object("abc1234".into()).permanent());
        assert!(Item::Object("abc1234:docs/x.md".into()).permanent());
        assert!(!Item::Object("feature/x:docs/x.md".into()).permanent());
        assert!(!Item::Object("main".into()).permanent());
    }
    #[test]
    fn token_sources() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path().to_string_lossy().to_string();
        std::fs::create_dir_all(dir.path().join("gh")).unwrap();
        std::fs::write(
            dir.path().join("gh/hosts.yml"),
            "github.com:\n    git_protocol: ssh\n    oauth_token: from-file\n    user: me\n",
        )
        .unwrap();
        let env = |pairs: Vec<(&'static str, String)>| {
            move |k: &str| pairs.iter().find(|(n, _)| *n == k).map(|(_, v)| v.clone())
        };
        let e = env(vec![("GH_TOKEN", "a".into()), ("GITHUB_TOKEN", "b".into())]);
        assert_eq!(token_from(&e).as_deref(), Some("a"));
        let e = env(vec![("GH_TOKEN", " ".into()), ("GITHUB_TOKEN", "b".into())]);
        assert_eq!(token_from(&e).as_deref(), Some("b"));
        let e = env(vec![
            ("XDG_CONFIG_HOME", d.clone()),
            ("HOME", "/nonexistent".into()),
        ]);
        assert_eq!(token_from(&e).as_deref(), Some("from-file"));
        let e = env(vec![("GH_CONFIG_DIR", format!("{d}/gh"))]);
        assert_eq!(token_from(&e).as_deref(), Some("from-file"));
        let e = env(vec![("HOME", "/nonexistent".into())]);
        assert_eq!(token_from(&e), None);
    }

    #[test]
    fn hosts_yml_variants() {
        assert_eq!(
            hosts_token("gitlab.com:\n  oauth_token: no\ngithub.com:\n  oauth_token: \"yes\"\n")
                .as_deref(),
            Some("yes")
        );
        // Multi-account layout: the active user's token.
        let multi = "github.com:\n    users:\n        other:\n            oauth_token: o\n        me:\n            oauth_token: m\n    git_protocol: https\n    user: me\n";
        assert_eq!(hosts_token(multi).as_deref(), Some("m"));
        // Token in the keychain: no oauth_token lines.
        assert_eq!(
            hosts_token("github.com:\n    users:\n        me:\n    user: me\n"),
            None
        );
        assert_eq!(hosts_token("enterprise.local:\n  oauth_token: x\n"), None);
    }

    type Reqs = Arc<Mutex<Vec<String>>>;

    /// Fake GraphQL endpoint: issue 1 exists, everything else is NOT_FOUND; 401 without the right token.
    fn server() -> (String, Reqs) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/graphql", listener.local_addr().unwrap());
        let reqs: Reqs = Arc::default();
        let r = reqs.clone();
        std::thread::spawn(move || {
            for mut s in listener.incoming().flatten() {
                let mut reader = BufReader::new(s.try_clone().unwrap());
                let mut head = String::new();
                let mut len = 0;
                loop {
                    let mut l = String::new();
                    if reader.read_line(&mut l).unwrap_or(0) == 0 || l == "\r\n" {
                        break;
                    }
                    if let Some(v) = l.to_ascii_lowercase().strip_prefix("content-length:") {
                        len = v.trim().parse().unwrap_or(0);
                    }
                    head.push_str(&l);
                }
                let mut body = vec![0; len];
                reader.read_exact(&mut body).unwrap();
                let body = String::from_utf8(body).unwrap();
                r.lock().unwrap().push(format!("{head}\n{body}"));
                let authed = head
                    .to_ascii_lowercase()
                    .contains("authorization: bearer secret-tok");
                let q: serde_json::Value = serde_json::from_str(&body).unwrap();
                let q = q["query"].as_str().unwrap_or("");
                let (status, resp) = if !authed {
                    (401, "{}".to_string())
                } else {
                    let mut data = serde_json::Map::new();
                    let mut errors = Vec::new();
                    for alias in ["i1", "i2", "d3"] {
                        if q.contains(&format!(" {alias}:")) {
                            if alias == "i1" {
                                data.insert(
                                    alias.into(),
                                    serde_json::json!({"__typename":"Issue"}),
                                );
                            } else {
                                data.insert(alias.into(), serde_json::Value::Null);
                                errors.push(serde_json::json!({"type":"NOT_FOUND","path":["repository",alias]}));
                            }
                        }
                    }
                    (
                        200,
                        serde_json::json!({"data":{"repository":data},"errors":errors}).to_string(),
                    )
                };
                let _ = write!(
                    s,
                    "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{resp}",
                    resp.len()
                );
            }
        });
        (url, reqs)
    }

    fn transport(url: &str, tok: &str) -> Transport<impl FnOnce() -> Option<String>> {
        let tok = tok.to_string();
        Transport {
            gh: None,
            api_url: url.to_string(),
            token: move || Some(tok),
        }
    }

    #[test]
    fn http_fallback_with_cache_and_offline() {
        let (url, reqs) = server();
        let dir = tempfile::tempdir().unwrap();
        let cache = dir.path().join("links.json");
        let config = Config::default();
        let items = [Item::Issue(1), Item::Issue(2), Item::Discussion(3)];
        let got = check_with(
            "me/proj",
            &items,
            &config,
            Some(cache.clone()),
            true,
            transport(&url, "secret-tok"),
        );
        assert_eq!(got.len(), 3);
        assert!(got[&Item::Issue(1)].found());
        assert!(!got[&Item::Issue(2)].found() && !got[&Item::Discussion(3)].found());
        {
            let r = reqs.lock().unwrap();
            assert_eq!(r.len(), 1, "one batched query");
            assert!(r[0].starts_with("POST /graphql"));
            assert!(
                r[0].to_ascii_lowercase()
                    .contains("accept: application/vnd.github+json")
            );
            assert!(
                r[0].to_ascii_lowercase()
                    .contains("x-github-api-version: 2022-11-28")
            );
        }
        let saved = std::fs::read_to_string(&cache).unwrap();
        assert!(saved.contains("gh:me/proj/issue/1"));
        assert!(!saved.contains("secret-tok"), "token never cached");
        // Fresh cache: no request, not even for the not-found ones (1h failure TTL).
        let got2 = check_with(
            "me/proj",
            &items,
            &config,
            Some(cache.clone()),
            true,
            transport(&url, "secret-tok"),
        );
        assert_eq!(got, got2);
        assert_eq!(reqs.lock().unwrap().len(), 1);
        // Offline: uncached items stay unknown, nothing is requested.
        let mut off = Config::default();
        off.links.offline = true;
        let got = check_with(
            "me/proj",
            &[Item::Issue(5)],
            &off,
            Some(cache.clone()),
            true,
            transport(&url, "secret-tok"),
        );
        assert!(got.is_empty());
        assert_eq!(reqs.lock().unwrap().len(), 1);
        // Bad token (401): unverified, not cached.
        let got = check_with(
            "me/proj",
            &[Item::Issue(6)],
            &config,
            Some(cache.clone()),
            true,
            transport(&url, "wrong"),
        );
        assert!(got.is_empty());
        assert!(!std::fs::read_to_string(&cache).unwrap().contains("issue/6"));
        // No token at all: nothing requested.
        let n = reqs.lock().unwrap().len();
        let t = Transport {
            gh: None,
            api_url: url.clone(),
            token: || None,
        };
        assert!(check_with("me/proj", &[Item::Issue(7)], &config, None, true, t).is_empty());
        assert_eq!(reqs.lock().unwrap().len(), n);
    }

    #[test]
    fn missing_gh_binary_falls_back() {
        let (url, reqs) = server();
        let t = Transport {
            gh: Some("explicit-no-such-gh".into()),
            api_url: url,
            token: || Some("secret-tok".to_string()),
        };
        let got = check_with(
            "me/proj",
            &[Item::Issue(1)],
            &Config::default(),
            None,
            true,
            t,
        );
        assert!(got[&Item::Issue(1)].found());
        assert_eq!(reqs.lock().unwrap().len(), 1);
    }
}
