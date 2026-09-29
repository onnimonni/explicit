// explicit-disable-file slop/* prose/* grammar/* -- examples of the patterns this module detects
//! Heuristic mannerism rules: contrast framing, em dashes, stacked hedges, triads.

use std::ops::Range;
use std::sync::LazyLock;

use regex::Regex;

use crate::diagnostic::{Finding, Severity};
use crate::segment::{Segment, SegmentKind};

fn re(p: &str) -> Regex {
    Regex::new(&p.replace('\'', "['’]")).expect("valid mannerism regex")
}

/// Words with byte offsets inside `text`.
pub fn words(text: &str) -> Vec<Range<usize>> {
    static WORD: LazyLock<Regex> = LazyLock::new(|| re(r"[\p{L}\p{N}]+(?:['\-][\p{L}\p{N}]+)*"));
    WORD.find_iter(text).map(|m| m.range()).collect()
}

pub fn word_count(text: &str) -> usize {
    words(text).len()
}

// Rule slop/not-just-but.

static NOT_JUST_RES: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    [
        // "not just X, but (also) Y", "not only X but also Y"
        r"(?i)\bnot\s+(?:just|only|merely|simply)\b[^.!?;]{1,100}?\bbut\b(?:\s+also)?",
        // "isn't just X, it's Y" / "is not merely X; it is Y"
        r"(?i)\b(?:isn't|is\s+not|aren't|are\s+not|wasn't|was\s+not|it's\s+not|this\s+isn't|that's\s+not)\s+(?:just|only|merely|simply)\b[^.!?;]{1,100}?(?:[,;:—–]|\s--)\s*(?:it's|it\s+is|they're|they\s+are|this\s+is|that's|it\s+was)\b",
        // "It's not X. It's Y." / "It's not about X, it's about Y."
        r"(?i)\b(?:it's|it\s+is|this\s+is|that's|this\s+isn't|that\s+isn't)\s+not\s+[^.!?;]{1,60}?(?:[.,;:—–]|\s--)\s*(?:it's|it\s+is|this\s+is|that's)\b",
        r"(?i)\b(?:it|this|that)\s+isn't\s+[^.!?;]{1,60}?(?:[.,;:—–]|\s--)\s*(?:it's|it\s+is|this\s+is|that's)\b",
    ]
    .iter()
    .map(|p| re(p))
    .collect()
});

pub fn not_just_but(seg: &Segment, sev: Severity, out: &mut Vec<Finding>) {
    let mut taken: Vec<Range<usize>> = Vec::new();
    for r in NOT_JUST_RES.iter() {
        for m in r.find_iter(&seg.text) {
            let range = m.range();
            // Never across a paragraph break.
            if seg.text[range.clone()].contains("\n\n")
                || taken
                    .iter()
                    .any(|t| t.start < range.end && range.start < t.end)
            {
                continue;
            }
            taken.push(range.clone());
            let snippet: String = seg.text[range.clone()]
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            out.push(
                Finding::new(
                    "slop/not-just-but",
                    sev,
                    seg.abs(range),
                    format!("Contrast framing \"{}\" is a common AI mannerism", truncate(&snippet, 60)),
                )
                .help("State the positive claim directly (\"It is Y\"); the \"not just X\" setup adds drama, not information."),
            );
        }
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let t: String = s.chars().take(max).collect();
    format!("{}...", t.trim_end())
}

// Rule slop/em-dash.

/// Absolute ranges of em dashes used as punctuation in a segment.
pub fn dashes(seg: &Segment) -> Vec<Range<usize>> {
    if seg.kind == SegmentKind::TableCell {
        // A lone dash is the conventional "empty" marker in tables.
        return Vec::new();
    }
    let t = &seg.text;
    let b = t.as_bytes();
    let mut out = Vec::new();
    for (i, c) in t.char_indices() {
        let len = c.len_utf8();
        let prev = t[..i].chars().next_back();
        let next = t[i + len..].chars().next();
        match c {
            // Runs of em dashes are decoration, not punctuation.
            '—' if prev != Some('—') && next != Some('—') => out.push(seg.abs(i..i + len)),
            // Spaced en dash used as an em dash: "word – word".
            '–' if prev == Some(' ') && next == Some(' ') => out.push(seg.abs(i..i + len)),
            '-' if i >= 1
                && b.get(i + 1) == Some(&b'-')
                && b[i - 1] == b' '
                && b.get(i + 2) == Some(&b' ')
                && i >= 2
                && !b[i - 2].is_ascii_whitespace()
                && b.get(i + 3)
                    .is_some_and(|c| !c.is_ascii_whitespace() && *c != b'-') =>
            {
                out.push(seg.abs(i..i + 2))
            }
            _ => {}
        }
    }
    out
}

pub fn em_dash(sec: &[&Segment], per_100: f64, sev: Severity, out: &mut Vec<Finding>) {
    let words: usize = sec.iter().map(|s| word_count(&s.text)).sum();
    let found: Vec<Range<usize>> = sec.iter().flat_map(|s| dashes(s)).collect();
    if found.len() < 3 || words == 0 {
        return;
    }
    let rate = found.len() as f64 * 100.0 / words as f64;
    if rate <= per_100 {
        return;
    }
    let n = found.len();
    for r in found {
        out.push(
            Finding::new(
                "slop/em-dash",
                sev,
                r,
                format!("Em dash overuse: {n} dashes in {words} words ({rate:.1} per 100, limit {per_100})"),
            )
            .help("Replace most em dashes with a comma, colon, parentheses or a full stop."),
        );
    }
}

// Rule slop/hedging.

const HEDGES: &[&str] = &[
    "might",
    "may",
    "possibly",
    "perhaps",
    "potentially",
    "arguably",
    "somewhat",
    "could",
    "seems",
    "seem",
    "likely",
    "generally",
    "typically",
    "maybe",
    "probably",
    "conceivably",
    "presumably",
];
const MODALS: &[&str] = &["might", "may", "could", "would", "can"];
const ADVERBS: &[&str] = &["possibly", "potentially", "perhaps", "conceivably", "maybe"];

fn sentences(text: &str) -> Vec<Range<usize>> {
    static SENT: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"[^.!?\n]+(?:\n[^.!?\n]+)*[.!?]*").expect("hardcoded regex is valid")
    });
    SENT.find_iter(text).map(|m| m.range()).collect()
}

pub fn hedging(seg: &Segment, sev: Severity, out: &mut Vec<Finding>) {
    for s in sentences(&seg.text) {
        let sent = &seg.text[s.clone()];
        let toks: Vec<(Range<usize>, String)> = words(sent)
            .into_iter()
            .map(|r| (s.start + r.start..s.start + r.end, sent[r].to_lowercase()))
            .collect();
        // Hedge tokens: (token index, hedge label, range).
        let mut hs: Vec<(usize, String, Range<usize>)> = Vec::new();
        let mut i = 0;
        while i < toks.len() {
            let w = toks[i].1.as_str();
            if w == "in"
                && toks.get(i + 1).is_some_and(|t| t.1 == "some")
                && toks.get(i + 2).is_some_and(|t| t.1 == "cases")
            {
                hs.push((
                    i,
                    "in some cases".into(),
                    toks[i].0.start..toks[i + 2].0.end,
                ));
                i += 3;
                continue;
            }
            if HEDGES.contains(&w) {
                hs.push((i, w.to_string(), toks[i].0.clone()));
            }
            i += 1;
        }
        // Doubled hedge: "might possibly", "could potentially", "perhaps might".
        let doubled = toks.windows(2).find(|p| {
            let (a, b) = (p[0].1.as_str(), p[1].1.as_str());
            (MODALS.contains(&a) && ADVERBS.contains(&b))
                || (ADVERBS.contains(&a) && MODALS.contains(&b))
        });
        if let Some(p) = doubled {
            let range = p[0].0.start..p[1].0.end;
            let phrase = &seg.text[range.clone()];
            out.push(
                Finding::new("slop/hedging", sev, seg.abs(range), format!("Doubled hedge \"{phrase}\""))
                    .help("One hedge is enough; keep the modal or the adverb, or state the claim plainly.")
                    .suggest(p[0].1.clone())
                    .suggest(p[1].1.clone()),
            );
            continue;
        }
        // Three distinct hedges within 12 words.
        for a in 0..hs.len() {
            let mut distinct: Vec<&str> = vec![hs[a].1.as_str()];
            let mut last = a;
            for (b, h) in hs.iter().enumerate().skip(a + 1) {
                if h.0 - hs[a].0 > 11 {
                    break;
                }
                if !distinct.contains(&h.1.as_str()) {
                    distinct.push(h.1.as_str());
                }
                last = b;
            }
            if distinct.len() >= 3 {
                let range = hs[a].2.start..hs[last].2.end;
                out.push(
                    Finding::new(
                        "slop/hedging",
                        sev,
                        seg.abs(range),
                        format!("Stacked hedges ({}) in one sentence", distinct.join(", ")),
                    )
                    .help("Commit to the claim or state the actual condition under which it holds; keep at most one hedge."),
                );
                break;
            }
        }
    }
}

// Rule slop/rule-of-three.

const TRIAD_ADJ: &[&str] = &[
    "fast",
    "simple",
    "clean",
    "powerful",
    "robust",
    "scalable",
    "secure",
    "reliable",
    "efficient",
    "flexible",
    "intuitive",
    "elegant",
    "lightweight",
    "modern",
    "maintainable",
    "performant",
    "seamless",
    "easy",
    "safe",
    "extensible",
    "modular",
    "accessible",
    "responsive",
    "affordable",
    "innovative",
    "sustainable",
    "transparent",
    "consistent",
    "concise",
    "clear",
    "readable",
    "testable",
    "portable",
    "predictable",
    "resilient",
    "fun",
    "engaging",
    "smart",
    "quick",
    "smooth",
];
const ADJ_SUFFIXES: &[&str] = &[
    "ful", "able", "ible", "ive", "ous", "al", "ent", "ant", "less", "ic", "ity", "ness", "ion",
    "ment", "ance", "ence",
];

fn triad_like(w: &str) -> bool {
    let w = w.to_lowercase();
    TRIAD_ADJ.contains(&w.as_str()) || (w.len() > 4 && ADJ_SUFFIXES.iter().any(|s| w.ends_with(s)))
}

static TRIAD_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b([A-Za-z][a-z-]+),\s+([a-z][a-z-]+),?\s+(?:and|or)\s+([a-z][a-z-]+)\b")
        .expect("hardcoded regex is valid")
});

pub fn rule_of_three(sec: &[&Segment], sev: Severity, out: &mut Vec<Finding>) {
    let mut triads: Vec<(Range<usize>, String)> = Vec::new();
    for seg in sec {
        for c in TRIAD_RE.captures_iter(&seg.text) {
            let items = [&c[1], &c[2], &c[3]];
            if items.iter().filter(|w| triad_like(w)).count() >= 2 {
                let m = c.get(0).expect("regex group 0 is not optional");
                let text: String = m.as_str().split_whitespace().collect::<Vec<_>>().join(" ");
                triads.push((seg.abs(m.range()), text));
            }
        }
    }
    if triads.len() < 2 {
        return;
    }
    let n = triads.len();
    for (range, text) in triads {
        out.push(
            Finding::new(
                "slop/rule-of-three",
                sev,
                range,
                format!("Reflexive triad \"{text}\" ({n} in this section)"),
            )
            .help("Keep the one or two qualities that matter and support them with specifics."),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seg(text: &str) -> Segment {
        Segment {
            range: 0..text.len(),
            text: text.to_string(),
            kind: SegmentKind::Paragraph,
        }
    }

    fn run(f: impl Fn(&Segment, Severity, &mut Vec<Finding>), text: &str) -> Vec<Finding> {
        let mut out = Vec::new();
        f(&seg(text), Severity::Warning, &mut out);
        out
    }

    #[test]
    fn not_just_but_positive() {
        for t in [
            "This is not just a linter, but a platform.",
            "It is not only fast but also correct.",
            "The CLI isn't just a wrapper, it's a full client.",
            "It's not a bug. It's a design decision.",
            "It’s not about speed, it’s about trust.",
        ] {
            assert_eq!(run(not_just_but, t).len(), 1, "{t}");
        }
    }

    #[test]
    fn not_just_but_negative() {
        for t in [
            "Not red but blue.",
            "It is not fast. We are working on it.",
            "This is not supported on Windows.",
            "Use not just once.\n\nBut later it works.",
        ] {
            assert!(run(not_just_but, t).is_empty(), "{t}");
        }
    }

    #[test]
    fn em_dash_density() {
        let text = "The cache — which is new — speeds things up — a lot.";
        let s = seg(text);
        let mut out = Vec::new();
        em_dash(&[&s], 2.0, Severity::Warning, &mut out);
        assert_eq!(out.len(), 3);
        assert_eq!(&text[out[0].range.clone()], "—");

        let spaced = "One -- two -- three -- four.";
        let s = seg(spaced);
        let mut out = Vec::new();
        em_dash(&[&s], 2.0, Severity::Warning, &mut out);
        assert_eq!(out.len(), 3);
        assert_eq!(&spaced[out[0].range.clone()], "--");
    }

    #[test]
    fn em_dash_below_threshold_or_decorative() {
        let long = format!("{} — a — b — c", "word ".repeat(200));
        let s = seg(&long);
        let mut out = Vec::new();
        em_dash(&[&s], 2.0, Severity::Warning, &mut out);
        assert!(out.is_empty());

        let deco = "—————— title ——————";
        let s = seg(deco);
        let mut out = Vec::new();
        em_dash(&[&s], 2.0, Severity::Warning, &mut out);
        assert!(out.is_empty());

        let flags = "use --foo -- --bar and x -- y";
        assert!(dashes(&seg(flags)).len() == 1);
    }

    #[test]
    fn hedging_positive() {
        let out = run(hedging, "This might possibly break.");
        assert_eq!(out.len(), 1);
        assert!(out[0].message.contains("might possibly"));
        let out = run(hedging, "It could perhaps be slow.");
        assert_eq!(out.len(), 1);
        let out = run(hedging, "This seems likely to be generally fine.");
        assert_eq!(out.len(), 1);
        assert!(out[0].message.contains("Stacked"));
        let out = run(hedging, "In some cases it may typically fail.");
        assert_eq!(out.len(), 1);
    }

    #[test]
    fn hedging_negative() {
        assert!(run(hedging, "This may fail when the disk is full.").is_empty());
        assert!(run(hedging, "It might fail. It could retry. It seems fine.").is_empty());
        assert!(
            run(
                hedging,
                "It may fail, and it may fail again, and it may fail once more."
            )
            .is_empty()
        );
    }

    #[test]
    fn rule_of_three_needs_two_triads() {
        let one = seg("It is fast, reliable, and scalable.");
        let mut out = Vec::new();
        rule_of_three(&[&one], Severity::Info, &mut out);
        assert!(out.is_empty());

        let two =
            seg("It is fast, reliable, and scalable. The API is clean, intuitive, and powerful.");
        let mut out = Vec::new();
        rule_of_three(&[&two], Severity::Info, &mut out);
        assert_eq!(out.len(), 2);

        let lists = seg("Buy apples, pears, and plums. Then red, green, or blue.");
        let mut out = Vec::new();
        rule_of_three(&[&lists], Severity::Info, &mut out);
        assert!(out.is_empty());
    }
}
