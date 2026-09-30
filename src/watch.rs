//! Watch mode: re-check changed files and the files that link to them. A change to the config,
//! a configured spelling dictionary (`[languages.*] dictionary_path`) or a vocab file
//! (`prose.vocab_files`) re-checks everything: dictionaries reload when their contents change.

use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::channel;
use std::time::Duration;

use notify::RecursiveMode;
use notify_debouncer_mini::new_debouncer;

use crate::config::{CONFIG_FILE, Config};
use crate::diagnostic::Diagnostic;
use crate::engine::{self, Options, Workspace};
use crate::output::{self, Format};

pub fn run(
    paths: &[PathBuf],
    load_config: &dyn Fn() -> Result<Config, String>,
    format: Format,
    opts: &Options,
) -> Result<(), String> {
    let mut config = load_config()?;
    let mut files = engine::discover(paths, &config)?;
    let mut ws = engine::build_workspace(&files, &config);
    let mut results: HashMap<PathBuf, Vec<Diagnostic>> = HashMap::new();
    for d in engine::check(&ws, &files, &config, opts) {
        results
            .entry(config.root.join(&d.path))
            .or_default()
            .push(d);
    }
    render(&results, &ws, format);

    let (tx, rx) = channel();
    let mut debouncer = new_debouncer(Duration::from_millis(250), tx).map_err(|e| e.to_string())?;
    for p in paths {
        debouncer
            .watcher()
            .watch(p, RecursiveMode::Recursive)
            .map_err(|e| format!("{}: {e}", p.display()))?;
    }
    // Dictionaries and vocab files may live outside the watched paths: watch their directories.
    let watched: Vec<PathBuf> = paths.iter().map(|p| canonical(p)).collect();
    let mut extra_dirs: HashSet<PathBuf> = HashSet::new();
    let mut inputs = input_files(&config);
    let mut watch_inputs =
        |inputs: &[PathBuf], debouncer: &mut notify_debouncer_mini::Debouncer<_>| {
            for dir in inputs.iter().filter_map(|f| f.parent()) {
                if !watched.iter().any(|w| dir.starts_with(w))
                    && extra_dirs.insert(dir.to_path_buf())
                {
                    let _ = debouncer.watcher().watch(dir, RecursiveMode::NonRecursive);
                }
            }
        };
    watch_inputs(&inputs, &mut debouncer);

    for batch in rx {
        let events = match batch {
            Ok(ev) => ev,
            Err(e) => {
                eprintln!("watch error: {e}");
                continue;
            }
        };
        // Event paths are raw; workspace keys and link targets are canonical.
        let changed: HashSet<PathBuf> = events
            .into_iter()
            .map(|e| e.path)
            .filter(|p| {
                !p.components().any(|c| {
                    matches!(
                        c.as_os_str().to_str(),
                        Some("target" | ".git" | "node_modules")
                    )
                })
            })
            .map(|p| canonical(&p))
            .collect();
        if changed.is_empty() {
            continue;
        }
        let dictionaries = inputs_changed(&changed, &inputs);
        if dictionaries {
            crate::rules::spell_lang::refresh_dictionaries();
        }
        if dictionaries
            || changed.iter().any(|p| {
                p.file_name().is_some_and(|n| {
                    n == CONFIG_FILE || n == ".gitignore" || n == ".explicitignore"
                })
            })
        {
            match load_config() {
                Ok(c) => config = c,
                Err(e) => {
                    eprintln!("config error: {e}");
                    continue;
                }
            }
            inputs = input_files(&config);
            watch_inputs(&inputs, &mut debouncer);
            files = match engine::discover(paths, &config) {
                Ok(f) => f,
                Err(e) => {
                    eprintln!("discover error: {e}");
                    continue;
                }
            };
            ws = engine::build_workspace(&files, &config);
            results.clear();
            for d in engine::check(&ws, &files, &config, opts) {
                results
                    .entry(config.root.join(&d.path))
                    .or_default()
                    .push(d);
            }
            render(&results, &ws, format);
            continue;
        }

        let new_files: HashSet<PathBuf> = match engine::discover(paths, &config) {
            Ok(f) => f.into_iter().collect(),
            Err(e) => {
                eprintln!("discover error: {e}");
                continue;
            }
        };
        // Links of changed files before reload: dropping a link re-checks its old target.
        let old_targets: HashSet<PathBuf> = changed
            .iter()
            .filter_map(|c| ws.files.get(c))
            .flat_map(|a| crate::links::local::targets(a, &config))
            .collect();
        for c in &changed {
            ws.invalidate(c);
            if new_files.contains(c) {
                if let Some(a) = engine::load(c, &config.root) {
                    ws.files.insert(c.clone(), Arc::new(a));
                }
            } else if ws.files.remove(c).is_some() {
                results.remove(c);
            }
        }
        let current: HashMap<PathBuf, Vec<PathBuf>> = ws
            .files
            .iter()
            .map(|(p, a)| (p.clone(), crate::links::local::targets(a, &config)))
            .collect();
        let affected = plan_recheck(&changed, &old_targets, &current);
        if affected.is_empty() {
            continue;
        }
        let targets: Vec<PathBuf> = affected.into_iter().collect();
        for t in &targets {
            results.remove(t);
        }
        for d in engine::check(&ws, &targets, &config, opts) {
            results
                .entry(config.root.join(&d.path))
                .or_default()
                .push(d);
        }
        render(&results, &ws, format);
    }
    Ok(())
}

/// Files besides the checked ones whose contents change results: configured spelling
/// dictionaries and vocab files, canonical.
pub fn input_files(config: &Config) -> Vec<PathBuf> {
    let mut out = crate::rules::spell_lang::dictionary_files(config);
    out.extend(config.prose.vocab_files.iter().map(|f| {
        if f.is_absolute() {
            f.clone()
        } else {
            config.root.join(f)
        }
    }));
    out.iter().map(|p| canonical(p)).collect()
}

/// Some changed path is an input file ([`input_files`]).
pub fn inputs_changed(changed: &HashSet<PathBuf>, inputs: &[PathBuf]) -> bool {
    inputs.iter().any(|f| changed.contains(f))
}

/// Canonical form of an event path; deleted files resolve via their parent.
fn canonical(p: &Path) -> PathBuf {
    if let Ok(c) = p.canonicalize() {
        return c;
    }
    match (p.parent(), p.file_name()) {
        (Some(dir), Some(name)) => dir
            .canonicalize()
            .map_or_else(|_| p.to_path_buf(), |d| d.join(name)),
        _ => p.to_path_buf(),
    }
}

/// Files to re-check: changed files still in the workspace, files whose links
/// point at (or into) a changed path, and old targets of changed files.
/// All paths canonical; `current` maps workspace files to their link targets.
pub fn plan_recheck(
    changed: &HashSet<PathBuf>,
    old_targets: &HashSet<PathBuf>,
    current: &HashMap<PathBuf, Vec<PathBuf>>,
) -> HashSet<PathBuf> {
    let mut out: HashSet<PathBuf> = HashSet::new();
    for (path, targets) in current {
        if changed.contains(path)
            || old_targets.contains(path)
            || targets
                .iter()
                .any(|t| changed.iter().any(|c| c.starts_with(t)))
        {
            out.insert(path.clone());
        }
    }
    out
}

fn render(results: &HashMap<PathBuf, Vec<Diagnostic>>, ws: &Workspace, format: Format) {
    let mut all: Vec<Diagnostic> = results.values().flatten().cloned().collect();
    all.sort_by(|a, b| (&a.path, a.range.start, &a.rule).cmp(&(&b.path, b.range.start, &b.rule)));
    let sources: HashMap<PathBuf, String> = ws
        .files
        .values()
        .map(|a| (a.file.rel.clone(), a.file.text.clone()))
        .collect();
    let mut out = std::io::stdout().lock();
    if format == Format::Human {
        let _ = write!(out, "\x1b[2J\x1b[H");
    }
    let _ = output::write(format, &all, &sources, &ws.root, &mut out);
    let _ = out.flush();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(ps: &[&str]) -> HashSet<PathBuf> {
        ps.iter().map(PathBuf::from).collect()
    }

    fn ws(entries: &[(&str, &[&str])]) -> HashMap<PathBuf, Vec<PathBuf>> {
        entries
            .iter()
            .map(|(p, t)| (PathBuf::from(p), t.iter().map(PathBuf::from).collect()))
            .collect()
    }

    #[test]
    fn dependents_and_changed_files_are_rechecked() {
        let current = ws(&[
            ("/r/a.md", &["/r/b.md"]),
            ("/r/b.md", &[]),
            ("/r/c.md", &["/r/docs"]),
            ("/r/d.md", &[]),
        ]);
        let got = plan_recheck(&set(&["/r/b.md"]), &HashSet::new(), &current);
        assert_eq!(got, set(&["/r/a.md", "/r/b.md"]));
        // Link to a directory: any change inside re-checks the linker.
        let got = plan_recheck(&set(&["/r/docs/x.md"]), &HashSet::new(), &current);
        assert_eq!(got, set(&["/r/c.md"]));
    }

    #[test]
    fn deleted_file_rechecks_linkers_only() {
        // b.md was deleted: removed from workspace, a.md still links to it.
        let current = ws(&[("/r/a.md", &["/r/b.md"])]);
        let got = plan_recheck(&set(&["/r/b.md"]), &HashSet::new(), &current);
        assert_eq!(got, set(&["/r/a.md"]));
    }

    #[test]
    fn removed_link_rechecks_old_target() {
        // a.md used to link to b.md; the new a.md has no links.
        let current = ws(&[("/r/a.md", &[]), ("/r/b.md", &[])]);
        let got = plan_recheck(&set(&["/r/a.md"]), &set(&["/r/b.md"]), &current);
        assert_eq!(got, set(&["/r/a.md", "/r/b.md"]));
    }

    /// Editing a configured dictionary or vocab file during `explicit watch` is an input change:
    /// the speller reloads (its dictionary's content hash changed) and a reloaded config reads
    /// the new vocab words.
    #[test]
    fn changed_dictionary_and_vocab_files_reload() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
        std::fs::write(root.join("xx.aff"), "SET UTF-8\n").unwrap();
        std::fs::write(root.join("xx.dic"), "1\nhello\n").unwrap();
        std::fs::write(root.join("words.txt"), "Frobnic\n").unwrap();
        std::fs::write(
            root.join(CONFIG_FILE),
            "[prose]\nvocab_files = [\"words.txt\"]\n\n[languages.xx]\ndictionary_path = \"xx.aff\"\n",
        )
        .unwrap();
        let load = || Config::load(&root.join(CONFIG_FILE)).unwrap();
        let config = load();
        let inputs = input_files(&config);
        assert!(inputs.contains(&root.join("xx.dic")), "{inputs:?}");
        assert!(inputs.contains(&root.join("words.txt")), "{inputs:?}");
        let sp = crate::rules::spell_lang::speller("xx", &config).unwrap();
        assert!(sp.check("hello") && !sp.check("world"));
        assert!(config.accepted_words().iter().any(|w| w == "Frobnic"));

        std::fs::write(root.join("xx.dic"), "2\nhello\nworld\n").unwrap();
        std::fs::write(root.join("words.txt"), "Blorb\n").unwrap();
        assert!(!inputs_changed(&set(&["/elsewhere/a.md"]), &inputs));
        let changed: HashSet<PathBuf> = [root.join("xx.dic"), root.join("words.txt")].into();
        assert!(inputs_changed(&changed, &inputs));
        // What the watch loop does on an input change.
        crate::rules::spell_lang::refresh_dictionaries();
        let config = load();
        let sp2 = crate::rules::spell_lang::speller("xx", &config).unwrap();
        assert!(sp2.check("world"), "reloaded dictionary");
        assert!(!Arc::ptr_eq(&sp, &sp2));
        assert!(config.accepted_words().iter().any(|w| w == "Blorb"));
        // Unchanged contents: the same speller.
        crate::rules::spell_lang::refresh_dictionaries();
        let sp3 = crate::rules::spell_lang::speller("xx", &load()).unwrap();
        assert!(Arc::ptr_eq(&sp2, &sp3));
    }

    #[test]
    fn canonical_resolves_symlinked_and_deleted_paths() {
        let dir = std::env::temp_dir().join(format!("explicit-watch-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let real = dir.canonicalize().unwrap();
        let link = dir.join("link");
        let _ = std::fs::remove_file(&link);
        #[cfg(unix)]
        std::os::unix::fs::symlink(&real, &link).unwrap();
        #[cfg(unix)]
        assert_eq!(canonical(&link.join("gone.md")), real.join("gone.md"));
        assert_eq!(canonical(&dir.join("gone.md")), real.join("gone.md"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
