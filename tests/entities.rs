//! Inflected `[[entity]]` and `[[vocab]]` names end to end: Finnish case endings on the last
//! word (`Rovio Entertainmentin`), abbreviations with a colon (`QXP:ssa`), the Swedish genitive,
//! in English, Finnish and Swedish text and gettext catalogs.

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

/// `(rule, flagged text, suggestions)` of `rel`'s diagnostics.
fn found(c: &Config, rel: &str) -> Vec<(String, String, Vec<String>)> {
    let path = c.root.join(rel);
    let text = std::fs::read_to_string(&path).unwrap();
    let ws = engine::build_workspace(std::slice::from_ref(&path), c);
    engine::check(&ws, &[path], c, &Options::default())
        .into_iter()
        .map(|d| {
            (
                d.rule.clone(),
                text[d.range.clone()].to_string(),
                d.suggestions.clone(),
            )
        })
        .collect()
}

fn spelled(c: &Config, rel: &str) -> Vec<String> {
    found(c, rel)
        .into_iter()
        .filter(|(r, _, _)| r == "spelling")
        .map(|(_, t, _)| t)
        .collect()
}

const ENTITIES: &str = r#"
[[entity]]
name = "Rovio Entertainment"
kind = "company"
relationship = "Game studio we license from"

[[entity]]
name = "Vexbrook Labs"
kind = "company"
relationship = "Customer"

[[entity]]
name = "Qorvia"
kind = "product"
relationship = "Partner app"

[[entity]]
name = "Zelkia Oyj"
kind = "company"
relationship = "Partner"
aliases = ["Zelkia", "Z-alusta", "QXP"]

[[vocab]]
term = "Blorp Qint"
description = "Internal tool"
"#;

#[test]
fn inflected_names_in_english_text() {
    let md = "# Notes\n\nThe Finnish page says Rovio Entertainmentin, Vexbrook Labsilla, Qorviassa, \
              Zelkian, Z-alustaan, QXP:ssa and Blorp Qintissa. Qorvia's and Vexbrook Labs' \
              offices.\n\nWrong: Blorp Qintxyz and Qorviaxyz.\n";
    let (_d, c) = project(&[("a.md", md)], "");
    let bare = spelled(&c, "a.md");
    assert!(bare.contains(&"Qintissa".to_string()), "{bare:?}");
    let (_d, c) = project(&[("a.md", md)], ENTITIES);
    // A bogus suffix is no inflection.
    assert_eq!(spelled(&c, "a.md"), ["Blorp", "Qintxyz", "Qorviaxyz"]);
}

#[cfg(feature = "voikko")]
#[test]
fn inflected_names_in_finnish_text() {
    let fi = "---\nlang: fi\n---\n\n# Muistio\n\n\
              Rovio Entertainmentin suurin huoli oli hitaat pelit.\n\n\
              Vexbrook Labsin tärkein tavoite oli nopeat julkaisut. Kävimme Vexbrook Labsilla. \
              Viesti tuli Vexbrook Labsista. Menimme Vexbrook Labsiin. Soitimme Vexbrook Labsille. \
              Olimme Vexbrook Labsissa. Toimimme Vexbrook Labsina.\n\n\
              Qorviassa ja Qorviasta. Zelkian tuote meni Z-alustaan. QXP:n tiimi. \
              Blorp Qintin avulla.\n\nQorviaxyz on väärin.\n";
    let (_d, c) = project(&[("a.md", fi)], "");
    let bare = spelled(&c, "a.md");
    // Without the config an inflected entity name is unknown; with it, only the bogus form stays.
    assert!(bare.contains(&"Zelkian".to_string()), "{bare:?}");
    let (_d, c) = project(&[("a.md", fi)], ENTITIES);
    assert_eq!(spelled(&c, "a.md"), ["Qorviaxyz"]);
}

#[cfg(feature = "swedish")]
#[test]
fn swedish_genitive_of_entities() {
    let sv = "---\nlang: sv\n---\n\n# Anteckningar\n\nQorvias inställningar och Vexbrook Labs \
              kundportal. Zelkias kontor ligger i Esbo.\n";
    let (_d, c) = project(&[("a.md", sv)], "");
    let bare = spelled(&c, "a.md");
    assert!(bare.contains(&"Qorvias".to_string()), "{bare:?}");
    let (_d, c) = project(&[("a.md", sv)], ENTITIES);
    assert!(spelled(&c, "a.md").is_empty(), "{:?}", spelled(&c, "a.md"));
}

#[test]
fn inflected_names_in_gettext_catalogs() {
    let po = |lang: &str, a: &str, b: &str| {
        format!(
            "msgid \"\"\nmsgstr \"\"\n\"Language: {lang}\\n\"\n\
             \"Content-Type: text/plain; charset=UTF-8\\n\"\n\n\
             msgid \"Vexbrook Labs customer portal\"\nmsgstr \"{a}\"\n\n\
             msgid \"Sent to Qorvia\"\nmsgstr \"{b}\"\n"
        )
    };
    let fi = po(
        "fi",
        "Vexbrook Labsin asiakasportaali",
        "Qorvialle lähetetty",
    );
    let sv = po("sv", "Vexbrook Labs kundportal", "Qorvias inställningar");
    let (_d, c) = project(&[("po/fi.po", &fi), ("po/sv.po", &sv)], ENTITIES);
    assert!(
        spelled(&c, "po/fi.po").is_empty(),
        "{:?}",
        spelled(&c, "po/fi.po")
    );
    assert!(
        spelled(&c, "po/sv.po").is_empty(),
        "{:?}",
        spelled(&c, "po/sv.po")
    );
}

#[test]
fn entity_name_casing_keeps_the_ending() {
    let md = "# Notes\n\nThe team at rovio entertainmentin office and vexbrook labsilla. \
              Rovio Entertainmentin and ROVIO ENTERTAINMENTIN are fine.\n";
    let (_d, c) = project(&[("a.md", md)], ENTITIES);
    let got: Vec<(String, Vec<String>)> = found(&c, "a.md")
        .into_iter()
        .filter(|(r, _, _)| r == "prose/entity-name")
        .map(|(_, t, s)| (t, s))
        .collect();
    assert_eq!(
        got,
        [
            (
                "rovio entertainmentin".to_string(),
                vec!["Rovio Entertainmentin".to_string()]
            ),
            (
                "vexbrook labsilla".to_string(),
                vec!["Vexbrook Labsilla".to_string()]
            ),
        ]
    );
}
