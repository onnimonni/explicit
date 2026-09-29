//! Discovery and orchestration.

use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use crate::config::Config;
use crate::diagnostic::Diagnostic;
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

/// Common English function words; they make up roughly a quarter of English prose.
const STOPWORDS: &[&str] = &[
    "the", "and", "of", "to", "is", "that", "for", "with", "it", "as", "are", "be", "this", "was",
    "by", "an", "or", "from", "at", "not", "you", "we", "can", "will", "have", "has", "if",
    "which", "your", "they", "their", "these", "when", "than", "but", "into", "its", "been",
    "were", "a",
];

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

/// Language code from Markdown front matter (`lang:` / `language:`), first two letters.
fn front_matter_language(a: &Analyzed) -> Option<String> {
    let range = a.md.as_ref()?.front_matter.clone()?;
    let fm = a.file.text.get(range)?;
    fm.lines().find_map(|l| {
        let (k, v) = l.split_once(':').or_else(|| l.split_once('='))?;
        if !matches!(k.trim_end(), "lang" | "language") || l.starts_with(char::is_whitespace) {
            return None;
        }
        let v = v.trim().trim_matches(|c| c == '"' || c == '\'');
        (!v.is_empty()).then(|| v.chars().take(2).collect::<String>().to_lowercase())
    })
}

/// Whether English prose rules (spelling, grammar, prose, slop) apply to this file.
fn prose_is_english(a: &Analyzed, config: &Config) -> bool {
    let lang = front_matter_language(a).unwrap_or_else(|| config.general.language.clone());
    if !crate::config::is_english(&lang) {
        return false;
    }
    !(config.general.detect_language && looks_non_english(&a.segments, config))
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

    let statuses: HashMap<String, RemoteStatus> = if opts.remote && config.links.remote {
        let mut urls: HashSet<crate::links::remote::Target> = HashSet::new();
        for a in &targets {
            urls.extend(crate::links::remote::urls(&FileCtx { a, config }));
        }
        let mut urls: Vec<_> = urls.into_iter().collect();
        urls.sort();
        // The link cache lives in the project cache dir and stays on with `--no-cache`
        // (that only disables the results cache), unless `links.cache = false`.
        let link_cache = config.links.cache.then(|| {
            let dir = opts
                .cache_dir
                .clone()
                .unwrap_or_else(|| crate::links::cache::default_dir(config));
            crate::links::cache::path_in(&dir)
        });
        crate::links::remote::check_all(&urls, config, link_cache)
    } else {
        HashMap::new()
    };

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
            let effective = config.for_path(&a.file.rel);
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
    statuses: &HashMap<String, RemoteStatus>,
) -> Vec<Diagnostic> {
    let effective = config.for_path(&a.file.rel);
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
    if is_md || config.comments.enabled {
        // Non-English prose keeps only user style rules.
        let english = prose_is_english(a, config);
        if english {
            crate::rules::grammar::check(&ctx, &mut out);
        }
        crate::rules::style::check(&ctx, &mut out);
        if english {
            crate::rules::slop::check(&ctx, &mut out);
            crate::rules::prose::check(&ctx, &mut out);
            if config.general.detect_language {
                drop_foreign_segment_findings(a, &mut out);
            }
        }
    }
    out
}

/// English-only rule families.
fn is_english_rule(rule: &str) -> bool {
    rule == "spelling"
        || ["grammar/", "prose/", "slop/"]
            .iter()
            .any(|p| rule.starts_with(p))
}

/// Drop English-only findings inside foreign-language stretches of an English file, e.g. a
/// quoted Finnish sentence or table cell: whole segments, or phrases between sentence
/// punctuation, quotes, brackets, table pipes and line breaks.
fn drop_foreign_segment_findings(a: &Analyzed, out: &mut Out) {
    let mut foreign: Vec<std::ops::Range<usize>> = Vec::new();
    for s in &a.segments {
        if looks_foreign(&s.text, 6, 6) {
            foreign.push(s.range.clone());
            continue;
        }
        let mut start = 0;
        let bounds = s
            .text
            .match_indices(|c: char| ".!?;:|\"“”«»()[]\n".contains(c))
            .map(|(i, m)| (i, i + m.len()))
            .chain([(s.text.len(), s.text.len())]);
        for (end, next) in bounds {
            if looks_foreign(&s.text[start..end], 3, 4) {
                foreign.push(s.abs(start..end));
            }
            start = next;
        }
    }
    if !foreign.is_empty() {
        out.retain(|f| {
            !is_english_rule(&f.rule) || !foreign.iter().any(|r| r.contains(&f.range.start))
        });
    }
}

/// At least `min_words` words, under 5% English stopwords and at least one in `ascii_div`
/// words with non-ASCII letters.
fn looks_foreign(text: &str, min_words: usize, ascii_div: usize) -> bool {
    let mut n = 0;
    let mut stop = 0;
    let mut non_ascii = 0;
    for w in text
        .split(|c: char| !c.is_alphabetic() && c != '\'')
        .map(|w| w.trim_matches('\''))
        .filter(|w| !w.is_empty())
    {
        n += 1;
        if !w.is_ascii() {
            non_ascii += 1;
        } else if STOPWORDS.contains(&w.to_ascii_lowercase().as_str()) {
            stop += 1;
        }
    }
    n >= min_words && stop * 20 < n && non_ascii * ascii_div >= n
}

/// Findings of rules that look at other files or the network (links, docs); never cached.
pub fn cross_findings(
    a: &Analyzed,
    ws: &Workspace,
    config: &Config,
    statuses: &HashMap<String, RemoteStatus>,
) -> Out {
    let ctx = FileCtx { a, config };
    let mut out: Out = Vec::new();
    if a.md.is_some() {
        crate::rules::docs::check(&ctx, ws, &mut out);
    }
    crate::links::local::check(&ctx, ws, &mut out);
    if !statuses.is_empty() {
        crate::links::remote::report(&ctx, statuses, &mut out);
    }
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
    statuses: &HashMap<String, RemoteStatus>,
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
    fn foreign_phrases() {
        assert!(looks_foreign("Tämä on hyvä käyttäjälle", 3, 4));
        assert!(!looks_foreign("The user sees the café menu", 3, 4));
        assert!(!looks_foreign("Hyvä päivä", 3, 4));
        // Quoted Finnish inside an English paragraph: only the quote is dropped.
        let text = format!("# T\n\n{EN} The label says \"Tämä älä käytä väärin\" and teh end.\n");
        let a = md(&text);
        let at = |needle: &str| text.find(needle).unwrap();
        let mut out: Out = vec![
            crate::diagnostic::Finding::new(
                "spelling",
                crate::diagnostic::Severity::Error,
                at("älä")..at("älä") + 4,
                "x",
            ),
            crate::diagnostic::Finding::new(
                "spelling",
                crate::diagnostic::Severity::Error,
                at("teh")..at("teh") + 3,
                "x",
            ),
            crate::diagnostic::Finding::new(
                "md/x",
                crate::diagnostic::Severity::Error,
                at("älä")..at("älä") + 4,
                "x",
            ),
        ];
        drop_foreign_segment_findings(&a, &mut out);
        let rules: Vec<(&str, usize)> = out
            .iter()
            .map(|f| (f.rule.as_str(), f.range.start))
            .collect();
        assert_eq!(rules, [("spelling", at("teh")), ("md/x", at("älä"))]);
    }

    #[test]
    fn front_matter_language_key() {
        let a = md("---\ntitle: X\nlang: fi-FI\n---\n\n# X\n");
        assert_eq!(front_matter_language(&a).as_deref(), Some("fi"));
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
        let bad = "# Otsikko\n\nTämä teksti on suomea ja sisältää sanoja joita ei ole englannissa.\n\n[rikki](puuttuu.md)\n";
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
