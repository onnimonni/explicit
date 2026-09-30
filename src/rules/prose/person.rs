//! `prose/ambiguous-person`: a name part two or more `[[person]]` entries share (`Sami`, a
//! shared last name), used alone, without a neighbor that tells them apart (the other name
//! part, an alias or a handle) and without exactly one of them mentioned earlier in the same
//! section, comment block or gettext entry.

use std::collections::{HashMap, HashSet};
use std::ops::Range;

use super::{WORD_RE, identifier_like, sev};
use crate::diagnostic::Finding;
use crate::rules::{FileCtx, Out, spell};
use crate::segment::{Segment, SegmentKind};
use crate::vocab::{NameLang, PersonIndex};

const RULE: &str = "prose/ambiguous-person";

/// A word of a segment and what it names.
struct Tok<'a> {
    range: Range<usize>,
    text: &'a str,
    /// The configured word it is a form of.
    base: Option<&'a str>,
}

pub(super) fn check(ctx: &FileCtx, out: &mut Out) {
    if !ctx.enabled(RULE) || ctx.config.persons.len() < 2 {
        return;
    }
    if ctx.config.is_test_path(&ctx.a.file.rel) {
        return;
    }
    let index = ctx.config.person_index("*");
    let mut shared = HashMap::<&str, usize>::new();
    for p in &index.persons {
        for part in p.parts.iter().collect::<HashSet<_>>() {
            *shared.entry(part.as_str()).or_default() += 1;
        }
    }
    if !shared.values().any(|&n| n >= 2) {
        return;
    }
    let speller = spell::dictionary(&ctx.config.prose.dialect);
    let mut mentioned: HashMap<usize, HashSet<usize>> = HashMap::new();
    for (scope, seg) in scopes(ctx) {
        let seen = mentioned.entry(scope).or_default();
        check_segment(seg, &index, speller, seen, out);
    }
}

/// Each segment with the id of its context: the Markdown section (a heading opens one), the
/// gettext entry, or the comment block (the segment itself).
fn scopes<'a>(ctx: &FileCtx<'a>) -> Vec<(usize, &'a Segment)> {
    let segs = &ctx.a.segments;
    let mut out = Vec::with_capacity(segs.len());
    // Ids above `segs.len()` are sections; below, single segments.
    let mut section = segs.len();
    for (i, seg) in segs.iter().enumerate() {
        let id = if let Some(po) = &ctx.a.po {
            po.entries
                .iter()
                .position(|e| e.range.contains(&seg.range.start))
                .map_or(i, |e| segs.len() + 1 + e)
        } else if ctx.a.md.is_some() && !seg.kind.is_comment() {
            if seg.kind == SegmentKind::Heading {
                section += 1;
            }
            section
        } else {
            i
        };
        out.push((id, seg));
    }
    out
}

fn check_segment(
    seg: &Segment,
    index: &PersonIndex,
    speller: &spell::Speller,
    seen: &mut HashSet<usize>,
    out: &mut Out,
) {
    let text = seg.text.as_str();
    let toks: Vec<Tok> = WORD_RE
        .find_iter(text)
        .map(|m| Tok {
            range: m.range(),
            text: m.as_str(),
            base: index.base(m.as_str(), NameLang::Any),
        })
        .collect();
    if toks.iter().all(|t| t.base.is_none()) {
        return;
    }
    let mut i = 0;
    while i < toks.len() {
        if let Some((p, len)) = full_mention(text, &toks, i, index) {
            seen.insert(p);
            i += len;
            continue;
        }
        let t = &toks[i];
        let cands = t.base.map_or(&[][..], |b| index.persons_of(b));
        match cands {
            [] => {}
            [p] => {
                seen.insert(*p);
            }
            _ => {
                if let Some(p) = resolve(&toks, i, cands, index, seen) {
                    seen.insert(p);
                } else if !identifier_like(text, &t.range) && !ordinary_word(text, t, speller) {
                    out.push(finding(seg, t, cands, index));
                }
            }
        }
        i += 1;
    }
}

/// The person whose full name or alias starts at token `i`, and its length in tokens.
fn full_mention(text: &str, toks: &[Tok], i: usize, index: &PersonIndex) -> Option<(usize, usize)> {
    let mut best: Option<(usize, usize)> = None;
    for (p, names) in index.persons.iter().enumerate() {
        for words in std::iter::once(&names.parts).chain(&names.aliases) {
            if words.len() < 2 || i + words.len() > toks.len() {
                continue;
            }
            let run = &toks[i..i + words.len()];
            let same = run
                .iter()
                .zip(words)
                .all(|(t, w)| t.base == Some(w.as_str()) || t.text == w.as_str());
            let adjacent = run.windows(2).all(|w| {
                text[w[0].range.end..w[1].range.start]
                    .chars()
                    .all(|c| c.is_whitespace() || c == '.')
            });
            if same && adjacent && best.is_none_or(|(_, n)| words.len() > n) {
                best = Some((p, words.len()));
            }
        }
    }
    best
}

/// Which of `cands` the ambiguous token `i` means: a neighbor within two words names exactly
/// one of them (`Virtanen, Sami`, `Sami (@samiv)`), or exactly one was mentioned earlier in
/// the context.
fn resolve(
    toks: &[Tok],
    i: usize,
    cands: &[usize],
    index: &PersonIndex,
    seen: &HashSet<usize>,
) -> Option<usize> {
    let lo = i.saturating_sub(2);
    let hi = (i + 2).min(toks.len() - 1);
    for (j, t) in toks.iter().enumerate().take(hi + 1).skip(lo) {
        if j == i {
            continue;
        }
        if let Some(b) = t.base
            && let [p] = index.persons_of(b)
            && cands.contains(p)
        {
            return Some(*p);
        }
    }
    let known: Vec<usize> = cands.iter().copied().filter(|p| seen.contains(p)).collect();
    match known.as_slice() {
        [p] => Some(*p),
        _ => None,
    }
}

/// The token reads as an ordinary word opening a sentence (`Will this work?`).
fn ordinary_word(text: &str, t: &Tok, speller: &spell::Speller) -> bool {
    let before = text[..t.range.start].trim_end();
    let sentence_start = before.is_empty() || before.ends_with(['.', '!', '?', ':', '\n']);
    sentence_start && spell::known(speller, &t.text.to_lowercase())
}

fn finding(seg: &Segment, t: &Tok, cands: &[usize], index: &PersonIndex) -> Finding {
    let described: Vec<String> = cands
        .iter()
        .map(|&p| {
            let n = &index.persons[p];
            format!("{} ({})", n.name, n.role)
        })
        .collect();
    let list = match described.split_last() {
        Some((last, rest)) if !rest.is_empty() => format!("{} or {last}", rest.join(", ")),
        _ => described.join(""),
    };
    let mut f = Finding::new(
        RULE,
        sev(RULE),
        seg.abs(t.range.clone()),
        format!("“{}” is ambiguous: {list}; write the full name", t.text),
    );
    for &p in cands {
        f = f.suggest(index.persons[p].name.clone());
    }
    f
}
