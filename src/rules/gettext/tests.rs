use std::path::PathBuf;

use crate::config::Config;
use crate::diagnostic::{Finding, Severity};
use crate::rules::Analyzed;
use crate::source::{FileKind, SourceFile};

fn analyzed(path: &str, src: &str) -> Analyzed {
    let p = PathBuf::from(path);
    let kind = FileKind::detect(&p).unwrap();
    Analyzed::new(SourceFile::new(p.clone(), p, kind, src.to_string()))
}

/// All findings (gettext and prose) with every gettext rule on.
fn all(path: &str, src: &str) -> Vec<Finding> {
    let config: Config =
        toml::from_str("[rules]\n\"gettext/plural-placeholder\" = \"info\"\n").unwrap();
    crate::engine::local_findings(&analyzed(path, src), &config)
}

fn run(path: &str, src: &str) -> Vec<Finding> {
    all(path, src)
        .into_iter()
        .filter(|f| f.rule.starts_with("gettext/"))
        .collect()
}

fn rule<'a>(f: &'a [Finding], id: &str) -> Vec<&'a Finding> {
    f.iter().filter(|f| f.rule == id).collect()
}

fn show(src: &str, f: &[Finding]) -> Vec<String> {
    f.iter()
        .map(|f| format!("{} {:?} {}", f.rule, &src[f.range.clone()], f.message))
        .collect()
}

const FI_HEADER: &str = "msgid \"\"\nmsgstr \"\"\n\"Language: fi\\n\"\n\"Content-Type: text/plain; charset=UTF-8\\n\"\n\"Plural-Forms: nplurals=2; plural=(n != 1);\\n\"\n\n";

fn fi(body: &str) -> String {
    format!("{FI_HEADER}{body}")
}

const FI_PATH: &str = "priv/gettext/fi/LC_MESSAGES/default.po";

#[test]
fn clean_elixir_catalog() {
    let src = fi(concat!(
        "#: lib/app_web/live/page.ex:12\n#, elixir-autogen, elixir-format\n",
        "msgid \"One file\"\nmsgid_plural \"%{count} files\"\n",
        "msgstr[0] \"Yksi tiedosto\"\nmsgstr[1] \"%{count} tiedostoa\"\n\n",
        "#, elixir-autogen, elixir-format\nmsgid \"Hello %{name}!\"\nmsgstr \"Hei %{name}!\"\n",
    ));
    let f = run(FI_PATH, &src);
    // msgstr[0] omitting %{count} is fine by default (plural-placeholder is off).
    assert_eq!(
        show(&src, &f),
        [format!(
            "gettext/plural-placeholder {:?} Translation is missing placeholder %{{count}}",
            "msgstr[0] \"Yksi tiedosto\""
        )]
    );
}

#[test]
fn elixir_placeholder_mismatch() {
    let src = fi(concat!(
        "#, elixir-format\nmsgid \"%{count} new messages from %{user}\"\n",
        "msgstr \"%{count} uutta viestiä käyttäjältä %{käyttäjä}\"\n\n",
        "#, elixir-format\nmsgid \"%{count} item\"\nmsgid_plural \"%{count} items\"\n",
        "msgstr[0] \"%{count} kohde\"\nmsgstr[1] \"kohdetta %{n}\"\n",
    ));
    let f = run(FI_PATH, &src);
    let p = rule(&f, "gettext/placeholders");
    assert_eq!(p.len(), 3, "{:?}", show(&src, &f));
    // Extra placeholder points exactly at itself.
    assert!(
        p.iter().any(|f| &src[f.range.clone()] == "%{n}"),
        "{:?}",
        show(&src, &f)
    );
    assert!(p.iter().any(|f| f.message.contains("%{user}")));
    // `%{käyttäjä}` is not a word placeholder (\w is Unicode, so it is one) and not in source.
    assert!(p.iter().any(|f| &src[f.range.clone()] == "%{käyttäjä}"));
}

#[test]
fn c_format_order_and_types() {
    let src = fi(concat!(
        "#, c-format\nmsgid \"%s has %d files\"\nmsgstr \"%2$d tiedostoa: %1$s\"\n\n",
        "#, c-format\nmsgctxt \"b\"\nmsgid \"%s has %d files\"\nmsgstr \"%d tiedostoa: %s\"\n\n",
        "#, c-format\nmsgctxt \"c\"\nmsgid \"100%% done\"\nmsgstr \"100 % valmis\"\n",
    ));
    let f = run(FI_PATH, &src);
    let p = rule(&f, "gettext/placeholders");
    // Positional reorder is fine; swapped unnumbered types are two mismatches (extra %d at
    // 1, extra %s at 2) plus a missing report.
    assert!(
        !p.iter().any(|f| src[f.range.clone()].contains("%2$d")),
        "{:?}",
        show(&src, &f)
    );
    assert!(
        p.iter().any(|f| &src[f.range.clone()] == "%d"),
        "{:?}",
        show(&src, &f)
    );
    // `% v` in the last translation is a printf conversion only with the space flag… `% v`
    // is not a valid conversion, so nothing is reported there.
    assert!(!p.iter().any(|f| src[f.range.clone()].contains("valmis")));
}

#[test]
fn python_format() {
    let src = fi(concat!(
        "#, python-format\nmsgid \"%(name)s logged in\"\nmsgstr \"%(nimi)s kirjautui\"\n\n",
        "#, python-brace-format\nmsgid \"{name} logged out\"\nmsgstr \"{name} kirjautui ulos\"\n",
    ));
    let f = run(FI_PATH, &src);
    let p = rule(&f, "gettext/placeholders");
    assert_eq!(p.len(), 2, "{:?}", show(&src, &f));
    assert!(p.iter().any(|f| &src[f.range.clone()] == "%(nimi)s"));
}

#[test]
fn icu_plural() {
    let src = fi(concat!(
        "msgid \"{count, plural, one {# file by {user}} other {# files by {user}}}\"\n",
        "msgstr \"{count, plural, one {# tiedosto} other {# tiedostoa käyttäjältä {user}}}\"\n\n",
        "msgid \"Hi {name}\"\nmsgstr \"Hei {nimi}\"\n",
    ));
    let f = run(FI_PATH, &src);
    let p = rule(&f, "gettext/placeholders");
    assert_eq!(p.len(), 2, "{:?}", show(&src, &f));
    assert!(p.iter().any(|f| &src[f.range.clone()] == "{nimi}"));
}

#[test]
fn plural_count_and_header_plural_forms() {
    let src = fi(concat!(
        "msgid \"One file\"\nmsgid_plural \"%d files\"\n",
        "msgstr[0] \"Yksi\"\nmsgstr[1] \"%d\"\nmsgstr[2] \"%d x\"\n",
    ));
    let f = run(FI_PATH, &src);
    assert_eq!(
        rule(&f, "gettext/plural-count").len(),
        1,
        "{:?}",
        show(&src, &f)
    );
    let bad = src.replace("plural=(n != 1)", "plural=(n > 1 ? 2 : 0)");
    let f = run(FI_PATH, &bad);
    let h = rule(&f, "gettext/header");
    assert!(
        h.iter().any(|f| f.message.contains("gives form 2")),
        "{:?}",
        show(&bad, &f)
    );
    let bad = src.replace("plural=(n != 1)", "plural=(n != 1");
    let f = run(FI_PATH, &bad);
    assert!(
        rule(&f, "gettext/header")
            .iter()
            .any(|f| f.message.starts_with("Invalid Plural-Forms"))
    );
}

#[test]
fn header_problems() {
    // No header at all.
    let src = "msgid \"a\"\nmsgstr \"b\"\n";
    let f = run(FI_PATH, src);
    assert_eq!(rule(&f, "gettext/header").len(), 1);
    // Language mismatch, non-UTF-8, plural entries without Plural-Forms.
    let src = "msgid \"\"\nmsgstr \"\"\n\"Language: sv\\n\"\n\"Content-Type: text/plain; charset=ISO-8859-1\\n\"\n\nmsgid \"a\"\nmsgid_plural \"as\"\nmsgstr[0] \"b\"\nmsgstr[1] \"bs\"\n";
    let f = run("locale/fi.po", src);
    let h = rule(&f, "gettext/header");
    let msgs: Vec<&str> = h.iter().map(|f| f.message.as_str()).collect();
    assert_eq!(h.len(), 3, "{msgs:?}");
    assert!(h.iter().all(|f| f.severity == Severity::Warning));
    let lang = h
        .iter()
        .find(|f| f.message.contains("path locale fi"))
        .unwrap();
    assert_eq!(&src[lang.range.clone()], "Language: sv");
    // Missing Language and the `CHARSET` placeholder charset in a PO file.
    let src = "msgid \"\"\nmsgstr \"Content-Type: text/plain; charset=CHARSET\\n\"\n";
    let f = run("x/fi/LC_MESSAGES/a.po", src);
    let h = rule(&f, "gettext/header");
    assert_eq!(h.len(), 2, "{:?}", show(src, &f));
    // The same header is fine in a template.
    assert!(run("x/a.pot", src).is_empty());
    // fi_FI in the path matches fi.
    assert!(
        !run("x/fi_FI/LC_MESSAGES/a.po", &fi(""))
            .iter()
            .any(|f| f.rule == "gettext/header")
    );
}

#[test]
fn syntax_and_duplicates() {
    let src = fi(concat!(
        "msgid \"a\"\nmsgstr \"b\\q\"\n\n",
        "msgid \"a\"\nmsgstr \"c\"\n\n",
        "msgctxt \"menu\"\nmsgid \"a\"\nmsgstr \"d\"\n\n",
        "msgid \"x\"\nmsgid_plural \"xs\"\n\n",
        "msgid \"open\nmsgstr \"auki\"\n",
        "bogus \"z\"\n",
    ));
    let f = run(FI_PATH, &src);
    let d = rule(&f, "gettext/duplicate");
    assert_eq!(d.len(), 1, "{:?}", show(&src, &f));
    assert!(d[0].help.as_deref().unwrap().contains("line 7"));
    let s = rule(&f, "gettext/syntax");
    let msgs: Vec<&str> = s.iter().map(|f| f.message.as_str()).collect();
    assert!(msgs.contains(&"msgid_plural without msgstr[N]"), "{msgs:?}");
    assert!(msgs.contains(&"unterminated string"), "{msgs:?}");
    assert!(msgs.contains(&"unknown keyword `bogus`"), "{msgs:?}");
    let esc = s
        .iter()
        .find(|f| f.message.starts_with("unknown escape"))
        .unwrap();
    assert_eq!(&src[esc.range.clone()], "\\q");
    assert!(s.iter().all(|f| f.severity == Severity::Error));
}

#[test]
fn states_untranslated_fuzzy_obsolete_same() {
    let src = fi(concat!(
        "msgid \"Save\"\nmsgstr \"\"\n\n",
        "#, fuzzy\nmsgid \"Open the file\"\nmsgstr \"Avaa\"\n\n",
        "msgid \"Please restart the application now.\"\nmsgstr \"Please restart the application now.\"\n\n",
        "msgid \"Acme Platform Suite\"\nmsgstr \"Acme Platform Suite\"\n\n",
        "#~ msgid \"Old\"\n#~ msgstr \"Vanha\"\n",
    ));
    let f = run(FI_PATH, &src);
    assert_eq!(rule(&f, "gettext/untranslated").len(), 1);
    let fz = rule(&f, "gettext/fuzzy");
    assert_eq!(fz.len(), 1);
    assert_eq!(&src[fz[0].range.clone()], "fuzzy");
    assert_eq!(rule(&f, "gettext/obsolete").len(), 1);
    let same = rule(&f, "gettext/same-as-source");
    assert_eq!(same.len(), 1, "{:?}", show(&src, &f));
    // An English catalog may repeat its source.
    let en = src.replace("Language: fi", "Language: en");
    let f = run("priv/gettext/en/LC_MESSAGES/default.po", &en);
    assert!(rule(&f, "gettext/same-as-source").is_empty());
}

#[test]
fn template_skips_translation_checks() {
    let src = "#, fuzzy\nmsgid \"\"\nmsgstr \"\"\n\"Content-Type: text/plain; charset=CHARSET\\n\"\n\"Plural-Forms: nplurals=INTEGER; plural=EXPRESSION;\\n\"\n\n#, elixir-format\nmsgid \"%{count} file\"\nmsgid_plural \"%{count} files\"\nmsgstr[0] \"\"\nmsgstr[1] \"\"\n";
    assert!(
        run("priv/gettext/default.pot", src).is_empty(),
        "{:?}",
        show(src, &run("priv/gettext/default.pot", src))
    );
}

#[test]
fn markup_whitespace_punctuation() {
    let src = fi(concat!(
        "msgid \"Click <b>here</b> to see [docs](https://x.y) and `mix`\\n\"\n",
        "msgstr \"Klikkaa <strong>tästä</strong> nähdäksesi ohjeet\"\n\n",
        "msgid \" Name:\"\nmsgstr \"Nimi\"\n",
    ));
    let f = run(FI_PATH, &src);
    let m = rule(&f, "gettext/markup");
    assert_eq!(m.len(), 1, "{:?}", show(&src, &f));
    assert!(
        m[0].message.contains("tags differ")
            && m[0].message.contains("link")
            && m[0].message.contains("code span")
    );
    let w = rule(&f, "gettext/whitespace");
    assert_eq!(w.len(), 2, "{:?}", show(&src, &f));
    let p = rule(&f, "gettext/punctuation");
    assert_eq!(p.len(), 1, "{:?}", show(&src, &f));
    assert_eq!(p[0].severity, Severity::Info);
    // Full-width punctuation matches.
    let zh = "msgid \"\"\nmsgstr \"Language: zh_CN\\nContent-Type: text/plain; charset=UTF-8\\n\"\n\nmsgid \"Done.\"\nmsgstr \"完成。\"\n";
    assert!(
        run("zh_CN.po", zh).is_empty(),
        "{:?}",
        show(zh, &run("zh_CN.po", zh))
    );
}

#[test]
fn accelerators_only_when_used() {
    let body = concat!(
        "msgid \"&File\"\nmsgstr \"&Tiedosto\"\n\n",
        "msgid \"&Edit\"\nmsgstr \"&Muokkaa\"\n\n",
        "msgid \"&View\"\nmsgstr \"Näytä\"\n\n",
        "msgid \"Tom &amp; Jerry\"\nmsgstr \"Tom &amp; Jerry\"\n",
    );
    let src = fi(body);
    let f = run(FI_PATH, &src);
    let a = rule(&f, "gettext/accelerator");
    assert_eq!(a.len(), 1, "{:?}", show(&src, &f));
    // Two msgids with & are not enough to call it an accelerator catalog.
    let src = fi(&body.replace("msgid \"&File\"", "msgid \"File\""));
    assert!(rule(&run(FI_PATH, &src), "gettext/accelerator").is_empty());
}

#[test]
fn crlf_utf8_multiline_ranges() {
    let src = fi(concat!(
        "msgid \"\"\n\"Päivitä sovellus \"\n\"%{count} kertaa\"\n",
        "msgstr \"\"\n\"Uppdatera \"\n\"%{räkning} gånger\"\n",
    ))
    .replace('\n', "\r\n");
    let f = run(FI_PATH, &src);
    let p = rule(&f, "gettext/placeholders");
    assert!(
        p.iter().any(|f| &src[f.range.clone()] == "%{räkning}"),
        "{:?}",
        show(&src, &f)
    );
    assert!(
        p.iter().any(|f| f.message.contains("%{count}")),
        "{:?}",
        show(&src, &f)
    );
    assert!(rule(&f, "gettext/syntax").is_empty());
}

#[test]
fn msgids_and_translations_are_prose() {
    let src = fi(concat!(
        "# Translators: keep it shrot\n",
        "#: lib/foo_widget.ex:3\n",
        "msgid \"\"\n\"Could not recieve \"\n\"the %{count} mesages\"\n",
        "msgstr \"Ei voitu vastaanottaa %{count} viestiä\"\n\n",
        "msgid \"shared.rendering.localizedFailure\"\nmsgstr \"Sivua ei voitu näyttää\"\n",
    ));
    let a = analyzed(FI_PATH, &src);
    // Two msgid pieces in one segment, placeholders blanked; key msgids skipped; Finnish
    // translations are prose in a Finnish region.
    let texts: Vec<&str> = a.segments.iter().map(|s| s.text.as_str()).collect();
    assert!(
        texts.iter().any(|t| t.contains("Could not recieve")
            && t.contains("mesages")
            && !t.contains("count")),
        "{texts:?}"
    );
    assert!(
        !texts.iter().any(|t| t.contains("localizedFailure")),
        "{texts:?}"
    );
    assert!(
        texts
            .iter()
            .any(|t| t.contains("Sivua ei voitu") && !t.contains("count")),
        "{texts:?}"
    );
    let regions = crate::lang_marks::regions(&a);
    assert_eq!(regions.len(), 2, "{regions:?}");
    assert!(
        regions
            .iter()
            .all(|(r, l)| l == "fi" && src[r.clone()].contains("voitu"))
    );
    assert!(texts.iter().any(|t| t.contains("shrot")), "{texts:?}");
    let f = all(FI_PATH, &src);
    let spelled: Vec<&str> = f
        .iter()
        .filter(|f| f.rule == "spelling")
        .map(|f| &src[f.range.clone()])
        .collect();
    for w in ["recieve", "mesages"] {
        assert!(spelled.contains(&w), "{w} not in {spelled:?}");
    }
    assert!(
        !spelled
            .iter()
            .any(|w| w.contains("voitu") || w.contains("widget")),
        "{spelled:?}"
    );
}

#[test]
fn english_catalog_checks_msgstr() {
    let src = "msgid \"\"\nmsgstr \"Language: en\\nContent-Type: text/plain; charset=UTF-8\\n\"\n\nmsgid \"auth.login.title\"\nmsgstr \"Plese log in\"\n";
    let a = analyzed("catalogs/en/messages.po", src);
    assert!(a.segments.iter().any(|s| s.text.contains("Plese")));
}

#[test]
fn suppression_in_translator_comment() {
    let src =
        fi("# explicit-disable-next-line gettext/untranslated\nmsgid \"Save\"\nmsgstr \"\"\n");
    let a = analyzed(FI_PATH, &src);
    let d = crate::engine::local_diagnostics(&a, &Config::default());
    assert!(!d.iter().any(|d| d.rule == "gettext/untranslated"), "{d:?}");
}

#[test]
fn multibyte_text_is_not_blanked() {
    let src = fi("msgid \"We couldn’t reach Laatupäivystys\\n\"\nmsgstr \"Ei yhteyttä\\n\"\n");
    let a = analyzed(FI_PATH, &src);
    let t = &a.segments[0].text;
    assert!(
        t.contains("couldn’t reach Laatupäivystys") && !t.contains("\\n"),
        "{t:?}"
    );
}

#[test]
fn template_owns_msgid_prose() {
    let dir = tempfile::tempdir().unwrap();
    let po_dir = dir.path().join("fi/LC_MESSAGES");
    std::fs::create_dir_all(&po_dir).unwrap();
    std::fs::write(dir.path().join("default.pot"), "msgid \"x\"\nmsgstr \"\"\n").unwrap();
    let src = fi("#. A dev nnote\nmsgid \"Recieve it\"\nmsgstr \"Vastaanota\"\n");
    let po = po_dir.join("default.po");
    let a = Analyzed::new(SourceFile::new(
        po.clone(),
        po,
        FileKind::Gettext,
        src.clone(),
    ));
    // Only the translation: msgids and extracted comments belong to the template.
    let texts: Vec<&str> = a.segments.iter().map(|s| s.text.as_str()).collect();
    assert_eq!(texts, ["Vastaanota"]);
    // Without the template the PO file's msgids are checked.
    std::fs::remove_file(dir.path().join("default.pot")).unwrap();
    let po = po_dir.join("default.po");
    let a = Analyzed::new(SourceFile::new(po.clone(), po, FileKind::Gettext, src));
    assert_eq!(a.segments.len(), 3);
}

#[test]
fn elixir_double_hash_comments() {
    let src = "## From Ecto.Changeset.cast/4\nmsgid \"can't be blank\"\nmsgstr \"\"\n";
    let a = analyzed("priv/gettext/errors.pot", src);
    let c = &a.comments[0];
    assert_eq!(
        &src[c.lines[0].content.clone()],
        "From Ecto.Changeset.cast/4"
    );
    let f = run("priv/gettext/errors.pot", src);
    assert_eq!(f.len(), 1);
    assert_eq!(f[0].severity, Severity::Warning);
}

#[test]
fn bad_escape_keeps_value_and_bad_header() {
    let src = "msgid \"\"\nmsgstr \"\"\n\"Language: en\\n\"\n\nmsgid \"\\q\"\nmsgstr \"x\"\n";
    let f = run("en.po", src);
    assert!(
        rule(&f, "gettext/duplicate").is_empty(),
        "{:?}",
        show(src, &f)
    );
    let src = "msgid \"\"\nmsgstr \"not-a-header\"\n";
    let f = run("en.po", src);
    assert!(
        rule(&f, "gettext/header")
            .iter()
            .any(|f| f.message.contains("Name: value"))
    );
}

/// Words with spelling findings in a catalog, in file order.
#[cfg(any(feature = "swedish", feature = "voikko"))]
fn spelled(path: &str, src: &str) -> Vec<String> {
    let mut f: Vec<Finding> = all(path, src)
        .into_iter()
        .filter(|f| f.rule == "spelling")
        .collect();
    f.sort_by_key(|f| f.range.start);
    f.iter().map(|f| src[f.range.clone()].to_string()).collect()
}

#[cfg(feature = "swedish")]
#[test]
fn swedish_translations_get_swedish_spelling() {
    let src = concat!(
        "msgid \"\"\nmsgstr \"\"\n\"Language: sv_SE\\n\"\n\n",
        "msgid \"Could not save the %{name} file\"\n",
        "msgstr \"Det gick inte att spara filen %{name} i systemt\"\n\n",
        "msgid \"Recieve\"\nmsgstr \"Ta emot\"\n",
    );
    // Language from the header, placeholders blanked; the English msgid keeps English spelling.
    assert_eq!(spelled("po/messages.po", src), ["systemt", "Recieve"]);
    // Language from the path.
    let src = src.replace("\"Language: sv_SE\\n\"\n", "");
    assert_eq!(
        spelled("locale/sv/LC_MESSAGES/app.po", &src),
        ["systemt", "Recieve"]
    );
}

#[cfg(feature = "voikko")]
#[test]
fn finnish_translations_get_finnish_spelling() {
    let src =
        fi("msgid \"Could not recieve the messages\"\nmsgstr \"Viestejä ei voitu vastanottaa\"\n");
    assert_eq!(spelled(FI_PATH, &src), ["recieve", "vastanottaa"]);
}
