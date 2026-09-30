//! `prose/entity-name`: `[[entity]]` names and aliases, and case-sensitive `[[vocab]]` terms,
//! written in their configured casing (`telia oy` -> `Telia Oy`). ALL CAPS is fine.

use super::{identifier_like, sev, verbatim};
use crate::diagnostic::Finding;
use crate::rules::{FileCtx, Out, spell};
use crate::segment::SegmentKind;
use crate::vocab::PhraseKind;

const RULE: &str = "prose/entity-name";

pub(super) fn check(ctx: &FileCtx, out: &mut Out) {
    if !ctx.enabled(RULE) {
        return;
    }
    let phrases = ctx.config.vocab_phrases();
    if phrases.is_empty() {
        return;
    }
    let speller = spell::dictionary(&ctx.config.prose.dialect);
    for seg in &ctx.a.segments {
        for m in phrases.find(&seg.text) {
            let matched = &seg.text[m.range.clone()];
            if !phrases.wrong_case(m.phrase, matched, speller) {
                continue;
            }
            // `telia.fi`, `telia-app`, `@telia`: a domain, compound or handle.
            let (prev, next) = (
                seg.text[..m.range.start].chars().next_back(),
                seg.text[m.range.end..].chars().next(),
            );
            if identifier_like(&seg.text, &m.range) || prev == Some('-') || next == Some('-') {
                continue;
            }
            let p = &phrases.list[m.phrase];
            let canonical = crate::vocab::recase(matched, &p.canonical);
            let what = match p.kind {
                PhraseKind::Entity if p.owner == p.canonical => format!("{}: {}", p.owner, p.note),
                PhraseKind::Entity => format!("Short for {}: {}", p.owner, p.note),
                PhraseKind::Vocab => format!("{}: {}", p.owner, p.note),
            };
            let range = seg.abs(m.range.clone());
            let mut f = Finding::new(
                RULE,
                sev(RULE),
                range.clone(),
                format!("Write \"{}\" as configured, not \"{matched}\"", p.canonical),
            )
            .help(what)
            .suggest(canonical.clone());
            // Heading edits would change the anchor; suggestion only.
            if seg.kind != SegmentKind::Heading && verbatim(ctx.src(), seg, &m.range) {
                f = f.fix(range, canonical);
            }
            out.push(f);
        }
    }
}
