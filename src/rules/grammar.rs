//! Spelling and grammar through Harper.
//!
//! Each prose segment is linted as its own plain-English document. Segment text
//! keeps source byte offsets (non-prose is blanked), so Harper's char spans map
//! back through the segment's own char -> byte table.

use std::cell::RefCell;
use std::collections::HashSet;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::{Arc, LazyLock, Mutex, PoisonError};

use harper_core::linting::{FlatConfig, LintGroup, LintKind, Suggestion};
use harper_core::parsers::PlainEnglish;
use harper_core::spell::{Dictionary, FstDictionary, MergedDictionary, MutableDictionary};
use harper_core::{Dialect, DictWordMetadata, Document, remove_overlaps_map};
use regex::Regex;

use super::{FileCtx, Out};
use crate::config::Config;
use crate::diagnostic::{Finding, Severity};
use crate::segment::Segment;

/// Words common in technical writing that Harper's dictionary lacks.
const TECH_WORDS: &[&str] = &[
    "API",
    "APIs",
    "CLI",
    "CLIs",
    "JSON",
    "YAML",
    "TOML",
    "UTF",
    "URL",
    "URLs",
    "HTTP",
    "HTTPS",
    "HTML",
    "CSS",
    "SQL",
    "XML",
    "CSV",
    "SSH",
    "TLS",
    "DNS",
    "UUID",
    "OAuth",
    "frontmatter",
    "mailto",
    "gitignore",
    "shellcheck",
    "Postgres",
    "OpenSSL",
    "heredoc",
    "heredocs",
    "uncached",
    "unresolvable",
    "Unresolvable",
    "stdin",
    "stdout",
    "stderr",
    "config",
    "configs",
    "repo",
    "repos",
    "async",
    "enum",
    "enums",
    "struct",
    "structs",
    "boolean",
    "namespace",
    "namespaces",
    "middleware",
    "runtime",
    "runtimes",
    "localhost",
    "filesystem",
    "filesystems",
    "codebase",
    "codebases",
    "changelog",
    "linter",
    "linters",
    "lint",
    "linting",
    "dedupe",
    "subcommand",
    "subcommands",
    "whitespace",
    "backend",
    "frontend",
    "toolchain",
    "toolchains",
    "regex",
    "regexes",
    "nullable",
    "iterator",
    "iterators",
    "mutex",
    "tuple",
    "tuples",
    "hostname",
    "symlink",
    "symlinks",
    "tokenizer",
    "unicode",
    "Markdown",
    "GitHub",
    "GitLab",
    "macOS",
    "Linux",
    "npm",
    "bun",
    "Rust",
    "Cargo",
    "rustc",
    "clippy",
    "Nix",
    "devenv",
    "Harper",
    "Vale",
    "markdownlint",
    "SARIF",
    "TODO",
    "FIXME",
    "README",
    "PRs",
    "CI",
    "e.g",
    "i.e",
];

pub fn check(ctx: &FileCtx, out: &mut Out) {
    let spelling = ctx.enabled("spelling");
    let grammar = ctx.family_enabled("grammar/");
    if !spelling && !grammar {
        return;
    }
    let is_code = ctx.a.md.is_none();
    let segments: Vec<&Segment> = ctx
        .a
        .segments
        .iter()
        .filter(|s| {
            !is_code
                || !ctx.config.comments.doc_only_grammar
                || s.kind == crate::segment::SegmentKind::DocComment
        })
        .collect();
    if segments.is_empty() {
        return;
    }
    let idents = identifiers(ctx);

    CHECKER.with(|cell| {
        let mut slot = cell.borrow_mut();
        let key = config_key(ctx.config);
        if slot.as_ref().is_none_or(|c| c.key != key) {
            *slot = None;
        }
        let checker = slot.get_or_insert_with(|| Checker::new(ctx.config, key));
        checker.select(is_code);
        for seg in segments {
            checker.lint_segment(seg, &idents, ctx, out);
        }
    });
}

thread_local! {
    static CHECKER: RefCell<Option<Checker>> = const { RefCell::new(None) };
}

struct Checker {
    key: u64,
    dict: Arc<MergedDictionary>,
    group: LintGroup,
    md_config: FlatConfig,
    code_config: FlatConfig,
    is_code: Option<bool>,
    /// Lowercased accept list: wins over Harper's own entry for the word, whose dialect
    /// metadata otherwise still flags it (`distilled`, `cancelled` under American).
    accepted: HashSet<String>,
}

fn config_key(c: &Config) -> u64 {
    let mut h = DefaultHasher::new();
    c.prose.dialect.hash(&mut h);
    c.accepted_words().hash(&mut h);
    c.prose.disable.hash(&mut h);
    c.comments.disable.hash(&mut h);
    for (k, l) in &c.rules {
        k.hash(&mut h);
        l.severity().is_some().hash(&mut h);
    }
    h.finish()
}

/// Harper's built-in and custom dictionary, built once per process for the current word list.
fn shared_dictionary(config: &Config) -> Arc<MergedDictionary> {
    static DICT: Mutex<Option<(u64, Arc<MergedDictionary>)>> = Mutex::new(None);
    let mut h = DefaultHasher::new();
    config.accepted_words().hash(&mut h);
    let key = h.finish();
    let mut slot = DICT.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some((k, d)) = slot.as_ref()
        && *k == key
    {
        return d.clone();
    }
    let mut custom = MutableDictionary::new();
    for w in TECH_WORDS
        .iter()
        .copied()
        .chain(config.accepted_words().iter().map(String::as_str))
    {
        custom.append_word_str(w, DictWordMetadata::default());
    }
    let mut merged = MergedDictionary::new();
    merged.add_dictionary(FstDictionary::curated());
    merged.add_dictionary(Arc::new(custom));
    let dict = Arc::new(merged);
    *slot = Some((key, dict.clone()));
    dict
}

/// Harper rule names explicitly configured by exact id (`grammar/Name`, `spelling`).
fn explicit_harper_rules(config: &Config) -> impl Iterator<Item = (&str, bool)> {
    config.rules.iter().filter_map(|(k, l)| {
        let name = if k == "spelling" {
            "SpellCheck"
        } else {
            k.strip_prefix("grammar/").filter(|n| !n.contains('*'))?
        };
        Some((name, l.severity().is_some()))
    })
}

fn dialect(name: &str) -> Dialect {
    match name.to_ascii_lowercase().as_str() {
        "british" | "uk" | "en-gb" => Dialect::British,
        "canadian" | "ca" | "en-ca" => Dialect::Canadian,
        "australian" | "au" | "en-au" => Dialect::Australian,
        "indian" | "in" | "en-in" => Dialect::Indian,
        _ => Dialect::American,
    }
}

impl Checker {
    fn new(config: &Config, key: u64) -> Checker {
        let dict = shared_dictionary(config);
        let group = LintGroup::new_curated(dict.clone(), dialect(&config.prose.dialect));
        let mut md_config = group.config.clone();
        for r in &config.prose.disable {
            md_config.set_rule_enabled(r, false);
        }
        let mut code_config = md_config.clone();
        for r in &config.comments.disable {
            code_config.set_rule_enabled(r, false);
        }
        // Explicit rule config wins over the disable lists and Harper's own defaults.
        for (name, on) in explicit_harper_rules(config) {
            md_config.set_rule_enabled(name, on);
            code_config.set_rule_enabled(name, on);
        }
        Checker {
            key,
            dict,
            group,
            md_config,
            code_config,
            is_code: None,
            accepted: config
                .accepted_words()
                .iter()
                .map(|w| w.to_lowercase())
                .collect(),
        }
    }

    fn select(&mut self, is_code: bool) {
        if self.is_code != Some(is_code) {
            self.group.config = if is_code {
                self.code_config.clone()
            } else {
                self.md_config.clone()
            };
            self.is_code = Some(is_code);
        }
    }

    fn lint_segment(
        &mut self,
        seg: &Segment,
        idents: &HashSet<String>,
        ctx: &FileCtx,
        out: &mut Out,
    ) {
        let doc = Document::new(&seg.text, &PlainEnglish, self.dict.as_ref());
        let mut lints = self.group.organized_lints(&doc);
        remove_overlaps_map(&mut lints);
        // char index -> byte offset within the segment text (plus end sentinel).
        let char_bytes: Vec<usize> = seg
            .text
            .char_indices()
            .map(|(i, _)| i)
            .chain([seg.text.len()])
            .collect();
        let to_byte = |c: usize| char_bytes[c.min(char_bytes.len() - 1)];

        for (name, lints) in lints {
            let is_spell = name == "SpellCheck";
            let rule = if is_spell {
                "spelling".to_string()
            } else {
                format!("grammar/{name}")
            };
            if !ctx.enabled(&rule) {
                continue;
            }
            for lint in lints {
                let (s, e) = (to_byte(lint.span.start), to_byte(lint.span.end));
                if s >= e {
                    continue;
                }
                let word = &seg.text[s..e];
                let word_rule = is_spell || name == "SplitWords";
                if word_rule
                    && (looks_like_identifier(word, idents)
                        || self.accepted.contains(&word.to_lowercase()))
                {
                    continue;
                }
                if is_spell
                    && seg.kind.is_comment()
                    && (word.chars().count() <= 2 || self.known_in_other_case(word))
                    || is_spell && self.known_possessive(word, idents)
                {
                    continue;
                }
                if !is_spell
                    && (touches_mask(seg, ctx.src(), s, e)
                        || masked_whitespace(seg, ctx.src(), s, e))
                {
                    continue;
                }
                let range = seg.abs(s..e);
                let severity = match lint.lint_kind {
                    LintKind::Spelling | LintKind::Typo => Severity::Error,
                    LintKind::Style
                    | LintKind::Enhancement
                    | LintKind::Readability
                    | LintKind::Miscellaneous => Severity::Info,
                    _ => Severity::Warning,
                };
                let mut f =
                    Finding::new(rule.clone(), severity, range.clone(), lint.message.clone());
                for sug in lint.suggestions.iter().take(3) {
                    match sug {
                        Suggestion::ReplaceWith(chars) => {
                            f = f.suggest(chars.iter().collect::<String>());
                        }
                        Suggestion::InsertAfter(chars) => {
                            f = f.suggest(format!("{word}{}", chars.iter().collect::<String>()));
                        }
                        Suggestion::Remove => f = f.suggest(""),
                    }
                }
                out.push(f);
            }
        }
    }
}

static IDENT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[A-Za-z_][A-Za-z0-9_]*").expect("hardcoded regex is valid"));

/// Identifiers used as code in this file: code outside comments, or code spans/blocks in Markdown.
fn identifiers(ctx: &FileCtx) -> HashSet<String> {
    let src = ctx.src();
    let mut set = HashSet::new();
    let mut add = |text: &str| {
        for m in IDENT_RE.find_iter(text) {
            set.insert(m.as_str().to_lowercase());
        }
    };
    if let Some(md) = &ctx.a.md {
        for r in md
            .code_spans
            .iter()
            .chain(md.code_blocks.iter().map(|c| &c.range))
        {
            add(&src[r.clone()]);
        }
    } else {
        let mut pos = 0;
        for c in &ctx.a.comments {
            add(&src[pos..c.range.start.max(pos)]);
            pos = pos.max(c.range.end);
        }
        add(&src[pos..]);
    }
    set
}

impl Checker {
    /// `aislop's`: harper does not derive possessives of custom words.
    fn known_possessive(&self, word: &str, idents: &HashSet<String>) -> bool {
        let base = word.strip_suffix("'s").or_else(|| word.strip_suffix("’s"));
        base.is_some_and(|b| {
            self.dict.contains_word_str(b)
                || looks_like_identifier(b, idents)
                || self.known_in_other_case(b)
        })
    }

    /// Comments often lowercase proper nouns and acronyms (`html`, `toml`, `british`).
    fn known_in_other_case(&self, word: &str) -> bool {
        let lower = word.to_lowercase();
        if TECH_WORDS.iter().any(|t| t.to_lowercase() == lower) {
            return true;
        }
        let mut cap = String::new();
        let mut chars = lower.chars();
        if let Some(f) = chars.next() {
            cap.extend(f.to_uppercase());
            cap.push_str(chars.as_str());
        }
        self.dict.contains_word_str(&cap) || self.dict.contains_word_str(&lower.to_uppercase())
    }
}

/// A lint about whitespace that only exists because markup (`#`, `>`, `//`) was blanked.
fn masked_whitespace(seg: &Segment, src: &str, s: usize, e: usize) -> bool {
    let text = &seg.text.as_bytes()[s..e];
    let orig = &src.as_bytes()[seg.range.start + s..seg.range.start + e];
    let at_line_start = seg.text[..s].ends_with('\n') || s == 0;
    text.iter().all(u8::is_ascii_whitespace)
        && (at_line_start || orig.iter().zip(text).any(|(o, t)| o != t))
}

/// Grammar lints next to blanked code or markup (`a`, `b` becomes "   ,") are artifacts.
fn touches_mask(seg: &Segment, src: &str, s: usize, e: usize) -> bool {
    let text = seg.text.as_bytes();
    let orig = &src.as_bytes()[seg.range.clone()];
    let lo = s.saturating_sub(2);
    let hi = (e + 2).min(text.len());
    // Blanked comment markers, heading hashes, quote arrows and list bullets at the start of a
    // line are structure, not inline code; everything else that was blanked counts.
    const MARKER: &[u8] = b"#>*+-/!;%|.)0123456789";
    let is_marker = |i: usize| {
        let line_start = text[..i]
            .iter()
            .rposition(|&b| b == b'\n')
            .map_or(0, |p| p + 1);
        (line_start..=i).all(|j| {
            text[j].is_ascii_whitespace()
                && (orig[j].is_ascii_whitespace() || MARKER.contains(&orig[j]))
        })
    };
    (lo..hi).any(|i| text[i] == b' ' && !orig[i].is_ascii_whitespace() && !is_marker(i))
}

fn looks_like_identifier(word: &str, idents: &HashSet<String>) -> bool {
    let inner_upper = word.chars().skip(1).any(|c| c.is_uppercase());
    word.contains(['_', '.', ':', '/', '@', '$', '<', '>'])
        || word.chars().any(|c| c.is_ascii_digit())
        || inner_upper
        || (word.len() > 1 && word.chars().all(|c| c.is_uppercase()))
        || idents.contains(&word.to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::Analyzed;
    use crate::source::{FileKind, SourceFile};

    fn run(src: &str) -> Vec<Finding> {
        let a = Analyzed::new(SourceFile::new(
            "a.md".into(),
            "a.md".into(),
            FileKind::Markdown,
            src.into(),
        ));
        let config = Config::default();
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
    fn finds_spelling_with_exact_range() {
        let src = "# Title\n\nThis sentance has a mistake.\n";
        let f = run(src);
        let s = f
            .iter()
            .find(|f| f.rule == "spelling")
            .expect("spelling finding");
        assert_eq!(&src[s.range.clone()], "sentance");
        assert!(s.suggestions.iter().any(|x| x == "sentence"));
    }

    #[test]
    fn ignores_code_and_identifiers() {
        let src = "Call `frobnicatr` then use frobnicatr again.\n\n```\nfrobnicatr()\n```\n";
        let f = run(src);
        assert!(f.iter().all(|f| f.rule != "spelling"), "{f:?}");
    }

    #[test]
    fn multibyte_offsets() {
        let src = "Café naïve — this sentance.\n";
        let f = run(src);
        let s = f
            .iter()
            .find(|f| f.rule == "spelling")
            .expect("spelling finding");
        assert_eq!(&src[s.range.clone()], "sentance");
    }

    fn run_with(src: &str, config: &Config) -> Vec<Finding> {
        let a = Analyzed::new(SourceFile::new(
            "a.md".into(),
            "a.md".into(),
            FileKind::Markdown,
            src.into(),
        ));
        let mut out = Vec::new();
        check(&FileCtx { a: &a, config }, &mut out);
        out
    }

    #[test]
    fn explicit_rule_enables_disabled_harper_rule() {
        let src = "I bought apples, pears and bananas today.\n";
        assert!(run(src).iter().all(|f| f.rule != "grammar/OxfordComma"));
        let mut config = Config::default();
        config
            .rules
            .insert("grammar/OxfordComma".into(), crate::config::Level::Error);
        let f = run_with(src, &config);
        assert!(f.iter().any(|f| f.rule == "grammar/OxfordComma"), "{f:?}");
    }

    #[test]
    fn accept_overrides_dialect_entry() {
        let src = "The queue distilled data. Distilled output.\n";
        assert!(
            run(src).iter().any(|f| f.rule == "spelling"),
            "precondition"
        );
        let mut config = Config::default();
        config.prose.accept.push("distilled".into());
        let f = run_with(src, &config);
        assert!(f.iter().all(|f| f.rule != "spelling"), "{f:?}");
    }

    #[test]
    fn dictionary_shared_across_threads() {
        let config = Config::default();
        let a = shared_dictionary(&config);
        let b = std::thread::spawn(move || shared_dictionary(&config))
            .join()
            .unwrap();
        assert!(Arc::ptr_eq(&a, &b));
    }

    #[test]
    fn grammar_rule_ids() {
        let f = run("This is a an example.\n");
        assert!(f.iter().any(|f| f.rule.starts_with("grammar/")), "{f:?}");
    }
}
