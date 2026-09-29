use super::check;
use crate::config::{Config, Level};
use crate::diagnostic::{Finding, Severity};
use crate::rules::{Analyzed, FileCtx};
use crate::source::{FileKind, SourceFile};

fn run_cfg(src: &str, cfg: &Config) -> Vec<Finding> {
    let a = Analyzed::new(SourceFile::new(
        "t.md".into(),
        "t.md".into(),
        FileKind::Markdown,
        src.to_string(),
    ));
    let ctx = FileCtx { a: &a, config: cfg };
    let mut out = Vec::new();
    check(&ctx, &mut out);
    out
}

fn run(src: &str) -> Vec<Finding> {
    run_cfg(src, &Config::default())
}

fn rule(src: &str, id: &str) -> Vec<Finding> {
    run(src).into_iter().filter(|f| f.rule == id).collect()
}

fn rule_cfg(src: &str, id: &str, cfg: &Config) -> Vec<Finding> {
    run_cfg(src, cfg)
        .into_iter()
        .filter(|f| f.rule == id)
        .collect()
}

fn enabled(id: &str) -> Config {
    let mut c = Config::default();
    c.rules.insert(id.to_string(), Level::Warning);
    c
}

fn fixed(src: &str, id: &str) -> String {
    let fs = rule(src, id);
    let fixes: Vec<_> = fs.iter().filter_map(|f| f.fix.as_ref()).collect();
    crate::fix::apply(src, &fixes).0
}

fn fixed_all(src: &str) -> String {
    let fs = run(src);
    let fixes: Vec<_> = fs.iter().filter_map(|f| f.fix.as_ref()).collect();
    crate::fix::apply(src, &fixes).0
}

const CLEAN: &str = "---\ntitle: x\n---\n\nIntro paragraph with `code` and <https://example.com>.\n\n## Section one\n\n- a\n- b\n  - nested\n\n1. one\n2. two\n\n```rust\nlet x = 1;   \n\tlet y = 2;\n```\n\n| a | b |\n| - | - |\n| 1 | 2 |\n\n### Sub\n\nText with a hard break  \nnext line. ![alt](i.png) [the guide](x.md)\n";

#[test]
fn clean_document_has_no_findings() {
    let fs = run(CLEAN);
    assert!(fs.is_empty(), "{fs:#?}");
}

#[test]
fn heading_increment() {
    assert_eq!(rule("# A\n\n### C\n", "md/heading-increment").len(), 1);
    assert!(rule("# A\n\n## B\n\n### C\n\n## D\n", "md/heading-increment").is_empty());
    assert!(rule("### Start deep\n", "md/heading-increment").is_empty());
}

#[test]
fn heading_style() {
    let fs = rule("# A\n\nB\n-\n", "md/heading-style");
    assert_eq!(fs.len(), 1);
    assert!(fs[0].message.contains("Expected ATX"));
    assert!(rule("A\n=\n\nB\n-\n\n### C\n", "md/heading-style").is_empty());
    assert!(rule("# A\n\n## B\n", "md/heading-style").is_empty());
    let mut cfg = Config::default();
    cfg.markdown.heading_style = "setext".into();
    assert_eq!(
        rule_cfg("# A\n\n### C\n", "md/heading-style", &cfg).len(),
        1
    );
}

#[test]
fn list_marker() {
    let src = "- a\n* b\n- c\n";
    let fs = rule(src, "md/list-marker");
    assert_eq!(fs.len(), 1);
    assert_eq!(fixed(src, "md/list-marker"), "- a\n- b\n- c\n");
    assert!(rule("* a\n* b\n", "md/list-marker").is_empty());
    let mut cfg = Config::default();
    cfg.markdown.list_marker = "dash".into();
    assert_eq!(rule_cfg("* a\n* b\n", "md/list-marker", &cfg).len(), 2);
}

#[test]
fn list_indent() {
    // Inconsistent sibling indentation.
    assert_eq!(rule("- a\n - b\n", "md/list-indent").len(), 1);
    // Inconsistent nesting step.
    let src = "- a\n  - b\n\n\n- c\n    - d\n";
    assert_eq!(rule(src, "md/list-indent").len(), 1);
    // Top-level indented.
    assert_eq!(rule(" - a\n - b\n", "md/list-indent").len(), 1);
    // Right-aligned ordered numbers are fine.
    let src = " 8. a\n 9. b\n10. c\n";
    assert!(
        rule(src, "md/list-indent")
            .iter()
            .all(|f| !f.message.starts_with("List item"))
    );
    assert!(rule("- a\n  - b\n    - c\n- d\n  - e\n", "md/list-indent").is_empty());
    assert!(rule("1. a\n   - b\n", "md/list-indent").is_empty());
    assert!(rule("> - a\n>   - b\n", "md/list-indent").is_empty());
}

#[test]
fn trailing_spaces() {
    let src = "Text   \n";
    let fs = rule(src, "md/no-trailing-spaces");
    assert_eq!(fs.len(), 1);
    assert_eq!(fixed(src, "md/no-trailing-spaces"), "Text\n");
    // 3 spaces before a continuation line keep the hard break.
    assert_eq!(fixed("a   \nb\n", "md/no-trailing-spaces"), "a  \nb\n");
    assert_eq!(fixed("a\n  \nb\t\n", "md/no-trailing-spaces"), "a\n\nb\n");
    assert!(rule("a  \nb\n", "md/no-trailing-spaces").is_empty());
    assert!(rule("```\ncode   \n```\n", "md/no-trailing-spaces").is_empty());
    // CRLF.
    let src = "a \r\nb\r\n";
    assert_eq!(fixed(src, "md/no-trailing-spaces"), "a\r\nb\r\n");
    assert!(rule("a\r\nb\r\n", "md/no-trailing-spaces").is_empty());
}

#[test]
fn hard_tabs() {
    assert_eq!(rule("a\tb\n", "md/no-hard-tabs").len(), 1);
    assert!(rule("```\n\tcode\n```\n", "md/no-hard-tabs").is_empty());
    assert!(rule("x `a\tb` y\n", "md/no-hard-tabs").is_empty());
    assert!(rule("---\nk:\tv\n---\n\ntext\n", "md/no-hard-tabs").is_empty());
}

#[test]
fn multiple_blanks() {
    let src = "a\n\n\n\nb\n";
    let fs = rule(src, "md/no-multiple-blanks");
    assert_eq!(fs.len(), 1);
    assert!(fs[0].message.starts_with("3 "));
    assert_eq!(fixed(src, "md/no-multiple-blanks"), "a\n\nb\n");
    assert_eq!(
        fixed("a\r\n\r\n\r\nb\r\n", "md/no-multiple-blanks"),
        "a\r\n\r\nb\r\n"
    );
    assert!(rule("```\na\n\n\nb\n```\n", "md/no-multiple-blanks").is_empty());
    assert!(rule("a\n\nb\n", "md/no-multiple-blanks").is_empty());
}

#[test]
fn line_length() {
    let long = format!("{}\n", "word ".repeat(30).trim_end());
    assert!(rule(&long, "md/line-length").is_empty(), "off by default");
    let cfg = enabled("md/line-length");
    let fs = rule_cfg(&long, "md/line-length", &cfg);
    assert_eq!(fs.len(), 1);
    assert_eq!(fs[0].severity, Severity::Warning);
    let url = format!("See https://example.com/{}\n", "x".repeat(200));
    assert!(rule_cfg(&url, "md/line-length", &cfg).is_empty());
    let code = format!("```\n{long}```\n");
    assert!(rule_cfg(&code, "md/line-length", &cfg).is_empty());
    let table = format!("| a | b |\n| - | - |\n| {} | x |\n", "word ".repeat(30));
    assert!(rule_cfg(&table, "md/line-length", &cfg).is_empty());
    let mut short = cfg.clone();
    short.markdown.line_length = 10;
    assert_eq!(
        rule_cfg("one two three four\n", "md/line-length", &short).len(),
        1
    );
}

#[test]
fn blanks_around_headings() {
    let src = "text\n# H\nmore\n";
    let fs = rule(src, "md/blanks-around-headings");
    assert_eq!(fs.len(), 2);
    assert_eq!(
        fixed(src, "md/blanks-around-headings"),
        "text\n\n# H\n\nmore\n"
    );
    assert_eq!(
        fixed("> a\n> # H\n", "md/blanks-around-headings"),
        "> a\n>\n> # H\n"
    );
    assert!(rule("# H\n\ntext\n", "md/blanks-around-headings").is_empty());
    assert!(rule("---\nt: 1\n---\n# H\n", "md/blanks-around-headings").is_empty());
    assert!(rule("H\n=\n\ntext\n", "md/blanks-around-headings").is_empty());
    assert_eq!(
        fixed("a\r\n# H\r\n", "md/blanks-around-headings"),
        "a\r\n\r\n# H\r\n"
    );
}

#[test]
fn heading_start_left() {
    let src = "  # H\n";
    assert_eq!(rule(src, "md/heading-start-left").len(), 1);
    assert_eq!(fixed(src, "md/heading-start-left"), "# H\n");
    assert!(rule("# H\n", "md/heading-start-left").is_empty());
    assert!(rule("- item\n\n  # H\n", "md/heading-start-left").is_empty());
}

#[test]
fn duplicate_heading() {
    let src = "# A\n\n## Setup\n\n## Setup\n";
    assert_eq!(rule(src, "md/no-duplicate-heading").len(), 1);
    // Same text under different parents is fine.
    let src = "# A\n\n## X\n\n### Setup\n\n## Y\n\n### Setup\n";
    assert!(rule(src, "md/no-duplicate-heading").is_empty());
}

#[test]
fn single_h1() {
    assert_eq!(rule("# A\n\n# B\n", "md/single-h1").len(), 1);
    assert!(rule("# A\n\n## B\n", "md/single-h1").is_empty());
    assert_eq!(rule("---\ntitle: T\n---\n\n# A\n", "md/single-h1").len(), 1);
}

#[test]
fn trailing_punctuation() {
    let fs = rule("# Hello.\n", "md/no-trailing-punctuation");
    assert_eq!(fs.len(), 1);
    assert_eq!(fs[0].range, 7..8);
    assert_eq!(rule("## Note: #\n", "md/no-trailing-punctuation").len(), 1);
    assert!(rule("# Why?\n", "md/no-trailing-punctuation").is_empty());
    assert!(rule("# Hello\n", "md/no-trailing-punctuation").is_empty());
}

#[test]
fn ol_prefix() {
    assert_eq!(rule("1. a\n3. b\n", "md/ol-prefix").len(), 1);
    assert_eq!(rule("1. a\n1. b\n2. c\n", "md/ol-prefix").len(), 1);
    assert!(rule("1. a\n2. b\n3. c\n", "md/ol-prefix").is_empty());
    assert!(rule("1. a\n1. b\n1. c\n", "md/ol-prefix").is_empty());
    assert!(rule("0. a\n1. b\n2. c\n", "md/ol-prefix").is_empty());
    assert!(rule("0. a\n0. b\n", "md/ol-prefix").is_empty());
}

#[test]
fn blanks_around_fences() {
    let src = "text\n```sh\nls\n```\nmore\n";
    let fs = rule(src, "md/blanks-around-fences");
    assert_eq!(fs.len(), 2);
    assert_eq!(
        fixed(src, "md/blanks-around-fences"),
        "text\n\n```sh\nls\n```\n\nmore\n"
    );
    assert!(
        rule(
            "text\n\n```sh\nls\n```\n\nmore\n",
            "md/blanks-around-fences"
        )
        .is_empty()
    );
    assert!(rule("```sh\nls\n", "md/blanks-around-fences").is_empty());
}

#[test]
fn blanks_around_lists() {
    let src = "text\n- a\n- b\n\n# H\n";
    // "text\n- a" is a list interrupting a paragraph.
    let fs = rule(src, "md/blanks-around-lists");
    assert_eq!(fs.len(), 1);
    assert_eq!(
        fixed(src, "md/blanks-around-lists"),
        "text\n\n- a\n- b\n\n# H\n"
    );
    let src = "- a\n- b\n```sh\nx\n```\n";
    assert_eq!(
        fixed(src, "md/blanks-around-lists"),
        "- a\n- b\n\n```sh\nx\n```\n"
    );
    // Heading rule already owns the insertion fix at this gap; the list finding is still reported.
    let fs = rule("- a\n- b\n# H\n", "md/blanks-around-lists");
    assert_eq!(fs.len(), 1);
    assert!(fs[0].fix.is_none());
    assert!(rule("text\n\n- a\n  - b\n\nmore\n", "md/blanks-around-lists").is_empty());
}

#[test]
fn no_double_blank_insertion() {
    assert_eq!(fixed_all("# H\n```sh\nx\n```\n"), "# H\n\n```sh\nx\n```\n");
}

#[test]
fn bare_urls() {
    let src = "See https://example.com/x for more.\n";
    let fs = rule(src, "md/no-bare-urls");
    assert_eq!(fs.len(), 1);
    assert_eq!(
        fixed(src, "md/no-bare-urls"),
        "See <https://example.com/x> for more.\n"
    );
    assert!(rule("See <https://example.com>.\n", "md/no-bare-urls").is_empty());
    assert!(rule("See [x](https://example.com).\n", "md/no-bare-urls").is_empty());
    assert!(rule("See `https://example.com`.\n", "md/no-bare-urls").is_empty());
    assert!(rule("```\nhttps://example.com\n```\n", "md/no-bare-urls").is_empty());
}

#[test]
fn bare_url_with_asterisks_not_corrupted() {
    // An unmatched `*` makes pulldown-cmark split the text; the whole URL is still wrapped.
    assert_eq!(
        fixed("See https://example.org/a*b now.\n", "md/no-bare-urls"),
        "See <https://example.org/a*b> now.\n"
    );
    // `*b*` parses as emphasis: reported, but no fix for the truncated prefix.
    let src = "See https://example.org/a*b*c now.\n";
    assert_eq!(rule(src, "md/no-bare-urls").len(), 1);
    assert_eq!(fixed(src, "md/no-bare-urls"), src);
    assert_eq!(
        fixed("See **https://example.org/x** now.\n", "md/no-bare-urls"),
        "See **<https://example.org/x>** now.\n"
    );
}

#[test]
fn emphasis_as_heading() {
    let src = "Intro.\n\n**Installation**\n\nRun it.\n";
    assert_eq!(rule(src, "md/no-emphasis-as-heading").len(), 1);
    assert_eq!(
        rule("Intro.\n\n_Note_\n\nx\n", "md/no-emphasis-as-heading").len(),
        1
    );
    assert!(rule("Intro.\n\n**Note:**\n\nx\n", "md/no-emphasis-as-heading").is_empty());
    assert!(
        rule(
            "Intro.\n\n**This is important.**\n\nx\n",
            "md/no-emphasis-as-heading"
        )
        .is_empty()
    );
    assert!(rule("**Bold** start\n", "md/no-emphasis-as-heading").is_empty());
    assert!(rule("**a** and **b**\n", "md/no-emphasis-as-heading").is_empty());
    assert!(rule("- **item**\n", "md/no-emphasis-as-heading").is_empty());
    assert!(rule("**bold**\ncontinues\n", "md/no-emphasis-as-heading").is_empty());
}

#[test]
fn space_in_code() {
    let src = "Use ` foo ` here.\n";
    assert_eq!(rule(src, "md/no-space-in-code").len(), 2);
    assert_eq!(fixed(src, "md/no-space-in-code"), "Use `foo` here.\n");
    // Long span: fixes touch only the edge spaces, never the content.
    let body = "x".repeat(1000);
    let src = format!("Use `  {body} ` here.\n");
    let fs = rule(&src, "md/no-space-in-code");
    assert_eq!(fs.len(), 2);
    for f in &fs {
        let fx = f.fix.as_ref().unwrap();
        assert!(fx.replacement.is_empty());
        assert!(src[fx.range.clone()].bytes().all(|b| b == b' '), "{fx:?}");
    }
    assert_eq!(fs[0].fix.as_ref().unwrap().range.len(), 2);
    assert_eq!(fs[1].fix.as_ref().unwrap().range.len(), 1);
    assert_eq!(
        fixed(&src, "md/no-space-in-code"),
        format!("Use `{body}` here.\n")
    );
    assert_eq!(
        fixed("Use `foo ` here.\n", "md/no-space-in-code"),
        "Use `foo` here.\n"
    );
    assert!(rule("Use `foo` here.\n", "md/no-space-in-code").is_empty());
    assert!(rule("Use `` `tick` `` here.\n", "md/no-space-in-code").is_empty());
    assert!(rule("Use ` ` here.\n", "md/no-space-in-code").is_empty());
}

#[test]
fn fenced_code_language() {
    assert_eq!(rule("```\nx\n```\n", "md/fenced-code-language").len(), 1);
    assert!(rule("```text\nx\n```\n", "md/fenced-code-language").is_empty());
}

#[test]
fn first_line_heading() {
    let cfg = enabled("md/first-line-heading");
    assert!(
        rule("Text\n", "md/first-line-heading").is_empty(),
        "off by default"
    );
    assert_eq!(
        rule_cfg("Text\n\n# H\n", "md/first-line-heading", &cfg).len(),
        1
    );
    assert_eq!(rule_cfg("## H\n", "md/first-line-heading", &cfg).len(), 1);
    assert!(rule_cfg("# H\n\ntext\n", "md/first-line-heading", &cfg).is_empty());
    assert!(rule_cfg("<!-- c -->\n\n# H\n", "md/first-line-heading", &cfg).is_empty());
    assert!(
        rule_cfg(
            "---\ntitle: T\n---\n\nText\n",
            "md/first-line-heading",
            &cfg
        )
        .is_empty()
    );
    assert!(rule_cfg("---\na: 1\n---\n\n# H\n", "md/first-line-heading", &cfg).is_empty());
}

#[test]
fn empty_links() {
    let fs = rule("A [link]() and [b](#).\n", "md/no-empty-links");
    assert_eq!(fs.len(), 2);
    assert_eq!(fs[0].severity, Severity::Error);
    assert!(rule("A [link](x.md).\n", "md/no-empty-links").is_empty());
}

#[test]
fn code_block_style() {
    assert_eq!(
        rule("Text\n\n    indented code\n", "md/code-block-style").len(),
        1
    );
    assert!(rule("```sh\nx\n```\n", "md/code-block-style").is_empty());
}

#[test]
fn trailing_newline() {
    assert_eq!(fixed("a", "md/single-trailing-newline"), "a\n");
    assert_eq!(fixed("a\n\n\n", "md/single-trailing-newline"), "a\n");
    assert_eq!(fixed("a\r\nb", "md/single-trailing-newline"), "a\r\nb\r\n");
    assert_eq!(fixed("a\r\n\r\n", "md/single-trailing-newline"), "a\r\n");
    assert!(rule("a\n", "md/single-trailing-newline").is_empty());
    assert!(rule("", "md/single-trailing-newline").is_empty());
    assert!(rule("a\n\n\n", "md/no-multiple-blanks").is_empty());
}

#[test]
fn heading_case_sentence() {
    let mut cfg = enabled("md/heading-case");
    cfg.markdown.heading_case = "sentence".into();
    let fs = rule_cfg("# Getting Started With The API\n", "md/heading-case", &cfg);
    assert_eq!(fs.len(), 1);
    assert_eq!(
        fs[0].suggestions,
        vec!["Getting started with the API".to_string()]
    );
    assert!(
        rule_cfg(
            "# Using GitHub and iOS 17 with `Foo`\n",
            "md/heading-case",
            &cfg
        )
        .is_empty()
    );
    assert!(rule_cfg("# Config: The basics\n", "md/heading-case", &cfg).is_empty());
    assert_eq!(
        rule_cfg("# lowercase start\n", "md/heading-case", &cfg).len(),
        1
    );
    assert!(
        rule("# Getting Started\n", "md/heading-case").is_empty(),
        "off by default"
    );
    cfg.prose.accept.push("Rust".into());
    assert!(rule_cfg("# Why Rust\n", "md/heading-case", &cfg).is_empty());
}

#[test]
fn heading_case_title() {
    let mut cfg = enabled("md/heading-case");
    cfg.markdown.heading_case = "title".into();
    let fs = rule_cfg("# Getting started with the API\n", "md/heading-case", &cfg);
    assert_eq!(fs.len(), 1);
    assert_eq!(
        fs[0].suggestions,
        vec!["Getting Started with the API".to_string()]
    );
    assert!(rule_cfg("# Getting Started with the API\n", "md/heading-case", &cfg).is_empty());
    assert!(rule_cfg("# What It Is For\n", "md/heading-case", &cfg).is_empty());
}

#[test]
fn image_alt() {
    assert_eq!(rule("![](x.png)\n", "md/image-alt").len(), 1);
    assert_eq!(rule("<img src=\"x.png\">\n", "md/image-alt").len(), 1);
    assert!(rule("![Logo](x.png)\n", "md/image-alt").is_empty());
    assert!(rule("<img src=\"x.png\" alt=\"Logo\">\n", "md/image-alt").is_empty());
}

#[test]
fn every_md_rule_is_tested() {
    let src = include_str!("tests.rs");
    for r in crate::rules::RULES
        .iter()
        .filter(|r| r.id.starts_with("md/"))
    {
        assert!(
            src.matches(&format!("\"{}\"", r.id)).count() > 1,
            "{} untested",
            r.id
        );
    }
}

#[test]
fn code_language_known() {
    for l in [
        "rust",
        "Rust",
        "sh",
        "console",
        "mermaid",
        "d2",
        "js title=\"x\"",
        "rust,ignore",
        "js {1,3}",
    ] {
        let src = format!("```{l}\nx\n```\n");
        assert!(rule(&src, "md/code-language-known").is_empty(), "{l}");
    }
    let fs = rule("```rsut\nx\n```\n", "md/code-language-known");
    assert_eq!(fs.len(), 1);
    assert_eq!(fs[0].message, "Unknown code block language 'rsut'");
    assert_eq!(fs[0].range, 3..7);
    assert_eq!(fs[0].suggestions.first().map(String::as_str), Some("rust"));
    // Missing language is MD040's job.
    assert!(rule("```\nx\n```\n", "md/code-language-known").is_empty());

    let mut cfg = Config::default();
    cfg.markdown.allowed_languages = vec!["mylang".into()];
    assert!(rule_cfg("```MyLang\nx\n```\n", "md/code-language-known", &cfg).is_empty());
    assert_eq!(
        rule("```mylang\nx\n```\n", "md/code-language-known").len(),
        1
    );
}

#[test]
fn missing_space_atx() {
    let src = "#Heading\n\ntext\n";
    assert_eq!(rule(src, "md/no-missing-space-atx").len(), 1);
    assert_eq!(fixed(src, "md/no-missing-space-atx"), "# Heading\n\ntext\n");
    assert_eq!(fixed("##Ä\r\n", "md/no-missing-space-atx"), "## Ä\r\n");
    assert!(rule("# Heading\n", "md/no-missing-space-atx").is_empty());
    assert!(rule("#123 was fixed\n", "md/no-missing-space-atx").is_empty());
    assert!(rule("```sh\n#comment\n```\n", "md/no-missing-space-atx").is_empty());
    assert!(rule("<div>\n#x\n</div>\n", "md/no-missing-space-atx").is_empty());
    assert!(rule("Use `#x` here\n", "md/no-missing-space-atx").is_empty());
}

#[test]
fn multiple_space_atx() {
    let src = "#  Heading\n";
    assert_eq!(rule(src, "md/no-multiple-space-atx").len(), 1);
    assert_eq!(fixed(src, "md/no-multiple-space-atx"), "# Heading\n");
    let src = "## Heading   ##\r\n";
    assert_eq!(fixed(src, "md/no-multiple-space-atx"), "## Heading ##\r\n");
    assert!(rule("# Heading #\n", "md/no-multiple-space-atx").is_empty());
    assert!(rule("# Héading\n", "md/no-multiple-space-atx").is_empty());
}

#[test]
fn missing_space_closed_atx() {
    let src = "#Heading#\n";
    assert_eq!(rule(src, "md/no-missing-space-closed-atx").len(), 1);
    assert!(rule(src, "md/no-missing-space-atx").is_empty());
    assert_eq!(
        fixed(src, "md/no-missing-space-closed-atx"),
        "# Heading #\n"
    );
    assert_eq!(
        fixed("## Heading##\n", "md/no-missing-space-closed-atx"),
        "## Heading ##\n"
    );
    assert!(rule("## Heading ##\n", "md/no-missing-space-closed-atx").is_empty());
    assert!(rule("# Learn C#\n", "md/no-missing-space-closed-atx").is_empty());
    assert!(rule("# Escaped \\#\n", "md/no-missing-space-closed-atx").is_empty());
}

#[test]
fn heading_like_paragraph() {
    assert_eq!(
        rule("####### Too deep\n", "md/no-heading-like-paragraph").len(),
        1
    );
    assert!(rule("###### Deep\n", "md/no-heading-like-paragraph").is_empty());
    assert!(rule("```\n####### x\n```\n", "md/no-heading-like-paragraph").is_empty());
}

#[test]
fn max_heading_length() {
    let long = format!("# {}\n", "word ".repeat(20).trim_end());
    assert!(
        rule(&long, "md/max-heading-length").is_empty(),
        "off by default"
    );
    let mut cfg = enabled("md/max-heading-length");
    assert_eq!(rule_cfg(&long, "md/max-heading-length", &cfg).len(), 1);
    assert!(rule_cfg("# Short\n", "md/max-heading-length", &cfg).is_empty());
    cfg.markdown.max_heading_length = 3;
    assert_eq!(rule_cfg("# Äöüå\n", "md/max-heading-length", &cfg).len(), 1);
}

#[test]
fn empty_section() {
    let fs = rule("# A\n\n## B\n\n## C\n\ntext\n", "md/no-empty-section");
    assert_eq!(fs.len(), 1);
    assert!(fs[0].message.contains("\"B\""));
    assert!(rule("# A\n\n## B\n\ntext\n", "md/no-empty-section").is_empty());
    // A deeper heading is content of its parent.
    assert!(rule("## A\n\n### B\n\ntext\n", "md/no-empty-section").is_empty());
    assert_eq!(rule("## A\r\n\r\n# B\r\n", "md/no-empty-section").len(), 1);
}

#[test]
fn list_marker_space() {
    let src = "-  a\n- b\n";
    assert_eq!(rule(src, "md/list-marker-space").len(), 1);
    assert_eq!(fixed(src, "md/list-marker-space"), "- a\n- b\n");
    assert_eq!(fixed("1.  ä\r\n", "md/list-marker-space"), "1. ä\r\n");
    assert!(rule("- a\n- b\n", "md/list-marker-space").is_empty());
    // Ordered items aligned on content.
    assert!(rule(" 9.  a\n10.  b\n", "md/list-marker-space").is_empty());
    assert!(rule("9.  a\n10. b\n", "md/list-marker-space").is_empty());
    // Indented code inside the item, empty items.
    assert!(rule("-     code\n", "md/list-marker-space").is_empty());
    assert!(rule("-\n  a\n", "md/list-marker-space").is_empty());
}

#[test]
fn space_in_emphasis() {
    let src = "Some ** bold ** and * em* text.\n";
    let fs = rule(src, "md/no-space-in-emphasis");
    assert_eq!(fs.len(), 3, "one finding per spaced edge");
    assert!(
        fs.iter()
            .all(|f| f.fix.as_ref().unwrap().replacement.is_empty())
    );
    assert_eq!(
        fixed(src, "md/no-space-in-emphasis"),
        "Some **bold** and *em* text.\n"
    );
    assert_eq!(
        fixed("Ä _ ö _ ü\r\n", "md/no-space-in-emphasis"),
        "Ä _ö_ ü\r\n"
    );
    assert!(rule("Some **bold** and *em*.\n", "md/no-space-in-emphasis").is_empty());
    assert!(rule("* item *x*\n", "md/no-space-in-emphasis").is_empty());
    assert!(rule("2 * 3 * 4\n", "md/no-space-in-emphasis").is_empty());
    assert!(rule("snake_case and foo_bar_baz\n", "md/no-space-in-emphasis").is_empty());
    assert!(rule("`* a *`\n", "md/no-space-in-emphasis").is_empty());
    assert!(rule("```\n* a *\n```\n", "md/no-space-in-emphasis").is_empty());
    assert!(rule("* * *\n", "md/no-space-in-emphasis").is_empty());
}

#[test]
fn space_in_links() {
    let src = "A [ link ](x.md) here.\n";
    let fs = rule(src, "md/no-space-in-links");
    assert_eq!(fs.len(), 2, "one finding per spaced edge");
    assert!(fs.iter().all(|f| f.fix.as_ref().unwrap().range.len() == 1));
    assert_eq!(fixed(src, "md/no-space-in-links"), "A [link](x.md) here.\n");
    assert_eq!(
        fixed("A [ä ][r]\r\n\r\n[r]: x.md\r\n", "md/no-space-in-links"),
        "A [ä][r]\r\n\r\n[r]: x.md\r\n"
    );
    assert!(rule("A [link](x.md).\n", "md/no-space-in-links").is_empty());
    assert!(rule("A [`a ]`](x.md).\n", "md/no-space-in-links").is_empty());
}

#[test]
fn reversed_links() {
    let src = "See (the docs)[https://example.com].\n";
    let fs = rule(src, "md/no-reversed-links");
    assert_eq!(fs.len(), 1);
    assert_eq!(fs[0].severity, Severity::Error);
    assert_eq!(
        fixed(src, "md/no-reversed-links"),
        "See [the docs](https://example.com).\n"
    );
    assert!(
        rule(
            "See [the docs](https://example.com).\n",
            "md/no-reversed-links"
        )
        .is_empty()
    );
    assert!(rule("See `(a)[b]`.\n", "md/no-reversed-links").is_empty());
    assert!(rule("Note (x)[^1].\n\n[^1]: n\n", "md/no-reversed-links").is_empty());
    assert!(rule("Call f(a)[i] here\n\n[i]: x.md\n", "md/no-reversed-links").is_empty());
    assert!(rule("```\n(a)[b]\n```\n", "md/no-reversed-links").is_empty());
}

#[test]
fn reversed_link_fix_keeps_code_span() {
    assert_eq!(
        fixed("(Use `foo`)[guide.md] here.\n", "md/no-reversed-links"),
        "[Use `foo`](guide.md) here.\n"
    );
}

#[test]
fn multiple_space_blockquote() {
    let src = ">  quote\n";
    assert_eq!(rule(src, "md/no-multiple-space-blockquote").len(), 1);
    assert_eq!(fixed(src, "md/no-multiple-space-blockquote"), "> quote\n");
    assert_eq!(
        fixed("> >  ä\r\n", "md/no-multiple-space-blockquote"),
        "> > ä\r\n"
    );
    assert!(rule("> quote\n", "md/no-multiple-space-blockquote").is_empty());
    assert!(rule(">     code\n", "md/no-multiple-space-blockquote").is_empty());
    assert!(rule("> - a\n>   b\n", "md/no-multiple-space-blockquote").is_empty());
}

#[test]
fn blanks_blockquote() {
    assert_eq!(rule("> a\n\n> b\n", "md/no-blanks-blockquote").len(), 1);
    assert_eq!(
        rule("> a\r\n\r\n> b\r\n", "md/no-blanks-blockquote").len(),
        1
    );
    assert!(rule("> a\n>\n> b\n", "md/no-blanks-blockquote").is_empty());
    assert!(rule("> a\n\ntext\n\n> b\n", "md/no-blanks-blockquote").is_empty());
}

#[test]
fn alert_syntax() {
    let fs = rule("> [!INFO]\n> text\n", "md/alert-syntax");
    assert_eq!(fs.len(), 1);
    assert_eq!(fs[0].severity, Severity::Error);
    assert_eq!(fs[0].suggestions, vec!["[!NOTE]".to_string()]);
    assert_eq!(
        rule("> [!DANGER]\n> x\n", "md/alert-syntax")[0].suggestions,
        vec!["[!CAUTION]".to_string()]
    );
    let src = "> [!WARNING] Be careful\n";
    assert_eq!(rule(src, "md/alert-syntax").len(), 1);
    assert_eq!(
        fixed(src, "md/alert-syntax"),
        "> [!WARNING]\n> Be careful\n"
    );
    assert_eq!(
        fixed("> [!TIP] ä\r\n", "md/alert-syntax"),
        "> [!TIP]\r\n> ä\r\n"
    );
    for ok in [
        "> [!NOTE]\n> x\n",
        "> [!note]\n> x\n",
        "> [!CAUTION]  \n> x\n",
        "> text\n> [!INFO]\n",
    ] {
        assert!(rule(ok, "md/alert-syntax").is_empty(), "{ok}");
    }
    assert!(rule("```\n> [!INFO]\n```\n", "md/alert-syntax").is_empty());
}

#[test]
fn hr_style() {
    let src = "a\n\n---\n\nb\n\n***\n\nc\n";
    assert_eq!(rule(src, "md/hr-style").len(), 1);
    assert_eq!(fixed(src, "md/hr-style"), "a\n\n---\n\nb\n\n---\n\nc\n");
    assert!(rule("a\n\n---\n\nb\n\n---\n", "md/hr-style").is_empty());
    // Front matter and setext underlines are not thematic breaks.
    assert!(rule("---\nt: 1\n---\n\nH\n---\n\n***\n", "md/hr-style").is_empty());
    let mut cfg = Config::default();
    cfg.markdown.hr_style = "***".into();
    assert_eq!(rule_cfg("a\r\n\r\n---\r\n", "md/hr-style", &cfg).len(), 1);
}

#[test]
fn code_fence_style() {
    let src = "```sh\na\n```\n\n~~~sh\nb\n~~~\n";
    assert_eq!(rule(src, "md/code-fence-style").len(), 1);
    assert_eq!(
        fixed(src, "md/code-fence-style"),
        "```sh\na\n```\n\n```sh\nb\n```\n"
    );
    // Unsafe conversion is reported without a fix.
    let src = "```sh\na\n```\n\n~~~md\n```\nx\n```\n~~~\n";
    let fs = rule(src, "md/code-fence-style");
    assert_eq!(fs.len(), 1);
    assert!(fs[0].fix.is_none());
    assert!(rule("~~~sh\na\n~~~\n\n~~~sh\nä\n~~~\n", "md/code-fence-style").is_empty());
    let mut cfg = Config::default();
    cfg.markdown.code_fence_style = "tilde".into();
    assert_eq!(
        rule_cfg("```sh\r\na\r\n```\r\n", "md/code-fence-style", &cfg).len(),
        1
    );
}

#[test]
fn emphasis_and_strong_style() {
    let src = "*a* and _b_, **c** and __d__.\n";
    assert_eq!(rule(src, "md/emphasis-style").len(), 1);
    assert_eq!(rule(src, "md/strong-style").len(), 1);
    assert_eq!(
        fixed(src, "md/emphasis-style"),
        "*a* and *b*, **c** and __d__.\n"
    );
    assert_eq!(
        fixed(src, "md/strong-style"),
        "*a* and _b_, **c** and **d**.\n"
    );
    assert!(rule("*ä* and *b*\n", "md/emphasis-style").is_empty());
    assert!(rule("**a** and **b**\n", "md/strong-style").is_empty());
    let mut cfg = Config::default();
    cfg.markdown.emphasis_style = "underscore".into();
    // Intraword emphasis cannot use underscores: reported without a fix.
    let fs = rule_cfg("foo*bar*baz\n", "md/emphasis-style", &cfg);
    assert_eq!(fs.len(), 1);
    assert!(fs[0].fix.is_none());
    assert!(rule("`_a_` and *b*\n", "md/emphasis-style").is_empty());
}

#[test]
fn table_column_count() {
    let src = "| a | b |\n| - | - |\n| 1 | 2 | 3 |\n| 4 |\n";
    let fs = rule(src, "md/table-column-count");
    assert_eq!(fs.len(), 2);
    assert_eq!(fs[0].severity, Severity::Error);
    assert!(
        rule(
            "| a | b |\n| - | - |\n| 1 | `x\\|y` |\n",
            "md/table-column-count"
        )
        .is_empty()
    );
    assert!(rule("a | b\n- | -\n1 | ä\r\n", "md/table-column-count").is_empty());
}

#[test]
fn table_pipe_style() {
    let src = "| a | b |\n| - | - |\n1 | 2\n";
    assert_eq!(rule(src, "md/table-pipe-style").len(), 1);
    assert!(rule("a | b\n- | -\n1 | 2\n", "md/table-pipe-style").is_empty());
    assert!(
        rule(
            "| a | b |\r\n| - | - |\r\n| 1 | 2 |\r\n",
            "md/table-pipe-style"
        )
        .is_empty()
    );
}

#[test]
fn blanks_around_tables() {
    let src = "text\n\n| a | b |\n| - | - |\n| 1 | 2 |\n> quote\n";
    let fs = rule(src, "md/blanks-around-tables");
    assert_eq!(fs.len(), 1);
    assert_eq!(
        fixed(src, "md/blanks-around-tables"),
        "text\n\n| a | b |\n| - | - |\n| 1 | 2 |\n\n> quote\n"
    );
    assert!(
        rule(
            "text\n\n| a | b |\n| - | - |\n\ntext\n",
            "md/blanks-around-tables"
        )
        .is_empty()
    );
}

#[test]
fn descriptive_link_text() {
    assert_eq!(
        rule(
            "For details [click here](x.md).\n",
            "md/descriptive-link-text"
        )
        .len(),
        1
    );
    assert_eq!(
        rule("See [Here](x.md).\n", "md/descriptive-link-text").len(),
        1
    );
    assert!(rule("See [the setup guide](x.md).\n", "md/descriptive-link-text").is_empty());
    assert!(rule("![here](x.png)\n", "md/descriptive-link-text").is_empty());
    let mut cfg = Config::default();
    cfg.markdown.non_descriptive_link_text = vec!["docs".into()];
    assert_eq!(
        rule_cfg("See [docs](x.md).\n", "md/descriptive-link-text", &cfg).len(),
        1
    );
    assert!(rule_cfg("See [here](x.md).\n", "md/descriptive-link-text", &cfg).is_empty());
}

#[test]
fn commands_show_output() {
    let fs = rule("```sh\n$ ls\n$ pwd\n```\n", "md/commands-show-output");
    assert_eq!(fs.len(), 1);
    assert_eq!(fs[0].severity, Severity::Info);
    assert_eq!(
        rule("```sh\r\n$ ls\r\n```\r\n", "md/commands-show-output").len(),
        1
    );
    assert!(
        rule(
            "```console\n$ ls\nfile.txt\n```\n",
            "md/commands-show-output"
        )
        .is_empty()
    );
    assert!(rule("```sh\nls\n```\n", "md/commands-show-output").is_empty());
    assert!(rule("```sh\n$HOME/bin\n```\n", "md/commands-show-output").is_empty());
}

#[test]
fn duplicate_definition() {
    let src = "[a] and [b]\n\n[a]: x.md\n[A]: y.md\n[b]: z.md\n";
    let fs = rule(src, "md/no-duplicate-definition");
    assert_eq!(fs.len(), 1);
    assert_eq!(fs[0].severity, Severity::Error);
    assert!(fs[0].message.contains("line 3"));
    assert_eq!(
        rule(
            "x[^1]\r\n\r\n[^1]: a\r\n\r\n[^1]: b\r\n",
            "md/no-duplicate-definition"
        )
        .len(),
        1
    );
    assert!(
        rule(
            "[a]\n\n[a]: x.md\n[b]: y.md\n",
            "md/no-duplicate-definition"
        )
        .is_empty()
    );
    assert!(
        rule(
            "[//]: # (one)\n\n[//]: # (two)\n",
            "md/no-duplicate-definition"
        )
        .is_empty()
    );
    assert!(
        rule(
            "[a]\n\n[a]: x.md\n\n```\n[a]: y.md\n```\n",
            "md/no-duplicate-definition"
        )
        .is_empty()
    );
}

#[test]
fn inline_html() {
    let src = "Text <span>x</span> and <br> and <kbd>k</kbd>.\n\n<div>\nblock\n</div>\n";
    assert!(rule(src, "md/no-inline-html").is_empty(), "off by default");
    let cfg = enabled("md/no-inline-html");
    let fs = rule_cfg(src, "md/no-inline-html", &cfg);
    assert_eq!(fs.len(), 2, "{fs:#?}");
    assert!(fs[0].message.contains("<span>"));
    assert!(rule_cfg("<!-- <div> -->\n\n`<div>`\n", "md/no-inline-html", &cfg).is_empty());
    assert!(rule_cfg("<!--\n<div>\n-->\n", "md/no-inline-html", &cfg).is_empty());
}

#[test]
fn front_matter_required() {
    let mut cfg = enabled("md/front-matter-required");
    assert!(rule_cfg("# H\n", "md/front-matter-required", &cfg).is_empty());
    cfg.markdown.front_matter_required = vec!["title".into(), "description".into()];
    let fs = rule_cfg(
        "---\ntitle: T\nnested:\n  description: x\n---\n\n# H\n",
        "md/front-matter-required",
        &cfg,
    );
    assert_eq!(fs.len(), 1);
    assert!(fs[0].message.contains("description"));
    assert_eq!(rule_cfg("# H\n", "md/front-matter-required", &cfg).len(), 2);
    assert!(
        rule_cfg(
            "---\r\ntitle: T\r\ndescription: ä\r\n---\r\n\r\n# H\r\n",
            "md/front-matter-required",
            &cfg
        )
        .is_empty()
    );
}

#[test]
fn task_list_style() {
    let src = "- [X] done\n- [] todo\n- [x] ok\n- [ ] open\n";
    let fs = rule(src, "md/task-list-style");
    assert_eq!(fs.len(), 2);
    assert_eq!(fs[0].severity, Severity::Info);
    assert_eq!(
        fixed(src, "md/task-list-style"),
        "- [x] done\n- [ ] todo\n- [x] ok\n- [ ] open\n"
    );
    assert_eq!(fixed("1. [X]\r\n", "md/task-list-style"), "1. [x]\r\n");
    assert!(rule("- [Xy] link\n- [] (x)\n", "md/task-list-style").len() == 1);
    assert!(rule("[X] not a list\n", "md/task-list-style").is_empty());
}

#[test]
fn footnotes() {
    let src = "Text[^1] and[^missing].\n\n[^1]: Note.\n[^unused]: Never used.\n";
    let fs = rule(src, "links/undefined-footnote");
    assert_eq!(fs.len(), 1);
    assert_eq!(fs[0].severity, Severity::Error);
    assert!(fs[0].message.contains("missing"));
    let fs = rule(src, "links/unused-footnote");
    assert_eq!(fs.len(), 1);
    assert!(fs[0].message.contains("unused"));
    let ok = "Ä[^Note]\r\n\r\n[^note]: ö\r\n";
    assert!(rule(ok, "links/undefined-footnote").is_empty());
    assert!(rule(ok, "links/unused-footnote").is_empty());
    assert!(
        rule(
            "Code `[^x]` and\n\n```\n[^y]\n```\n",
            "links/undefined-footnote"
        )
        .is_empty()
    );
}

#[test]
fn duplicate_anchor() {
    let src = "# A {#x}\n\n## B {#x}\n";
    assert_eq!(rule(src, "links/duplicate-anchor").len(), 1);
    let src = "# Intro\n\n## Other {#intro}\n";
    assert_eq!(rule(src, "links/duplicate-anchor").len(), 1);
    let src = "<a id=\"top\"></a>\n\n# Title {#top}\n";
    assert_eq!(rule(src, "links/duplicate-anchor").len(), 1);
    assert!(rule("# A {#a}\n\n## B {#b}\n\n## Ä\n", "links/duplicate-anchor").is_empty());
    // Generated slug collisions are deduplicated by numbering.
    assert!(rule("# A\n\n## A\n", "links/duplicate-anchor").is_empty());
}
