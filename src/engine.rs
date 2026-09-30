//! Discovery and orchestration.

use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use crate::config::Config;
use crate::diagnostic::Diagnostic;
use crate::diagnostic::{Finding, Severity};
use crate::lang_marks::Region;
use crate::links::Checked;
use crate::links::remote::RemoteStatus;
use crate::rules::spell::ProjectVocab;
use crate::rules::{Analyzed, FileCtx, Out, default_severity};
use crate::source::{FileKind, SourceFile};
use crate::suppress::Suppressions;

#[derive(Debug, Clone, Default)]
pub struct Options {
    /// Check http(s) links.
    pub remote: bool,
    /// Directory of the results cache; `None` disables caching.
    pub cache_dir: Option<PathBuf>,
    /// Directory of the link cache, also used with `--no-cache`; falls back to `cache_dir`.
    pub link_cache_dir: Option<PathBuf>,
    /// Drop cache entries of files not checked in this run (set when checking the whole root).
    pub prune_cache: bool,
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
    /// Project-wide spelling vocabulary, built on first use.
    vocab: OnceLock<Arc<ProjectVocab>>,
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

    /// Names the spell check takes from the whole project ([`ProjectVocab`]): manifests next
    /// to any checked file and capitalized words recurring across the checked files.
    pub fn project_vocab(&self, config: &Config) -> Arc<ProjectVocab> {
        self.vocab
            .get_or_init(|| {
                static CAPITALIZED: std::sync::LazyLock<regex::Regex> =
                    std::sync::LazyLock::new(|| {
                        regex::Regex::new(r"\b\p{Lu}\p{Ll}{2,}\b")
                            .expect("hardcoded regex is valid")
                    });
                let mut counts: HashMap<String, usize> = HashMap::new();
                for a in self.files.values() {
                    for seg in &a.segments {
                        for m in CAPITALIZED.find_iter(&seg.text) {
                            *counts.entry(m.as_str().to_string()).or_default() += 1;
                        }
                    }
                }
                let dirs = self.files.keys().filter_map(|p| p.parent());
                let speller = crate::rules::spell::dictionary(&config.prose.dialect);
                Arc::new(ProjectVocab::new(dirs, &counts, speller))
            })
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

/// List files to check under `paths`, honoring .gitignore and config excludes
/// (also for files named directly).
pub fn discover(paths: &[PathBuf], config: &Config) -> Result<Vec<PathBuf>, String> {
    discover_with(paths, config, true)
}

/// Like [`discover`]; `exclude = false` checks files named directly even when excluded.
pub fn discover_with(
    paths: &[PathBuf],
    config: &Config,
    exclude: bool,
) -> Result<Vec<PathBuf>, String> {
    let mut out = Vec::new();
    let excluder = if exclude {
        Some(Excluder::new(config)?)
    } else {
        None
    };
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
                let c = p
                    .canonicalize()
                    .map_err(|e| format!("{}: {e}", p.display()))?;
                if excluder.as_ref().is_none_or(|x| !x.is_excluded(&c)) {
                    out.push(c);
                }
            }
            continue;
        }
        // Canonical like the root, so include/exclude globs (relative to the root) match.
        let p = p.canonicalize().unwrap_or_else(|_| p.clone());
        let walker = ignore::WalkBuilder::new(&p)
            .git_ignore(config.general.respect_gitignore)
            .git_global(config.general.respect_gitignore)
            .git_exclude(config.general.respect_gitignore)
            .require_git(false)
            .add_custom_ignore_filename(".explicitignore")
            .overrides(overrides.clone())
            .filter_entry(|e| e.file_name() != crate::cache::DEFAULT_DIR)
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

/// Exclusion rules the directory walker applies, for files named on the command line:
/// `general.exclude`, then `.explicitignore`, `.ignore` and (with `respect_gitignore`)
/// `.gitignore` files from the file's directory up to the root, `.git/info/exclude` and the
/// global gitignore.
struct Excluder {
    root: PathBuf,
    respect_gitignore: bool,
    exclude: ignore::gitignore::Gitignore,
    /// Ignore files per directory, deepest precedence first within a directory.
    dirs: Mutex<HashMap<PathBuf, Arc<Vec<ignore::gitignore::Gitignore>>>>,
    /// `.git/info/exclude` and the global gitignore.
    git: OnceLock<[ignore::gitignore::Gitignore; 2]>,
}

impl Excluder {
    fn new(config: &Config) -> Result<Excluder, String> {
        let root = config
            .root
            .canonicalize()
            .unwrap_or_else(|_| config.root.clone());
        let exclude = crate::config::build_matcher(&root, &config.general.exclude)
            .map_err(|e| format!("general.exclude: {e}"))?;
        Ok(Excluder {
            root,
            respect_gitignore: config.general.respect_gitignore,
            exclude,
            dirs: Mutex::default(),
            git: OnceLock::new(),
        })
    }

    fn ignore_files(&self, dir: &Path) -> Arc<Vec<ignore::gitignore::Gitignore>> {
        let mut map = self.dirs.lock().unwrap_or_else(PoisonError::into_inner);
        map.entry(dir.to_path_buf())
            .or_insert_with(|| {
                let mut names = vec![".explicitignore", ".ignore"];
                if self.respect_gitignore {
                    names.push(".gitignore");
                }
                Arc::new(
                    names
                        .into_iter()
                        .map(|n| dir.join(n))
                        .filter(|p| p.is_file())
                        .map(|p| ignore::gitignore::Gitignore::new(p).0)
                        .collect(),
                )
            })
            .clone()
    }

    /// `path` must be canonical.
    fn is_excluded(&self, path: &Path) -> bool {
        let Ok(rel) = path.strip_prefix(&self.root) else {
            return false;
        };
        if self
            .exclude
            .matched_path_or_any_parents(rel, false)
            .is_ignore()
        {
            return true;
        }
        // Deeper ignore files win; the first directory with a match decides.
        for dir in path.ancestors().skip(1) {
            if !dir.starts_with(&self.root) {
                break;
            }
            let sub = path.strip_prefix(dir).unwrap_or(path);
            for gi in self.ignore_files(dir).iter() {
                let m = gi.matched_path_or_any_parents(sub, false);
                if !m.is_none() {
                    return m.is_ignore();
                }
            }
        }
        if !self.respect_gitignore {
            return false;
        }
        let git = self.git.get_or_init(|| {
            let mut info = ignore::gitignore::GitignoreBuilder::new(&self.root);
            info.add(self.root.join(".git/info/exclude"));
            let info = info
                .build()
                .unwrap_or_else(|_| ignore::gitignore::Gitignore::empty());
            let global = ignore::gitignore::GitignoreBuilder::new(&self.root)
                .build_global()
                .0;
            [info, global]
        });
        git.iter()
            .map(|gi| gi.matched_path_or_any_parents(rel, false))
            .find(|m| !m.is_none())
            .is_some_and(|m| m.is_ignore())
    }
}

pub fn load(path: &Path, root: &Path) -> Option<Analyzed> {
    let kind = FileKind::detect(path)?;
    let bytes = std::fs::read(path).ok()?;
    // Skip binary and generated files.
    if bytes.contains(&0) {
        return None;
    }
    let text = String::from_utf8(bytes).ok()?;
    let generated = if kind == FileKind::Markdown {
        is_generated_markdown(&text)
    } else {
        is_generated(&text)
    };
    if generated {
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

fn head_lower(text: &str, lines: usize) -> String {
    text.lines()
        .take(lines)
        .collect::<Vec<_>>()
        .join("\n")
        .to_lowercase()
}

/// Generated-file marker in a code file's first lines.
fn is_generated(text: &str) -> bool {
    let head = head_lower(text, 5);
    [
        "@generated",
        "code generated",
        "autogenerated",
        "auto-generated",
        "do not edit",
        "don't edit",
    ]
    .iter()
    .any(|m| head.contains(m))
}

/// Generated-file marker inside an HTML comment in a Markdown file's first 10 lines,
/// e.g. `<!-- This file is generated. Do not edit. -->` or `<!-- AUTO-GENERATED -->`.
fn is_generated_markdown(text: &str) -> bool {
    let head = head_lower(text, 10);
    let mut rest = head.as_str();
    while let Some(start) = rest.find("<!--") {
        let body = &rest[start + 4..];
        let end = body.find("-->").unwrap_or(body.len());
        let c = &body[..end];
        let no_edit = ["do not edit", "don't edit", "do not modify"]
            .iter()
            .any(|m| c.contains(m));
        if c.contains("autogenerated")
            || c.contains("auto-generated")
            || c.contains("@generated")
            || (c.contains("generated") && no_edit)
        {
            return true;
        }
        rest = &body[end..];
    }
    false
}

use crate::lang::STOPWORDS;

/// Cheap guess that prose is not English: at least 80 words, few English stopwords, and
/// either many words with non-ASCII letters or few dictionary words.
pub fn looks_non_english(segments: &[crate::segment::Segment], config: &Config) -> bool {
    let words: Vec<String> = segments
        .iter()
        .flat_map(|s| s.text.split(|c: char| !c.is_alphabetic() && c != '\''))
        .map(|w| w.trim_matches('\''))
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
        .collect();
    if words.len() < 80 {
        return false;
    }
    let n = words.len() as f64;
    let stop = words
        .iter()
        .filter(|w| STOPWORDS.contains(&w.as_str()))
        .count() as f64;
    if stop / n >= 0.08 {
        return false;
    }
    let non_ascii = words.iter().filter(|w| !w.is_ascii()).count() as f64;
    if non_ascii / n >= 0.05 {
        return true;
    }
    let known = if config.prose.engine.hunspell() {
        let dict = crate::rules::spell::dictionary(&config.prose.dialect);
        words
            .iter()
            .filter(|w| crate::rules::spell::known(dict, w))
            .count()
    } else {
        let dict = crate::rules::words::words();
        words.iter().filter(|w| dict.contains(w)).count()
    } as f64;
    known / n < 0.5
}

/// Language tag from Markdown front matter (`lang:` / `language:`), as written.
fn front_matter_language(a: &Analyzed) -> Option<String> {
    let range = a.md.as_ref()?.front_matter.clone()?;
    let fm = a.file.text.get(range)?;
    fm.lines().find_map(|l| {
        let (k, v) = l.split_once(':').or_else(|| l.split_once('='))?;
        if !matches!(k.trim_end(), "lang" | "language") || l.starts_with(char::is_whitespace) {
            return None;
        }
        let v = v.trim().trim_matches(|c| c == '"' || c == '\'');
        (!v.is_empty()).then(|| v.to_string())
    })
}

/// Primary language subtag of the file's prose: front matter, then `general.language` (as
/// `[[overrides]]` set it), then detection (`fi`, `sv`, or `und` for other languages).
pub fn document_language(a: &Analyzed, config: &Config) -> String {
    document_language_in(a, config, &crate::lang_marks::regions(a))
}

/// [`document_language`] with detection limited to prose outside the marked regions.
fn document_language_in(a: &Analyzed, config: &Config, marks: &[Region]) -> String {
    use crate::rules::spell_lang::primary;
    if let Some(tag) = front_matter_language(a) {
        return primary(&tag);
    }
    let configured = primary(&config.general.language);
    if configured != "en" {
        return configured;
    }
    if config.general.detect_language {
        let segments = unmarked_segments(a, marks);
        if looks_non_english(&segments, config) {
            return crate::lang::document_hint(&segments)
                .or_else(|| crate::lang::document_nordic(&segments))
                .unwrap_or("und")
                .to_string();
        }
    }
    configured
}

/// Segments of `a` with the marked regions blanked.
fn unmarked_segments<'a>(
    a: &'a Analyzed,
    marks: &[Region],
) -> std::borrow::Cow<'a, [crate::segment::Segment]> {
    if marks.is_empty() {
        return std::borrow::Cow::Borrowed(&a.segments);
    }
    let ranges: Vec<&std::ops::Range<usize>> = marks.iter().map(|(r, _)| r).collect();
    std::borrow::Cow::Owned(
        a.segments
            .iter()
            .map(|s| {
                let mut s = s.clone();
                s.blank_all(ranges.iter().copied());
                s
            })
            .filter(|s| !s.is_blank())
            .collect(),
    )
}

/// `md/front-matter-lang`: a Markdown file without a declared language (front matter or
/// `[[overrides]]`) whose prose outside marked regions is 80% Finnish or Swedish words. The fix
/// adds `lang:` to the front matter, or creates front matter holding only it.
fn front_matter_lang(ctx: &FileCtx, marks: &[Region], out: &mut Out) {
    let (a, config) = (ctx.a, ctx.config);
    let Some(md) = &a.md else {
        return;
    };
    if !ctx.enabled("md/front-matter-lang")
        || !config.general.detect_language
        || config.general.language_from_override
        || crate::rules::spell_lang::primary(&config.general.language) != "en"
        || front_matter_language(a).is_some()
    {
        return;
    }
    let segments = unmarked_segments(a, marks);
    let text = segments
        .iter()
        .map(|s| s.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    if !crate::lang::looks_nordic(&text, 20, true) {
        return;
    }
    let Some(lang) = crate::lang::decisive_nordic(&text) else {
        return;
    };
    let src = a.file.text.as_str();
    let name = if lang == "fi" { "Finnish" } else { "Swedish" };
    let line_end = |at: usize| src[at..].find('\n').map_or(src.len(), |i| at + i);
    let nl = if src[..line_end(0)].ends_with('\r') {
        "\r\n"
    } else {
        "\n"
    };
    let (range, fix) = match &md.front_matter {
        Some(fm) => {
            let first = fm.start..line_end(fm.start);
            // Insert after the opening `---` line.
            let fix = (src[first.clone()].trim_end() == "---" && first.end < fm.end)
                .then(|| (first.end + 1..first.end + 1, format!("lang: {lang}{nl}")));
            (first, fix)
        }
        None => {
            // A leading BOM or `---` / `+++` line would make new front matter ambiguous.
            let fix = (!src.starts_with(['\u{feff}', '-', '+']))
                .then(|| (0..0, format!("---{nl}lang: {lang}{nl}---{nl}")));
            (0..line_end(0), fix)
        }
    };
    let mut f = Finding::new(
        "md/front-matter-lang",
        Severity::Info,
        range,
        format!("The prose is {name}, but the file declares no language"),
    )
    .help(format!(
        "Add `lang: {lang}` to the front matter, so the file is checked as {name} without \
         relying on detection"
    ));
    if let Some((r, text)) = fix {
        f = f.fix(r, text);
    }
    out.push(f);
}

/// Whether English prose rules (spelling, grammar, prose, slop) apply to this file.
#[cfg(test)]
fn prose_is_english(a: &Analyzed, config: &Config) -> bool {
    document_language(a, config) == "en"
}

/// Run `f` over items on all cores.
pub fn par_map<T: Sync, R: Send>(items: &[T], f: impl Fn(&T) -> R + Sync) -> Vec<R> {
    let order: Vec<usize> = (0..items.len()).collect();
    par_map_in(items, &order, f)
}

/// [`par_map`], starting the heaviest items first so one big file does not finish last on a
/// single core. Results keep the input order.
pub fn par_map_heaviest_first<T: Sync, R: Send>(
    items: &[T],
    weight: impl Fn(&T) -> usize,
    f: impl Fn(&T) -> R + Sync,
) -> Vec<R> {
    let mut order: Vec<usize> = (0..items.len()).collect();
    order.sort_by_key(|&i| std::cmp::Reverse(weight(&items[i])));
    par_map_in(items, &order, f)
}

/// Run `f` over `items` on all cores, taking them in `order` (a permutation of the indices).
fn par_map_in<T: Sync, R: Send>(
    items: &[T],
    order: &[usize],
    f: impl Fn(&T) -> R + Sync,
) -> Vec<R> {
    let n = std::thread::available_parallelism()
        .map_or(4, |n| n.get())
        .min(items.len().max(1));
    let next = AtomicUsize::new(0);
    let results: Mutex<Vec<(usize, R)>> = Mutex::new(Vec::with_capacity(items.len()));
    std::thread::scope(|s| {
        for _ in 0..n {
            s.spawn(|| {
                loop {
                    let k = next.fetch_add(1, Ordering::Relaxed);
                    let Some(&i) = order.get(k) else {
                        break;
                    };
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
    check_with_stats(ws, targets, config, opts).0
}

/// [`check`], also returning results-cache hit and miss counts.
pub fn check_with_stats(
    ws: &Workspace,
    targets: &[PathBuf],
    config: &Config,
    opts: &Options,
) -> (Vec<Diagnostic>, crate::cache::Stats) {
    let targets: Vec<Arc<Analyzed>> = targets
        .iter()
        .filter_map(|p| ws.files.get(p).cloned())
        .collect();

    // The link cache lives in the project cache dir and stays on with `--no-cache`
    // (that only disables the results cache), unless `links.cache = false`.
    let link_cache = config.links.cache.then(|| {
        let dir = opts
            .cache_dir
            .clone()
            .unwrap_or_else(|| crate::links::cache::default_dir(config));
        crate::links::cache::path_in(&dir)
    });
    let network = opts.remote && config.links.remote;
    let remote: HashMap<String, RemoteStatus> = if network {
        let mut urls: HashSet<crate::links::remote::Target> = HashSet::new();
        for a in &targets {
            urls.extend(crate::links::remote::urls(&FileCtx { a, config }));
        }
        let mut urls: Vec<_> = urls.into_iter().collect();
        urls.sort();
        crate::links::remote::check_all(&urls, config, link_cache.clone())
    } else {
        HashMap::new()
    };
    // Same-repo links: local git always; issues via gh/token only with network access.
    let same_repo = crate::links::same_repo::check_all(
        targets.iter().map(|a| &**a),
        config,
        link_cache,
        network,
    );
    let statuses = Checked { remote, same_repo };

    let cache = opts.cache_dir.as_deref().map(crate::cache::Cache::open);
    // Config keys per distinct effective config (`None` = the base config).
    let base_key = cache.as_ref().map(|_| crate::cache::config_key(config));
    let override_keys: Mutex<HashMap<usize, u128>> = Mutex::new(HashMap::new());
    let vocab = ws.project_vocab(config);
    let vocab_key =
        u128::from(vocab.fingerprint) << 64 | u128::from(vocab.fingerprint.rotate_left(17));

    let per_file = par_map_heaviest_first(
        &targets,
        |a| a.file.text.len(),
        |a| {
            let effective =
                config.for_file(&a.file.rel, a.segments.iter().map(|s| s.text.as_str()));
            let fc: &Config = effective.as_deref().unwrap_or(config);
            let mut fresh = None;
            let mut hit = None;
            let mut diags = match (&cache, base_key) {
                (Some(cache), Some(base)) => {
                    let ck = match &effective {
                        None => base,
                        Some(e) => {
                            let id = Arc::as_ptr(e) as usize;
                            let known = override_keys
                                .lock()
                                .unwrap_or_else(PoisonError::into_inner)
                                .get(&id)
                                .copied();
                            known.unwrap_or_else(|| {
                                let k = crate::cache::config_key(e);
                                override_keys
                                    .lock()
                                    .unwrap_or_else(PoisonError::into_inner)
                                    .insert(id, k);
                                k
                            })
                        }
                    };
                    // Dependency names from manifests change spelling results too.
                    let deps = crate::rules::spell::dep_names(&fc.root, &a.file.path).fingerprint;
                    let ck = ck ^ (u128::from(deps) << 64 | u128::from(deps)) ^ vocab_key;
                    let key = crate::cache::file_key(&a.file.rel, &a.file.text, ck);
                    match cache.get(&a.file.rel, key) {
                        Some(d) => {
                            hit = Some(key);
                            d
                        }
                        None => {
                            let d = local_diagnostics_in(a, fc, &vocab);
                            fresh = Some((key, d.clone()));
                            d
                        }
                    }
                }
                _ => local_diagnostics_in(a, fc, &vocab),
            };
            diags.extend(cross_diagnostics(a, ws, fc, &statuses));
            (diags, fresh, hit)
        },
    );
    let mut all: Vec<Diagnostic> = Vec::new();
    let mut fresh = Vec::new();
    let mut hits = Vec::new();
    for (d, f, h) in per_file {
        all.extend(d);
        fresh.extend(f);
        hits.extend(h);
    }
    let stats = crate::cache::Stats {
        hits: hits.len(),
        misses: fresh.len(),
    };
    if let Some(cache) = cache
        && let Err(e) = cache.save(fresh, &hits, targets.len(), opts.prune_cache)
    {
        eprintln!("explicit: cannot write cache: {e}");
    }
    all.sort_by(|a, b| (&a.path, a.range.start, &a.rule).cmp(&(&b.path, b.range.start, &b.rule)));
    (all, stats)
}

/// All diagnostics for one file: [`local_diagnostics`] plus [`cross_diagnostics`].
pub fn check_file(
    a: &Analyzed,
    ws: &Workspace,
    config: &Config,
    statuses: &Checked,
) -> Vec<Diagnostic> {
    let effective = config.for_file(&a.file.rel, a.segments.iter().map(|s| s.text.as_str()));
    let config: &Config = effective.as_deref().unwrap_or(config);
    let mut d = local_diagnostics_in(a, config, &ws.project_vocab(config));
    d.extend(cross_diagnostics(a, ws, config, statuses));
    d
}

/// Findings of rules that depend only on the file and its effective `config` (cacheable).
pub fn local_findings(a: &Analyzed, config: &Config) -> Out {
    let ctx = FileCtx { a, config };
    let mut out: Out = Vec::new();
    let is_md = a.md.is_some();
    if is_md {
        crate::rules::structure::check(&ctx, &mut out);
        crate::rules::diagram::check(&ctx, &mut out);
        crate::rules::codeblock::check(&ctx, &mut out);
    }
    let is_po = a.po.is_some();
    if is_po {
        crate::rules::gettext::check(&ctx, &mut out);
    }
    if a.file.rel == std::path::Path::new(crate::config::CONFIG_FILE) {
        config_placeholders(&ctx, &mut out);
    }
    if is_md || is_po || config.comments.enabled {
        // Explicitly marked regions override detection (see `crate::lang_marks`).
        let marks = crate::lang_marks::regions(a);
        let marked: Vec<std::ops::Range<usize>> = marks.iter().map(|(r, _)| r.clone()).collect();
        let lang = document_language_in(a, config, &marks);
        crate::rules::style::check(&ctx, &mut out);
        if lang == "en" {
            english_prose(&ctx, &mut out);
            if config.general.detect_language {
                drop_foreign_segment_findings(a, &marked, &mut out);
                check_stretches(&ctx, "en", &marked, &mut out);
            }
        } else if let Some(sp) = crate::rules::spell_lang::speller(&lang, config) {
            // Native spelling and grammar; English-only rules stay inside English stretches.
            language_prose(&ctx, &lang, sp, &marked, &mut out);
        } else if matches!(lang.as_str(), "de" | "fr" | "es" | "pt" | "fi" | "sv") {
            let own = segments_minus(a, &marked);
            crate::rules::slop::languages::check(&ctx, &lang, Some(&own), &mut out);
        }
        if !marks.is_empty() {
            marked_prose(&ctx, &lang, &marks, &mut out);
        }
        if is_md {
            front_matter_lang(&ctx, &marks, &mut out);
        }
    }
    out
}

/// `config/placeholder`: `explicit.toml` explanations still reading `TODO`.
fn config_placeholders(ctx: &FileCtx, out: &mut Out) {
    const RULE: &str = "config/placeholder";
    if !ctx.enabled(RULE) {
        return;
    }
    for (range, key, value) in crate::config::placeholder_values(ctx.src()) {
        out.push(crate::diagnostic::Finding::new(
            RULE,
            crate::rules::default_severity(RULE).unwrap_or(Severity::Warning),
            range,
            format!(
                "`{key}` is a placeholder ({value:?}); write what it is, so explicit.toml \
                 documents the project"
            ),
        ));
    }
}

/// English rule families: spelling and grammar, slop, prose.
fn english_prose(ctx: &FileCtx, out: &mut Out) {
    crate::rules::grammar::check(ctx, out);
    crate::rules::slop::check(ctx, out);
    crate::rules::prose::check(ctx, out);
}

/// A finding of an English-only rule family that depends on the text's language.
fn language_dependent(f: &crate::diagnostic::Finding) -> bool {
    crate::lang::is_english_rule(&f.rule) && !LANGUAGE_NEUTRAL_PROSE.contains(&f.rule.as_str())
}

/// Marked regions of a file in language `doc_lang`: findings of the file's own checks inside
/// them are replaced by checks in the region's language (English rules, native prose
/// checks where available, and language-independent rules). English regions of an English file were
/// already checked as such.
fn marked_prose(ctx: &FileCtx, doc_lang: &str, marks: &[Region], out: &mut Out) {
    let own = |l: &str| doc_lang == "en" && l == "en";
    out.retain(|f| {
        !language_dependent(f)
            || !marks
                .iter()
                .any(|(r, l)| !own(l) && r.contains(&f.range.start))
    });
    let mut by_lang: std::collections::BTreeMap<&str, Vec<std::ops::Range<usize>>> =
        Default::default();
    for (r, l) in marks.iter().filter(|(_, l)| !own(l)) {
        by_lang.entry(l.as_str()).or_default().push(r.clone());
    }
    for (l, ranges) in by_lang {
        if l == "en" {
            let mut en = Vec::new();
            english_prose(ctx, &mut en);
            en.retain(|f| ranges.iter().any(|r| r.contains(&f.range.start)));
            out.extend(en);
        } else {
            crate::rules::slop::languages::check(ctx, l, Some(&ranges), out);
            if let Some(sp) = crate::rules::spell_lang::speller(l, ctx.config) {
                crate::rules::grammar::check_language(ctx, l, sp, None, Some(&ranges), out);
            }
        }
    }
}

/// `prose/*` rules that do not depend on English: configured names and terms, typography.
const LANGUAGE_NEUTRAL_PROSE: &[&str] = &[
    "prose/entity-name",
    "prose/ambiguous-person",
    "prose/terminology",
    "prose/smart-quotes",
    "prose/sentence-spacing",
];

/// Byte ranges of the segments of `a` minus `exclude`.
fn segments_minus(a: &Analyzed, exclude: &[std::ops::Range<usize>]) -> Vec<std::ops::Range<usize>> {
    let mut ex: Vec<&std::ops::Range<usize>> = exclude.iter().collect();
    ex.sort_by_key(|r| r.start);
    let mut out = Vec::new();
    for s in &a.segments {
        let mut pos = s.range.start;
        for r in ex
            .iter()
            .filter(|r| r.start < s.range.end && s.range.start < r.end)
        {
            if r.start > pos {
                out.push(pos..r.start);
            }
            pos = pos.max(r.end);
        }
        if pos < s.range.end {
            out.push(pos..s.range.end);
        }
    }
    out
}

/// Prose of a file in language `lang` (not English) with its speller `sp`: spelling in that
/// language outside confidently detected stretches of other supported languages;
/// English rules inside English stretches; language-neutral prose rules.
fn language_prose(
    ctx: &FileCtx,
    lang: &str,
    sp: Arc<dyn crate::rules::spell_lang::LangSpeller>,
    marked: &[std::ops::Range<usize>],
    out: &mut Out,
) {
    let a = ctx.a;
    let detect = ctx.config.general.detect_language;
    // Detection stays out of marked regions.
    let unmarked =
        |r: &std::ops::Range<usize>| !marked.iter().any(|m| m.start < r.end && r.start < m.end);
    let english: Vec<_> = if detect {
        crate::lang::english_ranges(&a.segments)
            .into_iter()
            .filter(unmarked)
            .collect()
    } else {
        Vec::new()
    };
    let others: Vec<_> = if detect {
        crate::lang::other_language_ranges(&a.segments, lang)
            .into_iter()
            .filter(|(r, _)| !english.iter().any(|e| e.start <= r.start && r.end <= e.end))
            .filter(|(r, _)| unmarked(r))
            .collect()
    } else {
        Vec::new()
    };
    let mut exclude = english.clone();
    exclude.extend(marked.iter().cloned());
    exclude.extend(others.iter().map(|(r, _)| r.clone()));
    let own = segments_minus(a, &exclude);
    // The other Nordic language vouches for longer words (a Finnish title in Swedish text).
    let neighbour = match lang {
        "fi" => crate::rules::spell_lang::speller("sv", ctx.config),
        "sv" => crate::rules::spell_lang::speller("fi", ctx.config),
        _ => None,
    };
    crate::rules::grammar::check_language(ctx, lang, sp.clone(), neighbour, Some(&own), out);
    crate::rules::slop::languages::check(ctx, lang, Some(&own), out);
    if !english.is_empty() {
        let mut en = Vec::new();
        crate::rules::grammar::check(ctx, &mut en);
        en.retain(|f| english.iter().any(|r| r.contains(&f.range.start)));
        out.extend(en);
    }
    let mut by_lang: std::collections::BTreeMap<&str, Vec<std::ops::Range<usize>>> =
        Default::default();
    for (r, l) in others {
        by_lang.entry(l).or_default().push(r);
    }
    for (l, ranges) in by_lang {
        crate::rules::slop::languages::check(ctx, l, Some(&ranges), out);
        if let Some(other) = crate::rules::spell_lang::speller(l, ctx.config) {
            let main = Some(sp.clone());
            crate::rules::grammar::check_language(ctx, l, other, main, Some(&ranges), out);
        }
    }
    let mut prose = Vec::new();
    crate::rules::prose::check(ctx, &mut prose);
    prose.retain(|f| LANGUAGE_NEUTRAL_PROSE.contains(&f.rule.as_str()));
    out.extend(prose);
}

/// Spelling of Finnish and Swedish stretches inside a file in language `lang` (other than
/// `lang`, outside `skip`), each with its language's speller when this build has one.
fn check_stretches(ctx: &FileCtx, lang: &str, skip: &[std::ops::Range<usize>], out: &mut Out) {
    let mut by_lang: std::collections::BTreeMap<&str, Vec<std::ops::Range<usize>>> =
        Default::default();
    for (r, l) in crate::lang::foreign_stretches(&ctx.a.segments, &ctx.a.file.text) {
        if let Some(l) = l.filter(|l| *l != lang)
            && !skip.iter().any(|s| s.contains(&r.start))
        {
            by_lang.entry(l).or_default().push(r);
        }
    }
    for (l, ranges) in by_lang {
        crate::rules::slop::languages::check(ctx, l, Some(&ranges), out);
        if let Some(sp) = crate::rules::spell_lang::speller(l, ctx.config) {
            crate::rules::grammar::check_language(ctx, l, sp, None, Some(&ranges), out);
        }
    }
}

/// Drop English-only findings inside foreign-language stretches of an English file, e.g. a
/// quoted Finnish sentence or table cell (see `crate::lang`); `marked` regions are left to
/// [`marked_prose`].
fn drop_foreign_segment_findings(a: &Analyzed, marked: &[std::ops::Range<usize>], out: &mut Out) {
    if marked.is_empty() {
        crate::lang::drop_foreign_findings(&a.segments, &a.file.text, out);
        return;
    }
    let (mut kept, mut rest): (Out, Out) = std::mem::take(out)
        .into_iter()
        .partition(|f| marked.iter().any(|r| r.contains(&f.range.start)));
    crate::lang::drop_foreign_findings(&a.segments, &a.file.text, &mut rest);
    kept.extend(rest);
    *out = kept;
}

/// Findings of rules that look at other files or the network (links, docs); never cached.
pub fn cross_findings(a: &Analyzed, ws: &Workspace, config: &Config, statuses: &Checked) -> Out {
    let ctx = FileCtx { a, config };
    let mut out: Out = Vec::new();
    if a.md.is_some() {
        crate::rules::docs::check(&ctx, ws, &mut out);
    }
    crate::links::local::check(&ctx, ws, &mut out);
    if !statuses.remote.is_empty() {
        crate::links::remote::report(&ctx, &statuses.remote, &mut out);
    }
    crate::links::same_repo::report(&ctx, &statuses.same_repo, &mut out);
    out
}

/// [`local_diagnostics`] with the workspace's [`ProjectVocab`].
fn local_diagnostics_in(
    a: &Analyzed,
    config: &Config,
    vocab: &Arc<ProjectVocab>,
) -> Vec<Diagnostic> {
    crate::rules::spell::with_project_vocab(Some(vocab.clone()), || local_diagnostics(a, config))
}

/// [`local_findings`] after suppressions and severities (`config` is the effective config).
pub fn local_diagnostics(a: &Analyzed, config: &Config) -> Vec<Diagnostic> {
    to_diagnostics(a, config, local_findings(a, config))
}

/// [`cross_findings`] after suppressions and severities (`config` is the effective config).
pub fn cross_diagnostics(
    a: &Analyzed,
    ws: &Workspace,
    config: &Config,
    statuses: &Checked,
) -> Vec<Diagnostic> {
    to_diagnostics(a, config, cross_findings(a, ws, config, statuses))
}

/// Apply suppressions, de-duplication and configured severities.
fn to_diagnostics(a: &Analyzed, config: &Config, mut out: Out) -> Vec<Diagnostic> {
    let sup = Suppressions::from(a);
    let mut seen = HashSet::new();
    // Terminology fixes are exact; drop the vaguer spelling hit on the same word.
    let terminology: HashSet<(usize, usize)> = out
        .iter()
        .filter(|f| f.rule == "prose/terminology")
        .map(|f| (f.range.start, f.range.end))
        .collect();
    out.retain(|f| f.rule != "spelling" || !terminology.contains(&(f.range.start, f.range.end)));
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
            let (end_line, end_column) = a.file.line_col(f.range.end.max(f.range.start));
            Some(Diagnostic {
                path: a.file.rel.clone(),
                rule: f.rule,
                severity,
                text: crate::diagnostic::snippet(&a.file.text, &f.range),
                range: f.range,
                line,
                column,
                end_line,
                end_column,
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
        if crate::rules::spell::is_manifest(path) {
            crate::rules::spell::forget_dep_names();
        }
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
        self.vocab = OnceLock::new();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FI: &str = "Tämä on esimerkkiteksti, joka on kirjoitettu suomeksi. Se kertoo \
        käyttäjälle miten ohjelma asennetaan ja miten sitä käytetään päivittäisessä työssä. \
        Ensin lataa paketti ja pura se haluamaasi hakemistoon. Sen jälkeen avaa pääte ja \
        siirry hakemistoon. Aja asennuskomento ja odota että kaikki riippuvuudet on ladattu. \
        Kun asennus on valmis, voit käynnistää ohjelman ja tarkistaa että se toimii oikein. \
        Jos kohtaat ongelmia, lue ohjeet uudelleen tai ota yhteyttä ylläpitäjään. Muista \
        myös päivittää ohjelma säännöllisesti, jotta saat uusimmat korjaukset ja ominaisuudet \
        käyttöösi. Kiitos että käytät ohjelmaamme ja toivomme että siitä on sinulle hyötyä \
        jokapäiväisessä työssäsi.";

    const EN: &str = "This is an example text that is written in English. It tells the user \
        how the program is installed and how it is used in daily work. First download the \
        package and extract it to a directory of your choice. Then open a terminal and move \
        to that directory. Run the install command and wait for all of the dependencies to \
        be downloaded. When the install is done, you can start the program and check that it \
        works as it should. If you run into problems, read the guide again or contact the \
        maintainer. Also remember to update the program on a regular basis so that you get \
        the latest fixes and features. Thank you for using our program and we hope that it \
        is useful to you in your daily work.";

    fn md(text: &str) -> Analyzed {
        Analyzed::new(SourceFile::new(
            PathBuf::from("/x/a.md"),
            PathBuf::from("a.md"),
            FileKind::Markdown,
            text.to_string(),
        ))
    }

    #[test]
    fn detects_non_english() {
        assert!(looks_non_english(&md(FI).segments, &Config::default()));
        assert!(!looks_non_english(&md(EN).segments, &Config::default()));
        // Too short to judge.
        assert!(!looks_non_english(
            &md("Tämä on lyhyt teksti.").segments,
            &Config::default()
        ));
    }

    #[test]
    fn front_matter_language_key() {
        let a = md("---\ntitle: X\nlang: fi-FI\n---\n\n# X\n");
        assert_eq!(front_matter_language(&a).as_deref(), Some("fi-FI"));
        let a = md("---\nlanguage: \"en\"\n---\n\n# X\n");
        assert!(prose_is_english(&a, &Config::default()));
        assert_eq!(front_matter_language(&md("# X\n")), None);
    }

    #[test]
    fn generated_markers() {
        assert!(is_generated_markdown(
            "<!-- This file is generated. Do not edit. -->\n# X\n"
        ));
        assert!(is_generated_markdown(
            "# Title\n\n<!-- AUTO-GENERATED -->\n"
        ));
        assert!(is_generated_markdown("<!--\n  @generated by tool\n-->\n"));
        assert!(!is_generated_markdown(
            "# Generated docs\n\nDo not edit the config.\n"
        ));
        assert!(!is_generated_markdown(
            "<!-- toc -->\n# Do not edit, generated later\n"
        ));
        assert!(is_generated("// Code generated by x. DO NOT EDIT.\n"));
        assert!(is_generated("# AutoGenerated file\n"));
        assert!(is_generated("/* Auto-generated */\n"));
        assert!(!is_generated("fn main() {}\n"));
    }

    fn project(files: &[(&str, &str)], config: &str) -> (tempfile::TempDir, Config) {
        let dir = tempfile::tempdir().unwrap();
        for (p, t) in files {
            let p = dir.path().join(p);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, t).unwrap();
        }
        std::fs::write(dir.path().join("explicit.toml"), config).unwrap();
        let c = Config::load(&dir.path().join("explicit.toml")).unwrap();
        (dir, c)
    }

    #[test]
    fn named_files_honor_excludes() {
        let (dir, c) = project(
            &[
                ("a.md", "# A\n"),
                ("vendor/b.md", "# B\n"),
                ("gen/c.md", "# C\n"),
                ("sub/d.md", "# D\n"),
                ("sub/.explicitignore", "d.md\n"),
                (".gitignore", "gen/\n"),
            ],
            "[general]\nexclude = [\"vendor/**\"]\n",
        );
        let named: Vec<PathBuf> = ["a.md", "vendor/b.md", "gen/c.md", "sub/d.md"]
            .iter()
            .map(|p| dir.path().join(p))
            .collect();
        let names = |v: Vec<PathBuf>| -> Vec<String> {
            v.iter()
                .filter(|p| p.extension().is_some_and(|e| e == "md"))
                .map(|p| p.strip_prefix(&c.root).unwrap().display().to_string())
                .collect()
        };
        assert_eq!(names(discover(&named, &c).unwrap()), ["a.md"]);
        assert_eq!(names(discover_with(&named, &c, false).unwrap()).len(), 4);
        // Directory walks agree.
        assert_eq!(
            names(discover(&[dir.path().to_path_buf()], &c).unwrap()),
            ["a.md"]
        );
    }

    fn rules_for(c: &Config, rel: &str) -> Vec<String> {
        let path = c.root.join(rel);
        let ws = build_workspace(std::slice::from_ref(&path), c);
        check(&ws, &[path], c, &Options::default())
            .into_iter()
            .map(|d| d.rule)
            .collect()
    }

    #[test]
    fn non_english_files_skip_prose_rules() {
        let bad = "# Otsikko\n\nTämä teksti on suomea ja sisältää sanoja, joita ei ole englannissa.\n\n[rikki](puuttuu.md)\n";
        let (_dir, c) = project(
            &[
                ("docs/fi/a.md", bad),
                ("b.md", &format!("---\nlang: fi\n---\n\n{bad}")),
                ("c.md", bad),
            ],
            "[general]\ndetect_language = false\n[[overrides]]\npaths = [\"docs/fi/**\"]\nlanguage = \"fi\"\n",
        );
        for rel in ["docs/fi/a.md", "b.md"] {
            let rules = rules_for(&c, rel);
            assert!(
                rules.contains(&"links/missing-file".to_string()),
                "{rel}: {rules:?}"
            );
            assert!(
                !rules
                    .iter()
                    .any(|r| r == "spelling" || r.starts_with("grammar/")),
                "{rel}: {rules:?}"
            );
        }
        assert!(rules_for(&c, "c.md").contains(&"spelling".to_string()));
    }

    /// `(rule, flagged text)` of `rel`'s diagnostics.
    #[cfg(any(feature = "voikko", feature = "swedish"))]
    fn found(c: &Config, rel: &str) -> Vec<(String, String)> {
        let path = c.root.join(rel);
        let text = std::fs::read_to_string(&path).unwrap();
        let ws = build_workspace(std::slice::from_ref(&path), c);
        check(&ws, &[path], c, &Options::default())
            .into_iter()
            .map(|d| (d.rule.clone(), text[d.range.clone()].to_string()))
            .collect()
    }

    #[cfg(feature = "voikko")]
    #[test]
    fn finnish_files_get_finnish_spelling() {
        let fi = "---\nlang: fi-FI\n---\n\n# Ohje\n\nTämä ohje kertoo, miten potilasasiakrja tallenetaan \
            järjestelmään. Kokous pidetään Tiistaina.\n\n\
            The English summary has a speling mistake and the words are otherwise fine.\n";
        let (_dir, c) = project(&[("a.md", fi)], "");
        let got = found(&c, "a.md");
        let spelled: Vec<&str> = got
            .iter()
            .filter(|(r, _)| r == "spelling")
            .map(|(_, t)| t.as_str())
            .collect();
        assert_eq!(
            spelled,
            ["potilasasiakrja", "tallenetaan", "speling"],
            "{got:?}"
        );
        assert!(got.contains(&("grammar/FinnishCapitalization".into(), "Tiistaina".into())));
        // English-only families stay off for the Finnish prose.
        assert!(!got.iter().any(|(r, _)| r.starts_with("slop/")), "{got:?}");
    }

    /// One-word Finnish cells of an English table: Finnish spelling when their column has
    /// Finnish cells (with `voikko`), else no English spelling finding either.
    #[test]
    fn short_finnish_table_cells_in_english_file() {
        let md = "# Requirements\n\nThe matrix below lists the requirements and their state.\n\n\
            | Id | Vaatimus | Tila |\n\
            |----|----------|------|\n\
            | R1 | Potilastiedot tallennetaan salattuna | Valmis |\n\
            | R2 | Lokitiedot säilytetään viisi vuotta | Kesken |\n\
            | R3 | Käyttäjä tunnistetaan vahvasti | Valmsi |\n\
            | R4 | Varmuuskopiot otetaan päivittäin | recieved |\n";
        let (_dir, c) = project(&[("a.md", md)], "");
        let path = c.root.join("a.md");
        let ws = build_workspace(std::slice::from_ref(&path), &c);
        let spelled: Vec<String> = check(&ws, &[path], &c, &Options::default())
            .into_iter()
            .filter(|d| d.rule == "spelling")
            .map(|d| md[d.range].to_string())
            .collect();
        let want: &[&str] = if cfg!(feature = "voikko") {
            &["Valmsi", "recieved"]
        } else {
            &["recieved"]
        };
        assert_eq!(spelled, want);
    }

    /// English words and passages inside a Finnish file follow `prose.dialect`.
    #[cfg(feature = "voikko")]
    #[test]
    fn english_inside_finnish_follows_the_dialect() {
        let fi = "---\nlang: fi\n---\n\n# Ohje\n\nTämä ohje kertoo, miten sovelluksen colour \
            valitaan asetuksista ja tallennetaan.\n\n\
            The colour of the button is set in the settings and saved to the profile.\n";
        let colours = |cfg: &str| {
            let (_dir, c) = project(&[("a.md", fi)], cfg);
            found(&c, "a.md")
                .into_iter()
                .filter(|(r, t)| r == "spelling" && t == "colour")
                .count()
        };
        assert_eq!(colours(""), 2);
        assert_eq!(colours("[prose]\ndialect = \"british\"\n"), 0);
    }

    #[cfg(feature = "voikko")]
    #[test]
    fn finnish_dictionary_path_from_config() {
        let dir = tempfile::tempdir().unwrap();
        let vfst = dir.path().join("dict/5/mor-standard");
        std::fs::create_dir_all(&vfst).unwrap();
        let data = miniz_oxide::inflate::decompress_to_vec_zlib(include_bytes!(
            "../dictionaries/fi/mor.vfst.zlib"
        ))
        .unwrap();
        std::fs::write(vfst.join("mor.vfst"), data).unwrap();
        std::fs::write(
            dir.path().join("a.md"),
            "---\nlang: fi\n---\n\n# Ohje\n\nKissa istuu talosa.\n",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("explicit.toml"),
            "[languages.fi]\ndictionary_path = \"dict\"\naccept = [\"talosa\"]\n",
        )
        .unwrap();
        let c = Config::load(&dir.path().join("explicit.toml")).unwrap();
        assert!(crate::rules::spell_lang::dictionary_files(&c)[0].ends_with("mor.vfst"));
        assert!(found(&c, "a.md").iter().all(|(r, _)| r != "spelling"));
    }

    #[cfg(feature = "swedish")]
    #[test]
    fn swedish_files_and_stretches() {
        let sv = "---\nlang: sv\n---\n\n# Rubrik\n\nKontakta kund tjänsten om sjukvårdssystemet \
            inte fungrar.\n";
        let (_dir, c) = project(
            &[
                ("sv.md", sv),
                (
                    "en.md",
                    "# Matrix\n\n| Id | Requirement | Source |\n|----|----|----|\n\
                     | R1 | Store the records | Uppgifterna ska sparas och inte delsa |\n",
                ),
            ],
            "",
        );
        let got = found(&c, "sv.md");
        assert!(
            got.contains(&("spelling".into(), "fungrar".into())),
            "{got:?}"
        );
        assert!(got.contains(&(
            "grammar/SwedishCompoundSplit".into(),
            "kund tjänsten".into()
        )));
        assert!(
            !got.iter().any(|(_, t)| t == "sjukvårdssystemet"),
            "{got:?}"
        );
        // A Swedish table cell in an English file is checked with the Swedish speller.
        let got = found(&c, "en.md");
        assert!(
            got.contains(&("spelling".into(), "delsa".into())),
            "{got:?}"
        );
        assert!(!got.iter().any(|(_, t)| t == "Uppgifterna"), "{got:?}");
    }

    /// Words with `spelling` findings in `rel`.
    #[cfg(any(feature = "voikko", feature = "swedish"))]
    fn spelled(c: &Config, rel: &str) -> Vec<String> {
        found(c, rel)
            .into_iter()
            .filter(|(r, _)| r == "spelling")
            .map(|(_, t)| t)
            .collect()
    }

    #[cfg(feature = "swedish")]
    #[test]
    fn marked_regions_override_detection() {
        let en = "# Guide\n\nThe installer copies teh files. The button says \
            <span lang=\"sv\">Ta emot</span> and <span lang='sv-FI'>Spara i systemt</span>.\n\n\
            <!-- explicit-lang sv -->\n\nFilerna sparas i systemt.\n\n\
            <!-- explicit-lang en -->\n\nBack in Englsh.\n\n<!-- explicit-lang end -->\n\n\
            <div lang=\"de\">\n\nUnbekannte Wrter werden übersprungen.\n\n</div>\n\nThe ende.\n";
        let sv = "---\nlang: sv\n---\n\n# Rubrik\n\nFilerna sparas i systemt.\n\n\
            <!-- explicit-lang en -->\n\nThis part is Englsh, marked as such.\n";
        let rs = "// Copies teh files.\n// explicit-lang sv\n// Filerna sparas i systemt.\n\
            fn a() {}\n// explicit-lang end\n// Back in Englsh.\nfn b() {}\n";
        let (_dir, c) = project(&[("en.md", en), ("sv.md", sv), ("a.rs", rs)], "");
        // English outside; Swedish and German spelling inside their explicit regions.
        assert_eq!(
            spelled(&c, "en.md"),
            ["teh", "systemt", "systemt", "Englsh", "Wrter", "ende"]
        );
        assert_eq!(spelled(&c, "sv.md"), ["systemt", "Englsh"]);
        assert_eq!(spelled(&c, "a.rs"), ["teh", "systemt", "Englsh"]);
    }

    #[cfg(feature = "voikko")]
    #[test]
    fn marked_finnish_region() {
        let en = "# Guide\n\nThe label reads <span lang=\"fi\">Tallenna tiedosto \
            levylle</span>, and teh button <span lang=\"fi\">tallenna tiedstoa</span> is broken.\n";
        let (_dir, c) = project(&[("en.md", en)], "");
        assert_eq!(spelled(&c, "en.md"), ["teh", "tiedstoa"]);
    }

    #[cfg(feature = "swedish")]
    #[test]
    fn per_language_vocab() {
        let text = "# Guide\n\nThe Zorpvardik app by Frobnik Ab uses Kundzorp.\n\n\
            <!-- explicit-lang sv -->\n\nAppen Zorpvardik från Frobnik Ab använder Kundzorp och \
            Blixtfrob.\n";
        let config = "[[vocab]]\nterm = \"Zorpvardik\"\ndescription = \"Swedish app name\"\n\
            lang = \"sv\"\n\n[[vocab]]\nterm = \"Kundzorp\"\ndescription = \"Everywhere\"\n\n\
            [[entity]]\nname = \"Frobnik Ab\"\nrelationship = \"Vendor\"\nlangs = [\"fi\", \"sv-FI\"]\n\n\
            [languages.sv]\naccept = [\"Blixtfrob\"]\n";
        let (_dir, c) = project(&[("a.md", text)], config);
        // Swedish-only terms are flagged in English and accepted in the Swedish region; terms
        // without a language are accepted in both.
        assert_eq!(spelled(&c, "a.md"), ["Zorpvardik", "Frobnik"]);
        assert!(c.vocab[0].applies_to("sv") && !c.vocab[0].applies_to("en"));
        assert!(c.entities[0].applies_to("fi") && c.vocab[1].applies_to("en"));
    }

    #[test]
    fn project_vocab_names_and_sub_manifests() {
        let (dir, c) = project(
            &[
                (
                    "README.md",
                    "# A\n\nZellij runs quuxlib. See [Frobozz](x.md).\n",
                ),
                (
                    "x.md",
                    "# X\n\nZellij and Zellij. Teh cat. Teh dog. Teh end.\n",
                ),
                ("y.md", "# Y\n\nThe zellij tool and Frobozz here.\n"),
                ("crates/q/notes.md", "# Q\n\nNotes.\n"),
                ("crates/q/Cargo.toml", "[dependencies]\nquuxlib = \"1\"\n"),
            ],
            "",
        );
        let files = discover(&[dir.path().to_path_buf()], &c).unwrap();
        let ws = build_workspace(&files, &c);
        let words: Vec<String> = check(&ws, &files, &c, &Options::default())
            .into_iter()
            .filter(|d| d.rule == "spelling")
            .map(|d| format!("{}:{}", d.path.display(), d.text))
            .collect();
        let v = ws.project_vocab(&c);
        assert!(v.has_name("Zellij") && !v.has_name("zellij") && !v.has_name("Teh"));
        assert!(v.has_dep("quuxlib"));
        // Recurring name and sub-project dependency accepted; lowercase form, a name only in
        // link text of another file, and a recurring typo still flagged.
        assert!(
            !words
                .iter()
                .any(|w| w.contains("Zellij") || w.contains("quuxlib")),
            "{words:?}"
        );
        assert!(
            !words
                .iter()
                .any(|w| w.starts_with("README.md") && w.contains("Frobozz")),
            "{words:?}"
        );
        assert!(words.iter().any(|w| w.contains("zellij")), "{words:?}");
        assert!(
            words
                .iter()
                .any(|w| w.starts_with("y.md") && w.contains("Frobozz")),
            "{words:?}"
        );
        assert_eq!(
            words.iter().filter(|w| w.contains("Teh")).count(),
            3,
            "{words:?}"
        );
    }
}
