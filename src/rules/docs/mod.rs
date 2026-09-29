//! Documentation-site checks: TOC sync, orphan pages, include directives.

mod include;
mod orphan;
mod toc;

pub(crate) use orphan::InboundLinks;

use std::path::{Component, Path, PathBuf};

use super::{FileCtx, Out};
use crate::engine::Workspace;

pub fn check(ctx: &FileCtx, ws: &Workspace, out: &mut Out) {
    if ctx.a.md.is_none() || !ctx.family_enabled("docs/") {
        return;
    }
    if ctx.enabled("docs/toc-sync") {
        toc::check(ctx, out);
    }
    if ctx.enabled("docs/orphan-page") {
        orphan::check(ctx, ws, out);
    }
    if ctx.enabled("docs/include-missing") {
        include::check(ctx, out);
    }
}

/// Resolve `.` and `..` without touching the filesystem.
pub(crate) fn normalize(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                if matches!(out.components().next_back(), Some(Component::Normal(_))) {
                    out.pop();
                } else if !out.has_root() {
                    out.push("..");
                }
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// Project root, canonicalized when possible (workspace paths are canonical).
pub(crate) fn project_root(ctx: &FileCtx) -> PathBuf {
    ctx.config
        .root
        .canonicalize()
        .unwrap_or_else(|_| ctx.config.root.clone())
}

#[cfg(test)]
mod tests;
