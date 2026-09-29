//! Raw findings of the grammar and spelling pass, before they become
//! [`crate::diagnostic::Finding`]s: our pattern rules and spell check produce them, and Harper's
//! lints (with the `harper` feature) convert into them. Spans are char indices into the segment
//! text.

use std::collections::BTreeMap;

/// A char range, end exclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Span {
        Span { start, end }
    }
}

/// The category of a lint; it picks the severity under `prose.grammar_level = "harper"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum LintKind {
    Capitalization,
    Enhancement,
    Grammar,
    #[default]
    Miscellaneous,
    Readability,
    Repetition,
    Spelling,
    Style,
    Typo,
    WordChoice,
    /// Any other Harper category.
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Suggestion {
    /// Replace the text with these chars.
    ReplaceWith(Vec<char>),
    /// Insert these chars after the text.
    InsertAfter(Vec<char>),
    /// Remove the text.
    Remove,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct Lint {
    pub span: Span,
    pub lint_kind: LintKind,
    pub suggestions: Vec<Suggestion>,
    pub message: String,
    /// Lower wins overlaps.
    pub priority: u8,
}

#[cfg(feature = "harper")]
impl From<harper_core::linting::Lint> for Lint {
    fn from(l: harper_core::linting::Lint) -> Lint {
        use harper_core::linting::{LintKind as H, Suggestion as S};
        let lint_kind = match l.lint_kind {
            H::Capitalization => LintKind::Capitalization,
            H::Enhancement => LintKind::Enhancement,
            H::Grammar => LintKind::Grammar,
            H::Miscellaneous => LintKind::Miscellaneous,
            H::Readability => LintKind::Readability,
            H::Repetition => LintKind::Repetition,
            H::Spelling => LintKind::Spelling,
            H::Style => LintKind::Style,
            H::Typo => LintKind::Typo,
            H::WordChoice => LintKind::WordChoice,
            _ => LintKind::Other,
        };
        Lint {
            span: Span::new(l.span.start, l.span.end),
            lint_kind,
            suggestions: l
                .suggestions
                .into_iter()
                .map(|s| match s {
                    S::ReplaceWith(c) => Suggestion::ReplaceWith(c),
                    S::InsertAfter(c) => Suggestion::InsertAfter(c),
                    S::Remove => Suggestion::Remove,
                })
                .collect(),
            message: l.message,
            priority: l.priority,
        }
    }
}

/// Harper's `remove_overlaps_map`: of overlapping lints (across all rules) keep the one that
/// starts first, then the longest, then the lowest priority value.
pub fn remove_overlaps_map<K: Ord>(lint_map: &mut BTreeMap<K, Vec<Lint>>) {
    let total: usize = lint_map.values().map(Vec::len).sum();
    if total < 2 {
        return;
    }
    struct Indexed {
        rule: usize,
        lint: usize,
        priority: u8,
        start: usize,
        end: usize,
    }
    let mut remove: Vec<Vec<bool>> = lint_map.values().map(|l| vec![false; l.len()]).collect();
    let mut spans = Vec::with_capacity(total);
    for (rule, lints) in lint_map.values().enumerate() {
        for (lint, l) in lints.iter().enumerate() {
            spans.push(Indexed {
                rule,
                lint,
                priority: l.priority,
                start: l.span.start,
                end: l.span.end,
            });
        }
    }
    // Two stable sorts, as Harper does: priority breaks ties of the same span.
    spans.sort_by_key(|s| s.priority);
    spans.sort_by_key(|s| (s.start, usize::MAX - s.end));
    let mut cur = 0;
    for s in spans {
        if s.start < cur {
            remove[s.rule][s.lint] = true;
        } else {
            cur = s.end;
        }
    }
    for (rule, lints) in lint_map.values_mut().enumerate() {
        if remove[rule].iter().any(|r| *r) {
            let mut i = 0;
            lints.retain(|_| {
                i += 1;
                !remove[rule][i - 1]
            });
        }
    }
}
