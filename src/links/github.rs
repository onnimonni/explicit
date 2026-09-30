//! `links/same-repo-url`: GitHub URLs into this repository's default branch that should be
//! relative paths, so they keep working on forks, branches and in local previews.

use std::collections::HashMap;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex, PoisonError};

use regex::Regex;

use super::{dest_range, encode_path_segment, percent_decode, sev};
use crate::diagnostic::Finding;
use crate::extract::markdown::LinkKind;
use crate::rules::{FileCtx, Out};

const RULE: &str = "links/same-repo-url";

/// This repository as seen from the checked project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Repo {
    /// Lowercase `owner/repo`.
    pub slug: String,
    /// Default branch names a URL ref may use.
    pub branches: Vec<String>,
    /// Working tree top (URL paths are relative to it).
    pub top: PathBuf,
    /// Git remote pointing at the repository on GitHub (for `<remote>/<branch>` lookups).
    pub remote: String,
}

/// What git metadata says about the working tree containing a directory.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct GitInfo {
    top: Option<PathBuf>,
    slug: Option<String>,
    default_branch: Option<String>,
    remote: Option<String>,
}

static GIT_CACHE: LazyLock<Mutex<HashMap<PathBuf, GitInfo>>> = LazyLock::new(Mutex::default);

/// Repo identity from `links.github_repo` / `links.github_default_branch`, else git metadata.
pub fn repo(ctx: &FileCtx) -> Option<Repo> {
    let lc = &ctx.config.links;
    let root = ctx
        .config
        .root
        .canonicalize()
        .unwrap_or_else(|_| ctx.config.root.clone());
    let git = {
        let mut g = GIT_CACHE.lock().unwrap_or_else(PoisonError::into_inner);
        g.entry(root.clone())
            .or_insert_with(|| git_info(&root))
            .clone()
    };
    let slug = match lc.github_repo.as_deref() {
        Some(r) => normalize_slug(r)?,
        None => git.slug?,
    };
    let branches = match lc.github_default_branch.clone().or(git.default_branch) {
        Some(b) => vec![b],
        None => ["main", "master", "HEAD"].map(String::from).to_vec(),
    };
    Some(Repo {
        slug,
        branches,
        top: git.top.unwrap_or(root),
        remote: git.remote.unwrap_or_else(|| "origin".into()),
    })
}

pub(crate) fn normalize_slug(s: &str) -> Option<String> {
    let s = s.trim().trim_end_matches('/');
    let s = s.strip_suffix(".git").unwrap_or(s);
    let (o, r) = s.split_once('/')?;
    (!o.is_empty() && !r.is_empty() && !r.contains('/')).then(|| format!("{o}/{r}").to_lowercase())
}

/// Find `.git` at or above `start` and read the GitHub remote and default branch.
fn git_info(start: &Path) -> GitInfo {
    let Some(dirs) = super::git_local::git_dirs(start) else {
        return GitInfo::default();
    };
    // Linked worktrees keep config and remote refs in the common dir.
    let Some(common) = dirs.common else {
        return GitInfo {
            top: Some(dirs.top),
            ..GitInfo::default()
        };
    };
    let config = std::fs::read_to_string(common.join("config")).unwrap_or_default();
    let (remote, slug) = github_remote(&config).unzip();
    let default_branch = remote.as_ref().and_then(|r| {
        let head = std::fs::read_to_string(common.join(format!("refs/remotes/{r}/HEAD"))).ok()?;
        head.trim()
            .strip_prefix(&format!("ref: refs/remotes/{r}/"))
            .map(String::from)
    });
    GitInfo {
        top: Some(dirs.top),
        slug,
        default_branch,
        remote,
    }
}

/// `(remote name, owner/repo)` of `origin` if it is on GitHub, else of the first GitHub remote.
fn github_remote(config: &str) -> Option<(String, String)> {
    let mut remotes: Vec<(String, String)> = Vec::new();
    let mut section: Option<String> = None;
    for line in config.lines() {
        let l = line.trim();
        if l.starts_with('[') {
            section = l
                .strip_prefix("[remote \"")
                .and_then(|r| r.strip_suffix("\"]"))
                .map(String::from);
        } else if let Some(name) = &section
            && let Some((k, v)) = l.split_once('=')
            && k.trim().eq_ignore_ascii_case("url")
            && let Some(slug) = parse_remote_url(v.trim())
        {
            remotes.push((name.clone(), slug));
        }
    }
    let i = remotes.iter().position(|(n, _)| n == "origin").unwrap_or(0);
    (!remotes.is_empty()).then(|| remotes.swap_remove(i))
}

/// `owner/repo` of a GitHub remote URL (https, `git@github.com:o/r.git`, `ssh://`, `git://`).
fn parse_remote_url(url: &str) -> Option<String> {
    static RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?i)^(?:(?:https?|ssh|git)://(?:[^@/]+@)?(?:www\.)?github\.com(?::\d+)?/|(?:[^@/]+@)?github\.com:)([^/]+/[^/]+?)(?:\.git)?/?$")
            .expect("hardcoded regex is valid")
    });
    normalize_slug(RE.captures(url)?.get(1)?.as_str())
}

/// A GitHub file URL split into parts.
#[derive(Debug, PartialEq, Eq)]
struct GhUrl<'a> {
    slug: String,
    /// `tree` URLs may point at directories.
    tree: bool,
    /// `<ref>/<path>`, still percent-encoded.
    rest: &'a str,
    query: Option<&'a str>,
    fragment: Option<&'a str>,
}

fn parse_github(url: &str) -> Option<GhUrl<'_>> {
    static RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?i)^https?://(?:(?:www\.)?github\.com/([^/?#]+)/([^/?#]+)/(blob|tree|raw)/|raw\.githubusercontent\.com/([^/?#]+)/([^/?#]+)/)([^?#]*)(?:\?([^#]*))?(?:#(.*))?$")
            .expect("hardcoded regex is valid")
    });
    let c = RE.captures(url)?;
    let owner = c.get(1).or(c.get(4))?.as_str();
    let repo = c.get(2).or(c.get(5))?.as_str();
    Some(GhUrl {
        slug: normalize_slug(&format!("{owner}/{repo}"))?,
        tree: c
            .get(3)
            .is_some_and(|m| m.as_str().eq_ignore_ascii_case("tree")),
        rest: c.get(6)?.as_str(),
        query: c.get(7).map(|m| m.as_str()),
        fragment: c.get(8).map(|m| m.as_str()),
    })
}

/// `L12`, `L12-L20`, `L12C3`...
pub(crate) fn is_line_fragment(f: &str) -> bool {
    f.strip_prefix('L')
        .is_some_and(|r| r.starts_with(|c: char| c.is_ascii_digit()))
}

/// Whether `path` (below `top`) exists with exactly this spelling, component by component.
pub(crate) fn exists_exact(top: &Path, rel: &str) -> Option<PathBuf> {
    let mut cur = top.to_path_buf();
    for part in rel.split('/').filter(|p| !p.is_empty()) {
        if part == "." || part == ".." {
            return None;
        }
        let found = std::fs::read_dir(&cur)
            .ok()?
            .flatten()
            .any(|e| e.file_name().to_str() == Some(part));
        if !found {
            return None;
        }
        cur.push(part);
    }
    Some(cur)
}

/// Relative replacement for `url` as seen from directory `from`, when it points into `repo`'s
/// default branch at something that exists locally.
pub fn rewrite(repo: &Repo, url: &str, from: &Path) -> Option<String> {
    let g = parse_github(url.trim())?;
    if g.slug != repo.slug {
        return None;
    }
    let rest = g.rest.strip_prefix("refs/heads/").unwrap_or(g.rest);
    let path = repo.branches.iter().find_map(|b| {
        rest.strip_prefix(b.as_str()).and_then(|p| {
            if p.is_empty() {
                Some("")
            } else {
                p.strip_prefix('/')
            }
        })
    })?;
    let mut keep_query = None;
    for param in g.query.unwrap_or("").split('&').filter(|p| !p.is_empty()) {
        match param {
            "raw=true" | "raw=1" => {}
            "plain=1" if g.fragment.is_some_and(is_line_fragment) => keep_query = Some(param),
            "plain=1" => {}
            // Unknown parameters may matter; leave the URL alone.
            _ => return None,
        }
    }
    let decoded = percent_decode(path);
    let target = exists_exact(&repo.top, &decoded)?;
    let is_dir = target.is_dir();
    if is_dir && !g.tree {
        return None;
    }
    let rel = super::local::relative_path(from, &target);
    let mut out: Vec<String> = rel
        .components()
        .map(|c| encode_path_segment(&c.as_os_str().to_string_lossy()))
        .collect::<Vec<_>>();
    if out.is_empty() {
        out.push(".".into());
    }
    let mut s = out.join("/");
    if is_dir {
        s.push('/');
    }
    if let Some(q) = keep_query {
        s.push('?');
        s.push_str(q);
    }
    if let Some(f) = g.fragment {
        s.push('#');
        s.push_str(f);
    }
    Some(s)
}

/// One URL occurrence: destination, report range and whether a fix may replace `range`.
struct Occ {
    url: String,
    range: Range<usize>,
    fixable: bool,
}

pub fn check(ctx: &FileCtx, out: &mut Out) {
    let Some(md) = &ctx.a.md else { return };
    let Some(repo) = repo(ctx) else { return };
    let src = ctx.src();
    let from = ctx
        .a
        .file
        .path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_default();
    let mut occs: Vec<Occ> = Vec::new();
    for l in md.links.iter().filter(|l| l.kind != LinkKind::Reference) {
        let Some(r) = dest_range(src, l) else {
            continue;
        };
        occs.push(Occ {
            url: l.dest.clone(),
            range: r,
            // GitHub does not autolink relative paths, so bare URLs and autolinks only get a suggestion.
            fixable: matches!(l.kind, LinkKind::Inline | LinkKind::Html),
        });
    }
    for d in &md.ref_defs {
        if let Some(i) = src
            .get(d.range.clone())
            .and_then(|raw| raw.find(d.dest.as_str()))
        {
            let start = d.range.start + i;
            occs.push(Occ {
                url: d.dest.clone(),
                range: start..start + d.dest.len(),
                fixable: true,
            });
        }
    }
    occs.sort_by_key(|o| o.range.start);
    occs.dedup_by(|a, b| a.range == b.range);
    for o in occs {
        let Some(rel) = rewrite(&repo, &o.url, &from) else {
            continue;
        };
        let mut f = Finding::new(
            RULE,
            sev(RULE),
            o.range.clone(),
            format!("URL points into this repository; use the relative path `{rel}`"),
        )
        .help("Relative paths keep working on forks, branches and in local previews; use a commit permalink to pin a version")
        .suggest(rel.clone());
        if o.fixable {
            f = f.fix(o.range, rel);
        }
        out.push(f);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remote_urls() {
        for u in [
            "https://github.com/Me/Proj.git",
            "https://github.com/me/proj",
            "https://user@github.com/me/proj/",
            "git@github.com:me/proj.git",
            "ssh://git@github.com/me/proj.git",
            "ssh://git@github.com:22/me/proj",
            "git://github.com/me/proj.git",
        ] {
            assert_eq!(parse_remote_url(u).as_deref(), Some("me/proj"), "{u}");
        }
        for u in [
            "https://gitlab.com/me/proj",
            "git@example.com:me/proj.git",
            "/local/path",
        ] {
            assert_eq!(parse_remote_url(u), None, "{u}");
        }
        let cfg = "[core]\n\tbare = false\n[remote \"upstream\"]\n\turl = https://github.com/up/proj\n[remote \"origin\"]\n\turl = git@github.com:me/proj.git\n";
        assert_eq!(
            github_remote(cfg),
            Some(("origin".into(), "me/proj".into()))
        );
        let cfg = "[remote \"origin\"]\n\turl = https://gitlab.com/me/proj\n[remote \"gh\"]\n\turl = https://github.com/me/proj\n";
        assert_eq!(github_remote(cfg), Some(("gh".into(), "me/proj".into())));
    }

    #[test]
    fn github_urls() {
        let g = parse_github("https://github.com/Me/Proj/blob/main/a%20b.md?plain=1#L3").unwrap();
        assert_eq!(g.slug, "me/proj");
        assert!(!g.tree);
        assert_eq!(g.rest, "main/a%20b.md");
        assert_eq!(g.query, Some("plain=1"));
        assert_eq!(g.fragment, Some("L3"));
        let g = parse_github("https://raw.githubusercontent.com/me/proj/refs/heads/main/x.png")
            .unwrap();
        assert_eq!(g.rest, "refs/heads/main/x.png");
        assert!(parse_github("https://github.com/me/proj/issues/1").is_none());
        assert!(parse_github("https://github.com/me/proj").is_none());
    }

    use crate::config::Config;
    use crate::rules::Analyzed;
    use crate::source::{FileKind, SourceFile};

    struct Env {
        _dir: tempfile::TempDir,
        root: PathBuf,
    }

    fn env(files: &[(&str, &str)]) -> Env {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
        for (p, text) in files {
            let full = root.join(p);
            std::fs::create_dir_all(full.parent().unwrap()).unwrap();
            std::fs::write(full, text).unwrap();
        }
        Env { _dir: dir, root }
    }

    const GIT_CONFIG: &str = "[remote \"origin\"]\n\turl = git@github.com:Me/Proj.git\n\tfetch = +refs/heads/*:refs/remotes/origin/*\n";

    fn run_in(root: &Path, config_root: &Path, file: &str, config: Option<Config>) -> Vec<Finding> {
        let path = root.join(file);
        let text = std::fs::read_to_string(&path).unwrap();
        let a = Analyzed::new(SourceFile::new(
            path,
            PathBuf::from(file),
            FileKind::Markdown,
            text,
        ));
        let config = Config {
            root: config_root.to_path_buf(),
            ..config.unwrap_or_default()
        };
        let ctx = FileCtx {
            a: &a,
            config: &config,
        };
        let mut out = Vec::new();
        check(&ctx, &mut out);
        out
    }

    fn run(e: &Env, file: &str) -> Vec<Finding> {
        run_in(&e.root, &e.root, file, None)
    }

    fn apply(src: &str, f: &[Finding]) -> String {
        let mut s = src.to_string();
        for x in f.iter().rev() {
            if let Some(fix) = &x.fix {
                s.replace_range(fix.range.clone(), &fix.replacement);
            }
        }
        s
    }

    #[test]
    fn rewrites_default_branch_urls() {
        let src = concat!(
            "# T\n\n",
            "[readme](https://github.com/me/proj/blob/main/README.md#usage)\n",
            "[tree](https://github.com/me/proj/tree/main/docs)\n",
            "[raw](https://github.com/me/proj/raw/main/docs/img/logo.png)\n",
            "![img](https://raw.githubusercontent.com/me/proj/refs/heads/main/docs/img/logo.png)\n",
            "![blob](https://github.com/me/proj/blob/main/docs/img/logo.png?raw=true)\n",
            "[lines](https://github.com/me/proj/blob/main/src/lib.rs?plain=1#L3-L5)\n",
            "[space](https://github.com/ME/PROJ/blob/main/docs/my%20file.md)\n",
            "<img src=\"https://raw.githubusercontent.com/me/proj/main/docs/img/logo.png\">\n",
            "[sha](https://github.com/me/proj/blob/0123abc/README.md)\n",
            "[long sha](https://github.com/me/proj/blob/0123456789abcdef0123456789abcdef01234567/README.md)\n",
            "[tag](https://github.com/me/proj/blob/v1.0/README.md)\n",
            "[other](https://github.com/else/lib/blob/main/README.md)\n",
            "[missing](https://github.com/me/proj/blob/main/docs/none.md)\n",
            "[case](https://github.com/me/proj/blob/main/readme.md)\n",
            "[dir as blob](https://github.com/me/proj/blob/main/docs)\n",
            "[query](https://github.com/me/proj/blob/main/README.md?foo=1)\n",
            "[ref][r]\n\n",
            "Bare https://github.com/me/proj/blob/main/README.md and <https://github.com/me/proj/blob/main/README.md>.\n\n",
            "[r]: https://github.com/me/proj/blob/main/src/lib.rs\n",
        );
        let e = env(&[
            ("docs/sub/a.md", src),
            ("README.md", "# R\n"),
            ("docs/img/logo.png", "png"),
            ("docs/my file.md", "x"),
            ("src/lib.rs", "fn x() {}\n"),
            (".git/config", GIT_CONFIG),
        ]);
        let f = run(&e, "docs/sub/a.md");
        let got: Vec<(&str, &str)> = f
            .iter()
            .map(|x| (&src[x.range.clone()], x.suggestions[0].as_str()))
            .collect();
        assert_eq!(
            got,
            vec![
                (
                    "https://github.com/me/proj/blob/main/README.md#usage",
                    "../../README.md#usage"
                ),
                ("https://github.com/me/proj/tree/main/docs", "../"),
                (
                    "https://github.com/me/proj/raw/main/docs/img/logo.png",
                    "../img/logo.png"
                ),
                (
                    "https://raw.githubusercontent.com/me/proj/refs/heads/main/docs/img/logo.png",
                    "../img/logo.png"
                ),
                (
                    "https://github.com/me/proj/blob/main/docs/img/logo.png?raw=true",
                    "../img/logo.png"
                ),
                (
                    "https://github.com/me/proj/blob/main/src/lib.rs?plain=1#L3-L5",
                    "../../src/lib.rs?plain=1#L3-L5"
                ),
                (
                    "https://github.com/ME/PROJ/blob/main/docs/my%20file.md",
                    "../my%20file.md"
                ),
                (
                    "https://raw.githubusercontent.com/me/proj/main/docs/img/logo.png",
                    "../img/logo.png"
                ),
                (
                    "https://github.com/me/proj/blob/main/README.md",
                    "../../README.md"
                ),
                (
                    "https://github.com/me/proj/blob/main/README.md",
                    "../../README.md"
                ),
                (
                    "https://github.com/me/proj/blob/main/src/lib.rs",
                    "../../src/lib.rs"
                ),
            ],
            "{f:?}"
        );
        assert!(f.iter().all(|x| x.rule == RULE));
        // Bare URLs and autolinks: suggestion only.
        assert!(f[8].fix.is_none() && f[9].fix.is_none());
        let fixed = apply(src, &f);
        assert!(fixed.contains("[readme](../../README.md#usage)"));
        assert!(fixed.contains("![img](../img/logo.png)"));
        assert!(fixed.contains("<img src=\"../img/logo.png\">"));
        assert!(fixed.contains("[r]: ../../src/lib.rs"));
        assert!(
            fixed.contains("Bare https://github.com/me/proj/blob/main/README.md and <https://")
        );
    }

    #[test]
    fn readme_images_and_default_branch() {
        let src = "![logo](https://github.com/me/proj/raw/trunk/assets/logo.png)\n![main](https://github.com/me/proj/raw/main/assets/logo.png)\n";
        let e = env(&[
            ("README.md", src),
            ("assets/logo.png", "x"),
            (".git/config", GIT_CONFIG),
            ("docs/.keep", ""),
        ]);
        // No origin/HEAD: the usual default branch names and HEAD are accepted.
        let f = run(&e, "README.md");
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].fix.as_ref().unwrap().replacement, "assets/logo.png");
        // origin/HEAD names the default branch (fresh cache key: a subdirectory root).
        std::fs::create_dir_all(e.root.join(".git/refs/remotes/origin")).unwrap();
        std::fs::write(
            e.root.join(".git/refs/remotes/origin/HEAD"),
            "ref: refs/remotes/origin/trunk\n",
        )
        .unwrap();
        let f = run_in(&e.root, &e.root.join("docs"), "README.md", None);
        assert_eq!(f.len(), 1, "{f:?}");
        assert!(e.root.join("docs").exists());
        assert!(src[f[0].range.clone()].contains("/trunk/"));
        // Config overrides both.
        let mut c = Config::default();
        c.links.github_default_branch = Some("main".into());
        c.links.github_repo = Some("else/lib".into());
        assert!(run_in(&e.root, &e.root, "README.md", Some(c.clone())).is_empty());
        c.links.github_repo = Some("me/proj".into());
        let f = run_in(&e.root, &e.root, "README.md", Some(c));
        assert_eq!(f.len(), 1);
        assert!(src[f[0].range.clone()].contains("/main/"));
    }

    #[test]
    fn worktree_git_file_and_no_remote() {
        let e = env(&[
            ("main/.git/config", GIT_CONFIG),
            ("main/.git/worktrees/wt/commondir", "../..\n"),
            ("wt/.git", "gitdir: ../main/.git/worktrees/wt\n"),
            (
                "wt/README.md",
                "[x](https://github.com/me/proj/blob/main/guide.md)\n",
            ),
            ("wt/guide.md", "# G\n"),
            (
                "plain/README.md",
                "[x](https://github.com/me/proj/blob/main/guide.md)\n",
            ),
            ("plain/guide.md", "# G\n"),
        ]);
        let wt = e.root.join("wt");
        let f = run_in(&wt, &wt, "README.md", None);
        assert_eq!(f.len(), 1, "{f:?}");
        assert_eq!(f[0].suggestions, ["guide.md"]);
        // No git metadata and no `links.github_repo`: the rule does nothing.
        let plain = e.root.join("plain");
        assert!(run_in(&plain, &plain, "README.md", None).is_empty());
    }
}
