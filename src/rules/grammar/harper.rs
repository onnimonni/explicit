//! The Harper side of [`super::Checker`] (cargo feature `harper`): Harper's dictionary, its
//! lint group per Markdown / code-comment config, and its spell check and suggestions.

use std::sync::{Arc, Mutex, PoisonError};

use harper_core::expr::{Filter, SequenceExpr};
use harper_core::linting::{FlatConfig, LintGroup};
use harper_core::parsers::PlainEnglish;
use harper_core::spell::{
    Dictionary, FstDictionary, MergedDictionary, MutableDictionary, suggest_correct_spelling,
};
use harper_core::{
    CharStringExt, DictWordMetadata, Document, TokenStringExt, remove_lints_overlapping_expr,
};

use super::{
    CURATED, HYBRID_HARPER, MAX_CHECKERS, RawLints, TECH_COMPOUNDS, TECH_WORDS,
    explicit_harper_rules, words_key,
};
use crate::config::{Config, Engine};
use crate::rules::lint::{Lint, LintKind, Span};
use crate::rules::patterns;
use crate::rules::words::Dialect;

pub(super) struct Harper {
    pub(super) dict: Arc<MergedDictionary>,
    pub(super) group: LintGroup,
    pub(super) md_config: FlatConfig,
    pub(super) code_config: FlatConfig,
    /// Harper's `word(s)` / lone `s` exception, as in its `SpellCheck`.
    plural_s: Filter,
    dialect: harper_core::Dialect,
}

/// A [`Harper`] plus which of our checks its config leaves on.
pub(super) struct Setup {
    pub harper: Harper,
    /// Our spell check (Harper's `SpellCheck` stays off) for Markdown / code comments.
    pub spell_md: bool,
    pub spell_code: bool,
    /// Some Harper rule is on for Markdown / code comments.
    pub harper_md: bool,
    pub harper_code: bool,
}

/// Harper's built-in and custom dictionary, built once per process for the current word list.
pub(super) fn shared_dictionary(config: &Config) -> Arc<MergedDictionary> {
    // A few word lists at once: per-path overrides may give files different vocabularies.
    static DICTS: Mutex<Vec<(u64, Arc<MergedDictionary>)>> = Mutex::new(Vec::new());
    let key = words_key(config);
    let mut cache = DICTS.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some((_, d)) = cache.iter().find(|(k, _)| *k == key) {
        return d.clone();
    }
    let mut custom = MutableDictionary::new();
    let accepted = config.accepted_words();
    // Harper splits `pre-commit` at the hyphen, so each part must be a word too.
    let parts = accepted
        .iter()
        .filter(|w| w.contains('-'))
        .flat_map(|w| w.split('-'))
        .filter(|p| !p.is_empty());
    for w in TECH_WORDS
        .iter()
        .chain(TECH_COMPOUNDS)
        .copied()
        .chain(accepted.iter().map(String::as_str))
        .chain(parts)
    {
        custom.append_word_str(w, DictWordMetadata::default());
    }
    let mut merged = MergedDictionary::new();
    merged.add_dictionary(FstDictionary::curated());
    merged.add_dictionary(Arc::new(custom));
    let dict = Arc::new(merged);
    if cache.len() >= 2 * MAX_CHECKERS {
        cache.remove(0);
    }
    cache.push((key, dict.clone()));
    dict
}

impl Harper {
    /// Harper for a Harper engine; `spelling` is whether the `spelling` rule is on.
    pub(super) fn setup(config: &Config, dialect: Dialect, spelling: bool) -> Setup {
        let engine = config.prose.engine;
        let dialect = dialect.to_harper();
        let dict = shared_dictionary(config);
        let mut g = LintGroup::new_curated(dict.clone(), dialect);
        let mut md_config = g.config.clone();
        for r in &config.prose.disable {
            md_config.set_rule_enabled(r, false);
        }
        let mut code_config = md_config.clone();
        for r in &config.comments.disable {
            code_config.set_rule_enabled(r, false);
        }
        let allow = match engine {
            Engine::Curated => Some(CURATED),
            Engine::Hybrid => Some(HYBRID_HARPER),
            _ => None,
        };
        if let Some(allow) = allow {
            let keys: Vec<String> = g.iter_keys().map(str::to_string).collect();
            for name in keys.iter().filter(|k| *k != "SpellCheck") {
                if !allow.contains(&name.as_str()) {
                    md_config.set_rule_enabled(name, false);
                    code_config.set_rule_enabled(name, false);
                }
            }
        }
        // Explicit rule config wins over the disable lists and Harper's own defaults.
        for (name, on) in explicit_harper_rules(config) {
            md_config.set_rule_enabled(name, on);
            code_config.set_rule_enabled(name, on);
        }
        // Spelling is ours: same dictionary and dialect, but filters run before the (costly)
        // suggestion search, whose results are shared across threads.
        let spell_md = spelling && md_config.is_rule_enabled("SpellCheck");
        let spell_code = spelling && code_config.is_rule_enabled("SpellCheck");
        md_config.set_rule_enabled("SpellCheck", false);
        code_config.set_rule_enabled("SpellCheck", false);
        // Rules whose findings would be dropped (`"grammar/*" = "off"`) need not run at all.
        // Our own pattern rules replace Harper's in the `hybrid` engine.
        for name in g.iter_keys() {
            let own = engine == Engine::Hybrid && name != "AnA" && patterns::RULES.contains(&name);
            if name != "SpellCheck"
                && (own || !crate::rules::rule_enabled(config, &format!("grammar/{name}")))
            {
                md_config.set_rule_enabled(name, false);
                code_config.set_rule_enabled(name, false);
            }
        }
        let harper_md = g.iter_keys().any(|k| md_config.is_rule_enabled(k));
        let harper_code = g.iter_keys().any(|k| code_config.is_rule_enabled(k));
        g.config = md_config.clone();
        Setup {
            harper: Harper {
                dict,
                group: g,
                md_config,
                code_config,
                plural_s: parenthetical_plural_s(),
                dialect,
            },
            spell_md,
            spell_code,
            harper_md,
            harper_code,
        }
    }

    /// Use the Markdown or the code-comment rule config.
    pub(super) fn select(&mut self, is_code: bool) {
        self.group.config = if is_code {
            self.code_config.clone()
        } else {
            self.md_config.clone()
        };
    }

    /// Harper's rule `name` runs under the selected config.
    pub(super) fn rule_on(&self, name: &str) -> bool {
        self.group.config.is_rule_enabled(name)
    }

    /// In Harper's dictionary in some capitalization.
    pub(super) fn contains(&self, w: &str) -> bool {
        self.dict.contains_word_str(w)
    }

    /// In Harper's dictionary, in the dialect: `Behaviour` is no more right in an American
    /// comment than in prose.
    pub(super) fn contains_in_dialect(&self, w: &str) -> bool {
        self.dict.contains_word_str(w)
            && self
                .dict
                .get_word_metadata_str(w)
                .is_none_or(|m| m.dialects.is_dialect_enabled(self.dialect))
    }

    /// Harper's rule lints (when `rules`) and its spell check (when `spell`) for `text`.
    pub(super) fn lints(&mut self, text: &str, rules: bool, spell: bool) -> RawLints {
        let doc = Document::new(text, &PlainEnglish, self.dict.as_ref());
        let mut lints = RawLints::new();
        if rules {
            for (k, v) in self.group.organized_lints(&doc) {
                lints.insert(k, v.into_iter().map(Lint::from).collect());
            }
        }
        if spell {
            // Same key as Harper's rule, so overlap resolution picks the same winners.
            lints.insert("SpellCheck".to_string(), self.misspelled(&doc));
        }
        lints
    }

    /// Words Harper's `SpellCheck` would flag, by the same known-word test.
    pub(super) fn misspelled(&self, doc: &Document) -> Vec<Lint> {
        let mut lints = Vec::new();
        for word in doc.iter_words() {
            let chars = doc.get_span_content(&word.span);
            if is_informal_laughter(chars) {
                continue;
            }
            if let Some(Some(meta)) = word.kind.as_word()
                && meta.dialects.is_dialect_enabled(self.dialect)
                && (self.dict.contains_exact_word(chars)
                    || self.dict.contains_exact_word(&chars.to_lower()))
            {
                continue;
            }
            lints.push(harper_core::linting::Lint {
                span: word.span,
                lint_kind: harper_core::linting::LintKind::Spelling,
                suggestions: Vec::new(),
                message: String::new(),
                priority: 63,
            });
        }
        remove_lints_overlapping_expr(&self.plural_s, doc, &mut lints);
        lints
            .into_iter()
            .map(|l| Lint {
                span: Span::new(l.span.start, l.span.end),
                lint_kind: LintKind::Spelling,
                priority: l.priority,
                ..Lint::default()
            })
            .collect()
    }

    /// Up to three corrections for `word`, as Harper's `SpellCheck` orders them, but searching
    /// at most edit distance 2 (3 for words of 8+ chars).
    pub(super) fn suggestions(&self, word: &str) -> Vec<String> {
        let chars: Vec<char> = word.chars().collect();
        let max = if chars.len() >= 8 { 3 } else { 2 };
        for dist in 2..=max {
            let s: Vec<String> = suggest_correct_spelling(&chars, 200, dist, self.dict.as_ref())
                .into_iter()
                .filter(|v| {
                    self.dict
                        .get_word_metadata(v)
                        .is_none_or(|m| m.dialects.is_dialect_enabled(self.dialect))
                })
                .take(3)
                .map(|v| v.iter().collect())
                .collect();
            if !s.is_empty() {
                return s;
            }
        }
        Vec::new()
    }
}

/// Harper's `SpellCheck` exception: `word(s)`, `(s)` and a lone `s`.
fn parenthetical_plural_s() -> Filter {
    Filter::new(vec![
        Box::new(
            SequenceExpr::default()
                .then_any_word()
                .then_kind_where(|kind| kind.is_open_round())
                .t_aco("s")
                .then_kind_where(|kind| kind.is_close_round()),
        ) as Box<dyn harper_core::expr::Expr>,
        Box::new(
            SequenceExpr::default()
                .then_kind_where(|kind| kind.is_open_round())
                .t_aco("s")
                .then_kind_where(|kind| kind.is_close_round()),
        ),
        Box::new(SequenceExpr::aco("s")),
    ])
}

/// `ha`, `hahaha`, `HaH`: not misspellings (Harper's `is_informal_laughter`).
fn is_informal_laughter(chars: &[char]) -> bool {
    chars.len() >= 2
        && chars.iter().enumerate().all(|(i, c)| {
            if i % 2 == 0 {
                matches!(c, 'h' | 'H')
            } else {
                matches!(c, 'a' | 'A')
            }
        })
}
