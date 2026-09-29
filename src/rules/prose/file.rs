// explicit-disable-file slop/* prose/* -- examples of the patterns this module detects
//! Whole-file rules: `prose/readability`, `prose/consistency`, `prose/acronym-defined`,
//! `prose/smart-quotes`.

use std::collections::{HashMap, HashSet};
use std::ops::Range;
use std::sync::LazyLock;

use regex::{Regex, RegexBuilder};

use super::lists::TERMINOLOGY;
use super::{WORD_RE, match_case, md_sections, sentences, sev};
use crate::diagnostic::Finding;
use crate::rules::{FileCtx, Out};
use crate::segment::{Segment, SegmentKind};

pub fn check(ctx: &FileCtx, out: &mut Out) {
    let is_md = ctx.a.md.is_some();
    if is_md && ctx.enabled("prose/readability") {
        readability(ctx, out);
    }
    if ctx.enabled("prose/consistency") {
        consistency(ctx, out);
    }
    if is_md && ctx.enabled("prose/acronym-defined") {
        acronyms(ctx, out);
    }
    if ctx.enabled("prose/smart-quotes") {
        smart_quotes(ctx, out);
    }
}

// prose/readability

/// Sections shorter than this give meaningless grades.
const MIN_SECTION_WORDS: usize = 100;

/// Heuristic English syllable count.
pub(super) fn syllables(word: &str) -> usize {
    let w: Vec<char> = word
        .chars()
        .filter(|c| c.is_alphabetic())
        .flat_map(char::to_lowercase)
        .collect();
    if w.is_empty() {
        return 0;
    }
    if w.len() <= 3 {
        return 1;
    }
    let vowel = |c: char| matches!(c, 'a' | 'e' | 'i' | 'o' | 'u' | 'y');
    let mut n = 0;
    let mut prev = false;
    for &c in &w {
        let v = vowel(c);
        if v && !prev {
            n += 1;
        }
        prev = v;
    }
    let len = w.len();
    // Silent final "e" ("make"), but not "-le" ("table").
    if w[len - 1] == 'e' && !(w[len - 2] == 'l' && !vowel(w[len - 3])) {
        n -= 1;
    }
    // "-ed"/"-es" rarely add a syllable ("used", "makes"), except `-ted`/`-ded`, `-ses`/`-ces`.
    if w[len - 2] == 'e' && !vowel(w[len - 3]) {
        let before = w[len - 3];
        let sounded = match w[len - 1] {
            'd' => matches!(before, 't' | 'd'),
            's' => matches!(before, 's' | 'z' | 'x' | 'c' | 'g' | 'h'),
            _ => true,
        };
        if !sounded {
            n -= 1;
        }
    }
    n.max(1)
}

pub(super) fn fk_grade(words: usize, sentences: usize, syllables: usize) -> f64 {
    let (w, s, y) = (words as f64, sentences.max(1) as f64, syllables as f64);
    0.39 * (w / s) + 11.8 * (y / w) - 15.59
}

fn readability(ctx: &FileCtx, out: &mut Out) {
    const RULE: &str = "prose/readability";
    let max = ctx.config.prose.max_grade;
    for sec in md_sections(&ctx.a.segments) {
        let (mut words, mut sents, mut syl) = (0, 0, 0);
        for seg in &sec {
            for s in sentences(&seg.text) {
                let mut any = false;
                for w in WORD_RE.find_iter(&seg.text[s]) {
                    if w.as_str().chars().any(char::is_alphabetic) {
                        words += 1;
                        syl += syllables(w.as_str());
                        any = true;
                    }
                }
                sents += usize::from(any);
            }
        }
        if words < MIN_SECTION_WORDS {
            continue;
        }
        let grade = fk_grade(words, sents, syl);
        if grade > max {
            let first = sec[0];
            let r = first.text.trim_end().len();
            let start = first.text.len() - first.text.trim_start().len();
            out.push(
                Finding::new(
                    RULE,
                    sev(RULE),
                    first.abs(start..r.max(start)),
                    format!(
                        "Section reads at grade {grade:.1} (max {max}): {words} words, {sents} sentences"
                    ),
                )
                .help("Use shorter sentences and simpler words."),
            );
        }
    }
}

// prose/consistency

/// Spelling variants of the same word; a variant with uppercase letters is case-sensitive.
const VARIANTS: &[&[&str]] = &[
    &["email", "e-mail"],
    &["color", "colour"],
    &["colors", "colours"],
    &["behavior", "behaviour"],
    &["behaviors", "behaviours"],
    &["favorite", "favourite"],
    &["gray", "grey"],
    &["center", "centre"],
    &["toward", "towards"],
    &["afterward", "afterwards"],
    &["OK", "okay"],
    &["canceled", "cancelled"],
    &["canceling", "cancelling"],
    &["labeled", "labelled"],
    &["modeling", "modelling"],
    &["traveling", "travelling"],
    &["analyze", "analyse"],
    &["organization", "organisation"],
    &["initialize", "initialise"],
    &["optimize", "optimise"],
    &["customize", "customise"],
    &["catalog", "catalogue"],
    &["judgment", "judgement"],
    &["front-end", "frontend"],
    &["back-end", "backend"],
    &["filename", "file name"],
    &["dataset", "data set"],
    &["website", "web site"],
    &["online", "on-line"],
    &["plugin", "plug-in"],
    &["plugins", "plug-ins"],
    &["startup", "start-up"],
];

static VARIANT_RES: LazyLock<Vec<Vec<Regex>>> = LazyLock::new(|| {
    VARIANTS
        .iter()
        .map(|group| {
            group
                .iter()
                .map(|v| {
                    let pat = format!(r"\b{}\b", regex::escape(v).replace(' ', r"\s+"));
                    RegexBuilder::new(&pat)
                        .case_insensitive(!v.chars().any(char::is_uppercase))
                        .build()
                        .expect("escaped variant is a valid regex")
                })
                .collect()
        })
        .collect()
});

fn consistency(ctx: &FileCtx, out: &mut Out) {
    const RULE: &str = "prose/consistency";
    let terminology = ctx.enabled("prose/terminology");
    for (group, res) in VARIANTS.iter().zip(VARIANT_RES.iter()) {
        // (segment, range) per variant, in file order.
        let hits: Vec<Vec<(&Segment, Range<usize>)>> = res
            .iter()
            .map(|re| {
                ctx.a
                    .segments
                    .iter()
                    .flat_map(|seg| re.find_iter(&seg.text).map(move |m| (seg, m.range())))
                    .filter(|(seg, r)| !super::identifier_like(&seg.text, r))
                    .collect()
            })
            .collect();
        let used = hits.iter().filter(|h| !h.is_empty()).count();
        if used < 2 {
            continue;
        }
        // Majority wins; ties go to the variant used first.
        let first_pos = |h: &Vec<(&Segment, Range<usize>)>| {
            h.first()
                .map_or(usize::MAX, |(s, r)| s.range.start + r.start)
        };
        let winner = (0..group.len())
            .filter(|&i| !hits[i].is_empty())
            .max_by(|&a, &b| {
                hits[a]
                    .len()
                    .cmp(&hits[b].len())
                    .then(first_pos(&hits[b]).cmp(&first_pos(&hits[a])))
            })
            .expect("at least two variants are used");
        for (i, h) in hits.iter().enumerate() {
            if i == winner {
                continue;
            }
            for (seg, r) in h {
                let matched = &seg.text[r.clone()];
                // prose/terminology already reports (and fixes) this spelling.
                if terminology && TERMINOLOGY.contains(matched) {
                    continue;
                }
                let good = if group.iter().any(|v| v.chars().any(char::is_uppercase)) {
                    group[winner].to_string()
                } else {
                    match_case(group[winner], matched)
                };
                out.push(
                    Finding::new(
                        RULE,
                        sev(RULE),
                        seg.abs(r.clone()),
                        format!(
                            "\"{matched}\" and \"{}\" both appear in this file",
                            group[winner]
                        ),
                    )
                    .help(format!(
                        "Pick one spelling; \"{}\" is used {} times, \"{}\" {} times.",
                        group[winner],
                        hits[winner].len(),
                        group[i],
                        h.len()
                    ))
                    .suggest(good),
                );
            }
        }
    }
}

// prose/acronym-defined

/// Acronyms every technical reader knows.
const KNOWN_ACRONYMS: &[&str] = &[
    "API",
    "URL",
    "URI",
    "HTTP",
    "HTTPS",
    "JSON",
    "YAML",
    "TOML",
    "XML",
    "HTML",
    "CSS",
    "CLI",
    "GUI",
    "CI",
    "CD",
    "PR",
    "MR",
    "OS",
    "UI",
    "UX",
    "ID",
    "CPU",
    "GPU",
    "RAM",
    "ROM",
    "SQL",
    "TODO",
    "FIXME",
    "XXX",
    "HACK",
    "NOTE",
    "FAQ",
    "PDF",
    "PNG",
    "JPG",
    "JPEG",
    "GIF",
    "SVG",
    "CSV",
    "TSV",
    "UTF",
    "ASCII",
    "USB",
    "SSD",
    "HDD",
    "IP",
    "TCP",
    "UDP",
    "DNS",
    "SSH",
    "SSL",
    "TLS",
    "VPN",
    "LAN",
    "WAN",
    "AWS",
    "GCP",
    "SDK",
    "IDE",
    "MIT",
    "BSD",
    "GPL",
    "USA",
    "US",
    "UK",
    "EU",
    "UN",
    "AI",
    "ML",
    "LLM",
    "OK",
    "PHP",
    "SMTP",
    "FTP",
    "SFTP",
    "REST",
    "RPC",
    "JWT",
    "CORS",
    "CRUD",
    "DOM",
    "JS",
    "TS",
    "PC",
    "TV",
    "AM",
    "PM",
    "GMT",
    "UTC",
    "ISO",
    "IEEE",
    "RFC",
    "README",
    "CHANGELOG",
    "LICENSE",
    "WIP",
    "EOF",
    "EOL",
    "LF",
    "CRLF",
    "TBD",
    "ASAP",
    "FYI",
    "IO",
    "OOM",
    "RSS",
    "VM",
    "PID",
    "UID",
    "GID",
    "CVE",
    "MB",
    "GB",
    "KB",
    "TB",
    "MIB",
    "GIB",
    "KIB",
    "SHA",
    "MD",
    "UUID",
    "GUID",
    "ANSI",
    "POSIX",
    "WASM",
    "ARM",
    "NASA",
    "ETA",
    "QA",
    "HR",
    "CEO",
    "CTO",
    "DIY",
    "LTS",
];

const ROMAN: &[&str] = &[
    "II", "III", "IV", "VI", "VII", "VIII", "IX", "XI", "XII", "XIV", "XV",
];

static ACRONYM_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b([A-Z]{2,5})(s?)\b").expect("hardcoded regex is valid"));

static DEFINED_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\(\s*([A-Z]{2,5})s?\s*\)").expect("hardcoded regex is valid"));

fn acronyms(ctx: &FileCtx, out: &mut Out) {
    const RULE: &str = "prose/acronym-defined";
    let words = ctx.config.accepted_words();
    let accepted: HashSet<&str> = words.iter().map(String::as_str).collect();
    // Absolute offset of the first "(ACR)" per acronym.
    let mut defined: HashMap<&str, usize> = HashMap::new();
    for seg in &ctx.a.segments {
        for c in DEFINED_RE.captures_iter(&seg.text) {
            let acr = c.get(1).expect("regex group 1 is not optional");
            defined
                .entry(acr.as_str())
                .or_insert(seg.range.start + acr.start());
        }
    }
    let mut seen: HashSet<&str> = HashSet::new();
    for seg in ctx
        .a
        .segments
        .iter()
        .filter(|s| !s.kind.is_comment() && s.kind != SegmentKind::Heading)
    {
        let caps: Vec<_> = ACRONYM_RE.captures_iter(&seg.text).collect();
        for (i, c) in caps.iter().enumerate() {
            let m = c.get(1).expect("regex group 1 is not optional");
            let acr = m.as_str();
            if KNOWN_ACRONYMS.contains(&acr)
                || ROMAN.contains(&acr)
                || accepted.contains(acr)
                || super::identifier_like(
                    &seg.text,
                    &c.get(0).expect("group 0 is the whole match").range(),
                )
                || !seen.insert(acr)
            {
                continue;
            }
            // Shouting ("DO NOT EDIT"), not an acronym.
            let adjacent = |j: Option<usize>| {
                j.and_then(|j| caps.get(j)).is_some_and(|o| {
                    let (a, b) = (
                        o.get(0).expect("regex group 0 is not optional"),
                        c.get(0).expect("regex group 0 is not optional"),
                    );
                    let between = if a.start() < b.start() {
                        &seg.text[a.end()..b.start()]
                    } else {
                        &seg.text[b.end()..a.start()]
                    };
                    between.trim().is_empty()
                })
            };
            if adjacent(i.checked_sub(1)) || adjacent(Some(i + 1)) {
                continue;
            }
            let abs = seg.range.start + m.start();
            // "Long Form (ACR)" before or at this use, or "ACR (long form)" right here.
            if defined.get(acr).is_some_and(|&d| d <= abs) {
                continue;
            }
            let after =
                seg.text[c.get(0).expect("group 0 is the whole match").end()..].trim_start();
            if after.starts_with('(') {
                continue;
            }
            out.push(
                Finding::new(
                    RULE,
                    sev(RULE),
                    seg.abs(m.range()),
                    format!("Acronym \"{acr}\" is used before it is defined"),
                )
                .help(format!("Spell it out on first use: \"Long Form ({acr})\".")),
            );
        }
    }
}

// prose/smart-quotes

fn smart_quotes(ctx: &FileCtx, out: &mut Out) {
    const RULE: &str = "prose/smart-quotes";
    // (straight, curly) occurrences for double quotes and single quotes/apostrophes.
    let mut double: (Vec<Range<usize>>, Vec<Range<usize>>) = Default::default();
    let mut single: (Vec<Range<usize>>, Vec<Range<usize>>) = Default::default();
    for seg in &ctx.a.segments {
        for (i, ch) in seg.text.char_indices() {
            let r = seg.abs(i..i + ch.len_utf8());
            match ch {
                '"' => double.0.push(r),
                '“' | '”' => double.1.push(r),
                '\'' => single.0.push(r),
                '‘' | '’' => single.1.push(r),
                _ => {}
            }
        }
    }
    for (kind, (straight, curly)) in [("double", double), ("single", single)] {
        if straight.is_empty() || curly.is_empty() {
            continue;
        }
        let straight_wins = straight.len() > curly.len()
            || (straight.len() == curly.len() && straight[0].start < curly[0].start);
        let (minority, style) = if straight_wins {
            (&curly, "straight")
        } else {
            (&straight, "curly")
        };
        let plain = if kind == "double" { "\"" } else { "'" };
        for r in minority {
            let mut f = Finding::new(
                RULE,
                sev(RULE),
                r.clone(),
                format!(
                    "Mixed straight and curly {kind} quotes; this file mostly uses {style} ones"
                ),
            );
            // Curly -> straight is always safe; straight -> curly needs open/close context.
            if straight_wins {
                f = f.suggest(plain).fix(r.clone(), plain);
            }
            out.push(f);
        }
    }
}
