// explicit-disable-file slop/* prose/* -- examples of the patterns this module detects
//! Sentence-level rules: `prose/passive`, `prose/weasel`, `prose/there-is`, `prose/so-start`,
//! `prose/sentence-length`, `prose/sentence-spacing`.

use std::sync::LazyLock;

use regex::Regex;

use super::{WORD_RE, quoted, sentences, sev};
use crate::diagnostic::Finding;
use crate::rules::{FileCtx, Out};
use crate::segment::Segment;

/// Irregular past participles (write-good), minus forms that are mostly adjectives or
/// identical to common base verbs (set, put, cut, let, read, run...).
const IRREGULAR: &[&str] = &[
    "awoken",
    "born",
    "beaten",
    "become",
    "begun",
    "bent",
    "bitten",
    "bled",
    "blown",
    "broken",
    "bred",
    "brought",
    "built",
    "burnt",
    "bought",
    "caught",
    "chosen",
    "clung",
    "crept",
    "dealt",
    "dug",
    "done",
    "drawn",
    "dreamt",
    "driven",
    "drunk",
    "eaten",
    "fallen",
    "fed",
    "felt",
    "fought",
    "found",
    "fled",
    "flung",
    "flown",
    "forbidden",
    "forecast",
    "foreseen",
    "foretold",
    "forgotten",
    "forgiven",
    "forsaken",
    "frozen",
    "gotten",
    "given",
    "gone",
    "grown",
    "hung",
    "heard",
    "hidden",
    "held",
    "kept",
    "knelt",
    "known",
    "laid",
    "led",
    "leapt",
    "learnt",
    "lent",
    "lain",
    "lost",
    "made",
    "meant",
    "met",
    "mistaken",
    "mown",
    "overcome",
    "overdone",
    "overtaken",
    "overthrown",
    "paid",
    "proven",
    "ridden",
    "rung",
    "risen",
    "sawn",
    "said",
    "seen",
    "sought",
    "sold",
    "sent",
    "sewn",
    "shaken",
    "shaven",
    "shorn",
    "shone",
    "shot",
    "shown",
    "shrunk",
    "sung",
    "sunk",
    "slain",
    "slid",
    "slung",
    "sown",
    "spoken",
    "sped",
    "spent",
    "spilt",
    "spun",
    "spread",
    "sprung",
    "stolen",
    "stuck",
    "stung",
    "struck",
    "strung",
    "striven",
    "sworn",
    "swept",
    "swollen",
    "swum",
    "swung",
    "taken",
    "taught",
    "torn",
    "told",
    "thought",
    "thrown",
    "thrust",
    "trodden",
    "understood",
    "upheld",
    "woken",
    "worn",
    "woven",
    "wept",
    "won",
    "withheld",
    "withstood",
    "wrung",
    "written",
];

/// Words ending in -ed that are not participles.
const NOT_PARTICIPLE: &[&str] = &[
    "need", "seed", "speed", "feed", "bed", "red", "shed", "embed", "indeed", "exceed", "proceed",
    "succeed", "breed", "bleed", "greed", "weed", "hundred", "sacred", "naked", "wicked", "rugged",
    "ragged", "kindred", "steed", "creed", "deed", "freed", "reed", "tweed",
];

static PASSIVE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"(?i)\b(?:am|are|is|was|were|be|been|being)(?:\s+(?:not|\w+ly))?\s+(\w{{2,}}ed|{})\b",
        IRREGULAR.join("|")
    ))
    .expect("hardcoded regex is valid")
});

/// Weasel words not already covered by the slop catalog (extremely, various, truly...).
static WEASEL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)\b(?:very|really|quite|fairly|rather|mostly|largely|it is said|it has been said|it is believed|some people (?:say|think|believe)|many people (?:say|think|believe)|some say|experts (?:say|agree|believe)|studies show|research shows)\b",
    )
    .expect("hardcoded regex is valid")
});

static THERE_IS_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^There(?:\s+(?:is|are|was|were)\b|['’]s\b)").expect("hardcoded regex is valid")
});

static SO_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^So(?:\s*,\s*|\s+)([\p{L}\p{N}][\p{L}\p{N}'’-]*)")
        .expect("hardcoded regex is valid")
});

/// "So far", "So that", "So-called"...: not the filler use.
const SO_KEEP: &[&str] = &[
    "far", "that", "much", "many", "long", "few", "little", "called",
];

static SPACING_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"[.!?]["')\]’”]? {2,}\p{Lu}"#).expect("hardcoded regex is valid")
});

pub fn check(ctx: &FileCtx, out: &mut Out) {
    let on = |r: &str| ctx.enabled(r);
    let passive = on("prose/passive");
    let weasel = on("prose/weasel");
    let there = on("prose/there-is");
    let so = on("prose/so-start");
    let length = on("prose/sentence-length");
    let spacing = on("prose/sentence-spacing");
    if !(passive || weasel || there || so || length || spacing) {
        return;
    }
    let max_words = ctx.config.prose.max_sentence_words;
    for seg in &ctx.a.segments {
        let q = quoted(seg);
        if passive && !q {
            passive_seg(seg, out);
        }
        if weasel && !q {
            weasel_seg(seg, out);
        }
        if spacing {
            spacing_seg(ctx.src(), seg, out);
        }
        if there || so || length {
            for s in sentences(&seg.text) {
                let text = &seg.text[s.clone()];
                if there {
                    there_is(seg, s.start, text, out);
                }
                if so {
                    so_start(seg, s.start, text, out);
                }
                if length {
                    let words = WORD_RE.find_iter(text).count();
                    if words > max_words {
                        const RULE: &str = "prose/sentence-length";
                        out.push(
                            Finding::new(
                                RULE,
                                sev(RULE),
                                seg.abs(s.clone()),
                                format!("Sentence has {words} words (max {max_words})"),
                            )
                            .help("Split it into shorter sentences."),
                        );
                    }
                }
            }
        }
    }
}

fn passive_seg(seg: &Segment, out: &mut Out) {
    const RULE: &str = "prose/passive";
    for c in PASSIVE_RE.captures_iter(&seg.text) {
        let (all, part) = (
            c.get(0).expect("regex group 0 is not optional"),
            c.get(1).expect("regex group 1 is not optional"),
        );
        if NOT_PARTICIPLE.contains(&part.as_str().to_lowercase().as_str()) {
            continue;
        }
        let shown = all
            .as_str()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        out.push(
            Finding::new(
                RULE,
                sev(RULE),
                seg.abs(all.range()),
                format!("\"{shown}\" may be passive voice"),
            )
            .help("Name who does the action and make them the subject."),
        );
    }
}

fn weasel_seg(seg: &Segment, out: &mut Out) {
    const RULE: &str = "prose/weasel";
    for m in WEASEL_RE.find_iter(&seg.text) {
        if weasel_idiom(m.as_str(), &seg.text[m.end()..]) {
            continue;
        }
        let shown = m.as_str().split_whitespace().collect::<Vec<_>>().join(" ");
        out.push(
            Finding::new(
                RULE,
                sev(RULE),
                seg.abs(m.range()),
                format!("\"{shown}\" is a weasel word"),
            )
            .help("Delete it, or replace it with a number, source or specific claim."),
        );
    }
}

/// Fixed phrases that are not hedges: "rather than", "quite a few", "quite a bit".
fn weasel_idiom(word: &str, after: &str) -> bool {
    let next: Vec<String> = after
        .split_whitespace()
        .take(2)
        .map(|w| {
            w.trim_matches(|c: char| !c.is_alphanumeric())
                .to_lowercase()
        })
        .collect();
    let next: Vec<&str> = next.iter().map(String::as_str).collect();
    match word.to_lowercase().as_str() {
        "rather" => next.first() == Some(&"than"),
        "quite" => matches!(next.as_slice(), ["a", "few" | "bit", ..]),
        _ => false,
    }
}

fn there_is(seg: &Segment, start: usize, text: &str, out: &mut Out) {
    const RULE: &str = "prose/there-is";
    if let Some(m) = THERE_IS_RE.find(text) {
        out.push(
            Finding::new(
                RULE,
                sev(RULE),
                seg.abs(start..start + m.end()),
                format!("Sentence starts with \"{}\"", m.as_str()),
            )
            .help(
                "Start with the real subject: \"There are three modes\" -> \"Three modes exist\".",
            ),
        );
    }
}

fn so_start(seg: &Segment, start: usize, text: &str, out: &mut Out) {
    const RULE: &str = "prose/so-start";
    let Some(c) = SO_RE.captures(text) else {
        return;
    };
    let next = c.get(1).expect("regex group 1 is not optional");
    if SO_KEEP.contains(&next.as_str().to_lowercase().as_str()) {
        return;
    }
    // Suggestion only: "So" often carries a causal link that plain deletion loses.
    out.push(
        Finding::new(
            RULE,
            sev(RULE),
            seg.abs(start..start + 2),
            "Sentence starts with \"So\"",
        )
        .help("Delete it; the sentence works without it."),
    );
}

fn spacing_seg(src: &str, seg: &Segment, out: &mut Out) {
    const RULE: &str = "prose/sentence-spacing";
    for m in SPACING_RE.find_iter(&seg.text) {
        let s = m.as_str();
        let spaces_start = m.start() + s.find(' ').expect("SPACING_RE match contains spaces");
        let spaces_end = m.start() + s.rfind(' ').expect("SPACING_RE match contains spaces") + 1;
        let abs = seg.abs(spaces_start..spaces_end);
        // Blanked markup also looks like spaces; only real spaces in the source count.
        if !src[abs.clone()].bytes().all(|b| b == b' ') {
            continue;
        }
        out.push(
            Finding::new(
                RULE,
                sev(RULE),
                abs.clone(),
                "Multiple spaces after sentence",
            )
            .fix(abs, " "),
        );
    }
}

#[cfg(test)]
mod weasel_tests {
    use super::*;
    use crate::segment::SegmentKind;

    fn hits(text: &str) -> Vec<String> {
        let seg = Segment {
            range: 0..text.len(),
            text: text.into(),
            kind: SegmentKind::Paragraph,
        };
        let mut out = Vec::new();
        weasel_seg(&seg, &mut out);
        out.iter()
            .map(|f| text[f.range.clone()].to_string())
            .collect()
    }

    #[test]
    fn idioms_are_not_weasels() {
        assert!(hits("Use tabs rather than spaces.").is_empty());
        assert!(hits("It takes quite a few steps and quite a bit of time.").is_empty());
        assert_eq!(
            hits("It is rather slow and quite good."),
            ["rather", "quite"]
        );
    }
}
