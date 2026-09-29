//! `docs/readme-absolute-image`: README images pointing at this repo on GitHub.

use std::path::Path;
use std::sync::LazyLock;

use regex::Regex;

use super::{normalize, project_root, relative_to};
use crate::diagnostic::{Finding, Severity};
use crate::links::percent_decode;
use crate::rules::{FileCtx, Out};

const RULE: &str = "docs/readme-absolute-image";

/// owner, repo, rest (`<ref>/<path>`).
static GITHUB_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^https?://(?:github\.com/([^/]+)/([^/]+)/(?:raw|blob)/|raw\.githubusercontent\.com/([^/]+)/([^/]+)/)([^?#]+)",
    )
    .expect("hardcoded regex is valid")
});

pub fn check(ctx: &FileCtx, out: &mut Out) {
    let Some(md) = &ctx.a.md else { return };
    let path = &ctx.a.file.path;
    let is_readme = path
        .file_stem()
        .is_some_and(|s| s.eq_ignore_ascii_case("readme"));
    if !is_readme {
        return;
    }
    let root = project_root(ctx);
    let dir = path.parent().unwrap_or(&root);
    let remotes = git_remotes(&root);
    let src = ctx.src();
    for l in md.links.iter().filter(|l| l.is_image) {
        let Some(c) = GITHUB_RE.captures(&l.dest) else {
            continue;
        };
        let owner = c.get(1).or(c.get(3)).map_or("", |m| m.as_str());
        let repo = c.get(2).or(c.get(4)).map_or("", |m| m.as_str());
        let repo = repo.strip_suffix(".git").unwrap_or(repo);
        if let Some(r) = &remotes {
            let want = format!("{owner}/{repo}").to_lowercase();
            if !r.contains(&want) {
                continue;
            }
        }
        let rest = c.get(5).map_or("", |m| m.as_str());
        let rest = rest
            .strip_prefix("refs/heads/")
            .or_else(|| rest.strip_prefix("refs/tags/"))
            .unwrap_or(rest);
        // Drop the ref (branch, tag or sha); branch names with `/` are not resolved.
        let Some((_, file)) = rest.split_once('/') else {
            continue;
        };
        let file = percent_decode(file);
        let target = normalize(&root.join(&file));
        if !target.starts_with(&root) || !target.is_file() {
            continue;
        }
        let rel = relative_to(&target, &normalize(dir)).replace(' ', "%20");
        let mut f = Finding::new(
            RULE,
            Severity::Info,
            l.range.clone(),
            format!("README image points at GitHub; use the local path `{rel}`"),
        )
        .help("Relative image paths render on GitHub and keep working on forks and branches")
        .suggest(rel.clone());
        if let Some(i) = src[l.range.clone()].find(l.dest.as_str()) {
            let start = l.range.start + i;
            f = f.fix(start..start + l.dest.len(), rel);
        }
        out.push(f);
    }
}

/// `owner/repo` (lowercase) of GitHub remotes in `.git/config`; `None` when unknown.
fn git_remotes(root: &Path) -> Option<Vec<String>> {
    let text = std::fs::read_to_string(root.join(".git").join("config")).ok()?;
    static RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?m)^\s*url\s*=\s*\S*github\.com[:/]([^/\s]+)/([^/\s]+?)(?:\.git)?/?\s*$")
            .expect("hardcoded regex is valid")
    });
    let v: Vec<String> = RE
        .captures_iter(&text)
        .map(|c| format!("{}/{}", &c[1], &c[2]).to_lowercase())
        .collect();
    (!v.is_empty()).then_some(v)
}
