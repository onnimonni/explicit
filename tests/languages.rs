//! Non-English spelling boundaries: capitalization, normalization and source offsets.
use explicit::config::Config;
use explicit::engine::{self, Options};

#[test]
fn recurring_german_nouns_do_not_become_project_names() {
    let dir = tempfile::tempdir().unwrap();
    let mut config = Config {
        root: dir.path().to_path_buf(),
        ..Config::default()
    };
    config.links.remote = false;
    config.links.cache = false;
    config.links.check_same_repo = false;
    let sources = [
        "---\nlang: de\n---\n\n# 1\n\nDie Dattenbank speichert den Bericht.\n",
        "---\nlang: de\n---\n\n# 1\n\nEine Dattenbank enthält die Daten.\n",
        "---\nlang: de\n---\n\n# 1\n\nWir öffnen die Dattenbank.\n",
    ];
    let paths: Vec<_> = sources
        .iter()
        .enumerate()
        .map(|(i, source)| {
            let path = dir.path().join(format!("page-{i}.md"));
            std::fs::write(&path, source).unwrap();
            path
        })
        .collect();
    let workspace = engine::build_workspace(&paths, &config);
    let diagnostics = engine::check(&workspace, &paths, &config, &Options::default());
    let spelling: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "spelling")
        .collect();
    assert_eq!(spelling.len(), 3, "{diagnostics:?}");
    for (i, diagnostic) in spelling.iter().enumerate() {
        assert_eq!(
            diagnostic.path,
            std::path::PathBuf::from(format!("page-{i}.md"))
        );
        assert_eq!(diagnostic.text, "Dattenbank");
        assert!(diagnostic.suggestions.iter().any(|s| s == "Datenbank"));
    }
}
