// explicit-disable-file slop/* prose/* grammar/* -- examples of the patterns this module detects
use std::path::Path;

use super::file::{fk_grade, syllables};
use super::lists::{INCLUSIVE, SIMPLIFY, TERMINOLOGY};
use super::*;
use crate::config::Config;
use crate::diagnostic::Finding;
use crate::rules::Analyzed;
use crate::source::{FileKind, SourceFile};

/// Run the prose rules on `src` (file name decides Markdown vs code) with extra config TOML.
fn run_cfg(name: &str, src: &str, cfg: &str) -> Vec<Finding> {
    let config: Config = toml::from_str(cfg).unwrap();
    let a = Analyzed::new(SourceFile::new(
        name.into(),
        name.into(),
        FileKind::detect(Path::new(name)).unwrap(),
        src.into(),
    ));
    let mut out = Vec::new();
    check(
        &FileCtx {
            a: &a,
            config: &config,
        },
        &mut out,
    );
    out
}

/// Findings of `rule` only, with that rule switched on.
fn run(rule: &str, src: &str) -> Vec<Finding> {
    run_file(rule, "a.md", src)
}

fn run_file(rule: &str, name: &str, src: &str) -> Vec<Finding> {
    run_cfg(name, src, &format!("[rules]\n\"{rule}\" = \"warning\"\n"))
        .into_iter()
        .filter(|f| f.rule == rule)
        .collect()
}

fn text<'a>(src: &'a str, f: &Finding) -> &'a str {
    &src[f.range.clone()]
}

fn apply(src: &str, f: &Finding) -> String {
    let fix = f.fix.as_ref().expect("fix");
    format!(
        "{}{}{}",
        &src[..fix.range.start],
        fix.replacement,
        &src[fix.range.end..]
    )
}

#[test]
fn lists_load() {
    assert!(INCLUSIVE.entries.len() >= 60);
    assert!(SIMPLIFY.entries.len() >= 80);
    assert!(TERMINOLOGY.entries.len() >= 80);
}

/// Nothing here may duplicate the slop catalogue.
#[test]
fn no_overlap_with_slop_catalogue() {
    let slop: toml::Value = toml::from_str(include_str!("../slop/phrases.toml")).unwrap();
    let mut slop_words = std::collections::HashSet::new();
    for p in slop["phrase"].as_array().unwrap() {
        match p.get("match") {
            Some(toml::Value::String(s)) => {
                slop_words.insert(s.to_lowercase());
            }
            Some(toml::Value::Array(a)) => {
                slop_words.extend(a.iter().filter_map(|v| v.as_str()).map(str::to_lowercase));
            }
            _ => {}
        }
    }
    for list in [
        include_str!("inclusive.toml"),
        include_str!("simplify.toml"),
    ] {
        let v: toml::Value = toml::from_str(list).unwrap();
        for e in v["entry"].as_array().unwrap() {
            for m in e["match"].as_array().unwrap() {
                let m = m.as_str().unwrap().to_lowercase();
                assert!(
                    !slop_words.contains(&m),
                    "{m:?} is already in the slop catalogue"
                );
            }
        }
    }
    for w in [
        "very", "really", "quite", "fairly", "rather", "several", "mostly", "largely",
    ] {
        assert!(
            !slop_words.contains(w),
            "weasel {w:?} is in the slop catalogue"
        );
    }
}

#[test]
fn sentence_split() {
    let t = "See e.g. this one. Next one!  J. Smith is here.\n\nNew para";
    let s: Vec<&str> = sentences(t).into_iter().map(|r| &t[r]).collect();
    assert_eq!(
        s,
        [
            "See e.g. this one.",
            "Next one!",
            "J. Smith is here.",
            "New para"
        ]
    );
}

// prose/inclusive

#[test]
fn inclusive_flags_with_case() {
    let src = "Add it to the whitelist. Guys, check the slave nodes.\n";
    let f = run("prose/inclusive", src);
    assert_eq!(f.len(), 3, "{f:?}");
    assert_eq!(text(src, &f[0]), "whitelist");
    assert_eq!(f[0].suggestions[0], "allowlist");
    assert_eq!(f[1].suggestions[0], "Folks");
    assert!(f.iter().all(|f| f.fix.is_none()));
}

#[test]
fn inclusive_exceptions() {
    let src = "She has a master's degree and mastery of the Master of Arts. \
               See my_master and github.com/whitelist too.\n";
    assert!(run("prose/inclusive", src).is_empty());
    let src = "Merge into the master branch.\n";
    let f = run("prose/inclusive", src);
    assert_eq!(f.len(), 1);
    assert_eq!(text(src, &f[0]), "master branch");
    assert_eq!(f[0].suggestions[0], "main branch");
}

// prose/simplify

#[test]
fn simplify_fix_preserves_case() {
    let src = "Käyttäjä: In order to build, run it prior to deploy.\n";
    let f = run("prose/simplify", src);
    assert_eq!(f.len(), 2, "{f:?}");
    assert_eq!(text(src, &f[0]), "In order to");
    assert_eq!(
        apply(src, &f[0]),
        "Käyttäjä: To build, run it prior to deploy.\n"
    );
    assert_eq!(
        apply(src, &f[1]),
        "Käyttäjä: In order to build, run it before deploy.\n"
    );
}

#[test]
fn simplify_suggestion_without_fix() {
    let src = "We commence at noon.\n";
    let f = run("prose/simplify", src);
    assert_eq!(f.len(), 1);
    assert_eq!(f[0].suggestions, ["start", "begin"]);
    assert!(f[0].fix.is_none());
}

#[test]
fn simplify_skips_quotes_and_markup_fix() {
    assert!(run("prose/simplify", "> In order to win, you must play.\n").is_empty());
    let src = "Do it in **order** to win.\n";
    let f = run("prose/simplify", src);
    assert_eq!(f.len(), 1);
    assert!(f[0].fix.is_none(), "fix would eat the ** markers");
}

// prose/terminology

#[test]
fn terminology_fixes() {
    let src = "Äö: Use Javascript on Github and MacOS.\n";
    let f = run("prose/terminology", src);
    assert_eq!(f.len(), 3, "{f:?}");
    assert_eq!(text(src, &f[0]), "Javascript");
    assert_eq!(
        apply(src, &f[0]),
        "Äö: Use JavaScript on Github and MacOS.\n"
    );
    assert_eq!(f[1].fix.as_ref().unwrap().replacement, "GitHub");
    assert_eq!(f[2].fix.as_ref().unwrap().replacement, "macOS");
}

#[test]
fn terminology_negative() {
    let src = "JavaScript on GitHub. Clone github.com/Github/x. `Javascript` in code. <https://Github.com>\n";
    assert!(run("prose/terminology", src).is_empty());
}

#[test]
fn terminology_in_comments() {
    let src = "// Parses Json from Github.\nfn main() { let javascript = 1; }\n";
    let f = run_file("prose/terminology", "a.rs", src);
    assert_eq!(f.len(), 2, "{f:?}");
    assert_eq!(text(src, &f[0]), "Json");
}

// prose/passive

#[test]
fn passive() {
    let src = "The file was written by Bob. It is not used. The flag was set.\n";
    let f = run("prose/passive", src);
    let hits: Vec<&str> = f.iter().map(|f| text(src, f)).collect();
    assert_eq!(hits, ["was written", "is not used"]);
    assert!(
        run(
            "prose/passive",
            "The need is real. It was red. We write it.\n"
        )
        .is_empty()
    );
    assert!(run("prose/passive", "> It was written long ago.\n").is_empty());
    assert!(
        run("prose/passive", "The file was written.\n")[0]
            .fix
            .is_none()
    );
}

#[test]
fn passive_off_by_default() {
    assert!(run_cfg("a.md", "It was written.\n", "").is_empty());
}

// prose/weasel

#[test]
fn weasel() {
    let src = "This is very fast and some people say it is quite good.\n";
    let f = run("prose/weasel", src);
    let hits: Vec<&str> = f.iter().map(|f| text(src, f)).collect();
    assert_eq!(hits, ["very", "some people say", "quite"]);
    assert!(run("prose/weasel", "> This is very fast.\n").is_empty());
    assert!(run("prose/weasel", "Every query is fast.\n").is_empty());
}

// prose/there-is

#[test]
fn there_is() {
    let src = "There are three modes. Done. There is more.\n";
    let f = run("prose/there-is", src);
    assert_eq!(f.len(), 2);
    assert_eq!(text(src, &f[0]), "There are");
    assert_eq!(text(src, &f[1]), "There is");
    assert!(
        run(
            "prose/there-is",
            "Is there a way? Here there is none. Therefore ok.\n"
        )
        .is_empty()
    );
}

// prose/so-start

#[test]
fn so_start_suggestion_only() {
    // No autofix: deleting "So" drops the causal link.
    let src = "So the cache is cold.\n";
    let f = run("prose/so-start", src);
    assert_eq!(f.len(), 1);
    assert_eq!(text(src, &f[0]), "So");
    assert!(f[0].fix.is_none());
    assert!(f[0].help.is_some());

    let src = "Ok. So, ärrä works.\n";
    let f = run("prose/so-start", src);
    assert_eq!(f.len(), 1);
    assert!(f[0].fix.is_none());
}

#[test]
fn so_start_negative() {
    assert!(
        run(
            "prose/so-start",
            "So far so good. It is so fast. So-called fix.\n"
        )
        .is_empty()
    );
    let src = "So **we** added it.\n";
    let f = run("prose/so-start", src);
    assert_eq!(f.len(), 1);
    assert!(f[0].fix.is_none());
}

// prose/sentence-length

#[test]
fn sentence_length() {
    let cfg = "[rules]\n\"prose/sentence-length\" = \"info\"\n[prose]\nmax_sentence_words = 5\n";
    let src = "Short one here. This sentence has far more than five words in it.\n";
    let f = run_cfg("a.md", src, cfg);
    assert_eq!(f.len(), 1, "{f:?}");
    assert!(text(src, &f[0]).starts_with("This sentence"));
    assert!(text(src, &f[0]).ends_with("in it."));
}

// prose/readability

#[test]
fn syllable_heuristic() {
    assert_eq!(syllables("cat"), 1);
    assert_eq!(syllables("make"), 1);
    assert_eq!(syllables("table"), 2);
    assert_eq!(syllables("readability"), 5);
    assert_eq!(syllables("used"), 1);
    assert_eq!(syllables("wanted"), 2);
    assert_eq!(syllables("makes"), 1);
    assert_eq!(syllables("uses"), 2);
    assert!(fk_grade(100, 10, 130) < 8.0);
}

#[test]
fn readability() {
    let hard = "Organizational interoperability necessitates comprehensive institutional \
                considerations regarding administrative responsibilities and infrastructural \
                dependencies. ";
    let easy = "The cat sat on the mat and it was a good day for us all. ";
    let hard_src = format!(
        "# Hard\n\n{}\n\n# Easy\n\n{}\n",
        hard.repeat(10),
        easy.repeat(10)
    );
    let f = run("prose/readability", &hard_src);
    assert_eq!(f.len(), 1, "{f:?}");
    assert!(text(&hard_src, &f[0]).starts_with("Organizational"));
    // Short sections are skipped.
    assert!(run("prose/readability", &format!("{hard}\n")).is_empty());
    // Comments are not checked.
    let code = format!("// {}\nfn x() {{}}\n", hard.repeat(10));
    assert!(run_file("prose/readability", "a.rs", &code).is_empty());
}

// prose/consistency

#[test]
fn consistency() {
    let src = "Pick a color. Another color. The colour is off.\n";
    let f = run("prose/consistency", src);
    assert_eq!(f.len(), 1, "{f:?}");
    assert_eq!(text(src, &f[0]), "colour");
    assert_eq!(f[0].suggestions[0], "color");
    assert!(run("prose/consistency", "A colour and a colour.\n").is_empty());
}

#[test]
fn consistency_ok_case_sensitive_and_terminology_skip() {
    let src = "OK. OK then. Okay.\n";
    let f = run("prose/consistency", src);
    assert_eq!(f.len(), 1);
    assert_eq!(f[0].suggestions[0], "OK");
    // e-mail is reported by prose/terminology (on by default), so consistency skips it.
    let src = "Send an email. Another email. An e-mail.\n";
    assert!(run("prose/consistency", src).is_empty());
}

// prose/acronym-defined

#[test]
fn acronym_defined() {
    let src = "We track the SLO. Later the SLO again. The API is fine. DO NOT EDIT.\n";
    let f = run("prose/acronym-defined", src);
    assert_eq!(f.len(), 1, "{f:?}");
    assert_eq!(text(src, &f[0]), "SLO");
    assert_eq!(f[0].range.start, src.find("SLO").unwrap());

    let ok = "A Service Level Objective (SLO) matters. The SLO holds. \
              Use MTTR (mean time to repair) and MTTRs.\n";
    assert!(run("prose/acronym-defined", ok).is_empty());
}

// prose/smart-quotes

#[test]
fn smart_quotes() {
    let src = "He said \"hi\" and “bye” then \"ok\".\n";
    let f = run("prose/smart-quotes", src);
    assert_eq!(f.len(), 2, "{f:?}");
    assert_eq!(text(src, &f[0]), "“");
    assert_eq!(
        apply(src, &f[0]),
        "He said \"hi\" and \"bye” then \"ok\".\n"
    );
    assert!(run("prose/smart-quotes", "Only “curly” here.\n").is_empty());
}

// prose/sentence-spacing

#[test]
fn sentence_spacing() {
    let src = "Ää one.  Two here.\n";
    let f = run("prose/sentence-spacing", src);
    assert_eq!(f.len(), 1);
    assert_eq!(apply(src, &f[0]), "Ää one. Two here.\n");
    assert!(run("prose/sentence-spacing", "End. `code` Next one. Fine.\n").is_empty());
}

#[test]
fn terminology_yields_to_harper() {
    let src = "Use Javascript and Github.\n";
    let config: Config = toml::from_str("").unwrap();
    let a = Analyzed::new(SourceFile::new(
        "a.md".into(),
        "a.md".into(),
        FileKind::Markdown,
        src.into(),
    ));
    let start = src.find("Github").unwrap();
    let mut out = vec![Finding::new(
        "grammar/OrthographicConsistency",
        crate::diagnostic::Severity::Warning,
        start..start + 6,
        "casing",
    )];
    check(
        &FileCtx {
            a: &a,
            config: &config,
        },
        &mut out,
    );
    let term: Vec<&str> = out
        .iter()
        .filter(|f| f.rule == "prose/terminology")
        .map(|f| text(src, f))
        .collect();
    assert_eq!(term, ["Javascript"]);
}

#[test]
fn no_fixes_in_headings() {
    // Rewriting heading text changes its anchor and breaks links; suggestions only.
    let f = run("prose/terminology", "# Using Github\n");
    assert_eq!(f.len(), 1, "{f:?}");
    assert!(f[0].fix.is_none());
    assert!(!f[0].suggestions.is_empty());
    let f = run("prose/simplify", "# In order to win\n");
    assert_eq!(f.len(), 1, "{f:?}");
    assert!(f[0].fix.is_none());
    let f = run("prose/so-start", "# So we added a cache\n");
    assert_eq!(f.len(), 1, "{f:?}");
    assert!(f[0].fix.is_none());
}
