//! Inline span rules: MD011, MD033, MD037, MD039, MD059.

use std::sync::LazyLock;

use regex::Regex;

use super::Ctx;
use crate::extract::markdown::{LinkKind, normalize_label};
use crate::rules::Out;

static REVERSED_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(^|[^\\\]])\(([^()]+)\)\[([^\]^][^\]]*)\]").expect("hardcoded regex is valid")
});
static TAG_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^<([A-Za-z][A-Za-z0-9-]*)").expect("hardcoded regex is valid"));

pub fn check(c: &mut Ctx, out: &mut Out) {
    if c.on("md/no-reversed-links") {
        reversed(c, out);
    }
    if c.on("md/no-space-in-emphasis") {
        emphasis_space(c, out);
    }
    if c.on("md/no-space-in-links") {
        link_space(c, out);
    }
    if c.on("md/descriptive-link-text") {
        descriptive(c, out);
    }
    if c.on("md/no-inline-html") {
        inline_html(c, out);
    }
}

fn reversed(c: &Ctx, out: &mut Out) {
    for i in 0..c.lines.len() {
        if !c.text_line(i) {
            continue;
        }
        let body = c.lines.v[i].body;
        let m = c.masked(i);
        for cap in REVERSED_RE.captures_iter(&m) {
            let (text, dest) = (
                cap.get(2).expect("regex group 2 is not optional"),
                cap.get(3).expect("regex group 3 is not optional"),
            );
            let end = dest.end() + 1;
            if m[end..].starts_with('(')
                || c.md
                    .ref_defs
                    .iter()
                    .any(|d| d.label == normalize_label(dest.as_str()))
            {
                continue;
            }
            let r = body + text.start() - 1..body + end;
            out.push(
                c.finding(
                    "md/no-reversed-links",
                    r.clone(),
                    "Reversed link syntax (text)[url]",
                )
                .fix(
                    r,
                    // Masked text blanks code spans; rebuild from the raw source.
                    format!(
                        "[{}]({})",
                        &c.src[body + text.start()..body + text.end()],
                        &c.src[body + dest.start()..body + dest.end()]
                    ),
                ),
            );
        }
    }
}

fn emphasis_space(c: &Ctx, out: &mut Out) {
    let delim = |p: usize| {
        c.md.emphasis.iter().chain(&c.md.strong).any(|r| {
            r.start == p || r.end.checked_sub(1) == Some(p) || r.end.checked_sub(2) == Some(p)
        })
    };
    for i in 0..c.lines.len() {
        let l = &c.lines.v[i];
        if !c.text_line(i) || c.md.thematic_breaks.iter().any(|t| t.contains(&l.start)) {
            continue;
        }
        let m = c.masked(i);
        let b = m.as_bytes();
        let first = b.iter().position(|&x| x != b' ' && x != b'\t').unwrap_or(0);
        let mut p = 0;
        while p < b.len() {
            let ch = b[p];
            if ch != b'*' && ch != b'_' {
                p += 1;
                continue;
            }
            let n = b[p..].iter().take_while(|&&x| x == ch).count();
            let prev = p.checked_sub(1).map(|k| b[k]);
            let list_marker = p == first && ch == b'*' && n == 1;
            let bad_prev =
                prev.is_some_and(|x| x == b'\\' || (ch == b'_' && x.is_ascii_alphanumeric()));
            if n > 3 || list_marker || bad_prev || delim(l.body + p) {
                p += n;
                continue;
            }
            let s = p + n;
            // Closing run: exactly `n` of `ch`.
            let close = (s..b.len()).find(|&q| {
                b[q] == ch && b[q..].iter().take_while(|&&x| x == ch).count() == n && b[q - 1] != ch
            });
            let Some(q) = close else {
                p += n;
                continue;
            };
            let after = b.get(q + n).copied();
            let inner = &m[s..q];
            let is_ws = |x: u8| x == b' ' || x == b'\t';
            let spaced =
                inner.bytes().next().is_some_and(is_ws) || inner.bytes().last().is_some_and(is_ws);
            let ok_after = after.is_none_or(|x| !(x.is_ascii_alphanumeric() || x == ch));
            if !spaced || !ok_after || !inner.chars().any(char::is_alphabetic) || delim(l.body + q)
            {
                p = if spaced { s } else { q + n };
                continue;
            }
            c.edge_spaces(
                "md/no-space-in-emphasis",
                "emphasis markers",
                l.body + s..l.body + q,
                (n, n),
                out,
            );
            p = q + n;
        }
    }
}

/// Byte offset of the `]` closing the link text that opens at `raw[0] == '['`.
fn text_end(raw: &str) -> Option<usize> {
    let b = raw.as_bytes();
    let mut depth = 0usize;
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'\\' => i += 1,
            b'`' => {
                let n = b[i..].iter().take_while(|&&x| x == b'`').count();
                let tick = "`".repeat(n);
                i = raw[i + n..].find(&tick).map_or(i + n, |k| i + n + k + n) - 1;
            }
            b'[' => depth += 1,
            b']' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

fn link_space(c: &Ctx, out: &mut Out) {
    for l in &c.md.links {
        if l.is_image || !matches!(l.kind, LinkKind::Inline | LinkKind::Reference) {
            continue;
        }
        let raw = &c.src[l.range.clone()];
        if !raw.starts_with('[') {
            continue;
        }
        let Some(e) = text_end(raw) else { continue };
        let inner = &raw[1..e];
        let t = inner.trim_matches([' ', '\t']);
        if t.is_empty() || t.len() == inner.len() {
            continue;
        }
        c.edge_spaces(
            "md/no-space-in-links",
            "link text",
            l.range.start + 1..l.range.start + e,
            (1, 1),
            out,
        );
    }
}

fn descriptive(c: &Ctx, out: &mut Out) {
    let bad: Vec<String> =
        c.fc.config
            .markdown
            .non_descriptive_link_text
            .iter()
            .map(|s| normalize_label(s))
            .collect();
    for l in &c.md.links {
        if l.is_image || !matches!(l.kind, LinkKind::Inline | LinkKind::Reference) {
            continue;
        }
        let t = normalize_label(l.text.trim_end_matches(['.', ':', '!', '…']));
        if bad.contains(&t) {
            out.push(
                c.finding(
                    "md/descriptive-link-text",
                    l.range.clone(),
                    format!("Link text \"{}\" is not descriptive", l.text.trim()),
                )
                .help("Describe the link target so it makes sense out of context"),
            );
        }
    }
}

fn inline_html(c: &Ctx, out: &mut Out) {
    let allowed = &c.fc.config.markdown.allowed_html;
    let mut ranges: Vec<_> = c.md.html.iter().collect();
    ranges.sort_by_key(|r| r.start);
    let mut in_comment = false;
    for r in ranges {
        let raw = &c.src[r.clone()];
        let mut i = 0;
        while i < raw.len() {
            if in_comment {
                match raw[i..].find("-->") {
                    Some(k) => {
                        i += k + 3;
                        in_comment = false;
                    }
                    None => break,
                }
                continue;
            }
            let Some(k) = raw[i..].find('<') else { break };
            i += k;
            if raw[i..].starts_with("<!--") {
                in_comment = true;
                i += 4;
                continue;
            }
            if let Some(m) = TAG_RE.captures(&raw[i..]) {
                let name = m.get(1).expect("regex group 1 is not optional");
                if !allowed
                    .iter()
                    .any(|a| a.eq_ignore_ascii_case(name.as_str()))
                {
                    let s = r.start + i + 1;
                    out.push(
                        c.finding(
                            "md/no-inline-html",
                            s..s + name.len(),
                            format!("Inline HTML element <{}>", name.as_str()),
                        )
                        .help("Use Markdown or add the element to markdown.allowed_html"),
                    );
                }
            }
            i += 1;
        }
    }
}
