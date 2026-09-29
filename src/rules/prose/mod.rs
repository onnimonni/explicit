//! Built-in prose style rules.
//!
//! - `prose/inclusive`, `prose/simplify`, `prose/terminology`: data-driven word lists
//!   (`inclusive.toml`, `simplify.toml`, `terminology.toml`), see `lists.rs`.
//! - `prose/passive`, `prose/weasel`, `prose/there-is`, `prose/so-start`,
//!   `prose/sentence-length`, `prose/sentence-spacing`: per segment / sentence, see `sentences.rs`.
//! - `prose/readability`, `prose/consistency`, `prose/acronym-defined`, `prose/smart-quotes`:
//!   whole-file checks, see `file.rs`.
//!
//! Stock AI phrases live in the slop catalogue and a/an, repeated words and spacing in Harper;
//! nothing here duplicates those.

mod file;
mod lists;
mod sentences;
#[cfg(test)]
mod tests;

use std::ops::Range;
use std::sync::LazyLock;

use regex::Regex;

use super::{FileCtx, Out};
use crate::diagnostic::Severity;
use crate::segment::{Segment, SegmentKind};

pub fn check(ctx: &FileCtx, out: &mut Out) {
    if !ctx.family_enabled("prose/") {
        return;
    }
    lists::check(ctx, out);
    sentences::check(ctx, out);
    file::check(ctx, out);
}

pub(crate) static WORD_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"[\p{L}\p{N}][\p{L}\p{N}'’-]*").expect("hardcoded regex is valid")
});

/// Default severity of a prose rule; the engine applies config overrides.
fn sev(rule: &str) -> Severity {
    super::default_severity(rule).unwrap_or(Severity::Warning)
}

/// Quoted text is someone else's words: style advice does not apply there.
fn quoted(seg: &Segment) -> bool {
    seg.kind == SegmentKind::BlockQuote
}

/// Match the capitalization of the replaced text.
fn match_case(replacement: &str, original: &str) -> String {
    let letters = || original.chars().filter(|c| c.is_alphabetic());
    if letters().count() > 1 && letters().all(char::is_uppercase) {
        replacement.to_uppercase()
    } else if original.chars().next().is_some_and(char::is_uppercase) {
        capitalize(replacement)
    } else {
        replacement.to_string()
    }
}

fn capitalize(w: &str) -> String {
    let mut c = w.chars();
    c.next()
        .map(|f| f.to_uppercase().chain(c).collect())
        .unwrap_or_default()
}

/// Whether the match at `r` is part of a path, domain, handle or identifier (`github.com`,
/// `foo/Github`, `@guys`, `my_master`), where prose rules must not apply.
fn identifier_like(text: &str, r: &Range<usize>) -> bool {
    let prev = text[..r.start].chars().next_back();
    let mut after = text[r.end..].chars();
    let next = after.next();
    let next2 = after.next();
    if prev.is_some_and(|c| matches!(c, '.' | '/' | '@' | '_' | '\\' | '#' | '$' | '~')) {
        return true;
    }
    match next {
        Some('/' | '@' | '_' | '\\') => true,
        Some('.' | ':') => next2.is_some_and(|c| c.is_alphanumeric()),
        _ => false,
    }
}

const ABBREVIATIONS: &[&str] = &[
    "e.g", "i.e", "etc", "vs", "cf", "mr", "mrs", "ms", "dr", "st", "no", "fig", "approx", "resp",
    "incl", "al",
];

/// Sentence ranges in `text` (trimmed, non-empty). Sentences end at `.`, `!` or `?` followed
/// by whitespace, at blank lines, and at the end of the text.
pub(crate) fn sentences(text: &str) -> Vec<Range<usize>> {
    let b = text.as_bytes();
    let mut out = Vec::new();
    let mut start = 0;
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        let mut cut = None;
        if matches!(c, b'.' | b'!' | b'?') {
            let mut j = i + 1;
            loop {
                if j < b.len() && matches!(b[j], b'"' | b'\'' | b')' | b']' | b'.' | b'!' | b'?') {
                    j += 1;
                } else if text[j..].starts_with(['”', '’']) {
                    j += 3;
                } else {
                    break;
                }
            }
            let at_space = j >= b.len() || b[j].is_ascii_whitespace();
            if at_space && !(c == b'.' && abbreviation_before(text, i)) {
                cut = Some(j);
            }
        } else if c == b'\n' {
            let rest = &text[i + 1..];
            let line = rest.split('\n').next().unwrap_or("");
            if line.trim().is_empty() && rest.contains('\n') {
                cut = Some(i);
            }
        }
        if let Some(end) = cut {
            push_trimmed(text, start..end, &mut out);
            start = end;
            i = end.max(i + 1);
        } else {
            i += 1;
        }
    }
    push_trimmed(text, start..b.len(), &mut out);
    out
}

fn abbreviation_before(text: &str, dot: usize) -> bool {
    let head = &text[..dot];
    let tok_start = head
        .char_indices()
        .rev()
        .find(|(_, c)| !(c.is_alphanumeric() || *c == '.'))
        .map_or(0, |(i, c)| i + c.len_utf8());
    let tok = &head[tok_start..];
    // Single-letter initials: "J. Smith".
    if tok.chars().count() == 1 && tok.chars().all(char::is_uppercase) {
        return true;
    }
    ABBREVIATIONS.contains(&tok.to_lowercase().as_str())
}

fn push_trimmed(text: &str, r: Range<usize>, out: &mut Vec<Range<usize>>) {
    let s = &text[r.clone()];
    let lead = s.len() - s.trim_start().len();
    let trail = s.len() - s.trim_end().len();
    if lead + trail < s.len() && s.chars().any(char::is_alphanumeric) {
        out.push(r.start + lead..r.end - trail);
    }
}

/// Markdown segments grouped between headings (headings excluded).
fn md_sections(segments: &[Segment]) -> Vec<Vec<&Segment>> {
    let mut out: Vec<Vec<&Segment>> = Vec::new();
    let mut cur: Vec<&Segment> = Vec::new();
    for s in segments.iter().filter(|s| !s.kind.is_comment()) {
        if s.kind == SegmentKind::Heading {
            if !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
            continue;
        }
        cur.push(s);
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// Whether `seg.text[r]` is the unmodified source (no blanked markup inside), so a fix over
/// that range cannot remove markup such as `**` or comment markers.
fn verbatim(src: &str, seg: &Segment, r: &Range<usize>) -> bool {
    src.get(seg.abs(r.clone())) == Some(&seg.text[r.clone()])
}
