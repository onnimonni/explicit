//! Syntax checks for data and graph languages in fenced code blocks.
//!
//! Opt out per block with a `skip-lint` flag in the info string:
//! ```` ```json skip-lint ```` or ```` ```json {skip-lint} ````
//! (`no-lint` and `ignore` are accepted aliases).

mod data;
mod dot;
mod json;

use std::ops::Range;

use super::diagram::fence_body;
use super::{FileCtx, Out};
use crate::source::floor_char;

/// Info-string flags that skip linting of a block.
pub const SKIP_FLAGS: &[&str] = &["skip-lint", "no-lint", "ignore"];

pub fn check(ctx: &FileCtx, out: &mut Out) {
    let Some(md) = &ctx.a.md else { return };
    if !ctx.family_enabled("codeblock/") {
        return;
    }
    let src = ctx.src();
    for block in md.code_blocks.iter().filter(|b| b.fenced) {
        let info = info_string(src, block.range.start);
        let mut words = info
            .split(|c: char| c.is_whitespace() || matches!(c, ',' | '{' | '}'))
            .filter(|w| !w.is_empty());
        let lang = words.next().unwrap_or("").to_ascii_lowercase();
        if words.any(|w| SKIP_FLAGS.iter().any(|f| w.eq_ignore_ascii_case(f))) {
            continue;
        }
        // Content lines inside block quotes carry `>` prefixes; skip those blocks.
        if in_blockquote(src, block.range.start) {
            continue;
        }
        let (body, offset) = fence_body(src, block.range.clone()).unwrap_or(("", block.range.end));
        if body.trim().is_empty() {
            if ctx.enabled("codeblock/empty") {
                let end = src[block.range.clone()]
                    .find(['\r', '\n'])
                    .map_or(block.range.end, |i| block.range.start + i);
                out.push(
                    crate::diagnostic::Finding::new(
                        "codeblock/empty",
                        crate::diagnostic::Severity::Warning,
                        block.range.start..end,
                        "Fenced code block is empty",
                    )
                    .help("Add content or remove the block"),
                );
            }
            continue;
        }
        match lang.as_str() {
            "json" if ctx.enabled("codeblock/json") => json::check(body, offset, false, out),
            "jsonc" if ctx.enabled("codeblock/json") => json::check(body, offset, true, out),
            "toml" if ctx.enabled("codeblock/toml") => data::check_toml(body, offset, out),
            "yaml" | "yml" if ctx.enabled("codeblock/yaml") => data::check_yaml(body, offset, out),
            "dot" | "graphviz" | "gv" if ctx.enabled("codeblock/dot") => {
                dot::check(body, offset, out)
            }
            _ => {}
        }
    }
}

/// Text after the opening fence characters on the fence line.
fn info_string(src: &str, start: usize) -> &str {
    let line = &src[start..];
    let line = &line[..line.find('\n').unwrap_or(line.len())];
    line.trim_start()
        .trim_start_matches(['`', '~'])
        .trim_end_matches('\r')
        .trim()
}

fn in_blockquote(src: &str, start: usize) -> bool {
    let line_start = src[..start].rfind('\n').map_or(0, |i| i + 1);
    src[line_start..start].contains('>')
}

/// A one-character range at byte `pos` of `body`, made absolute with `offset`.
pub(crate) fn point(body: &str, offset: usize, pos: usize) -> Range<usize> {
    let pos = floor_char(body, pos.min(body.len()));
    let len = body[pos..].chars().next().map_or(0, char::len_utf8);
    offset + pos..offset + pos + len
}

/// Byte offset of 1-based `line` / 1-based byte `col` in `body`.
pub(crate) fn line_col_to_byte(body: &str, line: usize, col: usize) -> usize {
    let mut start = 0;
    for _ in 1..line {
        match body[start..].find('\n') {
            Some(i) => start += i + 1,
            None => return body.len(),
        }
    }
    (start + col.saturating_sub(1)).min(body.len())
}

/// Lines with `...` / `…` placeholders outside of quoted strings (common elisions in docs).
/// A bare `...` line is accepted when `yaml_doc_end` is set, since YAML uses it to end documents.
pub(crate) fn has_placeholder(body: &str, yaml_doc_end: bool) -> bool {
    body.lines().any(|line| {
        let t = line.trim();
        if yaml_doc_end && line.trim_end() == "..." {
            return false;
        }
        let bare = strip_strings(t);
        bare.contains("...") || bare.contains('…')
    })
}

fn strip_strings(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut quote: Option<char> = None;
    let mut escaped = false;
    for c in line.chars() {
        match quote {
            Some(q) => {
                if escaped {
                    escaped = false;
                } else if c == '\\' && q == '"' {
                    escaped = true;
                } else if c == q {
                    quote = None;
                }
            }
            None if c == '"' || c == '\'' => quote = Some(c),
            None => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests;
