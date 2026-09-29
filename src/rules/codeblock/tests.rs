use std::path::PathBuf;

use super::json::strip_jsonc;
use crate::config::Config;
use crate::diagnostic::Finding;
use crate::rules::{Analyzed, FileCtx};
use crate::source::{FileKind, SourceFile};

fn run(src: &str) -> Vec<Finding> {
    let a = Analyzed::new(SourceFile::new(
        PathBuf::from("/t/a.md"),
        PathBuf::from("a.md"),
        FileKind::Markdown,
        src.to_string(),
    ));
    let config = Config::default();
    let ctx = FileCtx {
        a: &a,
        config: &config,
    };
    let mut out = Vec::new();
    super::check(&ctx, &mut out);
    out
}

fn rules(src: &str) -> Vec<String> {
    run(src).into_iter().map(|f| f.rule).collect()
}

fn at<'a>(src: &'a str, f: &Finding) -> &'a str {
    &src[f.range.clone()]
}

#[test]
fn json_valid() {
    assert!(rules("```json\n{\"a\": [1, 2], \"b\": \"ü\"}\n```\n").is_empty());
}

#[test]
fn json_invalid_range() {
    let src = "# T\n\n```json\n{\n  \"a\": 1,\n  \"b\": x\n}\n```\n";
    let f = run(src);
    assert_eq!(f.len(), 1);
    assert_eq!(f[0].rule, "codeblock/json");
    assert_eq!(at(src, &f[0]), "x");
}

#[test]
fn json_multibyte_and_crlf() {
    let src = "```json\r\n{\"ä\": \"öö\",\r\n \"b\": ?}\r\n```\r\n";
    let f = run(src);
    assert_eq!(f.len(), 1);
    assert_eq!(at(src, &f[0]), "?");
    let src = "```json\r\n{\"a\": 1}\r\n```\r\n";
    assert!(run(src).is_empty());
}

#[test]
fn json_trailing_comma_strict_vs_jsonc() {
    let src = "```json\n{\"a\": 1,}\n```\n";
    assert_eq!(rules(src), ["codeblock/json"]);
    let src =
        "```jsonc\n{\n  // comment ü\n  \"a\": [1, 2,], /* x */\n  \"b\": \"//not\",\n}\n```\n";
    assert!(rules(src).is_empty(), "{:?}", run(src));
    let src = "```jsonc\n{\n  // c\n  \"a\": ?\n}\n```\n";
    let f = run(src);
    assert_eq!(at(src, &f[0]), "?");
}

#[test]
fn jsonc_strip_keeps_offsets() {
    let s = "{/* é */\"a\":1,}";
    let out = strip_jsonc(s);
    assert_eq!(out.len(), s.len());
    assert!(serde_json::from_str::<serde_json::Value>(&out).is_ok());
}

#[test]
fn json_placeholders_skipped() {
    assert!(rules("```json\n{\n  \"a\": 1,\n  ...\n}\n```\n").is_empty());
    assert!(rules("```json\n[1, …]\n```\n").is_empty());
    // `...` inside a string is not a placeholder.
    assert_eq!(
        rules("```json\n{\"a\": \"...\",}\n```\n"),
        ["codeblock/json"]
    );
}

#[test]
fn json5_ignored() {
    assert!(rules("```json5\n{a: 1,}\n```\n").is_empty());
}

#[test]
fn skip_flags() {
    assert!(rules("```json skip-lint\n{bad\n```\n").is_empty());
    assert!(rules("```json {skip-lint}\n{bad\n```\n").is_empty());
    assert!(rules("```toml no-lint\n= x\n```\n").is_empty());
    assert!(rules("```json title=\"x\"\n{bad\n```\n").len() == 1);
}

#[test]
fn toml_blocks() {
    assert!(rules("```toml\n[a]\nb = \"ü\"\n```\n").is_empty());
    let src = "```toml\n[a]\nb = \"ü\"\nc = = 1\n```\n";
    let f = run(src);
    assert_eq!(f.len(), 1);
    assert_eq!(f[0].rule, "codeblock/toml");
    assert!(at(src, &f[0]).starts_with('='), "{:?}", at(src, &f[0]));
    let src = "```toml\r\n[a]\r\nx = 1\r\nx = 2\r\n```\r\n";
    let f = run(src);
    assert_eq!(f.len(), 1);
    assert!(!at(src, &f[0]).contains('\n'));
}

#[test]
fn yaml_blocks() {
    assert!(rules("```yaml\na: 1\nb:\n  - ü\n---\nc: 2\n...\n```\n").is_empty());
    let src = "```yml\nkey: \"ö\nother: 1\n```\n";
    let f = run(src);
    assert_eq!(rules(src), ["codeblock/yaml"]);
    assert!(f[0].range.start > src.find("key").unwrap());
    let src = "```yaml\r\na: [1, 2\r\n```\r\n";
    assert_eq!(rules(src), ["codeblock/yaml"]);
    let src = "```yaml\na: b: c\n```\n";
    let f = run(src);
    assert_eq!(f.len(), 1);
    assert_eq!(at(src, &f[0]), ":");
}

#[test]
fn dot_blocks() {
    assert!(
        rules(
            "```dot\ndigraph G {\n  a -> b [label=\"x -- y\"];\n  c [label=<<b>hi</b>>];\n}\n```\n"
        )
        .is_empty()
    );
    assert!(rules("```graphviz\nstrict graph { a -- b }\n```\n").is_empty());
    let src = "```dot\ndigraph { ä -- b }\n```\n";
    let f = run(src);
    assert_eq!(f.len(), 1);
    assert_eq!(at(src, &f[0]), "-");
    assert_eq!(f[0].range.start, src.find("--").unwrap());
    let src = "```dot\ngraph { a -> b }\n```\n";
    assert_eq!(rules(src), ["codeblock/dot"]);
    let src = "```dot\ndigraph { a [x=1 }\n```\n";
    let f = run(src);
    assert_eq!(at(src, &f[0]), "}");
    let src = "```dot\ndigraph { a -> b\n```\n";
    assert_eq!(at(src, &run(src)[0]), "{");
    let src = "```dot\nflowchart { }\n```\n";
    assert_eq!(at(src, &run(src)[0]), "f");
    let src = "```dot\ndigraph { a [label=\"x] }\n```\n";
    assert_eq!(at(src, &run(src)[0]), "\"");
}

#[test]
fn empty_blocks() {
    let src = "text\n\n```rust\n  \n```\n";
    let f = run(src);
    assert_eq!(f.len(), 1);
    assert_eq!(f[0].rule, "codeblock/empty");
    assert_eq!(at(src, &f[0]), "```rust");
    let src = "```\r\n```\r\n";
    let f = run(src);
    assert_eq!(at(src, &f[0]), "```");
    assert!(rules("```\nx\n```\n").is_empty());
}

#[test]
fn indented_in_list_and_blockquote() {
    assert!(rules("- item\n\n  ```yaml\n  a: 1\n  b: 2\n  ```\n").is_empty());
    assert!(rules("> ```json\n> {\"a\": 1}\n> ```\n").is_empty());
}

#[test]
fn respects_config_off() {
    let a = Analyzed::new(SourceFile::new(
        PathBuf::from("/t/a.md"),
        PathBuf::from("a.md"),
        FileKind::Markdown,
        "```json\n{bad\n```\n".into(),
    ));
    let mut config = Config::default();
    config
        .rules
        .insert("codeblock/*".into(), crate::config::Level::Off);
    let ctx = FileCtx {
        a: &a,
        config: &config,
    };
    let mut out = Vec::new();
    super::check(&ctx, &mut out);
    assert!(out.is_empty());
}
