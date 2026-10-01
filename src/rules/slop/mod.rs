//! AI slop and mannerisms.
//!
//! - `slop/phrase`, `slop/word`: the phrase catalogue (`phrases.toml` + `[slop] extra`).
//! - `slop/not-just-but`, `slop/hedging`: per segment; `slop/em-dash`, `slop/rule-of-three`,
//!   `slop/density`: per section (Markdown text between headings, or one comment block).
//! - `slop/negation-chain`, `slop/dont-verb-it`, `slop/echo-sentences`, `slop/stacked-questions`,
//!   `slop/anaphora`, `slop/stranded-auxiliary`, `slop/colon-triple`, `slop/not-but`: per
//!   segment; ported from `simonw/tools` `llm-cliche-highlighter` (see `cliche.rs`).
//! - `slop/comment-*`: comment heuristics ported from aislop (see `comments.rs`).
//!
//! Text inside block quotes is often quoted from someone else, so findings there are
//! still reported (the quote may be the author's own slop) but downgraded to info.

pub mod catalogue;
mod cliche;
mod comments;
pub(crate) mod languages;
mod mannerisms;

use std::collections::HashMap;

use super::{FileCtx, Out};
use crate::diagnostic::{Finding, Severity};
use crate::segment::{Segment, SegmentKind};
use catalogue::{Category, Kind};

/// Default severity of a slop rule (the engine applies config overrides).
pub(crate) fn sev(rule: &str) -> Severity {
    super::default_severity(rule).unwrap_or(Severity::Warning)
}

/// Severity for a finding inside `seg`: info inside block quotes.
fn seg_sev(rule: &str, seg: &Segment) -> Severity {
    if seg.kind == SegmentKind::BlockQuote {
        Severity::Info
    } else {
        sev(rule)
    }
}

/// Markdown segments between headings, or a single comment block.
fn sections(segments: &[Segment]) -> Vec<Vec<&Segment>> {
    let mut out: Vec<Vec<&Segment>> = Vec::new();
    let mut cur: Vec<&Segment> = Vec::new();
    for s in segments {
        if s.kind.is_comment() {
            if !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
            out.push(vec![s]);
            continue;
        }
        if s.kind == SegmentKind::Heading && !cur.is_empty() {
            out.push(std::mem::take(&mut cur));
        }
        cur.push(s);
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn default_advice(cat: Category) -> &'static str {
    match cat {
        Category::Boilerplate | Category::Sycophancy | Category::Closer | Category::Opener => {
            "Delete it and start with the substance."
        }
        Category::Filler => "Delete it; the sentence works without it.",
        Category::Hype => "Replace the claim with a concrete fact or number.",
        Category::Hedge => "Commit to the claim or state the condition under which it holds.",
        Category::Metaphor => "Say it literally.",
        Category::Vocabulary => "Use a plain, specific word.",
    }
}

/// Match the capitalization of the replaced text.
fn match_case(replacement: &str, original: &str) -> String {
    let upper_first = original.chars().next().is_some_and(char::is_uppercase);
    let all_upper = original.chars().filter(|c| c.is_alphabetic()).count() > 1
        && original
            .chars()
            .filter(|c| c.is_alphabetic())
            .all(char::is_uppercase);
    if all_upper {
        replacement.to_uppercase()
    } else if upper_first {
        let mut c = replacement.chars();
        c.next()
            .map(|f| f.to_uppercase().chain(c).collect())
            .unwrap_or_default()
    } else {
        replacement.to_string()
    }
}

pub fn check(ctx: &FileCtx, out: &mut Out) {
    if !ctx.family_enabled("slop/") {
        return;
    }
    let on = |r: &str| ctx.enabled(r);
    let (phrase, word, density) = (on("slop/phrase"), on("slop/word"), on("slop/density"));
    let (not_just, hedging, em_dash, triads) = (
        on("slop/not-just-but"),
        on("slop/hedging"),
        on("slop/em-dash"),
        on("slop/rule-of-three"),
    );
    let slop = &ctx.config.slop;

    if phrase || word || density || not_just || hedging || em_dash || triads {
        let cat = catalogue::for_config(ctx.config);
        for sec in sections(&ctx.a.segments) {
            let mut counts: HashMap<String, usize> = HashMap::new();
            let mut hits_total = 0usize;
            for seg in &sec {
                for hit in cat.find(&seg.text) {
                    let e = &cat.entries[hit.entry];
                    let matched = &seg.text[hit.range.clone()];
                    hits_total += 1;
                    *counts
                        .entry(matched.to_lowercase().replace('’', "'"))
                        .or_default() += 1;
                    let enabled = match e.kind {
                        Kind::Phrase => phrase,
                        Kind::Word => word,
                    };
                    if !enabled {
                        continue;
                    }
                    let rule = e.kind.rule();
                    let severity = match (seg.kind, e.severity) {
                        (SegmentKind::BlockQuote, _) | (_, None) => seg_sev(rule, seg),
                        (_, Some(s)) => s,
                    };
                    let mut f = Finding::new(
                        rule,
                        severity,
                        seg.abs(hit.range.clone()),
                        format!("\"{matched}\": {}", e.message),
                    )
                    .help(
                        e.advice
                            .clone()
                            .unwrap_or_else(|| default_advice(e.category).to_string()),
                    );
                    for r in &e.replace {
                        f = f.suggest(match_case(r, matched));
                    }
                    out.push(f);
                }
                if not_just {
                    mannerisms::not_just_but(seg, seg_sev("slop/not-just-but", seg), out);
                }
                if hedging {
                    mannerisms::hedging(seg, seg_sev("slop/hedging", seg), out);
                }
            }
            if em_dash {
                mannerisms::em_dash(&sec, slop.em_dash_per_100, sev("slop/em-dash"), out);
            }
            if triads {
                mannerisms::rule_of_three(&sec, sev("slop/rule-of-three"), out);
            }
            if density {
                density_check(
                    &sec,
                    hits_total,
                    &counts,
                    slop.density_per_100,
                    slop.density_min_words,
                    out,
                );
            }
        }
    }

    cliche_check(ctx, out);
    comments::check(ctx, out);
}

type SegCheck = fn(&Segment, Severity, &mut Out);

/// Structural cliché rules, one segment at a time.
fn cliche_check(ctx: &FileCtx, out: &mut Out) {
    // (rule, prose paragraphs/comments only, detector)
    const CHECKS: &[(&str, bool, SegCheck)] = &[
        ("slop/negation-chain", false, cliche::negation_chain),
        ("slop/dont-verb-it", false, cliche::dont_verb_it),
        ("slop/echo-sentences", true, cliche::echo_sentences),
        ("slop/stacked-questions", true, cliche::stacked_questions),
        ("slop/anaphora", true, cliche::anaphora),
        ("slop/stranded-auxiliary", false, cliche::stranded_auxiliary),
        ("slop/colon-triple", false, cliche::colon_triple),
        ("slop/not-but", false, cliche::not_but),
    ];
    let active: Vec<&(&str, bool, SegCheck)> =
        CHECKS.iter().filter(|(r, _, _)| ctx.enabled(r)).collect();
    if active.is_empty() {
        return;
    }
    for seg in &ctx.a.segments {
        for (rule, prose_only, f) in &active {
            let ok = if *prose_only {
                cliche::is_prose_block(seg.kind)
            } else {
                cliche::is_sentence_block(seg.kind)
            };
            if ok {
                f(seg, seg_sev(rule, seg), out);
            }
        }
    }
}

fn density_check(
    sec: &[&Segment],
    hits: usize,
    counts: &HashMap<String, usize>,
    per_100: f64,
    min_words: usize,
    out: &mut Out,
) {
    let words: usize = sec.iter().map(|s| mannerisms::word_count(&s.text)).sum();
    if words == 0 || words < min_words || hits == 0 {
        return;
    }
    let rate = hits as f64 * 100.0 / words as f64;
    if rate <= per_100 {
        return;
    }
    // Anchor on the first word of the section.
    let Some((seg, first)) = sec.iter().find_map(|s| {
        mannerisms::words(&s.text)
            .into_iter()
            .next()
            .map(|w| (s, w))
    }) else {
        return;
    };
    let mut top: Vec<(&String, &usize)> = counts.iter().collect();
    top.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    let list: Vec<String> = top
        .iter()
        .take(5)
        .map(|(w, n)| format!("\"{w}\" ({n})"))
        .collect();
    out.push(
        Finding::new(
            "slop/density",
            sev("slop/density"),
            seg.abs(first),
            format!("High slop density: {hits} stock phrases in {words} words ({rate:.1} per 100, limit {per_100})"),
        )
        .help(format!("Most frequent: {}. Rewrite the section in plain, specific language.", list.join(", "))),
    );
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::config::Config;
    use crate::rules::Analyzed;
    use crate::source::{FileKind, SourceFile};

    fn md(src: &str) -> Analyzed {
        Analyzed::new(SourceFile::new(
            PathBuf::from("/t.md"),
            PathBuf::from("t.md"),
            FileKind::Markdown,
            src.to_string(),
        ))
    }

    fn run_with(src: &str, config: &Config) -> Vec<Finding> {
        let a = md(src);
        let ctx = FileCtx { a: &a, config };
        let mut out = Vec::new();
        check(&ctx, &mut out);
        out
    }

    fn run(src: &str) -> Vec<Finding> {
        run_with(src, &Config::default())
    }

    #[test]
    fn phrase_findings_have_ranges_and_suggestions() {
        let src = "# Intro\n\nLet's delve into the `delve` code.\n";
        let out = run(src);
        let f = out
            .iter()
            .find(|f| f.rule == "slop/phrase")
            .expect("phrase finding");
        assert_eq!(&src[f.range.clone()], "delve");
        assert!(f.message.contains("\"delve\""));
        assert!(f.help.is_some());
        assert_eq!(f.suggestions[0], "explore");
        // Inline code is not prose.
        assert_eq!(out.iter().filter(|f| f.rule == "slop/phrase").count(), 1);
    }

    #[test]
    fn suggestions_match_case() {
        let out = run("Leverage the cache.\n");
        let f = out.iter().find(|f| f.rule == "slop/word").unwrap();
        assert_eq!(f.severity, Severity::Info);
        assert_eq!(f.suggestions, ["Use"]);
    }

    #[test]
    fn blockquote_is_info() {
        let out = run("> Let's delve into it.\n");
        let f = out.iter().find(|f| f.rule == "slop/phrase").unwrap();
        assert_eq!(f.severity, Severity::Info);
    }

    #[test]
    fn code_blocks_are_ignored() {
        assert!(run("```\nLet's delve into it — now — really — yes.\n```\n").is_empty());
    }

    #[test]
    fn density_per_section() {
        let filler = "The service stores rows in a table and returns them on request. ".repeat(10);
        let slop = "We leverage robust, seamless and crucial tooling to foster synergy. ".repeat(3);
        let src = format!("# A\n\n{filler}\n\n# B\n\n{slop}{filler}\n");
        let out = run(&src);
        let d: Vec<&Finding> = out.iter().filter(|f| f.rule == "slop/density").collect();
        assert_eq!(
            d.len(),
            1,
            "{:?}",
            out.iter().map(|f| &f.message).collect::<Vec<_>>()
        );
        assert_eq!(&src[d[0].range.clone()], "B");
        assert!(d[0].help.as_ref().unwrap().contains("leverage"));
    }

    #[test]
    fn density_respects_min_words() {
        let out = run("We leverage robust tooling.\n");
        assert!(out.iter().all(|f| f.rule != "slop/density"));
    }

    #[test]
    fn em_dash_per_section() {
        let src =
            "# A\n\nFast — small — and cheap — really.\n\n# B\n\nNo dashes here, just words.\n";
        let out = run(src);
        assert_eq!(out.iter().filter(|f| f.rule == "slop/em-dash").count(), 3);
    }

    #[test]
    fn mannerisms_run_on_markdown() {
        let out = run("This is not just a tool, but a platform. It might possibly work.\n");
        assert!(out.iter().any(|f| f.rule == "slop/not-just-but"));
        assert!(out.iter().any(|f| f.rule == "slop/hedging"));
    }

    #[test]
    fn rules_can_be_disabled() {
        let mut config = Config::default();
        config
            .rules
            .insert("slop/*".into(), crate::config::Level::Off);
        assert!(run_with("Let's delve into it.\n", &config).is_empty());
        let mut config = Config::default();
        config.slop.disable = vec!["delve".into()];
        assert!(
            run_with("Let's delve into it.\n", &config)
                .iter()
                .all(|f| !f.message.contains("delve"))
        );
    }

    #[test]
    fn sections_split_on_headings_and_comments() {
        let seg = |kind| Segment {
            range: 0..1,
            text: "x".into(),
            kind,
        };
        let segs = vec![
            seg(SegmentKind::Paragraph),
            seg(SegmentKind::Heading),
            seg(SegmentKind::Paragraph),
            seg(SegmentKind::ListItem),
            seg(SegmentKind::Comment),
            seg(SegmentKind::DocComment),
        ];
        let lens: Vec<usize> = sections(&segs).iter().map(Vec::len).collect();
        assert_eq!(lens, [1, 3, 1, 1]);
    }
}
