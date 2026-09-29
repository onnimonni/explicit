//! `docs/toc-sync`: a table of contents matches the headings that follow it.

use std::collections::HashSet;
use std::ops::Range;
use std::sync::LazyLock;

use regex::Regex;

use crate::diagnostic::{Finding, Severity};
use crate::extract::markdown::{Heading, MdDoc};
use crate::links::percent_decode;
use crate::rules::{FileCtx, Out};

const RULE: &str = "docs/toc-sync";

static OPEN_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)<!--\s*toc\s*-->").expect("hardcoded regex is valid"));
static CLOSE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)<!--\s*(?:tocstop|/toc)\s*-->").expect("hardcoded regex is valid")
});

struct Region {
    /// Where TOC links are collected.
    body: Range<usize>,
    /// Headings starting at or after this offset are expected in the TOC.
    after: usize,
    /// Heading to leave out (the "Contents" heading itself).
    skip_heading: Option<usize>,
    marker: bool,
}

struct Entry {
    anchor: String,
    range: Range<usize>,
}

pub fn check(ctx: &FileCtx, out: &mut Out) {
    let Some(md) = &ctx.a.md else { return };
    let src = ctx.src();
    for region in regions(src, md) {
        check_region(src, md, &region, out);
    }
}

fn regions(src: &str, md: &MdDoc) -> Vec<Region> {
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(open) = OPEN_RE.find_at(src, from) {
        from = open.end();
        if md.in_code(open.start()) {
            continue;
        }
        let Some(close) = CLOSE_RE
            .find_iter(&src[open.end()..])
            .map(|m| open.end() + m.start()..open.end() + m.end())
            .find(|m| !md.in_code(m.start))
        else {
            break;
        };
        out.push(Region {
            body: open.end()..close.start,
            after: close.end,
            skip_heading: None,
            marker: true,
        });
        from = close.end;
    }
    if !out.is_empty() {
        return out;
    }
    for (i, h) in md.headings.iter().enumerate() {
        let t = h.text.trim().to_lowercase();
        if !matches!(t.as_str(), "table of contents" | "contents" | "toc") {
            continue;
        }
        let next = md.headings.get(i + 1).map_or(src.len(), |n| n.range.start);
        let Some(list) = md
            .lists
            .iter()
            .filter(|l| l.depth == 0 && l.range.start >= h.range.end && l.range.start < next)
            .min_by_key(|l| l.range.start)
        else {
            continue;
        };
        out.push(Region {
            body: list.range.clone(),
            after: h.range.end,
            skip_heading: Some(i),
            marker: false,
        });
    }
    out
}

fn check_region(src: &str, md: &MdDoc, region: &Region, out: &mut Out) {
    let entries: Vec<Entry> = md
        .links
        .iter()
        .filter(|l| {
            !l.is_image
                && l.range.start >= region.body.start
                && l.range.end <= region.body.end
                && l.dest.starts_with('#')
        })
        .map(|l| Entry {
            anchor: percent_decode(&l.dest[1..]),
            range: l.range.clone(),
        })
        .collect();
    if entries.is_empty() && !region.marker {
        return;
    }
    let after: Vec<&Heading> = md
        .headings
        .iter()
        .enumerate()
        .filter(|(i, h)| h.range.start >= region.after && Some(*i) != region.skip_heading)
        .map(|(_, h)| h)
        .collect();

    let matched: Vec<u8> = after
        .iter()
        .filter(|h| entries.iter().any(|e| e.anchor == h.anchor))
        .map(|h| h.level)
        .collect();
    let (min_l, max_l) = match (matched.iter().min(), matched.iter().max()) {
        (Some(&a), Some(&b)) => (a, b),
        _ => (after.iter().map(|h| h.level).min().unwrap_or(1), 6),
    };
    let expected: Vec<&Heading> = after
        .iter()
        .copied()
        .filter(|h| (min_l..=max_l).contains(&h.level))
        .collect();

    let mut findings = Vec::new();
    let all_anchors: HashSet<&str> = md.anchors().collect();
    let mut seen: HashSet<&str> = HashSet::new();
    for e in &entries {
        if !all_anchors.contains(e.anchor.as_str()) {
            findings.push(
                Finding::new(
                    RULE,
                    Severity::Warning,
                    e.range.clone(),
                    format!(
                        "Stale table of contents entry: no heading with anchor #{}",
                        e.anchor
                    ),
                )
                .help("Update or remove the entry"),
            );
        } else if !seen.insert(e.anchor.as_str()) {
            findings.push(Finding::new(
                RULE,
                Severity::Warning,
                e.range.clone(),
                format!("Table of contents lists #{} more than once", e.anchor),
            ));
        }
    }
    for h in &expected {
        if !entries.iter().any(|e| e.anchor == h.anchor) {
            let text = src[h.range.clone()].trim_end();
            let range =
                h.range.start..h.range.start + text.find(['\r', '\n']).unwrap_or(text.len());
            findings.push(
                Finding::new(
                    RULE,
                    Severity::Warning,
                    range,
                    format!(
                        "Heading \"{}\" is missing from the table of contents",
                        h.text
                    ),
                )
                .help(format!(
                    "Add `- [{}](#{})` to the table of contents",
                    h.text, h.anchor
                )),
            );
        }
    }
    // Order: first occurrences of expected anchors must follow heading order.
    let expected_set: HashSet<&str> = expected.iter().map(|h| h.anchor.as_str()).collect();
    let mut firsts: Vec<&Entry> = Vec::new();
    let mut dedup: HashSet<&str> = HashSet::new();
    for e in &entries {
        if expected_set.contains(e.anchor.as_str()) && dedup.insert(e.anchor.as_str()) {
            firsts.push(e);
        }
    }
    let order: Vec<&str> = expected
        .iter()
        .map(|h| h.anchor.as_str())
        .filter(|a| dedup.contains(a))
        .collect();
    if let Some((e, want)) = firsts.iter().zip(&order).find(|(e, w)| e.anchor != **w) {
        findings.push(Finding::new(
            RULE,
            Severity::Warning,
            e.range.clone(),
            format!(
                "Table of contents entry #{} is out of order; expected #{want} here",
                e.anchor
            ),
        ));
    }

    if findings.is_empty() {
        return;
    }
    findings.sort_by_key(|f| f.range.start);
    if region.marker {
        let nl = if src.contains("\r\n") { "\r\n" } else { "\n" };
        let fix = generate(src, &expected, min_l, nl);
        let f = findings.remove(0);
        findings.insert(0, f.fix(region.body.clone(), fix));
    }
    out.extend(findings);
}

/// Heading inline source (keeps code spans and emphasis), without ATX `#` markers, a closing
/// `#` sequence, a `{#id}` attribute or a setext underline. Falls back to the plain text
/// (brackets escaped) when the source holds links or HTML, which cannot nest in link text.
fn link_text(src: &str, h: &Heading) -> String {
    let raw = src[h.range.clone()].trim_end();
    let mut text = if h.setext {
        let body = raw.rsplit_once('\n').map_or(raw, |(b, _)| b);
        body.lines().map(str::trim).collect::<Vec<_>>().join(" ")
    } else {
        let line = raw.lines().next().unwrap_or("").trim();
        let line = line.trim_start_matches('#').trim();
        // Closing sequence: `#`s preceded by a space (or the whole content).
        let stripped = line.trim_end_matches('#');
        if stripped.is_empty() || stripped.ends_with([' ', '\t']) {
            stripped.trim_end().to_string()
        } else {
            line.to_string()
        }
    };
    if h.explicit_id
        && text.ends_with('}')
        && let Some(i) = text.rfind('{')
    {
        text.truncate(i);
        text = text.trim_end().to_string();
    }
    let plain_bracket = |t: &str| {
        t.char_indices()
            .any(|(i, c)| matches!(c, '[' | ']') && !t[..i].ends_with('\\'))
    };
    if text.is_empty() || text.contains('<') || plain_bracket(&text) {
        return h.text.replace('[', "\\[").replace(']', "\\]");
    }
    text
}

/// Nested list with 2-space indents, framed by blank lines.
fn generate(src: &str, headings: &[&Heading], min_level: u8, nl: &str) -> String {
    if headings.is_empty() {
        return nl.to_string();
    }
    let lines: Vec<String> = headings
        .iter()
        .map(|h| {
            let indent = "  ".repeat(usize::from(h.level.saturating_sub(min_level)));
            let text = link_text(src, h);
            format!("{indent}- [{text}](#{})", h.anchor)
        })
        .collect();
    format!("{nl}{nl}{}{nl}{nl}", lines.join(nl))
}
