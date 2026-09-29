//! ATX heading spacing (MD018-MD021), heading-like paragraphs, heading length and empty sections.

use std::sync::LazyLock;

use regex::{Captures, Regex};

use super::Ctx;
use crate::rules::Out;

static MISSING_SPACE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^ {0,3}(#{1,6})[^#\s]").expect("hardcoded regex is valid"));
static CLOSED_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^( {0,3})(#{1,6})([ \t]*)([^# \t].*?)([ \t]*)(#+)[ \t]*$")
        .expect("hardcoded regex is valid")
});
static TOO_DEEP_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^ {0,3}#{7,}(\s|$)").expect("hardcoded regex is valid"));

pub fn check(c: &mut Ctx, out: &mut Out) {
    let missing = c.on("md/no-missing-space-atx");
    let closed = c.on("md/no-missing-space-closed-atx");
    let deep = c.on("md/no-heading-like-paragraph");
    if missing || closed || deep {
        for i in 0..c.lines.len() {
            if !c.text_line(i) || c.lines.v[i].table {
                continue;
            }
            line_checks(c, i, missing, closed, deep, out);
        }
    }
    if c.on("md/no-multiple-space-atx") {
        multiple_space(c, out);
    }
    if c.on("md/max-heading-length") {
        let max = c.fc.config.markdown.max_heading_length;
        for h in &c.md.headings {
            let n = h.text.chars().count();
            if max > 0 && n > max {
                out.push(c.finding(
                    "md/max-heading-length",
                    h.range.clone(),
                    format!("Heading is {n} characters, limit is {max}"),
                ));
            }
        }
    }
    if c.on("md/no-empty-section") {
        let hs = &c.md.headings;
        for w in hs.windows(2) {
            let (h, n) = (&w[0], &w[1]);
            if n.level <= h.level
                && h.range.end <= n.range.start
                && c.src[h.range.end..n.range.start].trim().is_empty()
            {
                out.push(
                    c.finding(
                        "md/no-empty-section",
                        h.range.clone(),
                        format!("Section \"{}\" has no content", h.text),
                    )
                    .help("Add content or remove the heading"),
                );
            }
        }
    }
}

/// Closed ATX heading with a missing inner space, excluding names like `C#`.
fn closed_missing<'t>(body: &'t str) -> Option<Captures<'t>> {
    let m = CLOSED_RE.captures(body)?;
    let (open_sp, text, close_sp, close) = (&m[3], &m[4], &m[5], &m[6]);
    if text.ends_with('\\') || (!open_sp.is_empty() && !close_sp.is_empty()) {
        return None;
    }
    let last = text.split_whitespace().last().unwrap_or("");
    let lang = !open_sp.is_empty()
        && close.len() == 1
        && last.chars().count() == 1
        && last.chars().all(|ch| ch.is_alphabetic());
    (!lang).then_some(m)
}

fn line_checks(c: &Ctx, i: usize, missing: bool, closed: bool, deep: bool, out: &mut Out) {
    let l = &c.lines.v[i];
    let body = &c.src[l.body..l.end];
    if deep && TOO_DEEP_RE.is_match(body) {
        out.push(
            c.finding(
                "md/no-heading-like-paragraph",
                l.body..l.end,
                "Line starts with 7 or more '#' and is not a heading",
            )
            .help("Headings go up to 6 levels; use ###### or plain text"),
        );
        return;
    }
    let cm = closed_missing(body);
    if let Some(m) = &cm {
        if closed {
            let (g3, g5) = (
                m.get(3).expect("regex group 3 is not optional"),
                m.get(5).expect("regex group 5 is not optional"),
            );
            let at = if g3.as_str().is_empty() {
                l.body + g3.start()
            } else {
                l.body + g5.start()
            };
            let mut f = c.finding(
                "md/no-missing-space-closed-atx",
                l.body..l.end,
                "Missing space inside closed ATX heading hashes",
            );
            // Insert both spaces when both are missing by replacing the text part.
            if g3.as_str().is_empty() && g5.as_str().is_empty() {
                let t = m.get(4).expect("regex group 4 is not optional");
                f = f.fix(
                    l.body + t.start()..l.body + t.end(),
                    format!(" {} ", t.as_str()),
                );
            } else {
                f = f.fix(at..at, " ");
            }
            out.push(f);
        }
        return;
    }
    if !missing {
        return;
    }
    let Some(m) = MISSING_SPACE_RE.captures(body) else {
        return;
    };
    let hashes = m.get(1).expect("regex group 1 is not optional");
    let rest = &body[hashes.end()..];
    let word = rest.split_whitespace().next().unwrap_or("");
    // `#123` issue references and keycap emoji are not headings.
    if word.chars().all(|ch| ch.is_ascii_digit()) || rest.starts_with(['\u{fe0f}', '\u{20e3}']) {
        return;
    }
    let at = l.body + hashes.end();
    out.push(
        c.finding(
            "md/no-missing-space-atx",
            l.body..l.end,
            "Missing space after '#' in ATX heading",
        )
        .fix(at..at, " "),
    );
}

fn multiple_space(c: &Ctx, out: &mut Out) {
    for h in c.md.headings.iter().filter(|h| !h.setext) {
        let i = c.lines.index_of(h.range.start);
        let l = &c.lines.v[i];
        let line = &c.src[l.body..l.end];
        let lead = line.len() - line.trim_start().len();
        let hashes = line[lead..].bytes().take_while(|&b| b == b'#').count();
        let s = lead + hashes;
        let ws = line[s..].len() - line[s..].trim_start().len();
        let t = line.trim_end();
        if t.len() <= s + ws {
            continue;
        }
        if ws > 1 {
            let r = l.body + s..l.body + s + ws;
            out.push(
                c.finding(
                    "md/no-multiple-space-atx",
                    r.clone(),
                    "Multiple spaces after '#' in ATX heading",
                )
                .fix(r, " "),
            );
        }
        // Closed heading: spaces before the closing hashes.
        if h.explicit_id || !t.ends_with('#') {
            continue;
        }
        let inner = t.trim_end_matches('#');
        let cws = inner.len() - inner.trim_end().len();
        if cws > 1 && inner.trim_end().len() > s + ws {
            let e = l.body + inner.len();
            let r = e - cws..e;
            out.push(
                c.finding(
                    "md/no-multiple-space-atx",
                    r.clone(),
                    "Multiple spaces before closing '#' in ATX heading",
                )
                .fix(r, " "),
            );
        }
    }
}
