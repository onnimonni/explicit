//! Link checking: local files and anchors, reference definitions and remote URLs.

pub mod cache;
pub mod local;
pub mod remote;

use std::ops::Range;

use crate::config::Config;
use crate::diagnostic::Severity;
use crate::extract::markdown::{Link, LinkKind};

/// Default severity of a links rule.
pub(crate) fn sev(rule: &str) -> Severity {
    crate::rules::default_severity(rule).unwrap_or(Severity::Warning)
}

/// Decode `%XX` escapes; returns the input unchanged if the result is not UTF-8.
pub fn percent_decode(s: &str) -> String {
    if !s.contains('%') {
        return s.to_string();
    }
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            let hex = |c: u8| (c as char).to_digit(16);
            if let (Some(h), Some(l)) = (hex(b[i + 1]), hex(b[i + 2])) {
                out.push((h * 16 + l) as u8);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8(out).unwrap_or_else(|_| s.to_string())
}

/// Percent-encode characters that break Markdown link destinations.
pub(crate) fn encode_path_segment(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            ' ' => out.push_str("%20"),
            '(' => out.push_str("%28"),
            ')' => out.push_str("%29"),
            '<' => out.push_str("%3C"),
            '>' => out.push_str("%3E"),
            '#' => out.push_str("%23"),
            '?' => out.push_str("%3F"),
            _ => out.push(c),
        }
    }
    out
}

/// True for `scheme:` prefixed destinations (http, mailto, data, ...).
pub(crate) fn has_scheme(dest: &str) -> bool {
    let Some((scheme, _)) = dest.split_once(':') else {
        return false;
    };
    let mut chars = scheme.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
}

pub(crate) fn is_http(url: &str) -> bool {
    let l = url.get(..8).unwrap_or(url).to_ascii_lowercase();
    l.starts_with("http://") || l.starts_with("https://")
}

/// URL without its `#fragment`.
pub fn strip_fragment(url: &str) -> &str {
    url.split_once('#').map_or(url, |(u, _)| u)
}

/// Levenshtein distance over chars.
pub(crate) fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0; b.len() + 1];
    for (i, ca) in a.iter().enumerate() {
        cur[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let cost = usize::from(ca != cb);
            cur[j + 1] = (prev[j] + cost).min(prev[j + 1] + 1).min(cur[j] + 1);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()]
}

/// Candidates close to `target` (case-insensitive equal first, then edit distance <= 2), best first.
pub(crate) fn near_misses<'a>(
    target: &str,
    candidates: impl IntoIterator<Item = &'a str>,
) -> Vec<String> {
    let t = target.to_lowercase();
    let max = if t.chars().count() <= 3 { 1 } else { 2 };
    let mut scored: Vec<(usize, String)> = candidates
        .into_iter()
        .filter(|c| *c != target)
        .filter_map(|c| {
            let d = levenshtein(&t, &c.to_lowercase());
            (d <= max).then(|| (d, c.to_string()))
        })
        .collect();
    scored.sort();
    scored.dedup_by(|a, b| a.1 == b.1);
    scored.into_iter().take(3).map(|(_, c)| c).collect()
}

pub(crate) fn is_excluded(config: &Config, url: &str) -> bool {
    config.link_excludes().iter().any(|r| r.is_match(url))
}

/// Byte range of the destination text itself in the source, when it appears verbatim.
pub(crate) fn dest_range(src: &str, link: &Link) -> Option<Range<usize>> {
    let r = link.range.clone();
    let raw = src.get(r.clone())?;
    match link.kind {
        LinkKind::Bare => (raw == link.dest).then_some(r),
        LinkKind::Autolink => {
            let inner = r.start + 1..r.end.saturating_sub(1);
            (src.get(inner.clone()) == Some(link.dest.as_str())).then_some(inner)
        }
        LinkKind::Inline | LinkKind::Html => {
            if link.dest.is_empty() {
                return None;
            }
            let i = raw.rfind(link.dest.as_str())?;
            Some(r.start + i..r.start + i + link.dest.len())
        }
        LinkKind::Reference => None,
    }
}

/// Range to report a link finding at: the destination for HTML (many links share one HTML range), else the link.
pub(crate) fn report_range(src: &str, link: &Link) -> Range<usize> {
    if link.kind == LinkKind::Html
        && let Some(r) = dest_range(src, link)
    {
        return r;
    }
    link.range.clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decoding() {
        assert_eq!(percent_decode("my%20file.md"), "my file.md");
        assert_eq!(percent_decode("100%"), "100%");
        assert_eq!(percent_decode("%zz"), "%zz");
        assert_eq!(percent_decode("%C3%A4"), "ä");
        assert_eq!(percent_decode("%FF"), "%FF");
    }

    #[test]
    fn schemes() {
        assert!(has_scheme("https://x"));
        assert!(has_scheme("mailto:a@b"));
        assert!(!has_scheme("./a:b"));
        assert!(!has_scheme("docs/x.md"));
    }

    #[test]
    fn distances() {
        assert_eq!(levenshtein("kitten", "sitting"), 3);
        assert_eq!(
            near_misses("instal.md", ["install.md", "other.md"]),
            vec!["install.md"]
        );
    }
}
