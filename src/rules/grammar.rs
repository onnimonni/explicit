//! Spelling and grammar: Harper, Hunspell (spellbook) and our own pattern rules.
//!
//! `prose.engine` picks the backend:
//! - `harper`: Harper with all enabled rules; our spell check on Harper's dictionary.
//! - `curated`: as `harper`, but only the rules in [`CURATED`].
//! - `spellbook`: no Harper; Hunspell spelling ([`super::spell`]) and pattern rules
//!   ([`super::patterns`]) under Harper's rule names.
//! - `hybrid`: `spellbook`, plus Harper's part-of-speech rules ([`HYBRID_HARPER`]) on full
//!   sentences only.
//!
//! Each prose segment is linted as its own plain-English document. Segment text
//! keeps source byte offsets (non-prose is blanked), so Harper's char spans map
//! back through the segment's own char -> byte table.
//!
//! Harper itself ([`harper`]) is behind the `harper` cargo feature; without it only the
//! `spellbook` engine exists.

#[cfg(feature = "harper")]
mod harper;
#[cfg(not(feature = "harper"))]
#[path = "grammar/no_harper.rs"]
mod harper;

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::{Arc, LazyLock, Mutex, PoisonError, RwLock};

use regex::Regex;

use self::harper::Harper;
use super::lint::{Lint, LintKind, Suggestion, remove_overlaps_map};
use super::words::Dialect;
use super::{FileCtx, Out, patterns, spell, spell_lang};
use crate::config::{Config, Engine, GrammarLevel};
use crate::diagnostic::{Finding, Severity};
use crate::segment::{Segment, SegmentKind};

/// Words common in technical writing that Harper's dictionary lacks.
pub(crate) const TECH_WORDS: &[&str] = &[
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
    "mtime",
    "mtimes",
    "inode",
    "inodes",
    "autolink",
    "autolinks",
    "cwd",
    "unpunctuated",
    "dedent",
    "dedented",
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
    // `etc` without its period is punctuation style, not spelling.
    "etc",
    // Developer shorthand common in changelogs and doc comments.
    "deps",
    "impl",
    "impls",
    "args",
    "env",
    "perf",
    "semver",
    "dep",
    "proc",
    "fn",
    "fns",
    // Tools and terms written in lowercase in READMEs and comments, which neither Hunspell nor
    // Harper lists (checked against both; derivations such as `upserted` already pass).
    "cron",
    "crontab",
    "crontabs",
    "pnpm",
    "npx",
    "bunx",
    "pipx",
    "venv",
    "virtualenv",
    "conda",
    "pytest",
    "mypy",
    "numpy",
    "eslint",
    "esbuild",
    "tsconfig",
    "rustfmt",
    "nixpkgs",
    "stdlib",
    "argv",
    "argc",
    "keyset",
    "keysets",
    "fanout",
    "prerender",
    "prerenders",
    "rehydrate",
    "gitignored",
    "gitattributes",
    "dotenv",
    "noop",
    "noops",
    "sudo",
    "mkdir",
    "chmod",
    "nginx",
    "kubectl",
    "kubeconfig",
    "cgroup",
    "cgroups",
    "tmpfs",
    "pubsub",
    "protobuf",
    "protobufs",
    "wasm",
    "Wasm",
    "repl",
    "varchar",
    "bitset",
    "bitsets",
    "rwlock",
    "lockless",
    "bcrypt",
    "scrypt",
    "nonces",
    "keystore",
    "keystores",
    "slugify",
    "permalink",
    "permalinks",
    "navbar",
    "navbars",
    "goroutine",
    "goroutines",
    "monomorphization",
    "microbenchmark",
    "microbenchmarks",
    "failback",
    "oncall",
    "authn",
    "authz",
    "tsx",
    "jsx",
    "Dependabot",
];

/// Closed technical compounds Harper's dictionary lacks and `SplitWords` wants to split
/// (`implementor` -> `implement or`). They are dictionary words here, so neither
/// `spelling` nor `SplitWords` reports them.
pub(crate) const TECH_COMPOUNDS: &[&str] = &[
    "teardown",
    "teardowns",
    "textarea",
    "textareas",
    "wellbeing",
    "implementor",
    "implementors",
    "auditability",
    "codegen",
    "dropdown",
    "dropdowns",
    "checkbox",
    "checkboxes",
    "tooltip",
    "tooltips",
    "sidebar",
    "sidebars",
    "lifecycle",
    "lifecycles",
    "cutover",
    "failover",
    "rollout",
    "rollouts",
    "runbook",
    "runbooks",
    "allowlist",
    "allowlists",
    "denylist",
    "denylists",
    "realtime",
    "retryable",
    "footgun",
    "footguns",
    "wordmark",
    "keypair",
    "keypairs",
    "passthrough",
    "healthcheck",
    "healthchecks",
    "webhook",
    "webhooks",
    "websocket",
    "websockets",
    "timestamp",
    "timestamps",
    "workflow",
    "workflows",
    "dataset",
    "datasets",
    "startup",
    "shutdown",
    "readonly",
    "nonce",
    "hotfix",
    "hotfixes",
    "offline",
    "debounce",
    "subtree",
    "subtrees",
    "subprocess",
    "subprocesses",
    "filename",
    "filenames",
    "pathname",
    "username",
    "usernames",
    "keyring",
    "codepath",
    "codepaths",
    "lookup",
    "lookups",
    "roundtrip",
    "roundtrips",
    "prefetch",
    "refetch",
    "selectability",
];

/// Harper rules that judge whole sentences. Table cells and unpunctuated list items or headings
/// are fragments ("pending review", "retry on timeout"), where these are almost always wrong.
/// Split-compound rules, off in headings and table header cells.
const COMPOUND_SPLIT_RULES: &[&str] = &["FinnishCompoundSplit", "SwedishCompoundSplit"];

/// `seg` is a cell of a Markdown table's header row: the next line is the delimiter row.
fn table_header_cell(seg: &Segment, src: &str) -> bool {
    if seg.kind != SegmentKind::TableCell {
        return false;
    }
    let rest = &src[seg.range.start.min(src.len())..];
    let Some(nl) = rest.find('\n') else {
        return false;
    };
    let next = rest[nl + 1..].lines().next().unwrap_or("").trim();
    next.contains('-')
        && next
            .chars()
            .all(|c| matches!(c, '|' | '-' | ':' | ' ' | '\t'))
}

const FRAGMENT_RULES: &[&str] = &[
    "SentenceCapitalization",
    "MissingTo",
    "MissingDeterminer",
    "MissingPreposition",
    "LongSentences",
    "NounVerbConfusion",
    "PronounVerbAgreement",
    "PronounAre",
    "PronounInflectionBe",
    "ModalBeAdjective",
    "ProgressiveNeedsBe",
    "SingleBe",
    "SomethingIs",
    "SubjectPronoun",
    "PhrasalVerbAsCompoundNoun",
    "InflectedVerbAfterTo",
    "SimplePastToPastParticiple",
    "QuantifierNeedsOf",
    "IAmAgreement",
    "WrongNegative",
    "UnclosedQuotes",
];

/// Harper rules about whitespace; table padding and trailing hard-break spaces are layout.
const WHITESPACE_RULES: &[&str] = &["Spaces", "NoFrenchSpaces", "QuoteSpacing"];

/// Rules precise enough to stay `warning` under the default `prose.grammar_level = "info"`.
const HIGH_CONFIDENCE: &[&str] = &[
    "AnA",
    "RepeatedWords",
    "ThenThan",
    "ItsPossessive",
    "LetsConfusion",
    "PossessiveYour",
    "Didnt",
    "TheMy",
    "CorrectNumberSuffix",
    "NumberSuffixCapitalization",
];

#[cfg(feature = "harper")]
/// Harper rules the `curated` engine runs: [`HIGH_CONFIDENCE`] plus plausibly precise
/// word-confusion rules.
const CURATED: &[&str] = &[
    "AnA",
    "RepeatedWords",
    "ThenThan",
    "ItsPossessive",
    "LetsConfusion",
    "PossessiveYour",
    "Didnt",
    "TheMy",
    "CorrectNumberSuffix",
    "NumberSuffixCapitalization",
    "ItsContraction",
    "TheyreConfusions",
    "ToTwoToo",
    "ThereOwn",
    "WereWhere",
    "ThatThan",
    "ThesesThese",
    "QuiteQuiet",
    "WaistWaste",
    "ThoughThought",
    "PronounContraction",
    "Theres",
    "EverEvery",
    "FeelFell",
    "ThingThink",
];

#[cfg(feature = "harper")]
/// Harper rules the `hybrid` engine runs: those that need part-of-speech tags.
const HYBRID_HARPER: &[&str] = &[
    "AnA",
    "ItsContraction",
    "ItsPossessive",
    "PossessiveYour",
    "LetsConfusion",
    "TheyreConfusions",
    "ToTwoToo",
    "PronounVerbAgreement",
    "ThereIsAgreement",
    "IAmAgreement",
    "WereWhere",
];

/// Default severity of a Harper finding under `level`; `[rules]` config may still override it.
pub fn default_severity(name: &str, kind: LintKind, level: GrammarLevel) -> Severity {
    if name == "SpellCheck" {
        return Severity::Error;
    }
    if name == "SplitWords" {
        return Severity::Info;
    }
    match level {
        GrammarLevel::Info if HIGH_CONFIDENCE.contains(&name) => Severity::Warning,
        GrammarLevel::Info => Severity::Info,
        GrammarLevel::Warning => Severity::Warning,
        GrammarLevel::Harper => match kind {
            LintKind::Spelling | LintKind::Typo => Severity::Error,
            LintKind::Style
            | LintKind::Enhancement
            | LintKind::Readability
            | LintKind::Miscellaneous => Severity::Info,
            _ => Severity::Warning,
        },
    }
}

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
            !is_code || !ctx.config.comments.doc_only_grammar || s.kind == SegmentKind::DocComment
        })
        .collect();
    if segments.is_empty() {
        return;
    }
    let idents = identifiers(ctx);

    CHECKERS.with(|cell| {
        let mut cache = cell.borrow_mut();
        let key = config_key(ctx.config);
        // Files may carry different configs (per-path overrides), so keep a few checkers.
        let idx = match cache.iter().position(|c| c.key == key) {
            Some(i) => i,
            None => {
                if cache.len() >= MAX_CHECKERS {
                    cache.remove(0);
                }
                cache.push(Checker::new(ctx.config, key));
                cache.len() - 1
            }
        };
        let checker = &mut cache[idx];
        checker.select(is_code);
        for seg in segments {
            checker.lint_segment(seg, &idents, ctx, out);
        }
    });
}

/// Spelling of `ctx`'s prose in language `code` with `speller`, limited to `ranges` (absolute
/// byte offsets) when given: the rest of each segment is blanked first.
pub fn check_language(
    ctx: &FileCtx,
    code: &str,
    speller: Arc<dyn spell_lang::LangSpeller>,
    secondary: Option<Arc<dyn spell_lang::LangSpeller>>,
    ranges: Option<&[std::ops::Range<usize>]>,
    out: &mut Out,
) {
    // English words inside the language's text follow the project's dialect.
    spell_lang::with_english_dialect(dialect(&ctx.config.prose.dialect), || {
        check_language_in(ctx, code, speller, secondary, ranges, out);
    });
}

fn check_language_in(
    ctx: &FileCtx,
    code: &str,
    speller: Arc<dyn spell_lang::LangSpeller>,
    secondary: Option<Arc<dyn spell_lang::LangSpeller>>,
    ranges: Option<&[std::ops::Range<usize>]>,
    out: &mut Out,
) {
    if !ctx.enabled("spelling") || ranges.is_some_and(<[_]>::is_empty) {
        return;
    }
    let is_code = ctx.a.md.is_none();
    let segments: Vec<std::borrow::Cow<Segment>> = ctx
        .a
        .segments
        .iter()
        .filter(|s| {
            !is_code || !ctx.config.comments.doc_only_grammar || s.kind == SegmentKind::DocComment
        })
        .filter_map(|s| match ranges {
            None => Some(std::borrow::Cow::Borrowed(s)),
            Some(rs) => {
                let inside: Vec<_> = rs
                    .iter()
                    .filter(|r| r.start < s.range.end && s.range.start < r.end)
                    .collect();
                if inside.is_empty() {
                    return None;
                }
                // Blank what lies outside the ranges.
                let mut gaps = Vec::new();
                let mut pos = s.range.start;
                let mut sorted = inside;
                sorted.sort_by_key(|r| r.start);
                for r in sorted {
                    if r.start > pos {
                        gaps.push(pos..r.start);
                    }
                    pos = pos.max(r.end);
                }
                if pos < s.range.end {
                    gaps.push(pos..s.range.end);
                }
                let mut seg = s.clone();
                seg.blank_all(&gaps);
                Some(std::borrow::Cow::Owned(seg))
            }
        })
        .collect();
    if segments.is_empty() {
        return;
    }
    let idents = identifiers(ctx);
    let mut h = DefaultHasher::new();
    (config_key(ctx.config), code).hash(&mut h);
    secondary
        .as_ref()
        .map(|s| Arc::as_ptr(s).cast::<()>() as usize)
        .hash(&mut h);
    let key = h.finish();
    let mut file_names = Vec::new();
    for seg in &segments {
        let chars: Vec<char> = seg.text.chars().collect();
        spell_lang::names(&*speller, code, &chars, &mut file_names);
    }
    CHECKERS.with(|cell| {
        let mut cache = cell.borrow_mut();
        let idx = match cache.iter().position(|c| c.key == key) {
            Some(i) => i,
            None => {
                if cache.len() >= MAX_CHECKERS {
                    cache.remove(0);
                }
                cache.push(Checker::new_lang(ctx.config, key, code, speller, secondary));
                cache.len() - 1
            }
        };
        let checker = &mut cache[idx];
        checker.file_names = file_names;
        checker.select(is_code);
        for seg in &segments {
            checker.lint_segment(seg, &idents, ctx, out);
        }
    });
}

const MAX_CHECKERS: usize = 4;

thread_local! {
    static CHECKERS: RefCell<Vec<Checker>> = const { RefCell::new(Vec::new()) };
}

/// Lints per Harper rule name for one segment text.
type RawLints = BTreeMap<String, Vec<Lint>>;

struct Checker {
    key: u64,
    engine: Engine,
    /// Harper's dictionary and rules (not built for the `spellbook` engine).
    harper: Option<Harper>,
    /// Hunspell dictionary for spelling (`spellbook` and `hybrid` engines).
    hunspell: Option<&'static spell::Speller>,
    dialect: Dialect,
    /// Key of the process-wide suggestion cache: word list and dialect.
    suggest_key: u64,
    is_code: Option<bool>,
    /// Our own spell check (Harper's `SpellCheck` stays off) for Markdown / code comments.
    spell_md: bool,
    spell_code: bool,
    /// Some Harper rule is on for Markdown / code comments.
    harper_md: bool,
    harper_code: bool,
    /// Our pattern rules on for Markdown / code comments (`spellbook`, `hybrid`).
    own_md: Vec<&'static str>,
    own_code: Vec<&'static str>,
    /// Harper rule name -> our rule id, `None` when off.
    rule_ids: HashMap<String, Option<String>>,
    /// Lowercased `prose.accept` and vocab words, including whole hyphenated entries.
    accepted: HashSet<String>,
    /// `prose.accept` and vocab words as written, for case-sensitive questions.
    accepted_exact: HashSet<String>,
    /// `prose.accept_patterns`, anchored to whole words.
    accept_patterns: Vec<Regex>,
    /// `[[entity]]` names and multi-word or case-sensitive `[[vocab]]` terms, accepted as
    /// whole phrases.
    phrases: Arc<crate::vocab::Phrases>,
    /// `prose/entity-name` reports case-sensitive terms in another casing.
    entity_case_on: bool,
    /// Spelling in another language (`fi`, `sv`) instead of English; no grammar rules.
    lang: Option<(String, Arc<dyn spell_lang::LangSpeller>)>,
    /// Words the file's main language knows, accepted inside a stretch of `lang`.
    secondary: Option<Arc<dyn spell_lang::LangSpeller>>,
    /// Capitalized words the current file uses as names inside sentences (`lang` only).
    file_names: Vec<String>,
}

fn config_key(c: &Config) -> u64 {
    let mut h = DefaultHasher::new();
    c.prose.dialect.hash(&mut h);
    c.prose.engine.hash(&mut h);
    c.accepted_words().hash(&mut h);
    c.prose.accept_patterns.hash(&mut h);
    c.vocab.hash(&mut h);
    c.entities.hash(&mut h);
    c.prose.disable.hash(&mut h);
    c.comments.disable.hash(&mut h);
    for (k, l) in &c.rules {
        k.hash(&mut h);
        l.severity().is_some().hash(&mut h);
    }
    h.finish()
}

fn words_key(config: &Config) -> u64 {
    let mut h = DefaultHasher::new();
    config.accepted_words().hash(&mut h);
    h.finish()
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

pub(super) fn dialect(name: &str) -> Dialect {
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
        // Config loading rejects Harper engines in builds without Harper.
        let engine = if cfg!(feature = "harper") {
            config.prose.engine
        } else {
            Engine::Spellbook
        };
        let dialect = dialect(&config.prose.dialect);
        let spelling = super::rule_enabled(config, "spelling");
        let hunspell = engine
            .hunspell()
            .then(|| spell::dictionary(&config.prose.dialect));
        let (mut spell_md, mut spell_code) = (
            spelling && own_rule_on(config, "SpellCheck", false),
            spelling && own_rule_on(config, "SpellCheck", true),
        );
        let (mut harper_md, mut harper_code) = (false, false);
        let harper = (engine != Engine::Spellbook).then(|| {
            let setup = Harper::setup(config, dialect, spelling);
            (spell_md, spell_code) = (setup.spell_md, setup.spell_code);
            (harper_md, harper_code) = (setup.harper_md, setup.harper_code);
            setup.harper
        });
        // Harper's names our pattern rules replace (`spellbook`, `hybrid`), plus the confusable
        // rules every engine runs.
        let own = |code: bool| -> Vec<&'static str> {
            let base: &[&'static str] = if engine.hunspell() {
                patterns::RULES
            } else {
                &[]
            };
            base.iter()
                .chain(patterns::CONFUSABLES.iter().filter(|r| !base.contains(r)))
                .copied()
                .filter(|r| {
                    own_rule_on(config, r, code)
                        && super::rule_enabled(config, &format!("grammar/{r}"))
                })
                .collect()
        };
        let mut suggest_key = DefaultHasher::new();
        (words_key(config), dialect, engine.hunspell()).hash(&mut suggest_key);
        let accepted = config
            .accepted_words()
            .iter()
            .map(|w| w.to_lowercase())
            .collect();
        let accepted_exact = config.accepted_words().iter().cloned().collect();
        // Invalid patterns are skipped here; config loading is where they get reported.
        let accept_patterns = config
            .prose
            .accept_patterns
            .iter()
            .filter_map(|p| Regex::new(&format!("^(?:{p})$")).ok())
            .collect();
        Checker {
            key,
            engine,
            harper,
            hunspell,
            dialect,
            suggest_key: suggest_key.finish(),
            is_code: None,
            spell_md,
            spell_code,
            harper_md,
            harper_code,
            own_md: own(false),
            own_code: own(true),
            rule_ids: HashMap::new(),
            accepted,
            accepted_exact,
            accept_patterns,
            phrases: config.vocab_phrases_in("en"),
            entity_case_on: super::rule_enabled(config, "prose/entity-name"),
            lang: None,
            secondary: None,
            file_names: Vec::new(),
        }
    }

    /// A checker for spelling in language `code` only: no Harper, no English pattern rules,
    /// `[languages.<code>] accept` words on top of the shared accept lists.
    fn new_lang(
        config: &Config,
        key: u64,
        code: &str,
        speller: Arc<dyn spell_lang::LangSpeller>,
        secondary: Option<Arc<dyn spell_lang::LangSpeller>>,
    ) -> Checker {
        let spelling = super::rule_enabled(config, "spelling");
        // Shared accept lists, vocab for this language and `[languages.<code>] accept`.
        let words = config.accepted_words_in(code);
        let mut suggest_key = DefaultHasher::new();
        (words_key(config), code, &*words).hash(&mut suggest_key);
        // Finnish grammar rules need the Voikko morphology.
        let own = |is_code: bool| -> Vec<&'static str> {
            let rules = match code {
                "fi" if speller.voikko().is_some() => super::grammar_fi::RULES,
                "sv" => super::grammar_sv::RULES,
                _ => &[],
            };
            rules
                .iter()
                .map(|(n, _)| *n)
                .filter(|n| {
                    own_rule_on(config, n, is_code)
                        && super::rule_enabled(config, &format!("grammar/{n}"))
                })
                .collect()
        };
        let all = || words.iter();
        Checker {
            key,
            engine: Engine::Spellbook,
            harper: None,
            hunspell: None,
            dialect: dialect(&config.prose.dialect),
            suggest_key: suggest_key.finish(),
            is_code: None,
            spell_md: spelling && own_rule_on(config, "SpellCheck", false),
            spell_code: spelling && own_rule_on(config, "SpellCheck", true),
            harper_md: false,
            harper_code: false,
            own_md: own(false),
            own_code: own(true),
            rule_ids: HashMap::new(),
            accepted: all().map(|w| w.to_lowercase()).collect(),
            accepted_exact: all().cloned().collect(),
            accept_patterns: config
                .prose
                .accept_patterns
                .iter()
                .filter_map(|p| Regex::new(&format!("^(?:{p})$")).ok())
                .collect(),
            phrases: config.vocab_phrases_in(code),
            entity_case_on: super::rule_enabled(config, "prose/entity-name"),
            lang: Some((code.to_string(), speller)),
            secondary,
            file_names: Vec::new(),
        }
    }

    /// `a` / `an` ending at `e` comes before something with no settled reading: a quoted or
    /// bracketed token (`a [`Stream`]`, `an 'x'`), an identifier (`u16`, `foo_bar`, `a::b`), an
    /// acronym (`a/an SQL`, `SQLite`: readers differ) or a word the dictionary does not know
    /// (`an rpath`, `a usize`). Also an article that is itself a label (`a)`, `a.k.a`).
    fn ana_unsure(&self, text: &str, e: usize) -> bool {
        let rest = &text[e..];
        if !rest.starts_with(char::is_whitespace) {
            return true;
        }
        let rest = rest.trim_start();
        if rest.starts_with(['[', '(', '\'', '"', '`', '&', '<', '{', '‘', '“', '*']) {
            return true;
        }
        let token: String = rest
            .chars()
            .take_while(|c| c.is_alphanumeric() || matches!(c, '_' | ':' | '\'' | '’'))
            .collect();
        let word = token.split(['\'', '’']).next().unwrap_or("");
        // `of a and b`, `a is less than b`: `a` is a variable, not an article.
        const NOT_AFTER_ARTICLE: &[&str] = &[
            "and", "or", "is", "as", "of", "in", "on", "at", "if", "it", "its", "into", "onto",
            "upon", "are", "was", "were", "has", "had", "have", "isn't", "aren't", "wasn't",
            "hasn't", "unless", "until", "with", "within",
        ];
        word.is_empty()
            || NOT_AFTER_ARTICLE.contains(&word)
            || token.contains('_')
            || token.contains("::")
            || word.chars().any(|c| c.is_ascii_digit())
            || word.chars().filter(|c| c.is_uppercase()).count() >= 2
            // `an fd`, `a sdk`: read letter by letter or as a word, readers differ.
            || !word.chars().any(|c| "aeiouyAEIOUY".contains(c))
            || (word.chars().count() > 1 && !self.known_word(word))
    }

    /// A word the active dictionary knows in some capitalization.
    fn known_word(&self, w: &str) -> bool {
        if let Some((_, sp)) = &self.lang {
            let lower = w.to_lowercase();
            return sp.check(w)
                || sp.check(&lower)
                || sp.check(&capitalize(&lower))
                || self.accepted.contains(&lower);
        }
        match (self.hunspell, &self.harper) {
            (Some(h), _) => {
                let lower = w.to_lowercase();
                spell::known(h, w)
                    || spell::known(h, &lower)
                    || spell::known(h, &capitalize(&lower))
                    || self.accepted.contains(&lower)
            }
            (None, Some(d)) => d.contains(w),
            (None, None) => false,
        }
    }

    /// `word` (or the hyphenated token around it) is in `prose.accept` or `accept_patterns`.
    fn accepted(&self, word: &str, token: &str) -> bool {
        [word, token].iter().any(|w| {
            self.accepted.contains(&w.to_lowercase())
                || self.accept_patterns.iter().any(|re| re.is_match(w))
        })
    }

    fn select(&mut self, is_code: bool) {
        if self.is_code != Some(is_code) {
            if let Some(h) = &mut self.harper {
                h.select(is_code);
            }
            self.is_code = Some(is_code);
        }
    }

    /// [`Self::raw_lints`], shared across files and threads: docs repeat paragraphs, table cells
    /// and comment boilerplate. Per-file filters run afterwards.
    fn cached_raw_lints(&mut self, text: &str, harper_ok: bool) -> Arc<RawLints> {
        #[derive(Default)]
        struct Cache {
            map: HashMap<(u64, bool, bool, Box<str>), Arc<RawLints>>,
            bytes: usize,
        }
        static CACHE: LazyLock<Mutex<Cache>> = LazyLock::new(Default::default);
        const MAX_ENTRIES: usize = 100_000;
        const MAX_BYTES: usize = 64 << 20;
        let key = (
            self.key,
            self.is_code == Some(true),
            harper_ok,
            Box::<str>::from(text),
        );
        if let Some(hit) = CACHE
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .map
            .get(&key)
        {
            return hit.clone();
        }
        let lints = Arc::new(self.raw_lints(text, harper_ok));
        let mut cache = CACHE.lock().unwrap_or_else(PoisonError::into_inner);
        if cache.map.len() >= MAX_ENTRIES || cache.bytes + text.len() > MAX_BYTES {
            cache.map.clear();
            cache.bytes = 0;
        }
        cache.bytes += text.len();
        cache.map.insert(key, lints.clone());
        lints
    }

    /// Harper's lints (when `harper_ok`), our pattern rules and our spelling lints (without
    /// suggestions yet), overlaps removed.
    fn raw_lints(&mut self, text: &str, harper_ok: bool) -> RawLints {
        let (spell, harper, own) = if self.is_code == Some(true) {
            (self.spell_code, self.harper_code, self.own_code.clone())
        } else {
            (self.spell_md, self.harper_md, self.own_md.clone())
        };
        if let Some((code, sp)) = &self.lang {
            let mut lints = BTreeMap::new();
            if !own.is_empty() {
                let chars: Vec<char> = text.chars().collect();
                if let Some(v) = sp.voikko() {
                    lints = super::grammar_fi::lints(v, &**sp, &chars, &own);
                } else if code == "sv" {
                    lints = super::grammar_sv::lints(&**sp, &chars, &own);
                }
            }
            if spell {
                let chars: Vec<char> = text.chars().collect();
                let mut found = spell_lang::misspelled(&**sp, code, &chars);
                if let Some(other) = &self.secondary {
                    found.retain(|l| {
                        let w: String = chars[l.span.start..l.span.end].iter().collect();
                        w.chars().count() < 5 || !other.check(&w)
                    });
                }
                lints.insert("SpellCheck".to_string(), found);
            }
            return lints;
        }
        let run_harper = harper && harper_ok && self.harper.is_some();
        let harper_spell = spell && self.hunspell.is_none();
        let mut lints = BTreeMap::new();
        if (run_harper || harper_spell)
            && let Some(h) = &mut self.harper
        {
            lints = h.lints(text, run_harper, harper_spell);
        }
        if (spell && self.hunspell.is_some()) || !own.is_empty() {
            let chars: Vec<char> = text.chars().collect();
            let tokens = patterns::tokenize(&chars);
            if spell && let Some(h) = self.hunspell {
                lints.insert(
                    "SpellCheck".to_string(),
                    spell::misspelled(h, &chars, &tokens),
                );
            }
            // Where Harper ran its own `AnA` / `PronounVerbAgreement`, ours stays out.
            let harper_has = |r: &str| {
                run_harper
                    && matches!(r, "AnA" | "PronounVerbAgreement")
                    && self.harper.as_ref().is_some_and(|h| h.rule_on(r))
            };
            let own: Vec<&str> = own.into_iter().filter(|r| !harper_has(r)).collect();
            let american = matches!(self.dialect, Dialect::American | Dialect::Canadian);
            for (k, v) in patterns::lint(&chars, &tokens, &own, american) {
                lints.entry(k).or_insert_with(Vec::new).extend(v);
            }
        }
        // Harper's `its` / `then` / `there own` findings get our word-class checks too.
        if patterns::VETOED
            .iter()
            .any(|n| lints.get(*n).is_some_and(|v| !v.is_empty()))
        {
            let chars: Vec<char> = text.chars().collect();
            let tokens = patterns::tokenize(&chars);
            patterns::veto(&chars, &tokens, &mut lints);
        }
        remove_overlaps_map(&mut lints);
        lints
    }

    /// Up to three corrections for `word`, as Harper's `SpellCheck` orders them, but searching
    /// at most edit distance 2 (3 for words of 8+ chars). Shared by all threads.
    fn suggestions(&self, word: &str) -> Arc<[String]> {
        type Cache = HashMap<(u64, Box<str>), Arc<[String]>>;
        static CACHE: LazyLock<RwLock<Cache>> = LazyLock::new(Default::default);
        let key = (self.suggest_key, Box::<str>::from(word));
        if let Some(hit) = CACHE
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .get(&key)
        {
            return hit.clone();
        }
        let mut found: Arc<[String]> = Arc::new([]);
        if let Some((_, sp)) = &self.lang {
            let mut s = sp.suggest(word);
            s.truncate(3);
            found = s.into();
        } else if let Some(h) = self.hunspell {
            found = spell::suggest(h, word).into();
        } else if let Some(h) = &self.harper {
            found = h.suggestions(word).into();
        }
        let mut cache = CACHE.write().unwrap_or_else(PoisonError::into_inner);
        if cache.len() >= 200_000 {
            cache.clear();
        }
        cache.insert(key, found.clone());
        found
    }

    /// `spelling` / `grammar/<name>` for a Harper rule name, or `None` when it is off. The
    /// checker's config key covers every rule level, so the answer is fixed per checker.
    fn rule_id(&mut self, name: &str, ctx: &FileCtx) -> Option<String> {
        if let Some(id) = self.rule_ids.get(name) {
            return id.clone();
        }
        let rule = if name == "SpellCheck" {
            "spelling".to_string()
        } else {
            format!("grammar/{name}")
        };
        let id = ctx.enabled(&rule).then_some(rule);
        self.rule_ids.insert(name.to_string(), id.clone());
        id
    }

    /// Our `spelling` finding for `word`, worded as Harper's `SpellCheck` words it.
    fn spelling_finding(&self, word: &str, range: std::ops::Range<usize>) -> Finding {
        let mut sugs: Vec<String> = self.suggestions(word).to_vec();
        // A capitalized misspelling gets capitalized suggestions (but not `macOS`-style ones).
        if word.chars().next().is_some_and(char::is_uppercase) {
            for s in &mut sugs {
                let mut it = s.chars();
                if let Some(f) = it.next()
                    && !it.clone().any(char::is_uppercase)
                {
                    *s = f.to_uppercase().chain(it).collect();
                }
            }
        }
        let message = match sugs.as_slice() {
            [one] => format!("Did you mean `{one}`?"),
            _ => format!("Did you mean to spell `{word}` this way?"),
        };
        let mut f = Finding::new("spelling", Severity::Error, range, message);
        for s in sugs {
            f = f.suggest(s);
        }
        f
    }

    /// `AnA` before numbers (`a 8-byte key`), in every engine: Harper's `AnA` reads words
    /// only, and our pattern rules see the segment text, where `8GB` / `10ms` are blanked.
    fn ana_numbers(&mut self, seg: &Segment, ctx: &FileCtx, out: &mut Out) {
        let code = self.is_code == Some(true);
        if !seg.text.contains(['a', 'A']) || !own_rule_on(ctx.config, "AnA", code) {
            return;
        }
        let Some(orig) = ctx.src().get(seg.range.clone()) else {
            return;
        };
        let found = patterns::ana_numbers(&seg.text, orig);
        if found.is_empty() {
            return;
        }
        let Some(rule) = self.rule_id("AnA", ctx) else {
            return;
        };
        let severity = default_severity(
            "AnA",
            LintKind::Miscellaneous,
            ctx.config.prose.grammar_level,
        );
        for (r, fix) in found {
            out.push(
                Finding::new(
                    rule.clone(),
                    severity,
                    seg.abs(r),
                    "Incorrect indefinite article.".to_owned(),
                )
                .suggest(fix.to_string()),
            );
        }
    }

    fn lint_segment(&mut self, seg: &Segment, idents: &Idents, ctx: &FileCtx, out: &mut Out) {
        if self.lang.is_none() {
            self.ana_numbers(seg, ctx, out);
        }
        let harper_ok = self.engine != Engine::Hybrid || full_sentence(seg);
        let lints = self.cached_raw_lints(&seg.text, harper_ok);
        if lints.values().all(Vec::is_empty) {
            return;
        }
        // char index -> byte offset within the segment text (plus end sentinel).
        let char_bytes: Vec<usize> = seg
            .text
            .char_indices()
            .map(|(i, _)| i)
            .chain([seg.text.len()])
            .collect();
        let to_byte = |c: usize| char_bytes[c.min(char_bytes.len() - 1)];
        let fragment = is_fragment(seg);
        let level = ctx.config.prose.grammar_level;
        // Configured phrases (`Telia Oy`): their words are accepted only inside them.
        let phrase_ranges = if self.phrases.is_empty() || !lints.contains_key("SpellCheck") {
            Vec::new()
        } else {
            self.phrases.accepted_ranges(&seg.text, self.entity_case_on)
        };

        for (name, lints) in lints.iter().filter(|(_, l)| !l.is_empty()) {
            if fragment && FRAGMENT_RULES.contains(&name.as_str()) {
                continue;
            }
            // Titles and table headers are noun phrases in telegraphic style
            // (`Tiedote henkilöstölle`), not split compounds.
            if COMPOUND_SPLIT_RULES.contains(&name.as_str())
                && (seg.kind == SegmentKind::Heading || table_header_cell(seg, ctx.src()))
            {
                continue;
            }
            let whitespace_rule = WHITESPACE_RULES.contains(&name.as_str());
            if whitespace_rule && seg.kind == SegmentKind::TableCell {
                continue;
            }
            let is_spell = name == "SpellCheck";
            let Some(rule) = self.rule_id(name, ctx) else {
                continue;
            };
            for lint in lints {
                let (s, e) = (to_byte(lint.span.start), to_byte(lint.span.end));
                if s >= e {
                    continue;
                }
                let word = &seg.text[s..e];
                let word_rule = is_spell || name == "SplitWords";
                let token = word_token(&seg.text, s, e);
                if word_rule
                    && (looks_like_identifier(word, idents)
                        || token.contains('_')
                        || idents.contains(&token.to_lowercase())
                        || self.accepted(word, token))
                {
                    continue;
                }
                if is_spell && phrase_ranges.iter().any(|r| r.start <= s && e <= r.end) {
                    continue;
                }
                if is_spell && self.lang.is_some() && self.lang_skips(seg, ctx.src(), s, word) {
                    continue;
                }
                if is_spell
                    && (jargon(seg, ctx, s, e, idents, self.lang.is_none())
                        || idents.project_name(self.name_speller(), word)
                        || emphasis_split(seg, ctx.src(), s, e)
                            .is_some_and(|w| self.known_word(&w)))
                {
                    continue;
                }
                // A lone letter is a label (`size S`, `(f)`) or tokenizer debris, never a typo.
                if is_spell && (word.chars().count() == 1 || possessive_suffix(&seg.text, s, word))
                {
                    continue;
                }
                if name == "SentenceCapitalization" && self.lowercase_by_convention(word, idents) {
                    continue;
                }
                if name == "AnA"
                    && (letter_label(&seg.text, s, word) || self.ana_unsure(&seg.text, e))
                {
                    continue;
                }
                if name == "SplitWords" && self.known_compound(word) {
                    continue;
                }
                if whitespace_rule && trailing_whitespace(&seg.text, e) {
                    continue;
                }
                if name == "CapitalizePersonalPronouns" && enumerator_i(&seg.text, s, e) {
                    continue;
                }
                if is_spell
                    && seg.kind.is_comment()
                    && (word.chars().count() <= 2 || self.known_in_other_case(word))
                    || is_spell && self.known_possessive(word, idents)
                {
                    continue;
                }
                // A repetition is an artifact only when blanked code sat between the two words.
                // `then` -> `than` is judged by the word before it: code after it is fine.
                let masked = if name == "RepeatedWords" {
                    masked_inside(seg, ctx.src(), s, e)
                } else {
                    let right = name != "ThenThan";
                    touches_mask(seg, ctx.src(), s, e, right)
                        || masked_whitespace(seg, ctx.src(), s, e)
                };
                if !is_spell && masked {
                    continue;
                }
                let range = seg.abs(s..e);
                if is_spell {
                    out.push(self.spelling_finding(word, range));
                    continue;
                }
                let severity = default_severity(name, lint.lint_kind, level);
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

/// Dotted, hyphenated or slashed names in code: `cc-rs`, `tokio-util`, `std/io`, `a.b`.
static COMPOUND_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"[A-Za-z0-9_]+(?:[-./][A-Za-z0-9_]+)+").expect("hardcoded regex is valid")
});

/// URLs in text: bare, autolinked, in Markdown links and in comments.
static URL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"https?://[^\s<>()\[\]"'`]+"#).expect("hardcoded regex is valid")
});

/// Names the spell check takes as code: [`identifiers`] of the file and the project's
/// dependency names ([`spell::dep_names`]). All lowercase.
struct Idents {
    file: HashSet<String>,
    deps: Arc<spell::DepNames>,
    /// Dependency names of every sub-project and recurring capitalized names.
    project: Option<Arc<spell::ProjectVocab>>,
    /// Capitalized words of this file's link texts, as written (`[Zellij](…)`).
    link_names: HashSet<String>,
}

impl Idents {
    fn contains(&self, lower: &str) -> bool {
        self.file.contains(lower)
            || self.deps.contains(lower)
            || self.project.as_ref().is_some_and(|p| p.has_dep(lower))
    }

    /// A product or proper name of the project: capitalized as written, recurring across the
    /// checked files or in a link text of this file, and not a likely typo
    /// ([`spell::project_name`]). Lowercase forms never match.
    fn project_name(&self, s: &spell::Speller, word: &str) -> bool {
        self.project.as_ref().is_some_and(|p| p.has_name(word))
            || (self.link_names.contains(word) && spell::project_name(s, word))
    }
}

/// HTML attributes naming things (not `alt` / `title` prose): their values count as code.
static HTML_ATTR_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?i)\b(?:href|src|srcset|class|id|name|data-[\w-]+|action|poster|type)\s*=\s*(?:"([^"]*)"|'([^']*)')"#,
    )
    .expect("hardcoded regex is valid")
});

/// Identifiers used as code in this file (code outside comments, or code spans/blocks in
/// Markdown), with the compound names among them (`cc-rs`, also as `cc_rs`), plus the path
/// segments of its URLs, link targets and naming HTML attributes (`rust-phf` in a GitHub
/// URL, a `[@user](https://github.com/user)` handle), and the project's dependency names.
fn identifiers(ctx: &FileCtx) -> Idents {
    let src = ctx.src();
    let mut set = HashSet::new();
    let mut add = |text: &str| {
        for m in IDENT_RE.find_iter(text) {
            let w = m.as_str().to_lowercase();
            if w.contains('_') {
                set.insert(w.replace('_', "-"));
            }
            set.insert(w);
        }
        for m in COMPOUND_RE.find_iter(text) {
            let w = m.as_str().to_lowercase();
            if w.contains(['-', '_']) {
                set.insert(w.replace('-', "_"));
                set.insert(w.replace('_', "-"));
            }
            set.insert(w);
        }
    };
    let mut url_words = |url: &str| -> Vec<String> {
        url.split(|c: char| !c.is_alphanumeric())
            .filter(|w| !w.is_empty())
            .map(str::to_lowercase)
            .collect()
    };
    let mut urls: Vec<String> = URL_RE
        .find_iter(src)
        .flat_map(|m| url_words(m.as_str()))
        .collect();
    if let Some(md) = &ctx.a.md {
        let dests = md
            .links
            .iter()
            .map(|l| l.dest.as_str())
            .chain(md.ref_defs.iter().map(|r| r.dest.as_str()));
        urls.extend(dests.flat_map(&mut url_words));
        for r in &md.html {
            for c in HTML_ATTR_RE.captures_iter(&src[r.clone()]) {
                if let Some(v) = c.get(1).or_else(|| c.get(2)) {
                    urls.extend(url_words(v.as_str()));
                }
            }
        }
    }
    if let Some(md) = &ctx.a.md {
        for r in md
            .code_spans
            .iter()
            .chain(md.code_blocks.iter().map(|c| &c.range))
        {
            add(&src[r.clone()]);
        }
    } else if let Some(po) = &ctx.a.po {
        // Catalog: references, flags, contexts and key msgids name things; msgids are prose.
        for r in po.identifier_ranges() {
            add(&src[r]);
        }
    } else {
        let mut pos = 0;
        for c in &ctx.a.comments {
            add(&src[pos..c.range.start.max(pos)]);
            pos = pos.max(c.range.end);
        }
        add(&src[pos..]);
    }
    set.extend(urls);
    let link_names = ctx.a.md.as_ref().map_or_else(HashSet::new, |md| {
        md.links
            .iter()
            .filter(|l| !l.is_image)
            .flat_map(|l| l.text.split(|c: char| !c.is_alphabetic()))
            .filter(|w| w.starts_with(char::is_uppercase))
            .map(str::to_string)
            .collect()
    });
    Idents {
        file: set,
        deps: spell::dep_names(&ctx.config.root, &ctx.a.file.path),
        project: spell::project_vocab(),
        link_names,
    }
}

/// Names and shorthand the spell check skips in every engine, judged on the source around
/// `seg.text[s..e]`:
/// - a part of a dotted / hyphenated / slashed name that appears as code or in a URL of the
///   file, or is followed by `crate`, `package` or `library` (`cc-rs`, `release-plz crate`),
///   or is a `scope/area:` prefix, or the compound has a part with digits (`i686-pc-windows`);
/// - an `@handle`;
/// - a conventional-commit scope: `feat(deps):`, `*(http)*`;
/// - two or three lowercase letters next to digits, brackets or name punctuation (`rb-sys`,
///   `v2rs`, `file.md`, `(pr 12)`), or inside link text.
fn jargon(
    seg: &Segment,
    ctx: &FileCtx,
    s: usize,
    e: usize,
    idents: &Idents,
    english: bool,
) -> bool {
    let src = ctx.src();
    let (a, b) = (seg.range.start + s, seg.range.start + e);
    let word = &src[a..b];
    let before = &src[..a];
    let after = &src[b..];
    let is_sep = |c: char| matches!(c, '-' | '.' | '/' | '_');
    // The compound around the word: alphanumeric runs joined by single separators.
    let mut start = a;
    loop {
        let mut it = src[..start].chars().rev();
        match (it.next(), it.next()) {
            (Some(sep), Some(c)) if is_sep(sep) && c.is_alphanumeric() => {
                start -= sep.len_utf8();
                start -= src[..start]
                    .chars()
                    .rev()
                    .take_while(|c| c.is_alphanumeric())
                    .map(char::len_utf8)
                    .sum::<usize>();
            }
            _ => break,
        }
    }
    let mut end = b;
    loop {
        let mut it = src[end..].chars();
        match (it.next(), it.next()) {
            (Some(sep), Some(c)) if is_sep(sep) && c.is_alphanumeric() => {
                end += sep.len_utf8();
                end += src[end..]
                    .chars()
                    .take_while(|c| c.is_alphanumeric())
                    .map(char::len_utf8)
                    .sum::<usize>();
            }
            _ => break,
        }
    }
    let compound = &src[start..end];
    if compound.len() > word.len() {
        let rest = src[end..].trim_start_matches([' ', '`']).to_lowercase();
        let named = ["crate", "package", "librar", "module", "plugin", "gem"]
            .iter()
            .any(|k| rest.starts_with(k));
        // `i686-pc-windows-gnullvm`, `user/169-drop-borrow`: a name with a numbered part.
        let numbered = compound
            .split(is_sep)
            .any(|p| p.chars().any(|c| c.is_ascii_digit()));
        // Finnish and Swedish hyphenate words onto numbers and acronyms (`UTF-8-merkistö`):
        // only code names count there.
        let numbered = numbered && (english || !compound.contains('-'));
        if idents.contains(&compound.to_lowercase())
            || (named && english)
            || numbered
            || (compound.contains('/') && src[end..].starts_with(':'))
        {
            return true;
        }
    }
    // `@finnbear`: a handle.
    if src[..start].ends_with('@') {
        return true;
    }
    // `Väinö`, `Göteborg`: a capitalized word with diacritics names someone or somewhere; the
    // English words that keep theirs (`café`, `naïve`) are in the dictionaries. (Not in
    // Finnish or Swedish text, where such words start sentences.)
    if english
        && word.starts_with(char::is_uppercase)
        && word.chars().any(|c| c.is_alphabetic() && !c.is_ascii())
    {
        return true;
    }
    let lower = word.chars().all(|c| c.is_ascii_lowercase());
    // `feat(deps):`, `*(streamable-http)*`, `fix(api)!:`
    let (open, close) = (&src[..start], &src[end..]);
    if lower
        && open.ends_with('(')
        && close.starts_with(')')
        && (close[1..].starts_with([':', '!', '*'])
            || open[..open.len() - 1].ends_with(|c: char| c.is_alphabetic() || c == '*'))
    {
        return true;
    }
    // `.exe`, `*.tmtheme`: a file extension.
    if lower
        && open.ends_with('.')
        && open[..open.len() - 1]
            .chars()
            .next_back()
            .is_none_or(|c| c.is_whitespace() || matches!(c, '*' | '(' | '"' | '\''))
    {
        return true;
    }
    // `- tcp: fix ...`: a lowercase area label opening a line or list item.
    let line_start = open.rsplit('\n').next().unwrap_or("");
    if lower
        && close.starts_with(':')
        && line_start
            .chars()
            .all(|c| c.is_whitespace() || "-*+>.)#/0123456789".contains(c))
    {
        return true;
    }
    if !(lower && (2..=3).contains(&word.len())) {
        return false;
    }
    let prev = before.chars().next_back();
    let next = after.chars().next();
    let name_char =
        |c: Option<char>| c.is_some_and(|c| c.is_ascii_digit() || "()[]{}-/#@_<>".contains(c));
    // A dot counts only inside a name (`file.md`), not as the end of a sentence.
    let dotted = (prev == Some('.') && before[..before.len() - 1].ends_with(char::is_alphanumeric))
        || (next == Some('.') && after[1..].starts_with(char::is_alphanumeric));
    let in_link_text = ctx.a.md.as_ref().is_some_and(|md| {
        md.links
            .iter()
            .any(|l| !l.is_image && l.range.start < a && b < l.range.end)
    });
    name_char(prev) || name_char(next) || dotted || in_link_text
}

/// The whole word when `seg.text[s..e]` is part of one split by emphasis markup:
/// `*ser*ializing`, `**foo**bar`.
fn emphasis_split(seg: &Segment, src: &str, s: usize, e: usize) -> Option<String> {
    let (a, b) = (seg.range.start + s, seg.range.start + e);
    let marker = |c: char| c == '*' || c == '_';
    let lm = src[..a].chars().rev().take_while(|&c| marker(c)).count();
    let rm = src[b..].chars().take_while(|&c| marker(c)).count();
    let left: String = if lm > 0 {
        let mut v: Vec<char> = src[..a - lm]
            .chars()
            .rev()
            .take_while(|c| c.is_alphabetic())
            .collect();
        v.reverse();
        v.into_iter().collect()
    } else {
        String::new()
    };
    let right: String = if rm > 0 {
        src[b + rm..]
            .chars()
            .take_while(|c| c.is_alphabetic())
            .collect()
    } else {
        String::new()
    };
    (!left.is_empty() || !right.is_empty()).then(|| format!("{left}{}{right}", &src[a..b]))
}

impl Checker {
    /// Language-mode spelling findings that are no typos: forms of names the file uses inside
    /// sentences (`Fimea` starting a sentence, `Fimean` elsewhere), and case endings glued to
    /// a code span (`` `namespace`ssaan ``).
    fn lang_skips(&self, seg: &Segment, src: &str, s: usize, word: &str) -> bool {
        let orig = &src.as_bytes()[seg.range.clone()];
        let glued =
            s > 0 && seg.text.as_bytes()[s - 1] == b' ' && !orig[s - 1].is_ascii_whitespace();
        // Part of an email address or URL in the source (`tuki@example.fi`).
        let chunk_start = orig[..s]
            .iter()
            .rposition(u8::is_ascii_whitespace)
            .map_or(0, |i| i + 1);
        let chunk_end = orig[s..]
            .iter()
            .position(u8::is_ascii_whitespace)
            .map_or(orig.len(), |i| s + i);
        let chunk = &orig[chunk_start..chunk_end];
        let address = chunk.contains(&b'@') || chunk.windows(3).any(|w| w == b"://");
        address
            || (glued && word.starts_with(char::is_lowercase))
            || (word.starts_with(char::is_uppercase)
                && spell_lang::is_name_form(word, &self.file_names))
            || self.configured_forms(word)
    }

    /// Forms of configured words in a Finnish or Swedish text: a hyphenated compound led by
    /// an entity or vocab term (`Kela-handläggare`), a Finnish inflection of an accepted
    /// word, vocab term or entity (`Telian`, `Telia:n`), and a Swedish closed compound led by
    /// one (`Teliaärendet`).
    fn configured_forms(&self, word: &str) -> bool {
        let Some((code, sp)) = &self.lang else {
            return false;
        };
        let single = |w: &str| {
            self.accepted(w, w)
                || self
                    .phrases
                    .list
                    .iter()
                    .any(|p| !p.canonical.contains(' ') && p.canonical.eq_ignore_ascii_case(w))
        };
        let lower = word.to_lowercase();
        let led = self.phrases.list.iter().any(|p| {
            let c = p.canonical.to_lowercase();
            !c.contains(' ')
                && lower
                    .strip_prefix(&c)
                    .and_then(|r| r.strip_prefix('-'))
                    .is_some_and(|rest| {
                        !rest.is_empty() && spell_lang::word_ok(&**sp, code, rest, None)
                    })
        });
        led || code == "fi" && spell_lang::finnish_inflection(&**sp, word, &single)
            || code == "sv" && spell_lang::swedish_led(&**sp, word, &single)
    }

    /// Dictionary for the project-name typo guard (Hunspell even for the Harper engines).
    fn name_speller(&self) -> &'static spell::Speller {
        self.hunspell
            .unwrap_or_else(|| spell::speller(self.dialect))
    }

    /// `aislop's`: harper does not derive possessives of custom words.
    fn known_possessive(&self, word: &str, idents: &Idents) -> bool {
        let base = word.strip_suffix("'s").or_else(|| word.strip_suffix("’s"));
        base.is_some_and(|b| {
            self.known_word(b) || looks_like_identifier(b, idents) || self.known_in_other_case(b)
        })
    }

    /// Names written in lowercase on purpose (`devenv`, `npm`, `bun's`, `v2`), which may
    /// start a sentence.
    fn lowercase_by_convention(&self, word: &str, idents: &Idents) -> bool {
        let base = word
            .strip_suffix("'s")
            .or_else(|| word.strip_suffix("’s"))
            .unwrap_or(word);
        !base.chars().any(char::is_uppercase)
            && (TECH_WORDS.contains(&base)
                || self.accepted_exact.contains(base)
                || looks_like_identifier(base, idents))
    }

    /// A closed compound `SplitWords` should not split: a known word in some casing.
    fn known_compound(&self, word: &str) -> bool {
        let lower = word.to_lowercase();
        TECH_COMPOUNDS.contains(&lower.as_str())
            || self.known_word(word)
            || self.known_in_other_case(word)
    }

    /// Comments often lowercase proper nouns and acronyms (`html`, `toml`, `british`).
    fn known_in_other_case(&self, word: &str) -> bool {
        let lower = word.to_lowercase();
        if TECH_WORDS.iter().any(|t| t.to_lowercase() == lower) {
            return true;
        }
        let cap = capitalize(&lower);
        if let Some((_, sp)) = &self.lang {
            return sp.check(&cap) || sp.check(&lower.to_uppercase());
        }
        match (self.hunspell, &self.harper) {
            (Some(h), _) => {
                spell::known(h, &cap)
                    || spell::known(h, &lower.to_uppercase())
                    || spell::harper_any_case(word, self.dialect)
            }
            (None, Some(d)) => [cap, lower.to_uppercase()]
                .iter()
                .any(|w| d.contains_in_dialect(w)),
            (None, None) => false,
        }
    }
}

fn capitalize(lower: &str) -> String {
    let mut cap = String::new();
    let mut chars = lower.chars();
    if let Some(f) = chars.next() {
        cap.extend(f.to_uppercase());
        cap.push_str(chars.as_str());
    }
    cap
}

/// Harper's default-on rules and our own: on unless a disable list names them, and an exact
/// `grammar/<Name>` (or `spelling`) entry wins.
fn own_rule_on(config: &Config, name: &str, code: bool) -> bool {
    if let Some((_, on)) = explicit_harper_rules(config).find(|(n, _)| *n == name) {
        return on;
    }
    !config.prose.disable.iter().any(|d| d == name)
        && !(code && config.comments.disable.iter().any(|d| d == name))
}

/// `hybrid` runs Harper only on prose that reads as sentences: paragraphs, quotes, list items
/// and comments of four or more words ending in sentence punctuation.
fn full_sentence(seg: &Segment) -> bool {
    let kind_ok = matches!(
        seg.kind,
        SegmentKind::Paragraph
            | SegmentKind::ListItem
            | SegmentKind::BlockQuote
            | SegmentKind::Comment
            | SegmentKind::DocComment
    );
    kind_ok
        && seg
            .text
            .split_whitespace()
            .filter(|w| w.chars().any(char::is_alphabetic))
            .nth(3)
            .is_some()
        && seg
            .text
            .trim_end()
            .trim_end_matches(['"', '\'', ')', ']', '”', '’', '*', '_'])
            .ends_with(['.', '!', '?', '…'])
}

/// Table cells, and list items or headings without end punctuation, are not sentences.
fn is_fragment(seg: &Segment) -> bool {
    match seg.kind {
        SegmentKind::TableCell => true,
        SegmentKind::ListItem | SegmentKind::Heading => !seg
            .text
            .trim_end()
            .trim_end_matches(['"', '\'', ')', ']', '”', '’', '*', '_'])
            .ends_with(['.', '!', '?', '…']),
        _ => false,
    }
}

/// The hyphenated or snake_case token around `text[s..e]` (`pre-commit` around `pre`).
fn word_token(text: &str, s: usize, e: usize) -> &str {
    let part = |c: char| c.is_alphanumeric() || c == '-' || c == '_';
    let start = text[..s]
        .char_indices()
        .rev()
        .take_while(|&(_, c)| part(c))
        .last()
        .map_or(s, |(i, _)| i);
    let end = text[e..]
        .char_indices()
        .find(|&(_, c)| !part(c))
        .map_or(text.len(), |(i, _)| e + i);
    text[start..end].trim_matches(['-', '_'])
}

/// An uppercase `A` mid-sentence is a label (`Plan A`, `Option A or Option B`), not an article.
fn letter_label(text: &str, s: usize, word: &str) -> bool {
    let first = word.split_whitespace().next().unwrap_or("");
    let before = text[..s].trim_end();
    first == "A" && !before.is_empty() && !before.ends_with(['.', '!', '?', ':'])
}

/// `s` of a possessive or contraction whose base Harper split off: after inline code, a
/// blanked URL or path, or an id (`` `Widget`'s ``, `RFC-7's`).
fn possessive_suffix(text: &str, s: usize, word: &str) -> bool {
    let bare = word.trim_start_matches(['\'', '’']);
    bare.eq_ignore_ascii_case("s") && (bare.len() < word.len() || text[..s].ends_with(['\'', '’']))
}

/// A whitespace span that runs to the end of its line: a hard break or trailing spaces.
fn trailing_whitespace(text: &str, e: usize) -> bool {
    text[e..]
        .split('\n')
        .next()
        .is_none_or(|rest| rest.chars().all(char::is_whitespace))
}

/// `i` used as a roman numeral: `(i)`, `[i]`, `r/i`, or an `i.` / `i)` list enumerator.
fn enumerator_i(text: &str, s: usize, e: usize) -> bool {
    let before = text[..s].trim_end_matches(' ');
    let prev = before.chars().next_back();
    let next = text[e..].chars().next();
    matches!(prev, Some('(' | '[' | '/'))
        || matches!(next, Some(')' | ']' | '/'))
        || (matches!(next, Some('.')) && (before.is_empty() || before.ends_with('\n')))
}

/// Some char of `text[s..e]` was blanked (code or markup between two words).
fn masked_inside(seg: &Segment, src: &str, s: usize, e: usize) -> bool {
    let orig = &src.as_bytes()[seg.range.start + s..seg.range.start + e];
    seg.text.as_bytes()[s..e]
        .iter()
        .zip(orig)
        .any(|(t, o)| t != o && !o.is_ascii_whitespace())
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
/// With `right` false, blanked text after the lint does not count.
fn touches_mask(seg: &Segment, src: &str, s: usize, e: usize, right: bool) -> bool {
    let text = seg.text.as_bytes();
    let orig = &src.as_bytes()[seg.range.clone()];
    let lo = s.saturating_sub(2);
    let hi = if right { (e + 2).min(text.len()) } else { e };
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
    let masked = |i: usize| text[i] == b' ' && !orig[i].is_ascii_whitespace() && !is_marker(i);
    // Real whitespace (line breaks, list indentation) between the lint and blanked code still
    // makes them neighbors: Harper reads an article followed by a line break and blanked code as the article before the next word.
    let space =
        |i: usize| text[i].is_ascii_whitespace() && (orig[i].is_ascii_whitespace() || is_marker(i));
    let after = (e..text.len()).find(|&i| !space(i));
    let left = (0..s).rev().find(|&i| !space(i));
    (lo..hi).any(masked) || (right && after.is_some_and(masked)) || left.is_some_and(masked)
}

fn looks_like_identifier(word: &str, idents: &Idents) -> bool {
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

    /// For Harper rules outside the `curated` allowlist and the `hybrid` default.
    #[cfg(feature = "harper")]
    fn run_harper(src: &str) -> Vec<Finding> {
        run_with(src, &with_engine(Engine::Harper))
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

    #[cfg(feature = "harper")]
    #[test]
    fn explicit_rule_enables_disabled_harper_rule() {
        let src = "I bought apples, pears and bananas today.\n";
        let mut config = with_engine(Engine::Hybrid);
        assert!(
            run_with(src, &config)
                .iter()
                .all(|f| f.rule != "grammar/OxfordComma")
        );
        config
            .rules
            .insert("grammar/OxfordComma".into(), crate::config::Level::Error);
        let f = run_with(src, &config);
        assert!(f.iter().any(|f| f.rule == "grammar/OxfordComma"), "{f:?}");
    }

    #[test]
    fn rules_turned_off_do_not_run() {
        let mut config = Config::default();
        config
            .rules
            .insert("grammar/*".into(), crate::config::Level::Off);
        let c = Checker::new(&config, 0);
        assert!(!c.harper_md && !c.harper_code && c.spell_md);
        let f = run_with("This is a an sentance.\n", &config);
        assert!(
            f.iter().all(|f| f.rule == "spelling") && !f.is_empty(),
            "{f:?}"
        );
        config
            .rules
            .insert("grammar/AnA".into(), crate::config::Level::Warning);
        #[cfg(feature = "harper")]
        {
            config.prose.engine = Engine::Hybrid;
            let c = Checker::new(&config, 0);
            let h = c.harper.as_ref().expect("harper engine");
            let on: Vec<_> = h
                .group
                .iter_keys()
                .filter(|k| h.md_config.is_rule_enabled(k))
                .collect();
            assert_eq!(on, ["AnA"]);
        }
    }

    #[cfg(feature = "harper")]
    #[test]
    fn dictionary_shared_across_threads() {
        use harper::shared_dictionary;
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

    fn rules_hit<'a>(f: &'a [Finding], src: &str) -> Vec<(&'a str, String)> {
        f.iter()
            .map(|f| (f.rule.as_str(), src[f.range.clone()].to_string()))
            .collect()
    }

    #[test]
    fn units_and_enumerators_are_not_words() {
        let src = "The build took 10.5s and 200ms, 3x faster with 4GB (12.5 s total).\n\n\
                   Section 4(2)(c) applies, see points (a), (ii) and [b], and step S-3.\n";
        let f = run(src);
        assert!(f.is_empty(), "{:?}", rules_hit(&f, src));
    }

    #[test]
    fn units_in_comments() {
        let src = "/// The cache warms in 10.5s, 3x faster; see (ii) and the `Cache`'s docs.\nfn f() {}\n";
        let a = Analyzed::new(SourceFile::new(
            "a.rs".into(),
            "a.rs".into(),
            FileKind::Code(crate::source::Lang::Rust),
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
        assert!(out.is_empty(), "{:?}", rules_hit(&out, src));
    }

    #[test]
    fn possessive_after_code_and_ids() {
        let src = "The `Widget`'s owner and RFC-7's scope and #12's fix and the `x`’s value.\n";
        let f = run(src);
        assert!(
            f.iter().all(|f| f.rule != "spelling"),
            "{:?}",
            rules_hit(&f, src)
        );
    }

    #[test]
    fn split_words_spares_known_compounds() {
        let src = "The implementor added a teardown step, a textarea, lifecycle checks and \
                   auditability, plus codegen for the dropdown.\n";
        let f = run(src);
        assert!(f.is_empty(), "{:?}", rules_hit(&f, src));
        // A real run-together word is still reported, as info.
        let f = run("The quickbrown fox jumps.\n");
        assert!(
            f.iter()
                .all(|f| f.rule != "grammar/SplitWords" || f.severity == Severity::Info)
        );
    }

    #[cfg(feature = "harper")]
    #[test]
    fn fragments_skip_sentence_rules() {
        let src = "# retry the request on timeout\n\n\
                   | status | notes |\n|---|---|\n| pending review | waiting on reviewer |\n\n\
                   - update docs for the release\n- migrate old tokens to new format\n";
        let f = run_harper(src);
        assert!(
            f.iter().all(|f| !FRAGMENT_RULES
                .iter()
                .any(|r| f.rule == format!("grammar/{r}"))),
            "{:?}",
            rules_hit(&f, src)
        );
        // Full sentences keep them.
        let f = run_harper("this sentence starts in lowercase.\n");
        assert!(
            f.iter().any(|f| f.rule == "grammar/SentenceCapitalization"),
            "{f:?}"
        );
        let f = run_harper("- this item is a full sentence.\n");
        assert!(
            f.iter().any(|f| f.rule == "grammar/SentenceCapitalization"),
            "{f:?}"
        );
    }

    #[cfg(feature = "harper")]
    #[test]
    fn table_padding_and_hard_breaks_are_layout() {
        let src = "| Name     | Value   |\n|----------|---------|\n| alpha    | first   |\n\n\
                   First line with a break  \nSecond line here.\n";
        let f = run_harper(src);
        assert!(
            f.iter().all(|f| !WHITESPACE_RULES
                .iter()
                .any(|r| f.rule == format!("grammar/{r}"))),
            "{:?}",
            rules_hit(&f, src)
        );
        let f = run_harper("There are two  spaces here.\n");
        assert!(f.iter().any(|f| f.rule == "grammar/Spaces"), "{f:?}");
    }

    #[cfg(feature = "harper")]
    #[test]
    fn roman_i_is_not_a_pronoun() {
        let src = "Choose i) the first or the r/i column.\n\ni. first option\n";
        let f = run_harper(src);
        assert!(
            f.iter()
                .all(|f| f.rule != "grammar/CapitalizePersonalPronouns"),
            "{:?}",
            rules_hit(&f, src)
        );
        let f = run_harper("Then i went home.\n");
        assert!(
            f.iter()
                .any(|f| f.rule == "grammar/CapitalizePersonalPronouns"),
            "{f:?}"
        );
    }

    #[test]
    fn article_before_code_on_next_line() {
        let src = "- The config may hold a\n  `timeout` key.\n";
        let f = run(src);
        assert!(
            f.iter().all(|f| f.rule != "grammar/AnA"),
            "{:?}",
            rules_hit(&f, src)
        );
    }

    fn run_ts(src: &str) -> Vec<Finding> {
        let a = Analyzed::new(SourceFile::new(
            "a.ts".into(),
            "a.ts".into(),
            FileKind::Code(crate::source::Lang::TypeScript),
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
    fn article_before_code_across_comment_marker() {
        let src = "/**\n * Returns a\n * `Result` value.\n */\nfunction f() {}\n";
        let f = run_ts(src);
        assert!(
            f.iter().all(|f| f.rule != "grammar/AnA"),
            "{:?}",
            rules_hit(&f, src)
        );
    }

    #[test]
    fn labels_tools_and_snake_case() {
        let src = "Plan A worked and Option A beat Option B.\n\n\
                   devenv adds a cache. The widget_ui crate is new.\n\n\
                   | Size | Cost |\n|---|---|\n| S | M |\n";
        let f = run(src);
        assert!(f.is_empty(), "{:?}", rules_hit(&f, src));
    }

    #[test]
    fn dependency_attribute_and_variant_names() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
        std::fs::write(
            root.join("Cargo.toml"),
            "[dependencies]\nzorblax-util = \"1\"\n",
        )
        .unwrap();
        let src = "We use zorblax and zorblax_util daily.\n\n\
                   <p class=\"frobnic\">Frobnic styles it.</p>\n\n\
                   See `snorkel_lib`: the snorkel-lib crate. We recieve data.\n\n\
                   Thanks @finnbear for i686-pc-windows-gnullvm support.\n";
        let a = Analyzed::new(SourceFile::new(
            root.join("a.md"),
            "a.md".into(),
            FileKind::Markdown,
            src.into(),
        ));
        let config = Config {
            root: root.clone(),
            ..Config::default()
        };
        let mut out = Vec::new();
        check(
            &FileCtx {
                a: &a,
                config: &config,
            },
            &mut out,
        );
        let words: Vec<&str> = out.iter().map(|f| &src[f.range.clone()]).collect();
        assert_eq!(words, ["recieve"]);
    }

    fn accepting(words: &[&str], patterns: &[&str]) -> Config {
        let mut c = Config::default();
        c.prose.accept = words.iter().map(|w| w.to_string()).collect();
        c.prose.accept_patterns = patterns.iter().map(|w| w.to_string()).collect();
        c
    }

    #[test]
    fn accept_list_hyphens_case_and_patterns() {
        let src = "The zorbl-quuxer tool runs frobnitzer, Frobnitzer and glorpify or glorpified.\n";
        let spelled = |c: &Config| {
            run_with(src, c)
                .into_iter()
                .filter(|f| f.rule == "spelling")
                .map(|f| src[f.range].to_string())
                .collect::<Vec<_>>()
        };
        let before = spelled(&Config::default());
        for w in ["zorbl", "frobnitzer", "Frobnitzer", "glorpify"] {
            assert!(before.iter().any(|b| b == w), "{w} in {before:?}");
        }
        let c = accepting(&["Zorbl-Quuxer", "FROBNITZER"], &["glorp\\w*"]);
        assert!(spelled(&c).is_empty(), "{:?}", spelled(&c));
        // Checkers for different configs coexist: switching back still reports.
        assert_eq!(spelled(&Config::default()), before);
        assert!(spelled(&c).is_empty());
    }

    #[test]
    fn severity_levels() {
        use crate::config::GrammarLevel;
        let src = "This is a apple. There are two  spaces here.\n";
        let sev = |c: &Config, rule: &str| {
            run_with(src, c)
                .into_iter()
                .find(|f| f.rule == rule)
                .map(|f| f.severity)
        };
        #[cfg(feature = "harper")]
        {
            let mut c = with_engine(Engine::Harper);
            assert_eq!(sev(&c, "grammar/AnA"), Some(Severity::Warning));
            assert_eq!(sev(&c, "grammar/Spaces"), Some(Severity::Info));
            c.prose.grammar_level = GrammarLevel::Warning;
            assert_eq!(sev(&c, "grammar/Spaces"), Some(Severity::Warning));
        }
        assert_eq!(
            sev(&with_engine(Engine::Spellbook), "grammar/AnA"),
            Some(Severity::Warning)
        );
        assert_eq!(
            default_severity("SpellCheck", LintKind::Spelling, GrammarLevel::Info),
            Severity::Error
        );
        assert_eq!(
            default_severity("X", LintKind::Style, GrammarLevel::Harper),
            Severity::Info
        );
        assert_eq!(
            default_severity("X", LintKind::Typo, GrammarLevel::Harper),
            Severity::Error
        );
    }

    fn with_engine(engine: Engine) -> Config {
        let mut c = Config::default();
        c.prose.engine = engine;
        c
    }

    const ENGINES: [Engine; 4] = [
        Engine::Harper,
        Engine::Curated,
        Engine::Spellbook,
        Engine::Hybrid,
    ];

    #[test]
    fn engines_basics() {
        for engine in ENGINES {
            let c = with_engine(engine);
            let src = "# Title\n\nThis sentance has a mistake.\n";
            let f = run_with(src, &c);
            let s = f
                .iter()
                .find(|f| f.rule == "spelling")
                .unwrap_or_else(|| panic!("{engine:?}: {f:?}"));
            assert_eq!(&src[s.range.clone()], "sentance", "{engine:?}");
            assert!(
                s.suggestions.iter().any(|x| x == "sentence"),
                "{engine:?}: {:?}",
                s.suggestions
            );
            for (src, rule, hit) in [
                ("We ate a apple for lunch today.\n", "grammar/AnA", "a"),
                (
                    "I made this for for you today.\n",
                    "grammar/RepeatedWords",
                    "for for",
                ),
                (
                    "This one is better then the old one.\n",
                    "grammar/ThenThan",
                    "then",
                ),
            ] {
                let f = run_with(src, &c);
                assert!(
                    f.iter()
                        .any(|f| f.rule == rule && &src[f.range.clone()] == hit),
                    "{engine:?} {rule}: {:?}",
                    rules_hit(&f, src)
                );
            }
            let src = "Call `frobnicatr` then use frobnicatr again.\n";
            let f = run_with(src, &c);
            assert!(f.iter().all(|f| f.rule != "spelling"), "{engine:?}: {f:?}");
        }
    }

    #[cfg(feature = "harper")]
    #[test]
    fn spellbook_engine_skips_harper() {
        let c = Checker::new(&with_engine(Engine::Spellbook), 0);
        assert!(c.harper.is_none() && c.hunspell.is_some());
        let c = Checker::new(&with_engine(Engine::Curated), 0);
        let h = c.harper.as_ref().unwrap();
        let on: Vec<_> = h
            .group
            .iter_keys()
            .filter(|k| h.md_config.is_rule_enabled(k))
            .collect();
        assert!(on.iter().all(|k| CURATED.contains(k)), "{on:?}");
        assert!(on.contains(&"AnA"));
    }

    #[test]
    fn spellbook_filters_and_patterns() {
        let c = with_engine(Engine::Spellbook);
        let src = "The build took 10.5s and 200ms, 3x faster with 4GB (12.5 s total).\n\n\
                   Section 4(2)(c) applies, see points (a), (ii) and [b], and step S-3.\n\n\
                   The `Widget`'s owner and RFC-7's scope. See https://example.com/foo_bar now.\n\n\
                   An hour, a user, a university, a one-off, an honest heir, a European, an HTML \
                   page, a JSON file, a URL or an URL, the 2nd and 21st.\n";
        let f = run_with(src, &c);
        assert!(f.is_empty(), "{:?}", rules_hit(&f, src));
        // Numbers with letter suffixes are blanked out of segment text (units), so the number
        // suffix rules only see what survives; the tokenizer tests cover them.
        let src = "They dint know my the plan.\n";
        let f = run_with(src, &c);
        let hits = rules_hit(&f, src);
        for want in [("grammar/Didnt", "dint"), ("grammar/TheMy", "my the")] {
            assert!(
                hits.iter().any(|(r, t)| (*r, t.as_str()) == want),
                "{want:?} in {hits:?}"
            );
        }
    }

    #[test]
    fn hybrid_runs_harper_on_sentences_only() {
        let c = with_engine(Engine::Hybrid);
        // Harper's POS rule on a sentence.
        let src = "I think its a good idea to go there.\n";
        let f = run_with(src, &c);
        assert!(
            f.iter().any(|f| f.rule == "grammar/ItsContraction"),
            "{:?}",
            rules_hit(&f, src)
        );
        // Table cells skip Harper but keep our pattern rules.
        let src = "| a | b |\n|---|---|\n| its because a apple | x |\n";
        let f = run_with(src, &c);
        assert!(
            f.iter().all(|f| f.rule != "grammar/ItsContraction"),
            "{f:?}"
        );
        assert!(f.iter().any(|f| f.rule == "grammar/AnA"), "{f:?}");
    }

    #[test]
    fn confusables_run_in_every_engine() {
        let src = "Always test you're own code, since you will not loose data.\n";
        for engine in ENGINES {
            let f = run_with(src, &with_engine(engine));
            let hits = rules_hit(&f, src);
            for want in ["grammar/YourYoure", "grammar/LoseLoose"] {
                assert!(hits.iter().any(|(r, _)| *r == want), "{engine:?}: {hits:?}");
            }
            // One finding per word, even where Harper has a rule for it too.
            assert_eq!(
                f.iter().filter(|f| f.rule == "grammar/YourYoure").count(),
                1,
                "{engine:?}"
            );
        }
    }

    #[test]
    fn article_before_acronym_identifier_or_variable() {
        let src =
            "Use a SQLite file, an rpath, a usize, a [`Stream`] of a and b, and an new one.\n";
        for engine in ENGINES {
            let f = run_with(src, &with_engine(engine));
            let ana: Vec<_> = rules_hit(&f, src)
                .into_iter()
                .filter(|(r, _)| *r == "grammar/AnA")
                .collect();
            assert_eq!(ana, [("grammar/AnA", "an".to_string())], "{engine:?}");
        }
    }

    #[test]
    fn article_before_number() {
        let src = "Buy a 8GB disk, a 8-byte key, an 100 ms delay and a `8` value.\n\nUse an 80% cut, a 1 in 5 chance.\n";
        for engine in ENGINES {
            let f = run_with(src, &with_engine(engine));
            let ana: Vec<(usize, String)> = f
                .iter()
                .filter(|f| f.rule == "grammar/AnA")
                .map(|f| {
                    (
                        f.range.start,
                        f.suggestions.first().cloned().unwrap_or_default(),
                    )
                })
                .collect();
            assert_eq!(
                ana,
                [(4, "an".to_string()), (16, "an".into()), (30, "a".into())],
                "{engine:?}"
            );
        }
    }

    #[test]
    fn repeated_word_next_to_code() {
        let src = "Use use `max_keys` to bound it, or pick `a` a better key.\n";
        let f = run_with(src, &with_engine(Engine::Hybrid));
        assert!(
            f.iter()
                .any(|f| f.rule == "grammar/RepeatedWords" && &src[f.range.clone()] == "Use use"),
            "{:?}",
            rules_hit(&f, src)
        );
        // Code between two words is not a repetition.
        let src = "Set it to `x` to be safe.\n";
        let f = run_with(src, &with_engine(Engine::Hybrid));
        assert!(f.iter().all(|f| f.rule != "grammar/RepeatedWords"));
    }

    /// Our Harper-dictionary spell check flags exactly what Harper's `SpellCheck` flags (in the
    /// `harper` engine other rules like `ExpandDependencies` may win the overlap for a word).
    #[cfg(feature = "harper")]
    #[test]
    fn harper_spelling_matches_spell_check() {
        use harper_core::linting::LintGroup;
        use harper_core::parsers::PlainEnglish;
        use harper_core::spell::{FstDictionary, MergedDictionary};
        use harper_core::{Dialect, Document};
        let text = "Bump deps, impl the semver serde iface and a syscall for the backends. \
                    It colour the Github oauth deque with a sentance, word(s) and hahaha.";
        let dict = FstDictionary::curated();
        let doc = Document::new(text, &PlainEnglish, dict.as_ref());
        let mut g = LintGroup::new_curated(dict.clone(), Dialect::American);
        let keys: Vec<String> = g.iter_keys().map(str::to_string).collect();
        for k in &keys {
            g.config.set_rule_enabled(k, k == "SpellCheck");
        }
        let harper: Vec<_> = g.organized_lints(&doc)["SpellCheck"]
            .iter()
            .map(|l| (l.span.start, l.span.end))
            .collect();
        let mut c = Checker::new(&with_engine(Engine::Harper), 0);
        let h = c.harper.as_mut().unwrap();
        h.dict = Arc::new({
            let mut m = MergedDictionary::new();
            m.add_dictionary(dict);
            m
        });
        let ours: Vec<_> = h
            .misspelled(&doc)
            .iter()
            .map(|l| (l.span.start, l.span.end))
            .collect();
        assert_eq!(ours, harper);
        assert!(harper.len() >= 8, "{harper:?}");
    }

    #[test]
    fn spellbook_falls_back_to_harper_words() {
        let us = spell::dictionary("american");
        for w in [
            "syscall",
            "deserialization",
            "backends",
            "Syscall's",
            "const",
        ] {
            assert!(spell::known(us, w), "{w}");
        }
        for w in ["sentance", "frobnicatr", "colour"] {
            assert!(!spell::known(us, w), "{w}");
        }
        assert!(spell::known(spell::dictionary("british"), "colour"));
        let src = "The syscall backends need deserialization of a sentance.\n";
        let f = run_with(src, &with_engine(Engine::Spellbook));
        let hits = rules_hit(&f, src);
        assert_eq!(hits, [("spelling", "sentance".to_string())], "{hits:?}");
    }

    #[test]
    fn jargon_is_not_misspelled() {
        let src = "\
# Changes

- Use `cc-rs` now; cc-rs and the release-plz crate ship it.
- *(streamable-http)* fix; feat(deps): bump.
- tcp: fix a stall.
- Thanks [@adamreichold](https://github.com/adamreichold) for phf ([#7](https://github.com/rust-phf/rust-phf/pull/7)).
- See [md](https://example.com/md) and file.md, run .exe tools, etc and more.
- *Ser*ializing and **foo**bar by Väinö in Göteborg, with a sentance.
";
        for engine in ENGINES {
            let f = run_with(src, &with_engine(engine));
            let spelled: Vec<&str> = f
                .iter()
                .filter(|f| f.rule == "spelling")
                .map(|f| &src[f.range.clone()])
                .collect();
            assert_eq!(spelled, ["sentance"], "{engine:?}");
        }
    }

    #[test]
    fn dialect_variants_in_comments_follow_the_dialect() {
        let src = "// The run was cancelled; its behaviour is odd.\nfunction f() {}\n";
        let a = Analyzed::new(SourceFile::new(
            "a.ts".into(),
            "a.ts".into(),
            FileKind::Code(crate::source::Lang::TypeScript),
            src.into(),
        ));
        for engine in ENGINES {
            let config = with_engine(engine);
            let mut f = Vec::new();
            check(
                &FileCtx {
                    a: &a,
                    config: &config,
                },
                &mut f,
            );
            let spelled: Vec<&str> = f
                .iter()
                .filter(|f| f.rule == "spelling")
                .map(|f| &src[f.range.clone()])
                .collect();
            assert!(spelled.contains(&"behaviour"), "{engine:?}: {spelled:?}");
        }
    }
}

#[cfg(test)]
mod accept_dialect_tests {
    use super::*;
    use crate::config::Engine;
    use crate::rules::Analyzed;
    use crate::source::{FileKind, SourceFile};

    fn run(src: &str, config: &Config) -> Vec<Finding> {
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

    /// The accept list wins over a dictionary entry whose dialect metadata flags the word
    /// (`distilled`, `cancelled` under American), in every engine.
    #[test]
    fn accept_overrides_dialect_entry() {
        let src = "The queue distilled data. Distilled output.\n";
        let mut engines = vec![Engine::Spellbook];
        if cfg!(feature = "harper") {
            engines.extend([Engine::Hybrid, Engine::Curated, Engine::Harper]);
        }
        for engine in engines {
            let mut config = Config::default();
            config.prose.engine = engine;
            let mut accepting = config.clone();
            accepting.prose.accept.push("distilled".into());
            let f = run(src, &accepting);
            assert!(f.iter().all(|f| f.rule != "spelling"), "{engine:?}: {f:?}");
            let _ = run(src, &config);
        }
    }
    fn vocab_config(engine: Engine) -> Config {
        let mut c: Config = toml::from_str(
            r#"
[[vocab]]
term = "Zorbax"
description = "Device register"
[[vocab]]
term = "FiQMEA"
description = "Agency"
case_sensitive = true
[[vocab]]
term = "Kvarn Hubb"
description = "Data hub"
[[entity]]
name = "Telia Oy"
kind = "company"
relationship = "Pharmacy partner"
aliases = ["Qelvio"]
"#,
        )
        .unwrap();
        c.prose.engine = engine;
        c
    }

    fn misspelled(src: &str, config: &Config) -> Vec<String> {
        run(src, config)
            .iter()
            .filter(|f| f.rule == "spelling")
            .map(|f| src[f.range.clone()].to_string())
            .collect()
    }

    /// `[[vocab]]` terms are accepted words; `[[entity]]` names only as whole phrases.
    #[test]
    fn vocab_and_entity_phrases() {
        let mut engines = vec![Engine::Spellbook];
        if cfg!(feature = "harper") {
            engines.extend([Engine::Hybrid, Engine::Harper]);
        }
        for engine in engines {
            let c = vocab_config(engine);
            let src = "Zorbax and zorbax-based checks. Telia Oy ships; telia oy and TELIA OY too. \
                       Qelvio's app. FiQMEA and FIQMEA agree; the Kvarn Hubb and kvarn hubb.\n";
            assert!(
                misspelled(src, &c).is_empty(),
                "{engine:?}: {:?}",
                misspelled(src, &c)
            );
            // Words of an entity name alone, or of a multi-word term alone, stay unknown.
            let src = "Telia said Oy. The Kvarn system.\n";
            assert_eq!(misspelled(src, &c), ["Telia", "Oy", "Kvarn"], "{engine:?}");
        }
        // A case-sensitive term in another casing: reported by prose/entity-name, not spelling;
        // with that rule off, by spelling.
        let mut c = vocab_config(Engine::Spellbook);
        assert!(misspelled("The fiqmea list.\n", &c).is_empty());
        c.rules
            .insert("prose/entity-name".into(), crate::config::Level::Off);
        assert_eq!(misspelled("The fiqmea list.\n", &c), ["fiqmea"]);
    }
}
