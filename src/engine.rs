//! Discovery and orchestration.

use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use crate::config::Config;
use crate::diagnostic::Diagnostic;
use crate::links::remote::RemoteStatus;
use crate::rules::{Analyzed, FileCtx, Out, default_severity};
use crate::source::{FileKind, SourceFile};
use crate::suppress::Suppressions;

#[derive(Debug, Clone, Default)]
pub struct Options {
    /// Check http(s) links.
    pub remote: bool,
}

/// Directory entry names, `None` when the directory cannot be read.
pub type DirListing = Option<Arc<Vec<OsString>>>;

/// All analyzed files, keyed by absolute path.
#[derive(Default)]
pub struct Workspace {
    pub files: HashMap<PathBuf, Arc<Analyzed>>,
    /// Lazily parsed files outside the checked set (link targets).
    extra: Mutex<HashMap<PathBuf, Option<Arc<Analyzed>>>>,
    /// Directory listings shared by all files (case-sensitive link lookups).
    dirs: Mutex<HashMap<PathBuf, DirListing>>,
    /// `docs/orphan-page` inbound link index, built on first use.
    pub(crate) inbound: OnceLock<crate::rules::docs::InboundLinks>,
    pub root: PathBuf,
}

impl Workspace {
    /// Analyzed Markdown file at `path`, parsing it on demand if it was not checked.
    pub fn markdown(&self, path: &Path) -> Option<Arc<Analyzed>> {
        if let Some(a) = self.files.get(path) {
            return Some(a.clone());
        }
        if let Some(a) = self
            .extra
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(path)
        {
            return a.clone();
        }
        // Read and parse without holding the lock; a concurrent duplicate parse is harmless.
        let parsed = (|| {
            let kind = FileKind::detect(path).filter(|k| *k == FileKind::Markdown)?;
            let text = std::fs::read_to_string(path).ok()?;
            let rel = path.strip_prefix(&self.root).unwrap_or(path).to_path_buf();
            Some(Arc::new(Analyzed::new(SourceFile::new(
                path.to_path_buf(),
                rel,
                kind,
                text,
            ))))
        })();
        self.extra
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .entry(path.to_path_buf())
            .or_insert(parsed)
            .clone()
    }

    /// Entry names of `dir`, read once per workspace.
    pub fn list_dir(&self, dir: &Path) -> DirListing {
        if let Some(l) = self
            .dirs
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(dir)
        {
            return l.clone();
        }
        let listing = std::fs::read_dir(dir).ok().map(|rd| {
            Arc::new(
                rd.filter_map(|e| e.ok().map(|e| e.file_name()))
                    .collect::<Vec<_>>(),
            )
        });
        self.dirs
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .entry(dir.to_path_buf())
            .or_insert(listing)
            .clone()
    }
}

/// List files to check under `paths`, honoring .gitignore and config excludes.
pub fn discover(paths: &[PathBuf], config: &Config) -> Result<Vec<PathBuf>, String> {
    let mut out = Vec::new();
    let mut overrides = ignore::overrides::OverrideBuilder::new(&config.root);
    for inc in &config.general.include {
        overrides.add(inc).map_err(|e| e.to_string())?;
    }
    for ex in &config.general.exclude {
        overrides
            .add(&format!("!{ex}"))
            .map_err(|e| e.to_string())?;
    }
    let overrides = overrides.build().map_err(|e| e.to_string())?;
    for p in paths {
        if p.is_file() {
            if FileKind::detect(p).is_some() {
                out.push(
                    p.canonicalize()
                        .map_err(|e| format!("{}: {e}", p.display()))?,
                );
            }
            continue;
        }
        let walker = ignore::WalkBuilder::new(p)
            .git_ignore(config.general.respect_gitignore)
            .git_global(config.general.respect_gitignore)
            .git_exclude(config.general.respect_gitignore)
            .require_git(false)
            .add_custom_ignore_filename(".explicitignore")
            .overrides(overrides.clone())
            .build();
        for entry in walker {
            let entry = entry.map_err(|e| e.to_string())?;
            if entry.file_type().is_some_and(|t| t.is_file())
                && FileKind::detect(entry.path()).is_some()
                && let Ok(c) = entry.path().canonicalize()
            {
                out.push(c);
            }
        }
    }
    out.sort();
    out.dedup();
    Ok(out)
}

pub fn load(path: &Path, root: &Path) -> Option<Analyzed> {
    let kind = FileKind::detect(path)?;
    let bytes = std::fs::read(path).ok()?;
    // Skip binary and generated files.
    if bytes.contains(&0) {
        return None;
    }
    let text = String::from_utf8(bytes).ok()?;
    if kind != FileKind::Markdown && is_generated(&text) {
        return None;
    }
    let rel = path.strip_prefix(root).unwrap_or(path).to_path_buf();
    Some(Analyzed::new(SourceFile::new(
        path.to_path_buf(),
        rel,
        kind,
        text,
    )))
}

fn is_generated(text: &str) -> bool {
    let head: String = text
        .lines()
        .take(5)
        .collect::<Vec<_>>()
        .join("\n")
        .to_lowercase();
    head.contains("@generated") || head.contains("code generated") || head.contains("do not edit")
}

/// Run `f` over items on all cores.
pub fn par_map<T: Sync, R: Send>(items: &[T], f: impl Fn(&T) -> R + Sync) -> Vec<R> {
    let n = std::thread::available_parallelism()
        .map_or(4, |n| n.get())
        .min(items.len().max(1));
    let next = AtomicUsize::new(0);
    let results: Mutex<Vec<(usize, R)>> = Mutex::new(Vec::with_capacity(items.len()));
    std::thread::scope(|s| {
        for _ in 0..n {
            s.spawn(|| {
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    if i >= items.len() {
                        break;
                    }
                    let r = f(&items[i]);
                    results
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .push((i, r));
                }
            });
        }
    });
    let mut r = results.into_inner().unwrap_or_else(PoisonError::into_inner);
    r.sort_by_key(|(i, _)| *i);
    r.into_iter().map(|(_, r)| r).collect()
}

pub fn build_workspace(paths: &[PathBuf], config: &Config) -> Workspace {
    let analyzed = par_map(paths, |p| load(p, &config.root));
    let files = analyzed
        .into_iter()
        .flatten()
        .map(|a| (a.file.path.clone(), Arc::new(a)))
        .collect();
    Workspace {
        files,
        root: config.root.clone(),
        ..Workspace::default()
    }
}

/// Check the given files (which must be in the workspace).
pub fn check(
    ws: &Workspace,
    targets: &[PathBuf],
    config: &Config,
    opts: &Options,
) -> Vec<Diagnostic> {
    let targets: Vec<Arc<Analyzed>> = targets
        .iter()
        .filter_map(|p| ws.files.get(p).cloned())
        .collect();

    let statuses: HashMap<String, RemoteStatus> = if opts.remote && config.links.remote {
        let mut urls: HashSet<String> = HashSet::new();
        for a in &targets {
            let ctx = FileCtx { a, config };
            if ctx.family_enabled("links/http") {
                urls.extend(crate::links::remote::urls(&ctx));
            }
        }
        let mut urls: Vec<String> = urls.into_iter().collect();
        urls.sort();
        crate::links::remote::check_all(&urls, config)
    } else {
        HashMap::new()
    };

    let per_file = par_map(&targets, |a| check_file(a, ws, config, &statuses));
    let mut all: Vec<Diagnostic> = per_file.into_iter().flatten().collect();
    all.sort_by(|a, b| (&a.path, a.range.start, &a.rule).cmp(&(&b.path, b.range.start, &b.rule)));
    all
}

pub fn check_file(
    a: &Analyzed,
    ws: &Workspace,
    config: &Config,
    statuses: &HashMap<String, RemoteStatus>,
) -> Vec<Diagnostic> {
    let ctx = FileCtx { a, config };
    let mut out: Out = Vec::new();
    let is_md = a.md.is_some();
    if is_md {
        crate::rules::structure::check(&ctx, &mut out);
        crate::rules::diagram::check(&ctx, &mut out);
        crate::rules::codeblock::check(&ctx, &mut out);
        crate::rules::docs::check(&ctx, ws, &mut out);
    }
    if is_md || config.comments.enabled {
        crate::rules::grammar::check(&ctx, &mut out);
        crate::rules::style::check(&ctx, &mut out);
        crate::rules::slop::check(&ctx, &mut out);
        crate::rules::prose::check(&ctx, &mut out);
    }
    crate::links::local::check(&ctx, ws, &mut out);
    if !statuses.is_empty() {
        crate::links::remote::report(&ctx, statuses, &mut out);
    }

    let sup = Suppressions::from(a);
    let mut seen = HashSet::new();
    out.into_iter()
        .filter(|f| !sup.is_suppressed(f))
        .filter(|f| {
            !sup.directive_ranges
                .iter()
                .any(|r| r.contains(&f.range.start) && !f.rule.starts_with("md/"))
        })
        .filter(|f| seen.insert((f.rule.clone(), f.range.start, f.range.end)))
        .filter_map(|f| {
            // Rules report their preferred severity; config overrides it or turns the rule off.
            let default = if default_severity(&f.rule).is_some() {
                Some(f.severity)
            } else {
                None
            };
            let severity = config.severity(&f.rule, default)?;
            let (line, column) = a.file.line_col(f.range.start);
            Some(Diagnostic {
                path: a.file.rel.clone(),
                rule: f.rule,
                severity,
                range: f.range,
                line,
                column,
                message: f.message,
                help: f.help,
                suggestions: f.suggestions,
                fix: f.fix,
            })
        })
        .collect()
}

impl Workspace {
    /// Drop lazily parsed copies so they are re-read.
    pub fn invalidate(&mut self, path: &Path) {
        self.extra
            .get_mut()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(path);
        let dirs = self.dirs.get_mut().unwrap_or_else(PoisonError::into_inner);
        dirs.remove(path);
        if let Some(parent) = path.parent() {
            dirs.remove(parent);
        }
        self.inbound = OnceLock::new();
    }
}
