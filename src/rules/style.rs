//! Vale-style rules defined in `explicit.toml` under `[[style]]`.

use std::collections::HashMap;
use std::ops::Range;
use std::sync::{LazyLock, Mutex};

use regex::{Regex, RegexBuilder};

use super::{FileCtx, Out};
use crate::config::{StyleKind, StyleRule};
use crate::diagnostic::{Finding, Severity};
use crate::segment::{Segment, SegmentKind};

static WORD_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"[\p{L}\p{N}][\p{L}\p{N}'’-]*").expect("hardcoded regex is valid")
});

/// Compiled pattern per (rule name, token), cached across files.
type PatternCache = HashMap<(String, String), Option<Regex>>;
static CACHE: LazyLock<Mutex<PatternCache>> = LazyLock::new(Mutex::default);

fn compile(rule: &StyleRule, token: &str) -> Option<Regex> {
    let key = (rule.name.clone(), token.to_string());
    let mut cache = CACHE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    cache
        .entry(key)
        .or_insert_with(|| {
            let pat = if rule.regex {
                token.to_string()
            } else {
                format!(r"\b{}\b", regex::escape(token))
            };
            RegexBuilder::new(&pat)
                .case_insensitive(rule.ignore_case.unwrap_or(!rule.regex))
                .build()
                .ok()
        })
        .clone()
}

fn fmt_message(rule: &StyleRule, default: &str, matched: &str) -> String {
    rule.message
        .as_deref()
        .unwrap_or(default)
        .replace("{}", matched)
}

fn in_scope(rule: &StyleRule, seg: &Segment) -> bool {
    match rule.scope.as_deref() {
        Some("markdown") => !seg.kind.is_comment(),
        Some("comments") => seg.kind.is_comment(),
        _ => true,
    }
}

pub fn check(ctx: &FileCtx, out: &mut Out) {
    for rule in &ctx.config.style {
        let id = format!("style/{}", rule.name);
        let Some(severity) = rule.level.severity() else {
            continue;
        };
        if ctx.config.severity(&id, Some(severity)).is_none() {
            continue;
        }
        for seg in ctx.a.segments.iter().filter(|s| in_scope(rule, s)) {
            match rule.kind {
                StyleKind::Existence => existence(rule, &id, severity, seg, out),
                StyleKind::Substitution => substitution(ctx.src(), rule, &id, severity, seg, out),
                StyleKind::Repetition => repetition(ctx.src(), rule, &id, severity, seg, out),
                StyleKind::Occurrence => occurrence(rule, &id, severity, seg, out),
                StyleKind::Capitalization => capitalization(rule, &id, severity, seg, out),
            }
        }
    }
}

fn existence(rule: &StyleRule, id: &str, sev: Severity, seg: &Segment, out: &mut Out) {
    for t in &rule.tokens {
        let Some(re) = compile(rule, t) else { continue };
        for m in re.find_iter(&seg.text) {
            out.push(Finding::new(
                id,
                sev,
                seg.abs(m.range()),
                fmt_message(rule, "Avoid '{}'", m.as_str()),
            ));
        }
    }
}

/// Whether a fix may rewrite `seg.text[r]`: not in a heading (anchors would change) and no
/// masked content (code spans, markup) inside, which the fix would silently delete.
fn fixable(src: &str, seg: &Segment, r: Range<usize>) -> bool {
    seg.kind != SegmentKind::Heading && src.get(seg.abs(r.clone())) == seg.text.get(r)
}

fn substitution(
    src: &str,
    rule: &StyleRule,
    id: &str,
    sev: Severity,
    seg: &Segment,
    out: &mut Out,
) {
    for (bad, good) in &rule.swap {
        let Some(re) = compile(rule, bad) else {
            continue;
        };
        for m in re.find_iter(&seg.text) {
            if m.as_str() == good {
                continue;
            }
            let range = seg.abs(m.range());
            let msg = rule
                .message
                .as_deref()
                .map(|t| t.replacen("{}", good, 1).replacen("{}", m.as_str(), 1))
                .unwrap_or_else(|| format!("Use '{good}' instead of '{}'", m.as_str()));
            let mut f = Finding::new(id, sev, range.clone(), msg).suggest(good.clone());
            if !rule.regex && fixable(src, seg, m.range()) {
                f = f.fix(range, good.clone());
            }
            out.push(f);
        }
    }
}

fn repetition(src: &str, rule: &StyleRule, id: &str, sev: Severity, seg: &Segment, out: &mut Out) {
    let words: Vec<_> = WORD_RE.find_iter(&seg.text).collect();
    for w in words.windows(2) {
        let between = &seg.text[w[0].end()..w[1].start()];
        let same = w[0].as_str().eq_ignore_ascii_case(w[1].as_str());
        let listed = rule.tokens.is_empty()
            || rule
                .tokens
                .iter()
                .any(|t| t.eq_ignore_ascii_case(w[0].as_str()));
        if same && listed && between.chars().all(|c| c == ' ' || c == '\t' || c == '\n') {
            let range = seg.abs(w[0].start()..w[1].end());
            let f = Finding::new(
                id,
                sev,
                range,
                fmt_message(rule, "'{}' is repeated", w[0].as_str()),
            );
            out.push(if fixable(src, seg, w[0].start()..w[1].end()) {
                f.fix(seg.abs(w[0].end()..w[1].end()), "")
            } else {
                f
            });
        }
    }
}

fn occurrence(rule: &StyleRule, id: &str, sev: Severity, seg: &Segment, out: &mut Out) {
    let max = rule.max.unwrap_or(1);
    let mut hits = Vec::new();
    for t in &rule.tokens {
        if let Some(re) = compile(rule, t) {
            hits.extend(
                re.find_iter(&seg.text)
                    .map(|m| (m.start(), m.end(), m.as_str().to_string())),
            );
        }
    }
    hits.sort();
    if hits.len() > max {
        let (s, e, text) = &hits[max];
        let msg = rule.message.clone().unwrap_or_else(|| {
            format!("'{text}' used {} times; at most {max} allowed", hits.len())
        });
        out.push(Finding::new(id, sev, seg.abs(*s..*e), msg));
    }
}

const SMALL_WORDS: &[&str] = &[
    "a", "an", "and", "as", "at", "but", "by", "for", "in", "nor", "of", "on", "or", "per", "the",
    "to", "vs", "via", "with", "from", "into", "over", "up",
];

fn capitalization(rule: &StyleRule, id: &str, sev: Severity, seg: &Segment, out: &mut Out) {
    if seg.kind != SegmentKind::Heading {
        return;
    }
    let title = rule.case.as_deref() == Some("title");
    if let Some(expected) = recase(&seg.text, title, &rule.exceptions) {
        let found = seg.text.trim();
        if expected != found {
            let start = seg.text.find(found).unwrap_or(0);
            let range = seg.abs(start..start + found.len());
            let case = if title { "title" } else { "sentence" };
            out.push(
                Finding::new(
                    id,
                    sev,
                    range,
                    fmt_message(
                        rule,
                        &format!("Heading should use {case} case: '{{}}'"),
                        &expected,
                    ),
                )
                .suggest(expected),
            );
        }
    }
}

/// Expected heading text in sentence or title case. `None` when there are no words to judge.
pub fn recase(text: &str, title: bool, exceptions: &[String]) -> Option<String> {
    let text = text.trim();
    let mut result = String::with_capacity(text.len());
    let mut last = 0;
    let mut any = false;
    for (i, m) in WORD_RE.find_iter(text).enumerate() {
        result.push_str(&text[last..m.start()]);
        last = m.end();
        let w = m.as_str();
        any = true;
        let keep = exceptions.iter().any(|e| e == w)
            || w.chars().skip(1).any(|c| c.is_uppercase())
            || w.chars().any(|c| c.is_ascii_digit());
        let after_colon = text[..m.start()].trim_end().ends_with(':');
        let new = if keep {
            w.to_string()
        } else if i == 0
            || after_colon
            || (title && !SMALL_WORDS.contains(&w.to_lowercase().as_str()))
        {
            capitalize(w)
        } else if title {
            w.to_lowercase()
        } else if w.chars().next().is_some_and(char::is_uppercase) && exceptions.is_empty() {
            // Sentence case without an exception list: proper nouns are unknowable, keep capitals.
            w.to_string()
        } else {
            w.to_lowercase()
        };
        result.push_str(&new);
    }
    result.push_str(&text[last..]);
    any.then_some(result)
}

fn capitalize(w: &str) -> String {
    let mut c = w.chars();
    match c.next() {
        Some(f) => f.to_uppercase().chain(c).collect(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::rules::Analyzed;
    use crate::source::{FileKind, SourceFile};

    fn run(src: &str, toml_rules: &str) -> Vec<Finding> {
        let config: Config = toml::from_str(toml_rules).unwrap();
        let a = Analyzed::new(SourceFile::new(
            "a.md".into(),
            "a.md".into(),
            FileKind::Markdown,
            src.into(),
        ));
        let mut out = Vec::new();
        check(
            &FileCtx {
                a: &a,
                config: &config,
            },
            &mut out,
        );
        out
    }

    #[test]
    fn existence_and_substitution() {
        let rules = r#"
            [[style]]
            name = "simple"
            kind = "existence"
            tokens = ["simply"]
            [[style]]
            name = "terms"
            kind = "substitution"
            swap = { "github" = "GitHub" }
        "#;
        let src = "Simply push to github. `simply` in code is fine. GitHub is right.\n";
        let f = run(src, rules);
        assert_eq!(f.len(), 2, "{f:?}");
        assert_eq!(&src[f[0].range.clone()], "Simply");
        assert_eq!(f[1].fix.as_ref().unwrap().replacement, "GitHub");
    }

    #[test]
    fn repetition_and_occurrence() {
        let rules = r#"
            [[style]]
            name = "rep"
            kind = "repetition"
            [[style]]
            name = "very"
            kind = "occurrence"
            tokens = ["very"]
            max = 1
        "#;
        let f = run("This is the the very very test.\n", rules);
        assert_eq!(f.iter().filter(|f| f.rule == "style/rep").count(), 2);
        assert_eq!(f.iter().filter(|f| f.rule == "style/very").count(), 1);
    }

    #[test]
    fn substitution_and_repetition_fix_guards() {
        let rules = r#"
            [[style]]
            name = "terms"
            kind = "substitution"
            swap = { "github" = "GitHub" }
            [[style]]
            name = "rep"
            kind = "repetition"
        "#;
        // Heading: suggestion only (a fix would break the anchor).
        let f = run("# Push to github\n", rules);
        assert_eq!(f.len(), 1, "{f:?}");
        assert_eq!(f[0].suggestions[0], "GitHub");
        assert!(f[0].fix.is_none());
        // Match spanning a blanked code span: no fix that would delete the code.
        let f = run("Say the `x` the end.\n", rules);
        let rep: Vec<_> = f.iter().filter(|f| f.rule == "style/rep").collect();
        assert_eq!(rep.len(), 1, "{f:?}");
        assert!(rep[0].fix.is_none());
        // Plain paragraph still fixes.
        let f = run("Push to github.\n", rules);
        assert!(f[0].fix.is_some());
    }

    #[test]
    fn regex_substitution_across_code_has_no_fix() {
        let rules = "[[style]]\nname=\"t\"\nkind=\"substitution\"\nregex=true\nswap={ \"Run\\\\s+for\" = \"Run\" }\n";
        let f = run("Run `explicit rules` for help.\n", rules);
        assert_eq!(f.len(), 1, "{f:?}");
        assert!(f[0].fix.is_none());
    }

    #[test]
    fn heading_case() {
        assert_eq!(
            recase("using the CLI tool", true, &[]).unwrap(),
            "Using the CLI Tool"
        );
        assert_eq!(
            recase("Using The Tool", false, &["Tool".into()]).unwrap(),
            "Using the Tool"
        );
        let f = run(
            "# getting started with explicit\n",
            "[[style]]\nname=\"hc\"\nkind=\"capitalization\"\ncase=\"title\"\n",
        );
        assert_eq!(f[0].suggestions[0], "Getting Started with Explicit");
    }
}
