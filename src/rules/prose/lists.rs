// explicit-disable-file slop/* prose/* -- examples of the patterns this module detects
//! Word-list rules: `prose/inclusive`, `prose/simplify`, `prose/terminology`.
//!
//! `inclusive.toml` and `simplify.toml` hold `entry = [{ match, replace, except?, fix?, note? }]`:
//! `match` variants are ASCII case-insensitive whole words (spaces match any whitespace, `'`
//! matches `’`), `replace` are suggestions in order, `except` phrases containing a match
//! suppress it, `fix = true` makes the first replacement an autofix (case preserved).
//! `terminology.toml` maps `Canonical = ["wrong", "forms"]`, matched case-sensitively.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::ops::Range;
use std::sync::LazyLock;

use regex::{Regex, RegexBuilder};
use serde::Deserialize;

use super::{identifier_like, match_case, quoted, sev, verbatim};
use crate::diagnostic::Finding;
use crate::rules::{FileCtx, Out};
use crate::segment::{Segment, SegmentKind};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawList {
    entry: Vec<RawEntry>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawEntry {
    #[serde(rename = "match")]
    matches: Vec<String>,
    replace: Vec<String>,
    #[serde(default)]
    except: Vec<String>,
    #[serde(default)]
    fix: bool,
    #[serde(default)]
    note: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawTerms {
    terms: BTreeMap<String, Vec<String>>,
}

pub(super) struct Entry {
    pub replace: Vec<String>,
    except: Option<Regex>,
    pub fix: bool,
    pub note: Option<String>,
}

pub(super) struct List {
    case_sensitive: bool,
    pub entries: Vec<Entry>,
    re: Regex,
    lookup: HashMap<String, usize>,
}

pub(super) struct Hit {
    pub range: Range<usize>,
    pub entry: usize,
}

fn key(s: &str, case_sensitive: bool) -> String {
    let k = s
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace('’', "'");
    if case_sensitive { k } else { k.to_lowercase() }
}

fn pattern(variant: &str) -> String {
    let body = variant
        .split_whitespace()
        .map(|w| regex::escape(w).replace('\'', "['’]"))
        .collect::<Vec<_>>()
        .join(r"\s+");
    let word = |c: Option<char>| c.is_some_and(|c| c.is_alphanumeric() || c == '_');
    let lead = if word(variant.chars().next()) {
        r"\b"
    } else {
        ""
    };
    let tail = if word(variant.chars().next_back()) {
        r"\b"
    } else {
        ""
    };
    format!("{lead}{body}{tail}")
}

fn alternation(variants: &[&String]) -> String {
    let mut v: Vec<&&String> = variants.iter().collect();
    // Longest first so "sanity checks" wins over "sanity check" at the same position.
    v.sort_by_key(|s| std::cmp::Reverse(s.len()));
    v.iter().map(|s| pattern(s)).collect::<Vec<_>>().join("|")
}

impl List {
    fn build(raw: Vec<RawEntry>, case_sensitive: bool) -> List {
        let mut lookup = HashMap::new();
        let mut variants = Vec::new();
        let mut entries = Vec::new();
        for (i, e) in raw.into_iter().enumerate() {
            for m in &e.matches {
                let prev = lookup.insert(key(m, case_sensitive), i);
                assert!(prev.is_none(), "duplicate prose list variant {m:?}");
            }
            variants.extend(e.matches.clone());
            let except = (!e.except.is_empty()).then(|| {
                let refs: Vec<&String> = e.except.iter().collect();
                RegexBuilder::new(&alternation(&refs))
                    .case_insensitive(true)
                    .build()
                    .expect("prose list exception regex")
            });
            entries.push(Entry {
                replace: e.replace,
                except,
                fix: e.fix,
                note: e.note,
            });
        }
        let refs: Vec<&String> = variants.iter().collect();
        let re = RegexBuilder::new(&alternation(&refs))
            .case_insensitive(!case_sensitive)
            .size_limit(1 << 24)
            .build()
            .expect("prose list regex");
        List {
            case_sensitive,
            entries,
            re,
            lookup,
        }
    }

    fn parse(src: &str, case_sensitive: bool) -> List {
        let raw: RawList = toml::from_str(src).expect("built-in prose list");
        List::build(raw.entry, case_sensitive)
    }

    pub fn find(&self, text: &str) -> Vec<Hit> {
        let mut out = Vec::new();
        for m in self.re.find_iter(text) {
            let range = m.range();
            if identifier_like(text, &range) {
                continue;
            }
            let Some(&entry) = self.lookup.get(&key(m.as_str(), self.case_sensitive)) else {
                continue;
            };
            let excepted = self.entries[entry].except.as_ref().is_some_and(|re| {
                re.find_iter(text)
                    .any(|x| x.start() <= range.start && range.end <= x.end())
            });
            if !excepted {
                out.push(Hit { range, entry });
            }
        }
        out
    }

    /// Whether `word` is exactly one of the listed variants.
    pub fn contains(&self, word: &str) -> bool {
        self.lookup.contains_key(&key(word, self.case_sensitive))
    }
}

pub(super) static INCLUSIVE: LazyLock<List> =
    LazyLock::new(|| List::parse(include_str!("inclusive.toml"), false));
pub(super) static SIMPLIFY: LazyLock<List> =
    LazyLock::new(|| List::parse(include_str!("simplify.toml"), false));
pub(super) static TERMINOLOGY: LazyLock<List> = LazyLock::new(|| {
    let raw: RawTerms = toml::from_str(include_str!("terminology.toml")).expect("terminology");
    let entries = raw
        .terms
        .into_iter()
        .map(|(canonical, wrong)| RawEntry {
            matches: wrong,
            replace: vec![canonical],
            except: Vec::new(),
            fix: true,
            note: None,
        })
        .collect();
    List::build(entries, true)
});

pub fn check(ctx: &FileCtx, out: &mut Out) {
    let inclusive = ctx.enabled("prose/inclusive");
    let simplify = ctx.enabled("prose/simplify");
    let terminology = ctx.enabled("prose/terminology");
    // Harper (OrthographicConsistency) already corrects the casing of many names, and the engine
    // runs grammar first: skip ranges it reported.
    let harper: HashSet<Range<usize>> = out
        .iter()
        .filter(|f| f.rule.starts_with("grammar/"))
        .map(|f| f.range.clone())
        .collect();
    for seg in &ctx.a.segments {
        if inclusive {
            inclusive_seg(ctx.src(), seg, out);
        }
        if simplify && !quoted(seg) {
            simplify_seg(ctx.src(), seg, out);
        }
        if terminology {
            terminology_seg(ctx.src(), seg, &harper, out);
        }
    }
}

fn suggestions(e: &Entry, matched: &str) -> Vec<String> {
    e.replace.iter().map(|r| match_case(r, matched)).collect()
}

fn inclusive_seg(src: &str, seg: &Segment, out: &mut Out) {
    const RULE: &str = "prose/inclusive";
    for hit in INCLUSIVE.find(&seg.text) {
        let e = &INCLUSIVE.entries[hit.entry];
        let matched = &seg.text[hit.range.clone()];
        let sugg = suggestions(e, matched);
        let help = e
            .note
            .clone()
            .unwrap_or_else(|| format!("Consider {}.", quote_list(&sugg)));
        let mut f = Finding::new(
            RULE,
            sev(RULE),
            seg.abs(hit.range.clone()),
            format!("\"{matched}\" may be insensitive or exclusionary"),
        )
        .help(help);
        if e.fix && seg.kind != SegmentKind::Heading && verbatim(src, seg, &hit.range) {
            f = f.fix(seg.abs(hit.range.clone()), sugg[0].clone());
        }
        for s in sugg {
            f = f.suggest(s);
        }
        out.push(f);
    }
}

fn simplify_seg(src: &str, seg: &Segment, out: &mut Out) {
    const RULE: &str = "prose/simplify";
    for hit in SIMPLIFY.find(&seg.text) {
        let e = &SIMPLIFY.entries[hit.entry];
        let matched = &seg.text[hit.range.clone()];
        let sugg = suggestions(e, matched);
        let mut f = Finding::new(
            RULE,
            sev(RULE),
            seg.abs(hit.range.clone()),
            format!("\"{matched}\" has a simpler alternative"),
        )
        .help(
            e.note
                .clone()
                .unwrap_or_else(|| format!("Use {}.", quote_list(&sugg))),
        );
        if e.fix && seg.kind != SegmentKind::Heading && verbatim(src, seg, &hit.range) {
            f = f.fix(seg.abs(hit.range.clone()), sugg[0].clone());
        }
        for s in sugg {
            f = f.suggest(s);
        }
        out.push(f);
    }
}

fn terminology_seg(src: &str, seg: &Segment, harper: &HashSet<Range<usize>>, out: &mut Out) {
    const RULE: &str = "prose/terminology";
    for hit in TERMINOLOGY.find(&seg.text) {
        let canonical = &TERMINOLOGY.entries[hit.entry].replace[0];
        let matched = &seg.text[hit.range.clone()];
        let range = seg.abs(hit.range.clone());
        if harper.contains(&range) {
            continue;
        }
        let mut f = Finding::new(
            RULE,
            sev(RULE),
            range.clone(),
            format!("Use \"{canonical}\" instead of \"{matched}\""),
        )
        .suggest(canonical.clone());
        // Heading edits would change the anchor; suggestion only.
        if seg.kind != SegmentKind::Heading && verbatim(src, seg, &hit.range) {
            f = f.fix(range, canonical.clone());
        }
        out.push(f);
    }
}

fn quote_list(items: &[String]) -> String {
    items
        .iter()
        .map(|s| format!("\"{s}\""))
        .collect::<Vec<_>>()
        .join(" or ")
}
