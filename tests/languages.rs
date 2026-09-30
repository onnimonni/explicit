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

#[test]
fn english_homographs_do_not_hide_native_accents() {
    use explicit::rules::spell_lang::{speller, word_ok};
    let config = Config::default();
    for (code, plain, accented) in [
        ("de", "uber", "über"),
        ("fr", "materiel", "matériel"),
        ("es", "cafe", "café"),
    ] {
        let sp = speller(code, &config).unwrap();
        assert!(
            word_ok(&*sp, code, accented, Some(' ')),
            "{code}: {accented}"
        );
        assert!(!word_ok(&*sp, code, plain, Some(' ')), "{code}: {plain}");
        assert!(
            word_ok(&*sp, code, "backend", Some(' ')),
            "{code}: technical loan"
        );
    }
    // Valid native homographs keep both meanings; accent inference must not override them.
    for (code, words) in [("pt", ["por", "pôr"]), ("es", ["esta", "está"])] {
        let sp = speller(code, &config).unwrap();
        for word in words {
            assert!(word_ok(&*sp, code, word, Some(' ')), "{code}: {word}");
        }
    }
    let french = speller("fr", &config).unwrap();
    assert!(
        word_ok(&*french, "fr", "Legal", Some(' ')),
        "paper format name"
    );
    assert!(
        word_ok(&*french, "fr", "légal", Some(' ')),
        "accented French adjective"
    );
}

#[test]
fn decomposed_accents_keep_native_grammar_and_source_spans() {
    for (code, prose, rule, text, fix) in [
        (
            "de",
            "Du pru\u{308}ft den Bericht.",
            "GermanPronounVerbAgreement",
            "pru\u{308}ft",
            "prüfst",
        ),
        (
            "fr",
            "Elle est arrive\u{301}.",
            "FrenchAdjectiveAgreement",
            "arrive\u{301}",
            "arrivée",
        ),
        (
            "es",
            "Tu\u{301} somos amables.",
            "SpanishPronounVerbAgreement",
            "somos",
            "eres",
        ),
        (
            "pt",
            "Voce\u{302} estão disponíveis.",
            "PortuguesePronounVerbAgreement",
            "estão",
            "está",
        ),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let source = format!("---\nlang: {code}\n---\n\n# 1\n\n{prose}\n");
        let path = dir.path().join("grammar.md");
        std::fs::write(&path, &source).unwrap();
        let config = Config {
            root: dir.path().to_path_buf(),
            ..Config::default()
        };
        let paths = [path];
        let workspace = engine::build_workspace(&paths, &config);
        let diagnostics = engine::check(&workspace, &paths, &config, &Options::default());
        let expected = format!("grammar/{rule}");
        let findings: Vec<_> = diagnostics.iter().filter(|d| d.rule == expected).collect();
        assert_eq!(findings.len(), 1, "{code}: {diagnostics:?}");
        assert_eq!(findings[0].text, text);
        assert_eq!(&source[findings[0].range.clone()], text);
        assert_eq!(findings[0].suggestions, [fix]);
    }
}

#[test]
fn french_homographs_stay_native_without_disabling_english_crossover() {
    let dir = tempfile::tempdir().unwrap();
    let source = "---\nlang: fr\n---\n\n# Exemple\n\n\
        Il commence à le lire. Il refuse de le faire.\n\n\
        The colour of the button is set in the settings and saved to the profile.\n";
    let path = dir.path().join("mixed.md");
    std::fs::write(&path, source).unwrap();
    for (dialect, expected) in [
        ("american", vec![("spelling", "colour")]),
        ("british", vec![]),
    ] {
        let mut config = Config {
            root: dir.path().to_path_buf(),
            ..Config::default()
        };
        config.prose.dialect = dialect.into();
        let paths = [path.clone()];
        let workspace = engine::build_workspace(&paths, &config);
        let diagnostics = engine::check(&workspace, &paths, &config, &Options::default());
        let actual: Vec<_> = diagnostics
            .iter()
            .map(|d| (d.rule.as_str(), d.text.as_str()))
            .collect();
        assert_eq!(actual, expected, "{dialect}: {diagnostics:?}");
    }
}
