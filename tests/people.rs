//! `[[person]]` and `[people]`: name acceptance in every language, the test-path escape hatch,
//! placeholder names, `prose/ambiguous-person` and `config/placeholder` end to end.

use explicit::config::Config;
use explicit::engine::{self, Options};

fn project(files: &[(&str, &str)], config: &str) -> (tempfile::TempDir, Config) {
    let dir = tempfile::tempdir().unwrap();
    for (p, t) in files {
        let p = dir.path().join(p);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, t).unwrap();
    }
    std::fs::write(dir.path().join("explicit.toml"), config).unwrap();
    let mut c = Config::load(&dir.path().join("explicit.toml")).unwrap();
    c.links.remote = false;
    c.general.cache = false;
    (dir, c)
}

/// `(rule, flagged text, line)` of `rel`'s diagnostics.
fn found(c: &Config, rel: &str) -> Vec<(String, String, usize)> {
    let path = c.root.join(rel);
    let text = std::fs::read_to_string(&path).unwrap();
    let ws = engine::build_workspace(std::slice::from_ref(&path), c);
    engine::check(&ws, &[path], c, &Options::default())
        .into_iter()
        .map(|d| (d.rule.clone(), text[d.range.clone()].to_string(), d.line))
        .collect()
}

fn spelled(c: &Config, rel: &str) -> Vec<String> {
    found(c, rel)
        .into_iter()
        .filter(|(r, _, _)| r == "spelling")
        .map(|(_, t, _)| t)
        .collect()
}

const PERSONS: &str = r#"
[[person]]
name = "Qorvath Blenkiss"
role = "Backend engineer, owns billing"
aliases = ["Qorry B."]
handles = ["@qblenk"]
"#;

#[test]
fn names_parts_aliases_handles_and_possessives() {
    let md = "# Team\n\nThe patch is by Qorvath Blenkiss. We thank Qorvath, Blenkiss and \
              Qorry for it. Qorvath's queue and Blenkiss' notes. Ask @qblenk. QORVATH BLENKISS \
              agreed, qorvath did not.\n";
    let (_d, c) = project(&[("a.md", md)], "");
    let bare = spelled(&c, "a.md");
    assert!(bare.contains(&"Qorry".to_string()), "{bare:?}");
    let (_d, c) = project(&[("a.md", md)], PERSONS);
    // Names are case-sensitive, except ALL CAPS.
    assert_eq!(spelled(&c, "a.md"), ["qorvath"]);
}

#[test]
fn per_language_persons() {
    let md = "# Team\n\nThe patch is by Qorvath Blenkiss.\n";
    let cfg = format!("{PERSONS}lang = \"fi\"\n");
    let (_d, c) = project(&[("a.md", md)], &cfg);
    assert_eq!(spelled(&c, "a.md"), ["Qorvath", "Blenkiss"]);
}

#[cfg(feature = "voikko")]
#[test]
fn finnish_inflections_of_names() {
    // Sentence-initial forms: inside a sentence the Finnish check takes capitalized words as
    // names already.
    let fi = "---\nlang: fi\n---\n\n# Tiimi\n\nBlekan koodi on valmis. Blekalle tervehdys. \
              Blenkiselle kiitos. Blenkisen tiimi hoitaa sen. Qorvolta kysytään. Zorvaselle \
              myös.\n";
    let cfg = "[[person]]\nname = \"Qorvo Blenkinen\"\nrole = \"Kehittäjä\"\n\n\
               [[person]]\nname = \"Blekka Zorvanen\"\nrole = \"Testaaja\"\n";
    let (_d, c) = project(&[("a.md", fi)], "");
    assert_eq!(
        spelled(&c, "a.md"),
        [
            "Blekan",
            "Blekalle",
            "Blenkiselle",
            "Blenkisen",
            "Qorvolta",
            "Zorvaselle"
        ]
    );
    let (_d, c) = project(&[("a.md", fi)], cfg);
    assert!(spelled(&c, "a.md").is_empty(), "{:?}", spelled(&c, "a.md"));
}

#[cfg(feature = "swedish")]
#[test]
fn swedish_genitive_of_names() {
    let sv = "---\nlang: sv\n---\n\n# Teamet\n\nQorvath Blenkiss ansvarar för faktureringen. \
              Qorvaths kod granskas av Zelmira. Zelmiras kommentarer kom i går.\n";
    let cfg = format!("{PERSONS}\n[[person]]\nname = \"Zelmira Quost\"\nrole = \"Granskare\"\n");
    let (_d, c) = project(&[("a.md", sv)], "");
    let bare = spelled(&c, "a.md");
    assert!(bare.contains(&"Qorvaths".to_string()), "{bare:?}");
    let (_d, c) = project(&[("a.md", sv)], &cfg);
    assert!(spelled(&c, "a.md").is_empty(), "{:?}", spelled(&c, "a.md"));
}

#[test]
fn test_paths_accept_made_up_names() {
    let rs = "// Checked by Qorvath Blenkiss and Matti Nykänen, see Grelda's notes.\n\
              fn f() {}\n";
    let files = [
        ("tests/foo.rs", rs),
        ("src/foo_test.go", rs),
        ("docs/foo.rs", rs),
        ("qa/foo.rs", rs),
    ];
    let (_d, c) = project(&files, "");
    assert!(spelled(&c, "tests/foo.rs").is_empty());
    assert!(spelled(&c, "src/foo_test.go").is_empty());
    assert_eq!(
        spelled(&c, "docs/foo.rs"),
        ["Qorvath", "Blenkiss", "Grelda's"]
    );
    // Typos stay typos in tests.
    let (_d, c) = project(
        &[("tests/a.rs", "// Teh name is Qorvath.\nfn f() {}\n")],
        "",
    );
    assert_eq!(spelled(&c, "tests/a.rs"), ["Teh"]);
    // Configurable.
    let (_d, c) = project(&files, "[people]\ntest_paths = [\"qa/**\"]\n");
    assert!(spelled(&c, "qa/foo.rs").is_empty());
    assert_eq!(spelled(&c, "tests/foo.rs").len(), 3);
}

#[test]
fn placeholder_names() {
    let md = "# Keys\n\nAlice sends Bob a key while Mallory and Trent watch. Teppo Testaaja, \
              Maija Meikäläinen and Erika Mustermann sign it for Joe Bloggs.\n";
    let (_d, c) = project(&[("a.md", md)], "");
    assert!(spelled(&c, "a.md").is_empty(), "{:?}", spelled(&c, "a.md"));
    let (_d, c) = project(&[("a.md", md)], "[people]\nplaceholders = false\n");
    assert!(!spelled(&c, "a.md").is_empty());
}

#[test]
fn ambiguous_persons_end_to_end() {
    let md = "# Team\n\nSami Virtanen owns billing. Sami reviews it.\n\n## Design\n\n\
              Sami draws.\n";
    let rs =
        "// Sami Korhonen wrote this.\n// Sami keeps it.\nfn a() {}\n\n// Sami left.\nfn b() {}\n";
    let cfg = "[[person]]\nname = \"Sami Virtanen\"\nrole = \"Backend engineer, owns billing\"\n\n\
               [[person]]\nname = \"Sami Korhonen\"\nrole = \"Designer\"\n";
    let files = [("a.md", md), ("src/a.rs", rs), ("tests/a.rs", rs)];
    let (_d, c) = project(&files, cfg);
    let amb = |rel: &str| -> Vec<usize> {
        found(&c, rel)
            .into_iter()
            .filter(|(r, _, _)| r == "prose/ambiguous-person")
            .map(|(_, _, l)| l)
            .collect()
    };
    assert_eq!(amb("a.md"), [7]);
    // One comment block is one context; the next block starts fresh.
    assert_eq!(amb("src/a.rs"), [5]);
    assert!(amb("tests/a.rs").is_empty());
}

#[test]
fn config_placeholders_reported_on_explicit_toml() {
    let cfg = "[[person]]\nname = \"Qorvath Blenkiss\"\nrole = \"TODO\"\n\n\
               [[entity]]\nname = \"Zorbia Oy\"\nrelationship = \"Payment partner\"\n\n\
               [[vocab]]\nterm = \"Kvelmo\"\ndescription = \"...\"\n\n\
               # [[vocab]]\n# description = \"TODO\"\n";
    let (_d, c) = project(&[("a.md", "# A\n\nText.\n")], cfg);
    let got: Vec<(String, String, usize)> = found(&c, "explicit.toml")
        .into_iter()
        .filter(|(r, _, _)| r == "config/placeholder")
        .collect();
    assert_eq!(
        got,
        [
            ("config/placeholder".into(), "\"TODO\"".into(), 3),
            ("config/placeholder".into(), "\"...\"".into(), 11),
        ]
    );
    // Other TOML files are not config.
    let (_d, c) = project(&[("other.toml", "description = \"TODO\"\n")], "");
    assert!(found(&c, "other.toml").is_empty());
}
