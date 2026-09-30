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

#[test]
fn decomposed_accents_preserve_whole_word_diagnostics_and_offsets() {
    let dir = tempfile::tempdir().unwrap();
    let source =
        "---\nlang: fr\n---\n\n# 1\n\nUne re\u{301}ponse arrive. Une re\u{301}ponnse arrive.\n";
    let path = dir.path().join("accents.md");
    std::fs::write(&path, source).unwrap();
    let config = Config {
        root: dir.path().to_path_buf(),
        ..Config::default()
    };
    let paths = [path];
    let workspace = engine::build_workspace(&paths, &config);
    let diagnostics = engine::check(&workspace, &paths, &config, &Options::default());
    let spelling: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "spelling")
        .collect();
    assert_eq!(spelling.len(), 1, "{diagnostics:?}");
    let finding = spelling[0];
    assert_eq!(finding.text, "re\u{301}ponnse");
    assert_eq!(&source[finding.range.clone()], "re\u{301}ponnse");
    assert!(finding.suggestions.iter().any(|s| s == "réponse"));
}

#[test]
fn native_grammar_runs_with_spelling_disabled() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("agreement.md");
    std::fs::write(&path, "---\nlang: fr\n---\n\n# 1\n\nNous est prêts.\n").unwrap();
    let mut config = Config {
        root: dir.path().to_path_buf(),
        ..Config::default()
    };
    config
        .rules
        .insert("spelling".into(), explicit::config::Level::Off);
    let paths = [path];
    let workspace = engine::build_workspace(&paths, &config);
    let diagnostics = engine::check(&workspace, &paths, &config, &Options::default());
    let grammar: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "grammar/FrenchPronounVerbAgreement")
        .collect();
    assert_eq!(grammar.len(), 1, "{diagnostics:?}");
    assert_eq!(grammar[0].text, "est");
    assert_eq!(grammar[0].suggestions, ["sommes"]);
    assert!(!diagnostics.iter().any(|d| d.rule == "spelling"));
}

#[test]
fn native_slop_respects_markers_code_and_quote_severity_without_spelling() {
    let dir = tempfile::tempdir().unwrap();
    let source = "# 1\n\nUn monde de possibilités illimitées.\n\n<div lang=\"fr\">\n\nUn monde de possibilités illimitées.\n\n> Un monde de possibilités illimitées.\n\n```text\nUn monde de possibilités illimitées.\n```\n\n</div>\n";
    let path = dir.path().join("claims.md");
    std::fs::write(&path, source).unwrap();
    let mut config = Config {
        root: dir.path().to_path_buf(),
        ..Config::default()
    };
    config.general.detect_language = false;
    config
        .rules
        .insert("spelling".into(), explicit::config::Level::Off);
    let paths = [path];
    let workspace = engine::build_workspace(&paths, &config);
    let diagnostics = engine::check(&workspace, &paths, &config, &Options::default());
    let slop: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "slop/phrase")
        .collect();
    assert_eq!(slop.len(), 2, "{diagnostics:?}");
    assert!(
        slop.iter()
            .all(|d| d.range.start > source.find("<div").unwrap())
    );
    assert_eq!(slop[0].severity, explicit::diagnostic::Severity::Warning);
    assert_eq!(slop[1].severity, explicit::diagnostic::Severity::Info);
}
