//! Heading rules: MD001, MD003, MD022, MD023, MD024, MD025, MD026, MD041, heading case.

use std::collections::HashMap;
use std::ops::Range;
use std::sync::LazyLock;

use regex::Regex;

use super::Ctx;
use crate::extract::markdown::Heading;
use crate::rules::Out;

static FM_TITLE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?m)^\s*["']?title["']?\s*[:=]"#).expect("hardcoded regex is valid")
});
static LINK_DEST_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\]\([^)]*\)|\]\[[^\]]*\]").expect("hardcoded regex is valid"));
static ATTR_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\s*\{[^}]*\}\s*$").expect("hardcoded regex is valid"));

const PUNCT: &str = ".,;:!。，；：！";

/// Source lines of a heading: (first, last) line index.
fn span(c: &Ctx, h: &Heading) -> (usize, usize) {
    let b = c.src.as_bytes();
    let mut e = h.range.end;
    while e > h.range.start && matches!(b[e - 1], b'\n' | b'\r') {
        e -= 1;
    }
    (
        c.lines.index_of(h.range.start),
        c.lines.index_of(e.saturating_sub(1).max(h.range.start)),
    )
}

/// Byte range of the heading's text in the source (ATX markers, closing hashes and attributes removed).
fn content_range(c: &Ctx, h: &Heading) -> Range<usize> {
    let (first, _) = span(c, h);
    let l = &c.lines.v[first];
    let line = &c.src[l.body..l.end];
    let lead = line.len() - line.trim_start().len();
    let mut s = lead;
    let mut t = line.trim_end();
    if !h.setext {
        s += line[lead..].bytes().take_while(|&b| b == b'#').count();
        s += line[s..].len() - line[s..].trim_start().len();
        if let Some(m) = ATTR_RE.find(t) {
            t = &t[..m.start()];
        }
        let stripped = t.trim_end_matches('#');
        if stripped.len() < t.len() && (stripped.ends_with([' ', '\t']) || stripped.len() <= s) {
            t = stripped.trim_end();
        }
    } else if let Some(m) = ATTR_RE.find(t) {
        t = &t[..m.start()];
    }
    let e = t.len().max(s);
    l.body + s..l.body + e
}

pub fn fm_title(c: &Ctx) -> bool {
    c.md.front_matter
        .as_ref()
        .is_some_and(|r| FM_TITLE_RE.is_match(&c.src[r.clone()]))
}

pub fn check(c: &mut Ctx, out: &mut Out) {
    let md = c.md;
    if c.on("md/heading-increment") {
        let mut prev = 0u8;
        for h in &md.headings {
            if prev > 0 && h.level > prev + 1 {
                out.push(
                    c.finding(
                        "md/heading-increment",
                        h.range.clone(),
                        format!("Heading level jumps from h{prev} to h{}", h.level),
                    )
                    .help(format!("Use h{} here", prev + 1)),
                );
            }
            prev = h.level;
        }
    }

    if c.on("md/heading-style") {
        let style = c.fc.config.markdown.heading_style.as_str();
        let setext = match style {
            "atx" => Some(false),
            "setext" => Some(true),
            _ => md.headings.first().map(|h| h.setext),
        };
        if let Some(want_setext) = setext {
            for h in &md.headings {
                let bad = if want_setext {
                    !h.setext && h.level <= 2
                } else {
                    h.setext
                };
                if bad {
                    let (want, got) = if want_setext {
                        ("setext", "ATX")
                    } else {
                        ("ATX", "setext")
                    };
                    out.push(c.finding(
                        "md/heading-style",
                        h.range.clone(),
                        format!("Expected {want} heading, found {got}"),
                    ));
                }
            }
        }
    }

    if c.on("md/blanks-around-headings") {
        for h in &md.headings {
            let (first, last) = span(c, h);
            if c.missing_blank_before(first) {
                let l = &c.lines.v[first];
                let f = c.finding(
                    "md/blanks-around-headings",
                    l.start..l.end,
                    "Heading should be preceded by a blank line",
                );
                let f = c.with_blank_line(f, l.start, first);
                out.push(f);
            }
            if c.missing_blank_after(last) {
                let at = c.lines.v[last + 1].start;
                let f = c.finding(
                    "md/blanks-around-headings",
                    c.eol(last),
                    "Heading should be followed by a blank line",
                );
                let f = c.with_blank_line(f, at, last);
                out.push(f);
            }
        }
    }

    if c.on("md/heading-start-left") {
        for h in &md.headings {
            if c.in_list(h.range.start) {
                continue;
            }
            let (first, _) = span(c, h);
            let l = &c.lines.v[first];
            let t = &c.src[l.body..l.end];
            let lead = t.len() - t.trim_start().len();
            if lead > 0 {
                let r = l.body..l.body + lead;
                let f = c.finding("md/heading-start-left", r.clone(), "Heading is indented");
                out.push(if h.setext { f } else { f.fix(r, "") });
            }
        }
    }

    if c.on("md/no-duplicate-heading") {
        let mut stack: Vec<usize> = Vec::new();
        let mut seen: HashMap<(Option<usize>, String), usize> = HashMap::new();
        for (i, h) in md.headings.iter().enumerate() {
            while stack
                .last()
                .is_some_and(|&p| md.headings[p].level >= h.level)
            {
                stack.pop();
            }
            let key = (
                stack.last().copied(),
                h.text.split_whitespace().collect::<Vec<_>>().join(" "),
            );
            if let Some(&j) = seen.get(&key) {
                let (line, _) = c.fc.a.file.line_col(md.headings[j].range.start);
                out.push(c.finding(
                    "md/no-duplicate-heading",
                    h.range.clone(),
                    format!(
                        "Duplicate sibling heading \"{}\" (first on line {line})",
                        h.text
                    ),
                ));
            } else {
                seen.insert(key, i);
            }
            stack.push(i);
        }
    }

    if c.on("md/single-h1") {
        let title = fm_title(c);
        let h1s: Vec<&Heading> = md.headings.iter().filter(|h| h.level == 1).collect();
        let skip = if title { 0 } else { 1 };
        for h in h1s.into_iter().skip(skip) {
            let msg = if title {
                "Top-level heading duplicates the front matter title"
            } else {
                "Multiple top-level headings"
            };
            out.push(
                c.finding("md/single-h1", h.range.clone(), msg)
                    .help("Use a single h1 per document"),
            );
        }
    }

    if c.on("md/no-trailing-punctuation") {
        for h in &md.headings {
            let Some(last) = h.text.chars().last() else {
                continue;
            };
            if !PUNCT.contains(last) {
                continue;
            }
            let cr = content_range(c, h);
            let src_last = c.src[cr.clone()].chars().last();
            let range = if src_last == Some(last) {
                cr.end - last.len_utf8()..cr.end
            } else {
                h.range.clone()
            };
            out.push(c.finding(
                "md/no-trailing-punctuation",
                range,
                format!("Heading ends with punctuation '{last}'"),
            ));
        }
    }

    if c.on("md/first-line-heading") {
        first_line_heading(c, out);
    }

    if c.on("md/heading-case") {
        heading_case(c, out);
    }
}

fn first_line_heading(c: &Ctx, out: &mut Out) {
    if fm_title(c) {
        return;
    }
    let start = c.lines.fm_end.map_or(0, |k| k + 1);
    let mut in_comment = false;
    for i in start..c.lines.len() {
        let l = &c.lines.v[i];
        let t = c.src[l.start..l.end].trim();
        if in_comment {
            in_comment = !t.contains("-->");
            continue;
        }
        if t.is_empty() {
            continue;
        }
        if t.starts_with("<!--") {
            in_comment = !t.contains("-->");
            continue;
        }
        let ok =
            c.md.headings
                .first()
                .is_some_and(|h| h.level == 1 && span(c, h).0 == i)
                || t.to_ascii_lowercase().starts_with("<h1");
        if !ok {
            out.push(c.finding(
                "md/first-line-heading",
                l.start..l.end,
                "First line should be a top-level heading",
            ));
        }
        return;
    }
}

const MINOR: &[&str] = &[
    "a", "an", "the", "and", "but", "or", "nor", "for", "so", "yet", "as", "at", "by", "in", "of",
    "off", "on", "per", "to", "up", "via", "vs", "with", "from", "into", "over", "onto", "than",
];

/// Heading text with markup stripped; code spans replaced by `\u{1}` runs.
fn case_text(c: &Ctx, h: &Heading) -> String {
    let cr = content_range(c, h);
    let mut s: Vec<u8> = c.src.as_bytes()[cr.clone()].to_vec();
    for sp in &c.md.code_spans {
        let a = sp.start.max(cr.start);
        let b = sp.end.min(cr.end);
        if a < b {
            for x in &mut s[a - cr.start..b - cr.start] {
                *x = 1;
            }
        }
    }
    let s = String::from_utf8_lossy(&s).into_owned();
    let s = LINK_DEST_RE.replace_all(&s, " ");
    s.replace(['[', ']', '*'], " ")
}

fn heading_case(c: &Ctx, out: &mut Out) {
    let title = c.fc.config.markdown.heading_case == "title";
    let accepted = c.fc.config.accepted_words();
    for h in &c.md.headings {
        let text = case_text(c, h);
        let tokens: Vec<&str> = text.split_whitespace().collect();
        let mut fixed: Vec<String> = Vec::new();
        let mut bad = Vec::new();
        let mut start = true;
        for (i, tok) in tokens.iter().enumerate() {
            let is_start = start;
            start = tok.ends_with(':');
            let word = tok.trim_matches(|ch: char| !ch.is_alphanumeric() && ch != '\u{1}');
            let replacement = case_fix(word, title, is_start, i + 1 == tokens.len(), &accepted);
            match replacement {
                Some(r) => {
                    bad.push(word.to_string());
                    fixed.push(tok.replacen(word, &r, 1));
                }
                None => fixed.push(tok.to_string()),
            }
        }
        if bad.is_empty() {
            continue;
        }
        let style = if title { "title" } else { "sentence" };
        let words = bad
            .iter()
            .map(|w| format!("\"{w}\""))
            .collect::<Vec<_>>()
            .join(", ");
        let suggestion = fixed.join(" ").replace('\u{1}', "`");
        let suggestion = if suggestion.contains('`') {
            h.text.clone()
        } else {
            suggestion
        };
        let f = c.finding(
            "md/heading-case",
            h.range.clone(),
            format!("Heading is not in {style} case: {words}"),
        );
        out.push(if suggestion != h.text {
            f.suggest(suggestion)
        } else {
            f
        });
    }
}

/// Returns the corrected word if it violates the case style.
fn case_fix(
    word: &str,
    title: bool,
    is_start: bool,
    is_last: bool,
    accepted: &[String],
) -> Option<String> {
    let mut chars = word.chars();
    let first = chars.next()?;
    if !first.is_alphabetic()
        || word.contains('\u{1}')
        || word.chars().any(|ch| ch.is_ascii_digit())
        || chars.any(|ch| ch.is_uppercase())
        || word == "I"
        || word.starts_with("I'")
        || word.starts_with("I’")
        || accepted.iter().any(|a| a == word)
    {
        return None;
    }
    let upper = first.is_uppercase();
    let capitalize = || {
        first
            .to_uppercase()
            .chain(word.chars().skip(1))
            .collect::<String>()
    };
    let lower = || {
        first
            .to_lowercase()
            .chain(word.chars().skip(1))
            .collect::<String>()
    };
    if title {
        let head = word.split('-').next().unwrap_or(word).to_lowercase();
        let minor = MINOR.contains(&head.as_str());
        (!upper && (is_start || is_last || !minor)).then(capitalize)
    } else if is_start {
        (!upper).then(capitalize)
    } else {
        upper.then(lower)
    }
}
