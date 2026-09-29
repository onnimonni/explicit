//! Results cache: cached runs report exactly what uncached runs report, also across copied
//! checkouts and a shared cache directory.

use std::path::{Path, PathBuf};

use explicit::cache::{DEFAULT_DIR, Stats};
use explicit::config::Config;
use explicit::engine::{self, Options};

fn config_at(root: &Path) -> Config {
    Config {
        root: root.canonicalize().expect("root exists"),
        ..Config::default()
    }
}

fn run(config: &Config, cache_dir: Option<&Path>) -> (String, Stats) {
    let files = engine::discover(std::slice::from_ref(&config.root), config).expect("discover");
    let ws = engine::build_workspace(&files, config);
    let opts = Options {
        cache_dir: cache_dir.map(Path::to_path_buf),
        prune_cache: true,
        ..Options::default()
    };
    let (d, stats) = engine::check_with_stats(&ws, &files, config, &opts);
    (serde_json::to_string_pretty(&d).unwrap(), stats)
}

fn write_project(root: &Path, a_text: &str) {
    std::fs::create_dir_all(root.join("docs")).unwrap();
    std::fs::write(root.join("a.md"), a_text).unwrap();
    std::fs::write(
        root.join("docs/b.md"),
        "# Other\n\nSee [a](../a.md) and [gone](gone.md).\n",
    )
    .unwrap();
    std::fs::write(
        root.join("explicit.toml"),
        "[prose]\naccept = [\"Kalevala\"]\n",
    )
    .unwrap();
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for e in std::fs::read_dir(from).unwrap() {
        let e = e.unwrap();
        let dst = to.join(e.file_name());
        if e.file_type().unwrap().is_dir() {
            copy_dir(&e.path(), &dst);
        } else {
            std::fs::copy(e.path(), dst).unwrap();
        }
    }
}

#[test]
fn cached_run_matches_uncached() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sample");
    let config = config_at(&root);
    let cache = tempfile::tempdir().unwrap();
    let (uncached, _) = run(&config, None);
    let (first, s1) = run(&config, Some(cache.path()));
    assert!(cache.path().join("results.json").exists());
    let (second, s2) = run(&config, Some(cache.path()));
    assert_eq!(uncached, first);
    assert_eq!(first, second);
    assert_eq!(s1.hits, 0);
    assert_eq!((s2.hits, s2.misses), (s1.misses, 0));
}

#[test]
fn edited_file_is_rechecked() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    write_project(&root, "# Title\n\nSome text.\n");
    let config = config_at(&root);
    let cache = root.join(DEFAULT_DIR);
    assert_eq!(run(&config, Some(&cache)).0, run(&config, None).0);
    // Cache dir is self-ignoring and skipped by discovery.
    assert!(cache.join(".gitignore").exists());
    let files = engine::discover(std::slice::from_ref(&root), &config).unwrap();
    assert!(files.iter().all(|f| !f.starts_with(&cache)));

    // Both an edited file and a deleted link target show up.
    std::fs::write(root.join("a.md"), "# Title\n\n\n\nSome  text.\n").unwrap();
    let (edited, s) = run(&config, Some(&cache));
    assert_eq!(edited, run(&config, None).0);
    assert_eq!((s.hits, s.misses), (2, 1), "b.md and explicit.toml hit");
    std::fs::remove_file(root.join("a.md")).unwrap();
    let (after, _) = run(&config, Some(&cache));
    assert_eq!(after, run(&config, None).0);
    assert!(after.contains("docs/b.md"));
}

#[test]
fn copied_checkout_hits_its_copied_cache() {
    let d1 = tempfile::tempdir().unwrap();
    let d2 = tempfile::tempdir().unwrap();
    let p1 = d1.path().join("proj");
    let p2 = d2.path().join("elsewhere/proj-copy");
    write_project(&p1, "# Title\n\nKalevala text  here.\n");
    let c1 = config_at(&p1);
    let (orig, s) = run(&c1, Some(&c1.root.join(DEFAULT_DIR)));
    assert_eq!(s.hits, 0);

    copy_dir(&p1, &p2);
    let c2 = config_at(&p2);
    let (copied, s) = run(&c2, Some(&c2.root.join(DEFAULT_DIR)));
    assert_eq!((s.hits, s.misses), (3, 0), "all hits in the copy");
    assert_eq!(copied, orig);
    assert_eq!(copied, run(&c2, None).0);
}

#[test]
fn worktrees_sharing_a_cache_dir_do_not_thrash() {
    let shared = tempfile::tempdir().unwrap();
    let d1 = tempfile::tempdir().unwrap();
    let d2 = tempfile::tempdir().unwrap();
    write_project(d1.path(), "# Title\n\nFirst version.\n");
    write_project(d2.path(), "# Title\n\nSecond  version.\n");
    let (c1, c2) = (config_at(d1.path()), config_at(d2.path()));
    // Round one fills the cache; b.md and explicit.toml are identical, so the second tree hits.
    assert_eq!(run(&c1, Some(shared.path())).1.misses, 3);
    let s = run(&c2, Some(shared.path())).1;
    assert_eq!((s.hits, s.misses), (2, 1));
    // Round two: everything hits in both trees.
    for c in [&c1, &c2] {
        let (out, s) = run(c, Some(shared.path()));
        assert_eq!((s.hits, s.misses), (3, 0));
        assert_eq!(out, run(c, None).0);
    }
    assert_eq!(explicit::cache::entry_count(shared.path()), 4);
}
