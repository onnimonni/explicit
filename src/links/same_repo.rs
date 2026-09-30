//! `links/same-repo-ref`: GitHub URLs into this repository are checked locally instead of over
//! anonymous HTTP (which answers 404 for everything in a private repository).
//!
//! - `blob|tree|raw|blame/<ref>/<path>`: the default branch is checked against the working tree
//!   (`links/missing-file`); other branches, tags and commits with `git cat-file` (ref and
//!   path at that ref).
//! - `commit/<sha>`, `commits/<ref>`, `compare/<a>...<b>`, `releases/tag/<tag>`: ref exists.
//! - `issues/<n>`, `pull/<n>`, `discussions/<n>`: GitHub GraphQL via `gh`, else a token.
//!
//! Anything unverifiable (no git, shallow clone, no gh or token) produces no finding.

use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;
use std::sync::LazyLock;

use regex::Regex;

use super::gh::{Answer, Item};
use super::git_local::{CloneKind, Obj};
use super::github::{Repo, exists_exact, normalize_slug};
use super::{is_excluded, percent_decode, sev};
use crate::config::Config;
use crate::diagnostic::{Finding, Severity};
use crate::rules::{Analyzed, FileCtx, Out};

pub const RULE: &str = "links/same-repo-ref";
const MISSING_FILE: &str = "links/missing-file";
/// Ref/path splits tried for branch names containing `/`.
const MAX_SPLITS: usize = 6;

/// What a same-repo URL points at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Kind {
    /// `blob|tree|raw|blame/<ref>/<path>` and raw.githubusercontent.com: decoded segments.
    File {
        segs: Vec<String>,
        raw: bool,
    },
    /// `commit/<sha>`, `commits/<ref>`.
    Rev(String),
    /// `compare/<a>...<b>` sides.
    Compare(Vec<String>),
    /// `releases/tag/<tag>`, `releases/download/<tag>/...`.
    Tag(String),
    Issue(u64),
    Pull(u64),
    Discussion(u64),
    /// In this repository but not checked (repository home, actions, wiki...).
    Other,
}

/// `Some` when `url` points into the repository `slug` (lowercase `owner/repo`).
pub(crate) fn parse(url: &str, slug: &str) -> Option<Kind> {
    static RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?i)^https?://(?:(?:www\.)?github\.com/([^/?#]+)/([^/?#]+)(/[^?#]*)?|raw\.githubusercontent\.com/([^/?#]+)/([^/?#]+)(/[^?#]*)?)(?:[?#].*)?$")
            .expect("hardcoded regex is valid")
    });
    let c = RE.captures(url.trim())?;
    let raw_host = c.get(4).is_some();
    let owner = c.get(1).or(c.get(4))?.as_str();
    let repo = c.get(2).or(c.get(5))?.as_str();
    if normalize_slug(&format!("{owner}/{repo}"))? != slug {
        return None;
    }
    let path = c.get(3).or(c.get(6)).map_or("", |m| m.as_str());
    let segs: Vec<String> = path
        .split('/')
        .filter(|s| !s.is_empty())
        .map(percent_decode)
        .collect();
    if raw_host {
        return Some(if segs.is_empty() {
            Kind::Other
        } else {
            Kind::File { segs, raw: true }
        });
    }
    let num = |i: usize| segs.get(i).and_then(|s| s.parse::<u64>().ok());
    let strip_ext = |s: &str| {
        s.strip_suffix(".patch")
            .or_else(|| s.strip_suffix(".diff"))
            .unwrap_or(s)
            .to_string()
    };
    let first = segs.first().map(|s| s.to_ascii_lowercase());
    Some(match first.as_deref() {
        Some(k @ ("blob" | "tree" | "raw" | "blame")) if segs.len() > 1 => Kind::File {
            segs: segs[1..].to_vec(),
            raw: k == "raw",
        },
        Some("commit") if segs.len() > 1 => Kind::Rev(strip_ext(&segs[1])),
        Some("commits") if segs.len() > 1 => Kind::Rev(segs[1..].join("/")),
        Some("compare") if segs.len() > 1 => {
            let spec = strip_ext(&segs[1..].join("/"));
            let sides = match spec.split_once("...") {
                Some((a, b)) => vec![a.to_string(), b.to_string()],
                None => match spec.split_once("..") {
                    Some((a, b)) => vec![a.to_string(), b.to_string()],
                    None => vec![spec],
                },
            };
            Kind::Compare(sides.into_iter().filter(|s| !s.is_empty()).collect())
        }
        Some("releases") => match segs.get(1).map(String::as_str) {
            Some("tag") if segs.len() > 2 => Kind::Tag(segs[2..].join("/")),
            Some("download") if segs.len() > 2 => Kind::Tag(segs[2].clone()),
            _ => Kind::Other,
        },
        Some("issues") => num(1).map_or(Kind::Other, Kind::Issue),
        Some("pull") => num(1).map_or(Kind::Other, Kind::Pull),
        Some("discussions") => num(1).map_or(Kind::Other, Kind::Discussion),
        _ => Kind::Other,
    })
}

fn is_sha(s: &str) -> bool {
    (7..=40).contains(&s.len()) && s.chars().all(|c| c.is_ascii_hexdigit())
}

/// Rev names worth asking git about (no rev syntax, forks or odd characters).
fn plain_ref(s: &str) -> bool {
    !s.is_empty()
        && !s.starts_with(['-', '/'])
        && !s.contains("..")
        && !s.contains(|c: char| c.is_whitespace() || c.is_control() || "~^:?*[\\@{".contains(c))
}

/// Git revs for a ref name as written in a URL: SHAs as is, names also on the GitHub remote.
fn revs(name: &str, repo: &Repo) -> Vec<String> {
    if !plain_ref(name) {
        Vec::new()
    } else if is_sha(name) {
        vec![name.to_ascii_lowercase()]
    } else if let Some(b) = name.strip_prefix("refs/heads/") {
        vec![
            name.to_string(),
            format!("refs/remotes/{}/{b}", repo.remote),
        ]
    } else if name.starts_with("refs/") {
        vec![name.to_string()]
    } else {
        vec![name.to_string(), format!("{}/{name}", repo.remote)]
    }
}

/// How a file URL's `<ref>/<path>` resolves.
enum FileRef {
    /// Default branch: path in the working tree.
    Default(String),
    /// Candidate `(ref name, revs, path)` splits, shortest ref first.
    Splits(Vec<(String, Vec<String>, String)>),
}

fn file_ref(segs: &[String], repo: &Repo) -> FileRef {
    let joined = segs.join("/");
    let rest = joined.strip_prefix("refs/heads/").unwrap_or(&joined);
    for b in &repo.branches {
        if rest == b {
            return FileRef::Default(String::new());
        }
        if let Some(p) = rest
            .strip_prefix(b.as_str())
            .and_then(|p| p.strip_prefix('/'))
        {
            return FileRef::Default(p.to_string());
        }
    }
    // `refs/heads/x` and `refs/tags/x` keep their prefix as part of the ref.
    let skip =
        if segs.len() > 2 && segs[0] == "refs" && matches!(segs[1].as_str(), "heads" | "tags") {
            2
        } else {
            0
        };
    let max = if is_sha(&segs[0]) { 1 } else { MAX_SPLITS };
    let splits = (skip + 1..=segs.len())
        .take(max)
        .map(|i| {
            let name = segs[..i].join("/");
            let revs = revs(&name, repo);
            (name, revs, segs[i..].join("/"))
        })
        .collect();
    FileRef::Splits(splits)
}

/// Object names to look up for one URL.
fn specs(kind: &Kind, repo: &Repo, out: &mut BTreeSet<String>) {
    let mut rev = |n: &str| {
        for r in revs(n, repo) {
            out.insert(format!("{r}^{{commit}}"));
        }
    };
    match kind {
        Kind::File { segs, .. } => {
            if let FileRef::Splits(splits) = file_ref(segs, repo) {
                for (_, revs, path) in splits {
                    for r in revs {
                        out.insert(format!("{r}^{{commit}}"));
                        if !path.is_empty() {
                            out.insert(format!("{r}:{path}"));
                        }
                    }
                }
            }
        }
        Kind::Rev(n) => rev(n),
        Kind::Compare(sides) => sides.iter().for_each(|s| rev(s)),
        Kind::Tag(t) if plain_ref(t) => {
            out.insert(format!("refs/tags/{t}"));
        }
        _ => {}
    }
}

/// Answers gathered before reporting.
#[derive(Debug, Clone, Default)]
pub struct Results {
    /// `git cat-file --batch-check` answers; empty when git did not run.
    pub git: HashMap<String, Obj>,
    pub clone: CloneKind,
    /// GitHub answers (issues, and refs/paths git could not confirm); missing keys are unverified.
    pub items: HashMap<Item, Answer>,
}

/// Whether a rev exists: `Some(true)`, `Some(false)` (missing), `None` (unknown).
fn rev_found(res: &Results, revs: &[String]) -> Option<bool> {
    for r in revs {
        match res.git.get(&format!("{r}^{{commit}}"))? {
            Obj::Found(_) | Obj::Ambiguous => return Some(true),
            Obj::Missing => {}
        }
    }
    (!revs.is_empty()).then_some(false)
}

fn rev_label(name: &str) -> String {
    if is_sha(name) {
        format!("commit {name}")
    } else {
        format!("ref `{name}`")
    }
}

/// What local git says about one URL.
enum Local {
    Fine,
    /// Definite local problem that GitHub cannot change (default branch path missing).
    Problem(&'static str, String),
    /// Not confirmed locally: ask GitHub. `definite`: git said missing (not merely a shallow or
    /// partial clone, or no git); `msg` describes it.
    Suspect {
        definite: bool,
        msg: String,
    },
}

/// What GitHub says about a suspect URL.
enum Remote {
    Fine,
    Missing(String),
    Unknown,
}

fn suspect(definite: bool, what: String) -> Local {
    Local::Suspect {
        definite,
        msg: format!("{what} not found in local git and could not verify on GitHub"),
    }
}

fn local_rev(n: &str, repo: &Repo, res: &Results) -> Local {
    let revs = revs(n, repo);
    match rev_found(res, &revs) {
        Some(true) => Local::Fine,
        _ if revs.is_empty() => Local::Fine,
        Some(false) => suspect(!res.clone.shallow, rev_label(n)),
        None => suspect(false, rev_label(n)),
    }
}

fn local_file(segs: &[String], raw: bool, repo: &Repo, res: &Results) -> Local {
    let splits = match file_ref(segs, repo) {
        FileRef::Default(path) => {
            return if !path.is_empty() && exists_exact(&repo.top, &path).is_none() {
                Local::Problem(
                    MISSING_FILE,
                    format!(
                        "`{path}` does not exist in the working tree (linked on the default branch)"
                    ),
                )
            } else {
                Local::Fine
            };
        }
        FileRef::Splits(s) => s,
    };
    let Some((first, first_revs, _)) = splits.first() else {
        return Local::Fine;
    };
    if first_revs.is_empty() {
        return Local::Fine;
    }
    let unknown = || suspect(false, rev_label(first));
    let mut ref_hit: Option<(String, String, bool)> = None;
    for (name, revs, path) in &splits {
        for r in revs {
            match res.git.get(&format!("{r}^{{commit}}")) {
                None => return unknown(),
                Some(Obj::Missing) => continue,
                Some(Obj::Ambiguous) => return Local::Fine,
                Some(Obj::Found(_)) => {}
            }
            if path.is_empty() {
                return Local::Fine;
            }
            if res.clone.partial {
                return unknown();
            }
            match res.git.get(&format!("{r}:{path}")) {
                None => return unknown(),
                Some(Obj::Found(t)) if !raw || t == "blob" => return Local::Fine,
                Some(Obj::Ambiguous) => return Local::Fine,
                Some(o) => {
                    let dir = matches!(o, Obj::Found(_));
                    ref_hit.get_or_insert((name.clone(), path.clone(), dir));
                }
            }
        }
    }
    match ref_hit {
        Some((name, path, dir)) => Local::Suspect {
            definite: true,
            msg: if dir {
                format!(
                    "`{path}` at `{name}` is a directory in local git (could not verify on GitHub)"
                )
            } else {
                format!(
                    "`{path}` not found at `{name}` in local git and could not verify on GitHub"
                )
            },
        },
        None => suspect(!res.clone.shallow, rev_label(first)),
    }
}

fn local(kind: &Kind, repo: &Repo, res: &Results) -> Local {
    match kind {
        Kind::File { segs, raw } => local_file(segs, *raw, repo, res),
        Kind::Rev(n) => local_rev(n, repo, res),
        Kind::Tag(t) if plain_ref(t) => match res.git.get(&format!("refs/tags/{t}")) {
            Some(Obj::Found(_) | Obj::Ambiguous) => Local::Fine,
            Some(Obj::Missing) => suspect(!res.clone.shallow, format!("tag `{t}`")),
            None => suspect(false, format!("tag `{t}`")),
        },
        // Compare: evaluated side by side in `evaluate`.
        _ => Local::Fine,
    }
}

/// GitHub items that can settle a suspect URL.
fn remote_items(kind: &Kind, repo: &Repo, out: &mut BTreeSet<Item>) {
    match kind {
        Kind::File { segs, .. } => {
            if let FileRef::Splits(splits) = file_ref(segs, repo) {
                for (name, _, path) in splits.into_iter().filter(|s| !s.1.is_empty()) {
                    out.insert(Item::Object(name.clone()));
                    if !path.is_empty() {
                        out.insert(Item::Object(format!("{name}:{path}")));
                    }
                }
            }
        }
        Kind::Rev(n) if plain_ref(n) => {
            out.insert(Item::Object(n.clone()));
        }
        Kind::Tag(t) if plain_ref(t) => {
            out.insert(Item::Tag(t.clone()));
        }
        _ => {}
    }
}

fn answered<'a>(res: &'a Results, it: &Item) -> Option<&'a Answer> {
    res.items.get(it)
}

fn remote_rev(n: &str, res: &Results) -> Remote {
    match answered(res, &Item::Object(n.to_string())) {
        Some(Answer::Found(_)) => Remote::Fine,
        Some(Answer::Missing) => {
            Remote::Missing(format!("{} not found locally or on GitHub", rev_label(n)))
        }
        None => Remote::Unknown,
    }
}

fn remote(kind: &Kind, repo: &Repo, res: &Results) -> Remote {
    match kind {
        Kind::File { segs, raw } => {
            let FileRef::Splits(splits) = file_ref(segs, repo) else {
                return Remote::Fine;
            };
            let splits: Vec<_> = splits.into_iter().filter(|s| !s.1.is_empty()).collect();
            let path_ans = |name: &str, path: &str| {
                if path.is_empty() {
                    answered(res, &Item::Object(name.to_string()))
                } else {
                    answered(res, &Item::Object(format!("{name}:{path}")))
                }
            };
            let ok = |a: Option<&Answer>| matches!(a, Some(Answer::Found(t)) if !raw || t == "Blob" || t.is_empty());
            if splits.iter().any(|(n, _, p)| ok(path_ans(n, p))) {
                return Remote::Fine;
            }
            let mut all_missing = true;
            for (name, _, path) in &splits {
                match answered(res, &Item::Object(name.clone())) {
                    Some(Answer::Found(_)) => {
                        return match path_ans(name, path) {
                            Some(Answer::Found(_)) => Remote::Missing(format!(
                                "`{path}` at `{name}` is a directory, not a file (checked locally and on GitHub)"
                            )),
                            Some(Answer::Missing) => Remote::Missing(format!(
                                "`{path}` does not exist at `{name}` (checked locally and on GitHub)"
                            )),
                            None => Remote::Unknown,
                        };
                    }
                    Some(Answer::Missing) => {}
                    None => all_missing = false,
                }
            }
            match splits.first() {
                Some((name, _, _)) if all_missing => Remote::Missing(format!(
                    "{} not found locally or on GitHub",
                    rev_label(name)
                )),
                _ => Remote::Unknown,
            }
        }
        Kind::Rev(n) => remote_rev(n, res),
        Kind::Tag(t) => match answered(res, &Item::Tag(t.clone())) {
            Some(Answer::Found(_)) => Remote::Fine,
            Some(Answer::Missing) => {
                Remote::Missing(format!("tag `{t}` not found locally or on GitHub"))
            }
            None => Remote::Unknown,
        },
        _ => Remote::Fine,
    }
}

/// `(rule, severity, message)` of a problem with a same-repo URL; `None` when fine or
/// unverifiable. Not found locally but on GitHub: fine. Missing on GitHub too: error. GitHub
/// unreachable after git said missing: info.
fn evaluate(kind: &Kind, repo: &Repo, res: &Results) -> Option<(&'static str, Severity, String)> {
    let slug = &repo.slug;
    let issue = |it: Item, what: String| {
        matches!(res.items.get(&it), Some(Answer::Missing))
            .then(|| (RULE, sev(RULE), format!("{what} not found in {slug}")))
    };
    match kind {
        Kind::Issue(n) => issue(Item::Issue(*n), format!("issue #{n}")),
        Kind::Pull(n) => issue(Item::Issue(*n), format!("pull request #{n}")),
        Kind::Discussion(n) => issue(Item::Discussion(*n), format!("discussion #{n}")),
        Kind::Compare(sides) => sides
            .iter()
            .find_map(|s| evaluate(&Kind::Rev(s.clone()), repo, res)),
        Kind::Other => None,
        _ => match local(kind, repo, res) {
            Local::Fine => None,
            Local::Problem(rule, msg) => Some((rule, sev(rule), msg)),
            Local::Suspect { definite, msg } => match remote(kind, repo, res) {
                Remote::Fine => None,
                Remote::Missing(m) => Some((RULE, sev(RULE), m)),
                Remote::Unknown => definite.then_some((RULE, Severity::Info, msg)),
            },
        },
    }
}

/// This repository when same-repo checking is on.
pub(crate) fn repo(ctx: &FileCtx) -> Option<Repo> {
    if !ctx.config.links.check_same_repo {
        return None;
    }
    super::github::repo(ctx)
}

/// Same-repo URLs of a file, with what they point at.
fn kinds(ctx: &FileCtx, repo: &Repo) -> Vec<(super::remote::Occurrence, Kind)> {
    super::remote::occurrences(ctx)
        .into_iter()
        .filter(|o| !is_excluded(ctx.config, &o.url))
        .filter_map(|o| parse(&o.url, &repo.slug).map(|k| (o, k)))
        .collect()
}

/// Run git, then GitHub (gh, or the token fallback; cache only without `network`) for issues
/// and whatever git could not confirm, for the same-repo URLs of `files`.
pub fn check_all<'a>(
    files: impl IntoIterator<Item = &'a Analyzed>,
    config: &Config,
    cache_path: Option<PathBuf>,
    network: bool,
) -> Results {
    let mut repo_found: Option<Repo> = None;
    let mut specs_set = BTreeSet::new();
    let mut all: Vec<Kind> = Vec::new();
    for a in files {
        let ctx = FileCtx { a, config };
        if !ctx.enabled(RULE) && !ctx.enabled(MISSING_FILE) {
            continue;
        }
        if repo_found.is_none() {
            repo_found = repo(&ctx);
        }
        let Some(repo) = &repo_found else {
            return Results::default();
        };
        for (_, kind) in kinds(&ctx, repo) {
            specs(&kind, repo, &mut specs_set);
            if !all.contains(&kind) {
                all.push(kind);
            }
        }
    }
    let Some(repo) = repo_found else {
        return Results::default();
    };
    let mut res = Results::default();
    if !specs_set.is_empty()
        && let Some(dirs) = super::git_local::git_dirs(&repo.top)
    {
        res.clone = super::git_local::clone_kind(&dirs);
        let specs: Vec<String> = specs_set.into_iter().collect();
        res.git = super::git_local::batch_check(&dirs.top, &specs).unwrap_or_default();
    }
    let mut items = BTreeSet::new();
    for kind in &all {
        match kind {
            Kind::Issue(n) | Kind::Pull(n) => {
                items.insert(Item::Issue(*n));
            }
            Kind::Discussion(n) => {
                items.insert(Item::Discussion(*n));
            }
            Kind::Compare(sides) => {
                for s in sides {
                    let k = Kind::Rev(s.clone());
                    if matches!(local(&k, &repo, &res), Local::Suspect { .. }) {
                        remote_items(&k, &repo, &mut items);
                    }
                }
            }
            k => {
                if matches!(local(k, &repo, &res), Local::Suspect { .. }) {
                    remote_items(k, &repo, &mut items);
                }
            }
        }
    }
    if !items.is_empty() {
        let items: Vec<Item> = items.into_iter().collect();
        res.items = super::gh::check(&repo.slug, &items, config, cache_path, network);
    }
    res
}

/// Findings for the same-repo URLs of one file.
pub fn report(ctx: &FileCtx, res: &Results, out: &mut Out) {
    let Some(repo) = repo(ctx) else { return };
    let ref_on = ctx.enabled(RULE);
    let file_on = ctx.enabled(MISSING_FILE);
    if !ref_on && !file_on {
        return;
    }
    for (o, kind) in kinds(ctx, &repo) {
        let Some((rule, severity, msg)) = evaluate(&kind, &repo, res) else {
            continue;
        };
        if (rule == RULE && !ref_on) || (rule == MISSING_FILE && !file_on) {
            continue;
        }
        let help = if severity == Severity::Info {
            "Fetch the ref, or let explicit ask GitHub: install and log in to gh, or set GH_TOKEN"
        } else {
            "Links into this repository are checked with local git and GitHub, not anonymous HTTP"
        };
        out.push(Finding::new(rule, severity, o.range, msg).help(help));
    }
}

/// Whether `url` points into this repository (such URLs are never sent to the HTTP checker).
pub(crate) fn is_same_repo(repo: &Repo, url: &str) -> bool {
    parse(url, &repo.slug).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::{FileKind, SourceFile};
    use std::path::Path;
    use std::process::Command;

    #[test]
    fn parses_urls() {
        let s = "me/proj";
        let p = |u: &str| parse(u, s);
        assert_eq!(
            p("https://github.com/Me/Proj/blob/v1.0/docs/a%20b.md#L3"),
            Some(Kind::File {
                segs: vec!["v1.0".into(), "docs".into(), "a b.md".into()],
                raw: false
            })
        );
        assert_eq!(
            p("https://raw.githubusercontent.com/me/proj/abc1234/x.png"),
            Some(Kind::File {
                segs: vec!["abc1234".into(), "x.png".into()],
                raw: true
            })
        );
        assert_eq!(
            p("https://github.com/me/proj/commit/abc1234.patch"),
            Some(Kind::Rev("abc1234".into()))
        );
        assert_eq!(
            p("https://github.com/me/proj/compare/v1...feature/x"),
            Some(Kind::Compare(vec!["v1".into(), "feature/x".into()]))
        );
        assert_eq!(
            p("https://github.com/me/proj/releases/tag/v2.0"),
            Some(Kind::Tag("v2.0".into()))
        );
        assert_eq!(
            p("https://github.com/me/proj/releases/download/v2.0/x.tar.gz"),
            Some(Kind::Tag("v2.0".into()))
        );
        assert_eq!(
            p("https://github.com/me/proj/pull/732/files"),
            Some(Kind::Pull(732))
        );
        assert_eq!(
            p("https://github.com/me/proj/issues/791#issuecomment-1"),
            Some(Kind::Issue(791))
        );
        assert_eq!(
            p("https://github.com/me/proj/discussions/5"),
            Some(Kind::Discussion(5))
        );
        assert_eq!(p("https://github.com/me/proj.git"), Some(Kind::Other));
        assert_eq!(
            p("https://github.com/me/proj/issues?q=is%3Aopen"),
            Some(Kind::Other)
        );
        assert_eq!(p("https://github.com/me/proj/actions"), Some(Kind::Other));
        assert_eq!(p("https://github.com/me/projx/issues/1"), None);
        assert_eq!(p("https://github.com/else/proj/issues/1"), None);
        assert_eq!(p("https://gitlab.com/me/proj/issues/1"), None);
    }

    fn git_available() -> bool {
        Command::new("git")
            .arg("--version")
            .output()
            .is_ok_and(|o| o.status.success())
    }

    fn git(dir: &Path, args: &[&str]) -> String {
        let o = Command::new("git")
            .arg("-C")
            .arg(dir)
            .args([
                "-c",
                "user.name=T",
                "-c",
                "user.email=t@example.com",
                "-c",
                "commit.gpgsign=false",
                "-c",
                "tag.gpgsign=false",
            ])
            .args(args)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .output()
            .unwrap();
        assert!(
            o.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&o.stderr)
        );
        String::from_utf8(o.stdout).unwrap().trim().to_string()
    }

    fn write(root: &Path, p: &str, text: &str) {
        let full = root.join(p);
        std::fs::create_dir_all(full.parent().unwrap()).unwrap();
        std::fs::write(full, text).unwrap();
    }

    /// Findings of `report` for `doc.md` in `root` after `check_all`.
    fn run(root: &Path, config: &Config) -> (Vec<Finding>, String) {
        let text = std::fs::read_to_string(root.join("doc.md")).unwrap();
        let a = Analyzed::new(SourceFile::new(
            root.join("doc.md"),
            PathBuf::from("doc.md"),
            FileKind::Markdown,
            text.clone(),
        ));
        let res = check_all([&a], config, None, false);
        let ctx = FileCtx { a: &a, config };
        let mut out = Vec::new();
        report(&ctx, &res, &mut out);
        (out, text)
    }

    #[test]
    fn validates_against_local_git() {
        if !git_available() {
            eprintln!("skipping: git not installed");
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
        git(&root, &["init", "-q", "-b", "main"]);
        git(
            &root,
            &["remote", "add", "origin", "git@github.com:Me/Proj.git"],
        );
        write(&root, "docs/old.md", "# Old\n");
        write(&root, "src/lib.rs", "\n");
        git(&root, &["add", "."]);
        git(&root, &["commit", "-q", "-m", "one"]);
        let sha = git(&root, &["rev-parse", "HEAD"]);
        git(&root, &["tag", "-a", "v1.0", "-m", "v1"]);
        git(&root, &["tag", "light"]);
        git(&root, &["branch", "feature/x"]);
        git(
            &root,
            &["update-ref", "refs/remotes/origin/only-remote", &sha],
        );
        git(&root, &["rm", "-q", "docs/old.md"]);
        git(&root, &["commit", "-q", "-m", "two"]);
        // Loose objects into a pack: lookups must also find packed objects and refs.
        git(&root, &["gc", "-q"]);
        let short = &sha[..7];
        let doc = format!(
            "# Doc\n\n\
             [ok sha path](https://github.com/me/proj/blob/{sha}/docs/old.md)\n\
             [ok short](https://github.com/me/proj/commit/{short})\n\
             [ok tag](https://github.com/me/proj/tree/v1.0/src)\n\
             [ok light](https://github.com/me/proj/releases/tag/light)\n\
             [ok slash branch](https://github.com/me/proj/blob/feature/x/docs/old.md)\n\
             [ok remote branch](https://github.com/me/proj/blob/only-remote/src/lib.rs)\n\
             [ok compare](https://github.com/me/proj/compare/v1.0...main)\n\
             [ok dir as blob](https://github.com/me/proj/blob/v1.0/src)\n\
             [ok main](https://github.com/me/proj/blob/main/src/lib.rs)\n\
             [ok issue unverified](https://github.com/me/proj/issues/1)\n\
             [bad sha](https://github.com/me/proj/commit/0123456789abcdef0123456789abcdef01234567)\n\
             [bad path](https://github.com/me/proj/blob/v1.0/docs/none.md)\n\
             [bad tag](https://github.com/me/proj/releases/tag/v9)\n\
             [bad ref](https://github.com/me/proj/tree/gone/docs)\n\
             [bad compare](https://github.com/me/proj/compare/v1.0...deadbeef1)\n\
             [bad raw dir](https://raw.githubusercontent.com/me/proj/v1.0/src)\n\
             [bad main](https://github.com/me/proj/blob/main/docs/old.md)\n"
        );
        write(&root, "doc.md", &doc);
        let config = Config {
            root: root.clone(),
            ..Config::default()
        };
        // No network and no cached GitHub answers: what git calls missing is info only.
        let (f, text) = run(&root, &config);
        let got: Vec<(&str, &str, Severity, &str)> = f
            .iter()
            .map(|x| {
                // Link label of the line the finding is on.
                let line = text[..x.range.start].rsplit('\n').next().unwrap_or("");
                (
                    line.split(']').next().unwrap_or(line),
                    x.rule.as_str(),
                    x.severity,
                    x.message.as_str(),
                )
            })
            .collect();
        let info = Severity::Info;
        assert_eq!(
            got,
            vec![
                (
                    "[bad sha",
                    RULE,
                    info,
                    "commit 0123456789abcdef0123456789abcdef01234567 not found in local git and could not verify on GitHub"
                ),
                (
                    "[bad path",
                    RULE,
                    info,
                    "`docs/none.md` not found at `v1.0` in local git and could not verify on GitHub"
                ),
                (
                    "[bad tag",
                    RULE,
                    info,
                    "tag `v9` not found in local git and could not verify on GitHub"
                ),
                (
                    "[bad ref",
                    RULE,
                    info,
                    "ref `gone` not found in local git and could not verify on GitHub"
                ),
                (
                    "[bad compare",
                    RULE,
                    info,
                    "commit deadbeef1 not found in local git and could not verify on GitHub"
                ),
                (
                    "[bad raw dir",
                    RULE,
                    info,
                    "`src` at `v1.0` is a directory in local git (could not verify on GitHub)"
                ),
                (
                    "[bad main",
                    MISSING_FILE,
                    Severity::Error,
                    "`docs/old.md` does not exist in the working tree (linked on the default branch)"
                ),
            ],
            "{f:#?}"
        );

        // Shallow clone: missing refs and commits are inconclusive; paths at found refs still count.
        std::fs::write(root.join(".git/shallow"), format!("{sha}\n")).unwrap();
        let (f, _) = run(&root, &config);
        let msgs: Vec<&str> = f.iter().map(|x| x.message.as_str()).collect();
        assert_eq!(msgs.len(), 3, "{msgs:?}");
        assert!(msgs[0].contains("docs/none.md"));
        std::fs::remove_file(root.join(".git/shallow")).unwrap();

        // Linked worktree: `.git` file and commondir.
        let wt = root.join("wt");
        git(
            &root,
            &["worktree", "add", "-q", wt.to_str().unwrap(), "feature/x"],
        );
        write(
            &wt,
            "doc.md",
            &format!(
                "[x](https://github.com/me/proj/commit/{short})\n[y](https://github.com/me/proj/releases/tag/nope)\n"
            ),
        );
        let wconfig = Config {
            root: wt.clone(),
            ..Config::default()
        };
        let (f, _) = run(&wt, &wconfig);
        assert_eq!(f.len(), 1, "{f:?}");
        assert!(f[0].message.contains("tag `nope`"));

        // Disabled: nothing.
        let mut off = config.clone();
        off.links.check_same_repo = false;
        assert!(run(&root, &off).0.is_empty());
    }

    fn test_repo() -> Repo {
        Repo {
            slug: "me/proj".into(),
            branches: vec!["main".into()],
            top: PathBuf::from("/nonexistent"),
            remote: "origin".into(),
        }
    }

    fn msg(kind: &Kind, res: &Results) -> Option<(Severity, String)> {
        evaluate(kind, &test_repo(), res).map(|x| (x.1, x.2))
    }

    #[test]
    fn issues_from_results() {
        let mut res = Results::default();
        res.items
            .insert(Item::Issue(1), Answer::Found(String::new()));
        res.items.insert(Item::Issue(2), Answer::Missing);
        res.items.insert(Item::Discussion(3), Answer::Missing);
        assert_eq!(msg(&Kind::Issue(1), &res), None);
        assert_eq!(
            msg(&Kind::Pull(2), &res),
            Some((
                Severity::Error,
                "pull request #2 not found in me/proj".into()
            ))
        );
        assert_eq!(
            msg(&Kind::Discussion(3), &res),
            Some((Severity::Error, "discussion #3 not found in me/proj".into()))
        );
        // Unverified items, and refs git never answered (no git) with no GitHub answer: silent.
        assert_eq!(msg(&Kind::Issue(9), &res), None);
        assert_eq!(msg(&Kind::Rev("abc1234".into()), &res), None);
        assert_eq!(msg(&Kind::Tag("v1".into()), &res), None);
        let file = Kind::File {
            segs: vec!["v1".into(), "x.md".into()],
            raw: false,
        };
        assert_eq!(msg(&file, &res), None);
    }

    /// Results where git answered "missing" for every object name of `kinds`.
    fn all_missing(kinds: &[&Kind]) -> Results {
        let mut specs_set = BTreeSet::new();
        for k in kinds {
            specs(k, &test_repo(), &mut specs_set);
        }
        Results {
            git: specs_set.into_iter().map(|s| (s, Obj::Missing)).collect(),
            ..Results::default()
        }
    }

    #[test]
    fn github_settles_what_git_misses() {
        let sha = Kind::Rev("abc1234".into());
        let tag = Kind::Tag("v9".into());
        let file = Kind::File {
            segs: vec!["v1".into(), "x.md".into()],
            raw: false,
        };
        let mut res = all_missing(&[&sha, &tag, &file]);
        // The ref exists locally but the path does not.
        res.git
            .insert("v1^{commit}".into(), Obj::Found("commit".into()));
        let mut remote = BTreeSet::new();
        for k in [&sha, &tag, &file] {
            assert!(matches!(
                local(k, &test_repo(), &res),
                Local::Suspect { .. }
            ));
            remote_items(k, &test_repo(), &mut remote);
        }
        assert_eq!(
            remote,
            BTreeSet::from([
                Item::Object("abc1234".into()),
                Item::Object("v1".into()),
                Item::Object("v1/x.md".into()),
                Item::Object("v1:x.md".into()),
                Item::Tag("v9".into()),
            ])
        );
        // GitHub unreachable: info.
        for k in [&sha, &tag, &file] {
            assert_eq!(msg(k, &res).map(|m| m.0), Some(Severity::Info), "{k:?}");
        }
        // GitHub has them: nothing to report.
        let mut found = res.clone();
        for (it, ty) in [
            (Item::Object("abc1234".into()), "Commit"),
            (Item::Tag("v9".into()), "Ref"),
            (Item::Object("v1".into()), "Commit"),
            (Item::Object("v1:x.md".into()), "Blob"),
        ] {
            found.items.insert(it, Answer::Found(ty.into()));
        }
        for k in [&sha, &tag, &file] {
            assert_eq!(msg(k, &found), None, "{k:?}");
        }
        // GitHub lacks them too: errors.
        let mut missing = res.clone();
        for it in [
            Item::Object("abc1234".into()),
            Item::Tag("v9".into()),
            Item::Object("v1:x.md".into()),
            Item::Object("v1/x.md".into()),
        ] {
            missing.items.insert(it, Answer::Missing);
        }
        missing
            .items
            .insert(Item::Object("v1".into()), Answer::Found("Commit".into()));
        let errs: Vec<_> = [&sha, &tag, &file]
            .iter()
            .map(|k| msg(k, &missing))
            .collect();
        assert_eq!(
            errs,
            vec![
                Some((
                    Severity::Error,
                    "commit abc1234 not found locally or on GitHub".into()
                )),
                Some((
                    Severity::Error,
                    "tag `v9` not found locally or on GitHub".into()
                )),
                Some((
                    Severity::Error,
                    "`x.md` does not exist at `v1` (checked locally and on GitHub)".into()
                )),
            ]
        );
        // Shallow clone: silent locally, but GitHub still decides.
        let mut shallow = all_missing(&[&sha]);
        shallow.clone.shallow = true;
        assert_eq!(msg(&sha, &shallow), None);
        shallow
            .items
            .insert(Item::Object("abc1234".into()), Answer::Missing);
        assert_eq!(msg(&sha, &shallow).map(|m| m.0), Some(Severity::Error));
        // Raw URL of a directory on GitHub.
        let raw = Kind::File {
            segs: vec!["abc1234".into(), "src".into()],
            raw: true,
        };
        let mut r = all_missing(&[&raw]);
        r.items.insert(
            Item::Object("abc1234".into()),
            Answer::Found("Commit".into()),
        );
        r.items.insert(
            Item::Object("abc1234:src".into()),
            Answer::Found("Tree".into()),
        );
        assert!(msg(&raw, &r).unwrap().1.contains("is a directory"));
    }
}
