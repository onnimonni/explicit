//! `docs/orphan-page`: pages under `docs.root` that no other page links to.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::{normalize, project_root};
use crate::config::glob_match;
use crate::diagnostic::{Finding, Severity};
use crate::engine::Workspace;
use crate::links::{has_scheme, percent_decode};
use crate::rules::{FileCtx, Out};

const RULE: &str = "docs/orphan-page";

pub fn check(ctx: &FileCtx, ws: &Workspace, out: &mut Out) {
    let path = &ctx.a.file.path;
    let root = project_root(ctx);
    let docs_root = normalize(&root.join(&ctx.config.docs.root));
    if !path.starts_with(&docs_root) || exempt(ctx, path, &root) {
        return;
    }
    let index = ws.inbound.get_or_init(|| InboundLinks::build(ws, &root));
    if index.linked(path) || in_mkdocs_nav(index.mkdocs.as_deref(), &docs_root, path) {
        return;
    }
    let first_line = ctx.src().find(['\r', '\n']).unwrap_or(ctx.src().len());
    out.push(
        Finding::new(
            RULE,
            Severity::Warning,
            0..first_line,
            "Page is not linked from any other checked Markdown file",
        )
        .help("Link to it from an index or navigation page, or add it to docs.orphan_exempt"),
    );
}

fn exempt(ctx: &FileCtx, path: &Path, root: &Path) -> bool {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let lower = name.to_lowercase();
    if lower == "index.md" || lower == "readme.md" || name.starts_with('_') {
        return true;
    }
    let rel = path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/");
    ctx.config
        .docs
        .orphan_exempt
        .iter()
        .any(|g| glob_match(g, &rel) || glob_match(g, &name))
}

/// Which pages link to which, computed once per workspace.
#[derive(Debug, Default)]
pub struct InboundLinks {
    /// Maps each resolved link target (without extension) to the pages linking to it.
    by_target: HashMap<PathBuf, Vec<PathBuf>>,
    /// Contents of `mkdocs.yml` / `mkdocs.yaml`, if present.
    mkdocs: Option<String>,
}

impl InboundLinks {
    fn build(ws: &Workspace, root: &Path) -> InboundLinks {
        let mut by_target: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new();
        for (p, a) in &ws.files {
            let Some(md) = &a.md else { continue };
            let base = p.parent().unwrap_or(root);
            for l in &md.links {
                // Keyed without extension so `page`, `page/` and `page.html` (static site
                // generator URLs) all count.
                if let Some(r) = resolve(&l.dest, base, root) {
                    by_target
                        .entry(r.with_extension(""))
                        .or_default()
                        .push(p.clone());
                }
            }
        }
        let mkdocs = std::fs::read_to_string(root.join("mkdocs.yml"))
            .or_else(|_| std::fs::read_to_string(root.join("mkdocs.yaml")))
            .ok();
        InboundLinks { by_target, mkdocs }
    }

    /// Whether another page links to `target`.
    fn linked(&self, target: &Path) -> bool {
        self.by_target
            .get(&target.with_extension(""))
            .is_some_and(|srcs| srcs.iter().any(|s| s != target))
    }
}

fn resolve(dest: &str, base: &Path, root: &Path) -> Option<PathBuf> {
    let dest = dest.trim();
    if dest.is_empty() || dest.starts_with('#') || has_scheme(dest) || dest.starts_with("//") {
        return None;
    }
    let path = dest.split(['#', '?']).next().unwrap_or("");
    let decoded = percent_decode(path);
    let decoded = decoded.trim_end_matches('/');
    if decoded.is_empty() {
        return None;
    }
    let joined = match decoded.strip_prefix('/') {
        Some(r) => root.join(r),
        None => base.join(decoded),
    };
    Some(normalize(&joined))
}

/// mkdocs lists pages in `nav:` relative to `docs_dir`.
fn in_mkdocs_nav(mkdocs: Option<&str>, docs_root: &Path, path: &Path) -> bool {
    let Some(text) = mkdocs else {
        return false;
    };
    let Ok(rel) = path.strip_prefix(docs_root) else {
        return false;
    };
    let rel = rel.to_string_lossy().replace('\\', "/");
    text.lines().any(|l| {
        let l = l.trim().trim_start_matches('-').trim();
        let v = l
            .rsplit(": ")
            .next()
            .unwrap_or(l)
            .trim()
            .trim_matches(['"', '\'']);
        v == rel
    })
}
