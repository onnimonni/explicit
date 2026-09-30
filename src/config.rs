//! `explicit.toml` configuration.

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use serde::Deserialize;

use crate::diagnostic::Severity;

pub const CONFIG_FILE: &str = "explicit.toml";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Off,
    Info,
    Warn,
    #[serde(alias = "warning")]
    Warning,
    Error,
}

impl Level {
    pub fn severity(self) -> Option<Severity> {
        match self {
            Level::Off => None,
            Level::Info => Some(Severity::Info),
            Level::Warn | Level::Warning => Some(Severity::Warning),
            Level::Error => Some(Severity::Error),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    #[serde(skip)]
    pub root: PathBuf,
    pub general: General,
    /// Rule id or glob (`slop/*`) -> level.
    pub rules: BTreeMap<String, Level>,
    pub prose: Prose,
    pub comments: Comments,
    pub markdown: Markdown,
    pub links: Links,
    pub slop: Slop,
    pub docs: Docs,
    /// Vale-style rules.
    #[serde(rename = "style")]
    pub style: Vec<StyleRule>,
    /// Domain vocabulary (`[[vocab]]`): accepted words, each with what it means.
    pub vocab: Vec<Vocab>,
    /// Organizations, people and products (`[[entity]]`): names accepted as whole phrases.
    #[serde(rename = "entity")]
    pub entities: Vec<Entity>,
    /// Collaborators (`[[person]]`): names accepted with their parts and inflections, and
    /// checked for ambiguity (`prose/ambiguous-person`).
    #[serde(rename = "person")]
    pub persons: Vec<Person>,
    /// `[people]`: test paths and placeholder names.
    pub people: People,
    /// Per-path settings (`[[overrides]]`), applied in order.
    pub overrides: Vec<Override>,
    /// Per-language settings (`[languages.fi]`), keyed by primary language subtag.
    pub languages: BTreeMap<String, LanguageConfig>,
    /// Derived values computed on first use.
    /// Public only so `Config { .., ..Default::default() }` works; the contents are opaque.
    #[serde(skip)]
    #[doc(hidden)]
    pub cache: ConfigCache,
}

/// Lazily derived, read-only data shared by all files checked with one config.
#[derive(Debug, Clone, Default)]
pub struct ConfigCache {
    /// (fingerprint of the inputs, value): a config mutated after first use is recomputed.
    accepted: OnceLock<(u64, Arc<Vec<String>>)>,
    phrases: OnceLock<(u64, Arc<crate::vocab::Phrases>)>,
    people: OnceLock<(u64, Arc<crate::vocab::PersonIndex>)>,
    test_paths: OnceLock<(u64, Arc<ignore::gitignore::Gitignore>)>,
    link_excludes: OnceLock<(u64, Arc<Vec<regex::Regex>>)>,
    overrides: OverrideCache,
}

/// Compiled `[[overrides]]` globs and effective configs keyed by the matched override indices.
#[derive(Debug, Default)]
struct OverrideCache {
    matchers: OnceLock<Vec<ignore::gitignore::Gitignore>>,
    effective: Mutex<HashMap<Vec<usize>, Arc<Config>>>,
    /// `prose.dialect = "auto"` resolved to British, keyed by the address of the config it
    /// came from (0 for this one).
    british: Mutex<HashMap<usize, Arc<Config>>>,
}

impl Clone for OverrideCache {
    /// A clone may be mutated, so it starts empty.
    fn clone(&self) -> Self {
        OverrideCache::default()
    }
}

/// Settings for files matching `paths`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Override {
    /// Gitignore-style globs relative to the project root.
    pub paths: Vec<String>,
    /// Replaces `prose.dialect`.
    pub dialect: Option<String>,
    /// Replaces `general.language` (a BCP 47 tag: `fi`, `sv-FI`, `en-GB`).
    pub language: Option<String>,
    /// Merged over `[rules]`.
    pub rules: BTreeMap<String, Level>,
    /// Appended to `prose.accept`.
    pub accept: Vec<String>,
    /// Appended to `[[vocab]]`.
    pub vocab: Vec<Vocab>,
    /// Appended to `[[entity]]`.
    #[serde(rename = "entity")]
    pub entities: Vec<Entity>,
    /// Appended to `[[person]]`.
    #[serde(rename = "person")]
    pub persons: Vec<Person>,
}

/// A `[[vocab]]` term: accepted by the spell check like `prose.accept`, and documented.
#[derive(Debug, Clone, Default, Hash, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Vocab {
    pub term: String,
    /// What the term means (required).
    #[serde(default)]
    pub description: String,
    /// Accept only this exact casing (and ALL CAPS); other casings get `prose/entity-name`.
    #[serde(default)]
    pub case_sensitive: bool,
    /// Other accepted spellings.
    #[serde(default)]
    pub aliases: Vec<String>,
    /// Language the term belongs to (`fi`); with `langs`, unset means every language.
    #[serde(default)]
    pub lang: Option<String>,
    /// Languages the term belongs to (`["fi", "sv"]`).
    #[serde(default)]
    pub langs: Vec<String>,
}

impl Vocab {
    /// The term is accepted in text of language `code` (a primary subtag).
    pub fn applies_to(&self, code: &str) -> bool {
        applies_to(self.lang.as_ref(), &self.langs, code)
    }
}

/// An `[[entity]]`: an organization, person or product. Its name is accepted only as the whole
/// phrase (`Oy` inside `Telia Oy`), aliases as standalone words.
#[derive(Debug, Clone, Default, Hash, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Entity {
    pub name: String,
    /// `company`, `authority`, `person`, `product`, ...
    #[serde(default)]
    pub kind: String,
    /// How the project relates to it (required).
    #[serde(default)]
    pub relationship: String,
    /// Short forms, accepted as words.
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub url: Option<String>,
    /// Language the name belongs to (`fi`); with `langs`, unset means every language.
    #[serde(default)]
    pub lang: Option<String>,
    /// Languages the name belongs to (`["fi", "sv"]`).
    #[serde(default)]
    pub langs: Vec<String>,
}

impl Entity {
    /// The name is accepted in text of language `code` (a primary subtag).
    pub fn applies_to(&self, code: &str) -> bool {
        applies_to(self.lang.as_ref(), &self.langs, code)
    }
}

/// A `[[person]]`: a collaborator. The full name, each name part, aliases and handles are
/// accepted (with possessive and Finnish / Swedish inflections); a name part two persons share
/// is reported when used alone without context (`prose/ambiguous-person`).
#[derive(Debug, Clone, Default, Hash, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Person {
    /// Full name.
    pub name: String,
    /// Who they are or how they relate to the project (required).
    #[serde(default)]
    pub role: String,
    /// Other names (`Sami V.`), accepted as phrases.
    #[serde(default)]
    pub aliases: Vec<String>,
    /// Handles (`@samiv`).
    #[serde(default)]
    pub handles: Vec<String>,
    /// Language the name belongs to (`fi`); with `langs`, unset means every language.
    #[serde(default)]
    pub lang: Option<String>,
    /// Languages the name belongs to (`["fi", "sv"]`).
    #[serde(default)]
    pub langs: Vec<String>,
}

impl Person {
    /// The name is accepted in text of language `code` (a primary subtag).
    pub fn applies_to(&self, code: &str) -> bool {
        applies_to(self.lang.as_ref(), &self.langs, code)
    }
}

/// `[people]`: where made-up names are fine, and built-in placeholder names.
#[derive(Debug, Clone, Hash, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct People {
    /// Gitignore-style globs of test files: unknown capitalized names pass the spell check
    /// there and `prose/ambiguous-person` is off.
    pub test_paths: Vec<String>,
    /// Accept well-known placeholder names (`Alice`, `John Doe`, `Matti Meikäläinen`)
    /// everywhere.
    pub placeholders: bool,
}

impl Default for People {
    fn default() -> Self {
        People {
            test_paths: [
                "tests/**",
                "test/**",
                "spec/**",
                "**/__tests__/**",
                "**/fixtures/**",
                "**/testdata/**",
                "**/*_test.*",
                "**/*.test.*",
                "**/*.spec.*",
                "**/test_*.*",
            ]
            .map(String::from)
            .to_vec(),
            placeholders: true,
        }
    }
}

/// `lang` / `langs` of a vocab entry include `code`; neither set means all languages.
fn applies_to(lang: Option<&String>, langs: &[String], code: &str) -> bool {
    let mut all = lang.into_iter().chain(langs).peekable();
    all.peek().is_none() || all.any(|l| crate::rules::spell_lang::primary(l) == code)
}

/// Whether a language code (`en`, `en-GB`, `fi`, `none`) means English. Empty counts as English.
pub fn is_english(lang: &str) -> bool {
    let l = lang.trim().to_ascii_lowercase();
    l.is_empty() || l == "en" || l.starts_with("en-") || l.starts_with("en_") || l == "english"
}

fn fingerprint(v: &impl Hash) -> u64 {
    let mut h = DefaultHasher::new();
    v.hash(&mut h);
    h.finish()
}

/// Cached value when `key` matches the one it was built from, else a fresh uncached value.
fn cached<T>(cell: &OnceLock<(u64, Arc<T>)>, key: u64, build: impl Fn() -> T) -> Arc<T> {
    let (k, v) = cell.get_or_init(|| (key, Arc::new(build())));
    if *k == key {
        v.clone()
    } else {
        Arc::new(build())
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            root: PathBuf::from("."),
            general: General::default(),
            rules: BTreeMap::new(),
            prose: Prose::default(),
            comments: Comments::default(),
            markdown: Markdown::default(),
            links: Links::default(),
            slop: Slop::default(),
            docs: Docs::default(),
            style: Vec::new(),
            vocab: Vec::new(),
            entities: Vec::new(),
            persons: Vec::new(),
            people: People::default(),
            overrides: Vec::new(),
            languages: BTreeMap::new(),
            cache: ConfigCache::default(),
        }
    }
}

/// `[languages.<code>]`: the spelling dictionary and extra words for one prose language.
#[derive(Debug, Clone, Default, Hash, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LanguageConfig {
    /// Finnish: a Voikko `mor.vfst` or a directory holding one (`5/mor-standard/mor.vfst`).
    /// Other languages: a Hunspell `.aff` file (its `.dic` beside it) or a directory with
    /// `index.aff` / `index.dic`. Relative to the root; replaces the bundled dictionary.
    pub dictionary_path: Option<PathBuf>,
    /// Words accepted only in text of this language.
    pub accept: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct General {
    /// Extra glob patterns to exclude (gitignore syntax).
    pub exclude: Vec<String>,
    /// Only check these globs, if set.
    pub include: Vec<String>,
    pub respect_gitignore: bool,
    /// Minimum severity that makes `check` exit with 1.
    pub fail_on: Severity,
    /// Prose language, a BCP 47 tag (`en`, `fi`, `sv-FI`, `none`, ...). Finnish and Swedish
    /// files get Finnish / Swedish spelling; other non-English files only structure, link and
    /// style rules.
    pub language: String,
    /// Detect the language of files and of stretches inside them (a Finnish table cell in an
    /// English file): Finnish and Swedish get their own spelling, other languages are skipped.
    pub detect_language: bool,
    /// Reuse results for unchanged files (`check` only).
    pub cache: bool,
    /// Cache directory, relative to the root (default `.explicit_cache`).
    pub cache_dir: Option<PathBuf>,
    /// `language` was set by a matching `[[overrides]]` entry.
    #[serde(skip)]
    pub language_from_override: bool,
}

impl Default for General {
    fn default() -> Self {
        General {
            exclude: Vec::new(),
            include: Vec::new(),
            respect_gitignore: true,
            fail_on: Severity::Warning,
            language: "en".into(),
            detect_language: true,
            cache: true,
            cache_dir: None,
            language_from_override: false,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Prose {
    /// auto (default), american, british, canadian, australian, indian. `auto` checks a file
    /// as British when British spellings clearly dominate it, else as American.
    pub dialect: String,
    /// Words accepted by the spell checker, matched case-insensitively. Hyphenated entries
    /// (`pre-commit`, `Wi-Fi`) are accepted as whole tokens.
    pub accept: Vec<String>,
    /// Regexes for accepted words, matched against whole words (`^(?:pattern)$`).
    pub accept_patterns: Vec<String>,
    /// Files with one accepted word per line.
    pub vocab_files: Vec<PathBuf>,
    /// Harper rules to disable everywhere (by harper rule name).
    pub disable: Vec<String>,
    /// Default severity of `grammar/*` findings.
    pub grammar_level: GrammarLevel,
    /// Spelling/grammar backend (`spellbook` is the default).
    pub engine: Engine,
    /// `prose/sentence-length`: maximum words per sentence.
    pub max_sentence_words: usize,
    /// `prose/readability`: maximum Flesch-Kincaid grade per section.
    pub max_grade: f64,
}

/// How `grammar/*` findings get their default severity (`[rules]` still overrides it).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GrammarLevel {
    /// `info`, except a hand-picked set of high-confidence rules that stay `warning`.
    #[default]
    Info,
    /// Every grammar rule is a `warning`.
    Warning,
    /// Severity follows Harper's lint kind (typos error, style info, the rest warning).
    Harper,
}

/// Spelling and grammar backend. `spellbook` is the default; the others need the `harper`
/// cargo feature.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Engine {
    /// Harper with all its enabled rules; spelling on Harper's dictionary.
    Harper,
    /// Harper limited to an allowlist of high-precision rules.
    Curated,
    /// No Harper: Hunspell spelling (spellbook) and our own pattern rules.
    #[default]
    Spellbook,
    /// `spellbook`, plus Harper's part-of-speech rules on full sentences only.
    Hybrid,
}

impl Engine {
    /// Spelling through Hunspell dictionaries instead of Harper's.
    pub fn hunspell(self) -> bool {
        matches!(self, Engine::Spellbook | Engine::Hybrid)
    }

    /// Runs Harper, so needs a build with the `harper` feature.
    pub fn needs_harper(self) -> bool {
        self != Engine::Spellbook
    }

    /// An error for an engine this build lacks.
    pub fn check_available(self) -> Result<(), String> {
        if self.needs_harper() && !cfg!(feature = "harper") {
            let name = format!("{self:?}").to_lowercase();
            return Err(format!(
                "prose.engine = \"{name}\" needs Harper, but explicit was built without it \
                 (cargo feature `harper`); use engine = \"spellbook\""
            ));
        }
        Ok(())
    }
}

impl Default for Prose {
    fn default() -> Self {
        Prose {
            dialect: "auto".into(),
            accept: Vec::new(),
            accept_patterns: Vec::new(),
            vocab_files: Vec::new(),
            disable: vec!["ExpandConfiguration".into(), "OxfordComma".into()],
            grammar_level: GrammarLevel::Info,
            engine: Engine::default(),
            max_sentence_words: 30,
            max_grade: 12.0,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Comments {
    pub enabled: bool,
    /// Only check doc comments for grammar/spelling. Slop rules see all comments.
    pub doc_only_grammar: bool,
    /// Harper rules disabled in comments (fragments are common there).
    pub disable: Vec<String>,
}

impl Default for Comments {
    fn default() -> Self {
        Comments {
            enabled: true,
            doc_only_grammar: false,
            disable: vec![
                "SentenceCapitalization".into(),
                "Spaces".into(),
                "LongSentences".into(),
                "UnclosedQuotes".into(),
                "NoFrenchSpaces".into(),
                "SplitWords".into(),
                "Dashes".into(),
                "UseEllipsisCharacter".into(),
                "DisjointPrefixes".into(),
                "CapitalizePersonalPronouns".into(),
                "ExpandConfiguration".into(),
                // Doc comments drop the subject ("Maps X to Y") and name code ("pulldown-cmark").
                "MissingTo".into(),
                "PhrasalVerbAsCompoundNoun".into(),
                "OrthographicConsistency".into(),
                "ExpandDirectory".into(),
            ],
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Markdown {
    pub line_length: usize,
    /// `consistent`, `dash`, `asterisk`, `plus`
    pub list_marker: String,
    /// `consistent`, `atx`, `setext`
    pub heading_style: String,
    /// Heading case: `off`, `sentence`, `title`
    pub heading_case: String,
    /// Extra fenced code block languages accepted by `md/code-language-known`.
    pub allowed_languages: Vec<String>,
    /// `consistent` or the exact thematic break text, e.g. `---`.
    pub hr_style: String,
    /// `consistent`, `backtick`, `tilde`
    pub code_fence_style: String,
    /// `consistent`, `asterisk`, `underscore`
    pub emphasis_style: String,
    /// `consistent`, `asterisk`, `underscore`
    pub strong_style: String,
    /// Link texts flagged by `md/descriptive-link-text` (case-insensitive).
    pub non_descriptive_link_text: Vec<String>,
    /// Maximum heading length in characters for `md/max-heading-length`.
    pub max_heading_length: usize,
    /// HTML elements allowed by `md/no-inline-html`.
    pub allowed_html: Vec<String>,
    /// Front matter keys required by `md/front-matter-required`.
    pub front_matter_required: Vec<String>,
}

impl Default for Markdown {
    fn default() -> Self {
        Markdown {
            line_length: 120,
            list_marker: "consistent".into(),
            heading_style: "consistent".into(),
            heading_case: "off".into(),
            allowed_languages: Vec::new(),
            hr_style: "consistent".into(),
            code_fence_style: "consistent".into(),
            emphasis_style: "consistent".into(),
            strong_style: "consistent".into(),
            non_descriptive_link_text: [
                "click here",
                "here",
                "link",
                "more",
                "read more",
                "this",
                "this link",
            ]
            .map(String::from)
            .to_vec(),
            max_heading_length: 60,
            allowed_html: [
                "br", "details", "summary", "img", "sup", "sub", "kbd", "a", "picture", "source",
                "video",
            ]
            .map(String::from)
            .to_vec(),
            front_matter_required: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Links {
    /// Check http(s) links over the network.
    pub remote: bool,
    /// Never touch the network; use cache only.
    pub offline: bool,
    pub timeout_secs: u64,
    pub concurrency: usize,
    /// Regexes of URLs to skip.
    pub exclude: Vec<String>,
    /// HTTP status codes counted as OK besides 2xx.
    pub accept_status: Vec<u16>,
    /// Keep remote results in `<cache_dir>/links.json` (independent of `general.cache`).
    pub cache: bool,
    /// How long OK results (links and images) are reused.
    pub cache_ttl_hours: u64,
    /// How long failures are reused; short so fixes show up quickly.
    pub cache_failed_ttl_hours: u64,
    /// Where root-relative links (`/docs/x.md`) resolve. Defaults to project root.
    pub root_dir: Option<PathBuf>,
    /// Allow requests (and redirects) to localhost, private, link-local and unique-local addresses.
    pub allow_private: bool,
    /// This repository on GitHub (`owner/repo`); default: parsed from the git remote.
    pub github_repo: Option<String>,
    /// Default branch for `links/same-repo-url`; default: `origin/HEAD`, else main/master/HEAD.
    pub github_default_branch: Option<String>,
    /// Check links into this repository with local git and gh instead of anonymous HTTP.
    pub check_same_repo: bool,
}

impl Default for Links {
    fn default() -> Self {
        Links {
            remote: true,
            offline: false,
            timeout_secs: 10,
            concurrency: 16,
            exclude: vec![
                r"^https?://(localhost|127\.0\.0\.1|0\.0\.0\.0|\[::1\])([:/]|$)".into(),
                r"^https?://example\.(com|org|net)".into(),
            ],
            accept_status: vec![429],
            cache: true,
            cache_ttl_hours: 168,
            cache_failed_ttl_hours: 1,
            root_dir: None,
            allow_private: false,
            github_repo: None,
            github_default_branch: None,
            check_same_repo: true,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Slop {
    /// Extra phrase catalogs (same schema as the built-in one).
    pub extra: Vec<PathBuf>,
    /// Built-in entry ids to disable.
    pub disable: Vec<String>,
    /// Weighted slop hits per 100 words that trigger `slop/density`.
    pub density_per_100: f64,
    /// Minimum words before density is scored.
    pub density_min_words: usize,
    /// Em dashes per 100 words that trigger `slop/em-dash`.
    pub em_dash_per_100: f64,
}

impl Default for Slop {
    fn default() -> Self {
        Slop {
            extra: Vec::new(),
            disable: Vec::new(),
            density_per_100: 1.5,
            density_min_words: 100,
            em_dash_per_100: 2.0,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Docs {
    /// `docs/orphan-page`: directory whose pages must be linked from somewhere.
    pub root: PathBuf,
    /// `docs/orphan-page`: globs (relative to the project root, or file names) never reported.
    pub orphan_exempt: Vec<String>,
    /// `docs/include-missing`: extra directories include paths resolve against.
    pub include_dirs: Vec<PathBuf>,
}

impl Default for Docs {
    fn default() -> Self {
        Docs {
            root: PathBuf::from("docs"),
            orphan_exempt: Vec::new(),
            include_dirs: vec![PathBuf::from("_includes")],
        }
    }
}

/// A Vale-style rule.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StyleRule {
    /// Rule id; reported as `style/<name>`.
    pub name: String,
    /// existence | substitution | repetition | occurrence | capitalization
    pub kind: StyleKind,
    pub message: Option<String>,
    #[serde(default = "default_warn")]
    pub level: Level,
    /// existence: words/phrases (or regexes when `regex = true`).
    #[serde(default)]
    pub tokens: Vec<String>,
    /// substitution: bad -> good.
    #[serde(default)]
    pub swap: BTreeMap<String, String>,
    #[serde(default)]
    pub regex: bool,
    #[serde(default)]
    pub ignore_case: Option<bool>,
    /// occurrence: max matches per segment.
    #[serde(default)]
    pub max: Option<usize>,
    /// capitalization: `sentence` or `title`, applied to headings.
    #[serde(default)]
    pub case: Option<String>,
    /// capitalization: words exempt from casing.
    #[serde(default)]
    pub exceptions: Vec<String>,
    /// Limit to `markdown`, `comments`, or both (default).
    #[serde(default)]
    pub scope: Option<String>,
}

fn default_warn() -> Level {
    Level::Warn
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StyleKind {
    Existence,
    Substitution,
    Repetition,
    Occurrence,
    Capitalization,
}

impl Config {
    /// Find `explicit.toml` walking up from `start`; fall back to defaults rooted at `start`.
    pub fn discover(start: &Path) -> Result<Config, String> {
        let start = start
            .canonicalize()
            .map_err(|e| format!("{}: {e}", start.display()))?;
        if let Some(candidate) = find_config(&start) {
            return Config::load(&candidate);
        }
        let root = if start.is_file() {
            start.parent().unwrap_or(&start).to_path_buf()
        } else {
            start
        };
        let root = find_git_root(&root).unwrap_or(root);
        Ok(Config {
            root,
            ..Config::default()
        })
    }

    pub fn load(path: &Path) -> Result<Config, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let mut cfg: Config =
            toml::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        // Absolute root, so paths relative to it can be re-expressed relative to the cwd.
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        cfg.root = parent
            .canonicalize()
            .unwrap_or_else(|_| parent.to_path_buf());
        cfg.validate()?;
        Ok(cfg)
    }

    fn validate(&self) -> Result<(), String> {
        self.prose.engine.check_available()?;
        for p in &self.prose.accept_patterns {
            regex::Regex::new(p)
                .map_err(|e| format!("prose.accept_patterns: bad regex {p:?}: {e}"))?;
        }
        for r in &self.style {
            if r.regex {
                for t in r.tokens.iter().chain(r.swap.keys()) {
                    regex::Regex::new(t)
                        .map_err(|e| format!("style rule {}: bad regex {t:?}: {e}", r.name))?;
                }
            }
        }
        for p in &self.links.exclude {
            regex::Regex::new(p).map_err(|e| format!("links.exclude: bad regex {p:?}: {e}"))?;
        }
        validate_vocab(&self.vocab, &self.entities, "")?;
        validate_persons(&self.persons, "")?;
        build_matcher(&self.root, &self.people.test_paths)
            .map_err(|e| format!("people.test_paths: {e}"))?;
        for (i, o) in self.overrides.iter().enumerate() {
            if o.paths.is_empty() {
                return Err(format!("overrides[{i}]: `paths` is empty"));
            }
            build_matcher(&self.root, &o.paths).map_err(|e| format!("overrides[{i}]: {e}"))?;
            validate_vocab(&o.vocab, &o.entities, &format!("overrides[{i}]: "))?;
            validate_persons(&o.persons, &format!("overrides[{i}]: "))?;
        }
        Ok(())
    }

    /// Config for the file at `rel` (relative to the root) with matching `[[overrides]]` applied;
    /// `None` when no override matches. Built once per distinct set of matching overrides.
    pub fn for_path(&self, rel: &Path) -> Option<Arc<Config>> {
        if self.overrides.is_empty() || rel.has_root() {
            return None;
        }
        let matchers = self.cache.overrides.matchers.get_or_init(|| {
            self.overrides
                .iter()
                .map(|o| {
                    build_matcher(&self.root, &o.paths)
                        .unwrap_or_else(|_| ignore::gitignore::Gitignore::empty())
                })
                .collect()
        });
        let hits: Vec<usize> = matchers
            .iter()
            .enumerate()
            .filter(|(_, m)| m.matched_path_or_any_parents(rel, false).is_ignore())
            .map(|(i, _)| i)
            .collect();
        if hits.is_empty() {
            return None;
        }
        let mut map = self
            .cache
            .overrides
            .effective
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        Some(
            map.entry(hits)
                .or_insert_with_key(|hits| Arc::new(self.with_overrides(hits)))
                .clone(),
        )
    }

    /// [`Config::for_path`], with `prose.dialect = "auto"` resolved for the file's prose
    /// `texts`: British when British spellings dominate it
    /// ([`crate::rules::spell::british_dominates`]). The resolved dialect is part of the
    /// returned config, so of the results cache key too.
    pub fn for_file<'a>(
        &self,
        rel: &Path,
        texts: impl IntoIterator<Item = &'a str>,
    ) -> Option<Arc<Config>> {
        let effective = self.for_path(rel);
        let c: &Config = effective.as_deref().unwrap_or(self);
        if !c.prose.dialect.eq_ignore_ascii_case("auto")
            || !crate::rules::spell::british_dominates(texts)
        {
            return effective;
        }
        let id = effective.as_ref().map_or(0, |e| Arc::as_ptr(e) as usize);
        let mut map = self
            .cache
            .overrides
            .british
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        Some(
            map.entry(id)
                .or_insert_with(|| {
                    let mut b = c.clone();
                    b.prose.dialect = "british".into();
                    Arc::new(b)
                })
                .clone(),
        )
    }

    fn with_overrides(&self, hits: &[usize]) -> Config {
        let mut c = self.clone();
        c.overrides = Vec::new();
        c.cache = ConfigCache::default();
        for o in hits.iter().filter_map(|&i| self.overrides.get(i)) {
            if let Some(d) = &o.dialect {
                c.prose.dialect.clone_from(d);
            }
            if let Some(l) = &o.language {
                c.general.language.clone_from(l);
                c.general.language_from_override = true;
            }
            c.rules.extend(o.rules.iter().map(|(k, v)| (k.clone(), *v)));
            c.prose.accept.extend(o.accept.iter().cloned());
            c.vocab.extend(o.vocab.iter().cloned());
            c.entities.extend(o.entities.iter().cloned());
            c.persons.extend(o.persons.iter().cloned());
        }
        c
    }

    /// Effective severity of a rule: exact id wins, then the longest matching glob, then the default.
    ///
    /// Only an exact id can enable a rule that is off by default (`default == None`); a glob
    /// re-levels rules that are on by default, and `"family/*" = "off"` turns them all off.
    pub fn severity(&self, rule: &str, default: Option<Severity>) -> Option<Severity> {
        if let Some(l) = self.rules.get(rule) {
            return l.severity();
        }
        let mut best: Option<(&str, Level)> = None;
        for (pat, lvl) in &self.rules {
            if glob_match(pat, rule) && best.is_none_or(|(b, _)| pat.len() > b.len()) {
                best = Some((pat, *lvl));
            }
        }
        match best {
            Some((_, l)) => default.and(l.severity()),
            None => default,
        }
    }

    /// Accepted words (matched case-insensitively) in English text from `prose.accept`, vocab
    /// files, `[[vocab]]` terms and `[[entity]]` aliases (those for all languages or English);
    /// read once per config. Multi-word and case-sensitive entries are
    /// [`Config::vocab_phrases_in`] instead.
    pub fn accepted_words(&self) -> Arc<Vec<String>> {
        let key = fingerprint(&(
            &self.root,
            &self.prose.accept,
            &self.prose.vocab_files,
            &self.vocab,
            &self.entities,
        ));
        cached(&self.cache.accepted, key, || self.read_accepted_words("en"))
    }

    /// [`Config::accepted_words`] for text in language `code` (a primary subtag): vocab
    /// entries for that language and `[languages.<code>] accept`. Not cached; spell checkers
    /// read it once when built.
    pub fn accepted_words_in(&self, code: &str) -> Arc<Vec<String>> {
        if code == "en" && !self.languages.contains_key("en") {
            return self.accepted_words();
        }
        let mut words = self.read_accepted_words(code);
        if let Some(l) = self.languages.get(code) {
            words.extend(l.accept.iter().cloned());
        }
        Arc::new(words)
    }

    /// [`Config::vocab_phrases`] limited to entries for language `code`.
    pub fn vocab_phrases_in(&self, code: &str) -> Arc<crate::vocab::Phrases> {
        let all = |v: &Vocab| v.lang.is_none() && v.langs.is_empty();
        let all_e = |e: &Entity| e.lang.is_none() && e.langs.is_empty();
        if self.vocab.iter().all(all) && self.entities.iter().all(all_e) {
            return self.vocab_phrases();
        }
        let (v, e) = self.vocab_for(code);
        Arc::new(crate::vocab::Phrases::new(&v, &e))
    }

    /// `[[vocab]]` and `[[entity]]` entries for language `code`.
    fn vocab_for(&self, code: &str) -> (Vec<Vocab>, Vec<Entity>) {
        (
            self.vocab
                .iter()
                .filter(|v| v.applies_to(code))
                .cloned()
                .collect(),
            self.entities
                .iter()
                .filter(|e| e.applies_to(code))
                .cloned()
                .collect(),
        )
    }

    /// `[[entity]]` names and aliases, multi-word and case-sensitive `[[vocab]]` entries of all
    /// languages, as phrases matched in text; built once per config.
    pub fn vocab_phrases(&self) -> Arc<crate::vocab::Phrases> {
        let key = fingerprint(&(&self.vocab, &self.entities));
        cached(&self.cache.phrases, key, || {
            crate::vocab::Phrases::new(&self.vocab, &self.entities)
        })
    }

    /// `[[person]]` names and `[people] placeholders` for text in language `code` (`en`,
    /// `fi`, `sv`; `*` for every entry); built once per config for all languages.
    pub fn person_index(&self, code: &str) -> Arc<crate::vocab::PersonIndex> {
        if code != "*" && self.persons.iter().any(|p| !p.applies_to(code)) {
            let persons: Vec<Person> = self
                .persons
                .iter()
                .filter(|p| p.applies_to(code))
                .cloned()
                .collect();
            return Arc::new(crate::vocab::PersonIndex::new(
                &persons,
                self.people.placeholders,
            ));
        }
        let key = fingerprint(&(&self.persons, self.people.placeholders));
        cached(&self.cache.people, key, || {
            crate::vocab::PersonIndex::new(&self.persons, self.people.placeholders)
        })
    }

    /// The file at `rel` (relative to the root) matches `[people] test_paths`.
    pub fn is_test_path(&self, rel: &Path) -> bool {
        if self.people.test_paths.is_empty() || rel.has_root() {
            return false;
        }
        let key = fingerprint(&(&self.root, &self.people.test_paths));
        let m = cached(&self.cache.test_paths, key, || {
            build_matcher(&self.root, &self.people.test_paths)
                .unwrap_or_else(|_| ignore::gitignore::Gitignore::empty())
        });
        m.matched_path_or_any_parents(rel, false).is_ignore()
    }

    fn read_accepted_words(&self, code: &str) -> Vec<String> {
        let mut words = self.prose.accept.clone();
        let (v, e) = self.vocab_for(code);
        words.extend(crate::vocab::accepted_words(&v, &e));
        for f in &self.prose.vocab_files {
            let p = if f.is_absolute() {
                f.clone()
            } else {
                self.root.join(f)
            };
            if let Ok(text) = std::fs::read_to_string(&p) {
                words.extend(
                    text.lines()
                        .map(str::trim)
                        .filter(|l| !l.is_empty() && !l.starts_with('#'))
                        .map(String::from),
                );
            }
        }
        words
    }

    /// `links.exclude` regexes, compiled once per config (invalid ones are rejected by `load`).
    pub fn link_excludes(&self) -> Arc<Vec<regex::Regex>> {
        cached(
            &self.cache.link_excludes,
            fingerprint(&self.links.exclude),
            || {
                self.links
                    .exclude
                    .iter()
                    .filter_map(|p| regex::Regex::new(p).ok())
                    .collect()
            },
        )
    }
}

/// `[[vocab]]` and `[[entity]]` entries document the project: each needs its explanation.
fn validate_vocab(vocab: &[Vocab], entities: &[Entity], at: &str) -> Result<(), String> {
    for (i, v) in vocab.iter().enumerate() {
        if v.term.trim().is_empty() {
            return Err(format!("{at}vocab[{i}]: `term` is empty"));
        }
        if v.description.trim().is_empty() {
            return Err(format!(
                "{at}vocab[{i}] ({:?}): `description` is required; say what the term means, \
                 so explicit.toml documents the project's vocabulary",
                v.term
            ));
        }
        if v.aliases.iter().any(|a| a.trim().is_empty()) {
            return Err(format!("{at}vocab[{i}] ({:?}): empty alias", v.term));
        }
        if v.lang.iter().chain(&v.langs).any(|l| l.trim().is_empty()) {
            return Err(format!("{at}vocab[{i}] ({:?}): empty language", v.term));
        }
    }
    for (i, e) in entities.iter().enumerate() {
        if e.name.trim().is_empty() {
            return Err(format!("{at}entity[{i}]: `name` is empty"));
        }
        if e.relationship.trim().is_empty() {
            return Err(format!(
                "{at}entity[{i}] ({:?}): `relationship` is required; say how the project \
                 relates to it (\"Pharmacy partner that dispenses our prescriptions\")",
                e.name
            ));
        }
        if e.aliases.iter().any(|a| a.trim().is_empty()) {
            return Err(format!("{at}entity[{i}] ({:?}): empty alias", e.name));
        }
        if e.lang.iter().chain(&e.langs).any(|l| l.trim().is_empty()) {
            return Err(format!("{at}entity[{i}] ({:?}): empty language", e.name));
        }
    }
    Ok(())
}

static PLACEHOLDER_RE: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
    regex::Regex::new(r#"\b(description|relationship|role)\s*=\s*("(?:[^"\\\n]|\\.)*"|'[^'\n]*')"#)
        .expect("hardcoded regex is valid")
});

/// `description`, `relationship` and `role` values in the text of an `explicit.toml` that are
/// placeholders (`"TODO"`, `"TBD"`, `"FIXME"`, `"..."`, empty), as (byte range of the quoted
/// value, key, value): `explicit vocab suggest` output pasted without filling it in.
pub fn placeholder_values(text: &str) -> Vec<(std::ops::Range<usize>, String, String)> {
    let mut out = Vec::new();
    for c in PLACEHOLDER_RE.captures_iter(text) {
        let (Some(key), Some(val)) = (c.get(1), c.get(2)) else {
            continue;
        };
        // Skip commented-out examples: a `#` outside a string earlier on the line.
        let line_start = text[..key.start()].rfind('\n').map_or(0, |i| i + 1);
        let mut quote: Option<char> = None;
        let mut commented = false;
        for ch in text[line_start..key.start()].chars() {
            match (quote, ch) {
                (None, '#') => {
                    commented = true;
                    break;
                }
                (None, '"' | '\'') => quote = Some(ch),
                (Some(q), _) if ch == q => quote = None,
                _ => {}
            }
        }
        if commented || quote.is_some() {
            continue;
        }
        let inner = &val.as_str()[1..val.as_str().len() - 1];
        if is_placeholder(inner) {
            out.push((val.range(), key.as_str().to_string(), inner.to_string()));
        }
    }
    out
}

fn is_placeholder(v: &str) -> bool {
    let v = v.trim();
    if v.chars()
        .all(|c| c.is_whitespace() || matches!(c, '.' | '…' | '-' | '?' | '_'))
    {
        return true;
    }
    // `TODO fill in`, `todo:`, `tbd`; not `Todo list app`.
    let lower = v.to_lowercase();
    ["TODO", "TBD", "FIXME", "XXX"].iter().any(|p| {
        let upper = v.strip_prefix(p);
        let any = lower.strip_prefix(&p.to_lowercase());
        upper.is_some_and(|rest| !rest.starts_with(char::is_alphanumeric))
            || any.is_some_and(|rest| rest.trim().is_empty() || rest.starts_with(':'))
    })
}

/// `[[person]]` entries document collaborators: each needs a role.
fn validate_persons(persons: &[Person], at: &str) -> Result<(), String> {
    for (i, p) in persons.iter().enumerate() {
        if p.name.trim().is_empty() {
            return Err(format!("{at}person[{i}]: `name` is empty"));
        }
        if p.role.trim().is_empty() {
            return Err(format!(
                "{at}person[{i}] ({:?}): `role` is required; say who they are or how they \
                 relate to the project (\"Backend engineer, owns billing\")",
                p.name
            ));
        }
        if p.aliases
            .iter()
            .chain(&p.handles)
            .any(|a| a.trim().is_empty())
        {
            return Err(format!(
                "{at}person[{i}] ({:?}): empty alias or handle",
                p.name
            ));
        }
        if p.lang.iter().chain(&p.langs).any(|l| l.trim().is_empty()) {
            return Err(format!("{at}person[{i}] ({:?}): empty language", p.name));
        }
    }
    Ok(())
}

/// `explicit.toml` that applies to `start` (a file or directory): the nearest one walking up,
/// stopping at the repository root.
pub fn find_config(start: &Path) -> Option<PathBuf> {
    let mut dir = if start.is_file() {
        start.parent().unwrap_or(start).to_path_buf()
    } else {
        start.to_path_buf()
    };
    loop {
        let candidate = dir.join(CONFIG_FILE);
        if candidate.is_file() {
            return Some(candidate);
        }
        if dir.join(".git").exists() || !dir.pop() {
            return None;
        }
    }
}

/// Gitignore-style matcher for `globs`, relative to `root`.
pub fn build_matcher(
    root: &Path,
    globs: &[String],
) -> Result<ignore::gitignore::Gitignore, String> {
    let mut b = ignore::gitignore::GitignoreBuilder::new(root);
    for g in globs {
        b.add_line(None, g)
            .map_err(|e| format!("bad glob {g:?}: {e}"))?;
    }
    b.build().map_err(|e| e.to_string())
}

fn find_git_root(start: &Path) -> Option<PathBuf> {
    start
        .ancestors()
        .find(|d| d.join(".git").exists())
        .map(Path::to_path_buf)
}

/// Minimal glob: `*` matches any run of characters (including `/`).
pub fn glob_match(pat: &str, s: &str) -> bool {
    if !pat.contains('*') {
        return pat == s;
    }
    let parts: Vec<&str> = pat.split('*').collect();
    let mut rest = s;
    for (i, part) in parts.iter().enumerate() {
        if i == 0 {
            match rest.strip_prefix(part) {
                Some(r) => rest = r,
                None => return false,
            }
        } else if i == parts.len() - 1 {
            return rest.ends_with(part);
        } else {
            match rest.find(part) {
                Some(idx) => rest = &rest[idx + part.len()..],
                None => return false,
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn globs() {
        assert!(glob_match("slop/*", "slop/phrase"));
        assert!(glob_match("*", "md/x"));
        assert!(!glob_match("md/*", "slop/x"));
        assert!(glob_match("harper/*Case*", "harper/SentenceCaseX"));
    }

    #[test]
    fn severity_precedence() {
        let mut c = Config::default();
        c.rules.insert("slop/*".into(), Level::Off);
        c.rules.insert("slop/phrase".into(), Level::Error);
        assert_eq!(
            c.severity("slop/phrase", Some(Severity::Info)),
            Some(Severity::Error)
        );
        assert_eq!(c.severity("slop/density", Some(Severity::Info)), None);
        assert_eq!(
            c.severity("md/x", Some(Severity::Info)),
            Some(Severity::Info)
        );
    }

    #[test]
    fn globs_do_not_enable_default_off_rules() {
        let mut c = Config::default();
        c.rules.insert("md/*".into(), Level::Error);
        c.rules.insert("prose/passive".into(), Level::Warn);
        c.rules.insert("prose/*".into(), Level::Info);
        // Default-on rule is re-leveled by the glob.
        assert_eq!(
            c.severity("md/heading-style", Some(Severity::Warning)),
            Some(Severity::Error)
        );
        // Default-off rule stays off under a glob...
        assert_eq!(c.severity("md/line-length", None), None);
        // ...but an exact id enables it.
        assert_eq!(c.severity("prose/passive", None), Some(Severity::Warning));
        c.rules.insert("md/*".into(), Level::Off);
        assert_eq!(
            c.severity("md/heading-style", Some(Severity::Warning)),
            None
        );
    }

    #[test]
    fn auto_dialect_resolves_per_file() {
        const GB: &str = "The colour reflects its behaviour; we organise the catalogue.";
        const US: &str = "The color reflects its behavior; we organize the catalog.";
        let c = Config::default();
        assert_eq!(c.prose.dialect, "auto");
        let gb = c.for_file(Path::new("a.md"), [GB]).unwrap();
        assert_eq!(gb.prose.dialect, "british");
        // Cached, and part of the results cache key.
        assert!(Arc::ptr_eq(
            &gb,
            &c.for_file(Path::new("b.md"), [GB]).unwrap()
        ));
        assert_ne!(crate::cache::config_key(&gb), crate::cache::config_key(&c));
        assert!(c.for_file(Path::new("a.md"), [US]).is_none());
        // An explicit dialect wins.
        for d in ["american", "canadian"] {
            let mut forced = Config::default();
            forced.prose.dialect = d.into();
            assert!(forced.for_file(Path::new("a.md"), [GB]).is_none());
        }
    }

    #[test]
    fn overrides_apply_per_path() {
        let text = r#"
[rules]
"md/line-length" = "warn"
[prose]
accept = ["Base"]
[[overrides]]
paths = ["docs/fi/**"]
language = "fi"
accept = ["Moi"]
rules = { "md/line-length" = "off" }
[[overrides]]
paths = ["docs/**"]
dialect = "british"
"#;
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join(CONFIG_FILE);
        std::fs::write(&p, text).unwrap();
        let c = Config::load(&p).unwrap();
        assert!(c.root.is_absolute());
        assert!(c.for_path(Path::new("README.md")).is_none());
        let fi = c.for_path(Path::new("docs/fi/a.md")).unwrap();
        assert_eq!(fi.general.language, "fi");
        assert_eq!(fi.prose.dialect, "british");
        assert_eq!(fi.prose.accept, ["Base", "Moi"]);
        assert_eq!(fi.severity("md/line-length", None), None);
        // Cached per matched set.
        assert!(Arc::ptr_eq(
            &fi,
            &c.for_path(Path::new("docs/fi/sub/b.md")).unwrap()
        ));
        let en = c.for_path(Path::new("docs/guide.md")).unwrap();
        assert_eq!(en.general.language, "en");
        assert_eq!(en.prose.dialect, "british");
        assert!(!is_english(&fi.general.language));
        assert!(is_english("en-GB") && !is_english("none"));
        std::fs::write(&p, "[[overrides]]\nlanguage = \"fi\"\n").unwrap();
        assert!(Config::load(&p).unwrap_err().contains("paths"));
    }

    #[test]
    fn engines_need_harper_feature() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join(CONFIG_FILE);
        std::fs::write(&p, "[prose]\nengine = \"spellbook\"\n").unwrap();
        assert_eq!(Config::load(&p).unwrap().prose.engine, Engine::Spellbook);
        std::fs::write(&p, "[prose]\nengine = \"curated\"\n").unwrap();
        let curated = Config::load(&p);
        assert_eq!(Config::default().prose.engine, Engine::Spellbook);
        if cfg!(feature = "harper") {
            assert_eq!(curated.unwrap().prose.engine, Engine::Curated);
        } else {
            let e = curated.unwrap_err();
            assert!(
                e.contains("built without") && e.contains("spellbook"),
                "{e}"
            );
        }
    }

    #[test]
    fn vocab_and_entities_need_explanations() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join(CONFIG_FILE);
        let ok = r#"
[[vocab]]
term = "Astori"
description = "Device register"
aliases = ["ASTORI"]
[[vocab]]
term = "FiMEA"
description = "Agency"
case_sensitive = true
[[entity]]
name = "Telia Oy"
kind = "company"
relationship = "Pharmacy partner"
aliases = ["Telia"]
[[overrides]]
paths = ["docs/**"]
vocab = [{ term = "Kalevala", description = "Epic" }]
entity = [{ name = "Kela", relationship = "Pays refunds" }]
"#;
        std::fs::write(&p, ok).unwrap();
        let c = Config::load(&p).unwrap();
        assert_eq!(c.entities[0].aliases, ["Telia"]);
        // Case-sensitive terms and entity names are phrases, not accepted words.
        assert_eq!(*c.accepted_words(), ["Astori", "ASTORI", "Telia"]);
        let docs = c.for_path(Path::new("docs/a.md")).unwrap();
        assert_eq!(docs.vocab.len(), 3);
        assert_eq!(docs.entities.len(), 2);
        assert!(docs.accepted_words().iter().any(|w| w == "Kalevala"));
        // Telia Oy, its alias, the case-sensitive FiMEA and Kela.
        assert_eq!(docs.vocab_phrases().list.len(), 4);
        // Vocab changes the cache key.
        let mut c2 = c.clone();
        c2.vocab[0].description = "Other".into();
        assert_ne!(crate::cache::config_key(&c), crate::cache::config_key(&c2));
        for (bad, want) in [
            (
                "[[vocab]]\nterm = \"X\"\n",
                "vocab[0] (\"X\"): `description` is required",
            ),
            (
                "[[vocab]]\nterm = \"X\"\ndescription = \" \"\n",
                "`description` is required",
            ),
            (
                "[[entity]]\nname = \"Y Oy\"\nkind = \"company\"\n",
                "entity[0] (\"Y Oy\"): `relationship` is required",
            ),
            (
                "[[entity]]\nname = \"\"\nrelationship = \"r\"\n",
                "`name` is empty",
            ),
            (
                "[[overrides]]\npaths = [\"a\"]\nentity = [{ name = \"Z\" }]\n",
                "overrides[0]: entity[0]",
            ),
            (
                "[[vocab]]\nterm = \"X\"\ndescription = \"d\"\nbogus = 1\n",
                "unknown field",
            ),
        ] {
            std::fs::write(&p, bad).unwrap();
            let e = Config::load(&p).unwrap_err();
            assert!(e.contains(want), "{bad}: {e}");
        }
    }

    #[test]
    fn persons_need_roles_and_change_the_cache_key() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join(CONFIG_FILE);
        std::fs::write(
            &p,
            r#"
[[person]]
name = "Sami Virtanen"
role = "Backend engineer, owns billing"
aliases = ["Sami V."]
handles = ["@samiv"]
[people]
test_paths = ["qa/**"]
[[overrides]]
paths = ["docs/**"]
person = [{ name = "Anna Korhonen", role = "Support" }]
"#,
        )
        .unwrap();
        let c = Config::load(&p).unwrap();
        assert!(c.people.placeholders);
        assert!(c.is_test_path(Path::new("qa/x/a.rs")));
        assert!(!c.is_test_path(Path::new("tests/a.rs")));
        let docs = c.for_path(Path::new("docs/a.md")).unwrap();
        assert_eq!(docs.persons.len(), 2);
        let idx = docs.person_index("en");
        assert_eq!(idx.persons_of("Korhonen"), [1]);
        assert_eq!(idx.persons_of("samiv"), [0]);
        // Persons change the cache key.
        let mut c2 = c.clone();
        c2.persons[0].aliases.push("Samppa".into());
        assert_ne!(crate::cache::config_key(&c), crate::cache::config_key(&c2));
        let mut c3 = c.clone();
        c3.people.placeholders = false;
        assert_ne!(crate::cache::config_key(&c), crate::cache::config_key(&c3));
        // Default test paths.
        let d = Config::default();
        for rel in [
            "tests/a.rs",
            "src/__tests__/a.ts",
            "pkg/testdata/x.md",
            "a/fixtures/b.md",
            "src/foo_test.go",
            "src/foo.test.ts",
            "src/foo.spec.js",
            "py/test_foo.py",
        ] {
            assert!(d.is_test_path(Path::new(rel)), "{rel}");
        }
        assert!(!d.is_test_path(Path::new("src/testing.rs")));
        assert!(!d.is_test_path(Path::new("docs/a.md")));
        for (bad, want) in [
            (
                "[[person]]\nname = \"Sami Virtanen\"\n",
                "person[0] (\"Sami Virtanen\"): `role` is required",
            ),
            (
                "[[person]]\nname = \"X\"\nrole = \"  \"\n",
                "`role` is required",
            ),
            (
                "[[person]]\nname = \"\"\nrole = \"r\"\n",
                "person[0]: `name` is empty",
            ),
            (
                "[[person]]\nname = \"X\"\nrole = \"r\"\nhandles = [\"\"]\n",
                "empty alias or handle",
            ),
            (
                "[[overrides]]\npaths = [\"a\"]\nperson = [{ name = \"Z\" }]\n",
                "overrides[0]: person[0] (\"Z\"): `role` is required",
            ),
            ("[people]\nbogus = 1\n", "unknown field"),
        ] {
            std::fs::write(&p, bad).unwrap();
            let e = Config::load(&p).unwrap_err();
            assert!(e.contains(want), "{bad}: {e}");
        }
    }

    #[test]
    fn placeholder_values_in_config_text() {
        let text = "[[vocab]]\nterm = \"A\"\ndescription = \"TODO\"\n\
                    [[vocab]]\nterm = \"B\"\ndescription = \"Todo list app\"\n\
                    entity = [{ name = \"C\", relationship = 'tbd' }]\n\
                    role = \"FIXME: fill in\"  # a comment\n\
                    # role = \"TODO\"\n\
                    message = \"x # role = 'TODO'\"\n\
                    role = \" \"\nrole = \"…\"\nrole = \"Designer\"\n";
        let got: Vec<(&str, String)> = placeholder_values(text)
            .into_iter()
            .map(|(r, k, _)| (&text[r], k))
            .collect();
        assert_eq!(
            got,
            [
                ("\"TODO\"", "description".to_string()),
                ("'tbd'", "relationship".to_string()),
                ("\"FIXME: fill in\"", "role".to_string()),
                ("\" \"", "role".to_string()),
                ("\"…\"", "role".to_string()),
            ]
        );
    }

    #[test]
    fn accepted_words_cached() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("vocab.txt"), "# c\nFoo\n\nBar\n").unwrap();
        let mut c = Config {
            root: dir.path().to_path_buf(),
            ..Config::default()
        };
        c.prose.accept.push("Baz".into());
        c.prose.vocab_files.push("vocab.txt".into());
        assert_eq!(*c.accepted_words(), ["Baz", "Foo", "Bar"]);
        std::fs::remove_file(dir.path().join("vocab.txt")).unwrap();
        // Read once: removing the file afterwards changes nothing.
        assert_eq!(*c.accepted_words(), ["Baz", "Foo", "Bar"]);
        assert!(Arc::ptr_eq(&c.accepted_words(), &c.accepted_words()));
        // Mutating the config after first use is still honored.
        c.prose.accept.push("Qux".into());
        assert_eq!(*c.accepted_words(), ["Baz", "Qux"]);
    }
}
