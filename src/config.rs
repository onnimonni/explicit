//! `explicit.toml` configuration.

use std::collections::BTreeMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

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
    link_excludes: OnceLock<(u64, Arc<Vec<regex::Regex>>)>,
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
            cache: ConfigCache::default(),
        }
    }
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
}

impl Default for General {
    fn default() -> Self {
        General {
            exclude: Vec::new(),
            include: Vec::new(),
            respect_gitignore: true,
            fail_on: Severity::Warning,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Prose {
    /// american, british, canadian, australian, indian
    pub dialect: String,
    /// Words accepted by the spell checker.
    pub accept: Vec<String>,
    /// Files with one accepted word per line.
    pub vocab_files: Vec<PathBuf>,
    /// Harper rules to disable everywhere (by harper rule name).
    pub disable: Vec<String>,
    /// `prose/sentence-length`: maximum words per sentence.
    pub max_sentence_words: usize,
    /// `prose/readability`: maximum Flesch-Kincaid grade per section.
    pub max_grade: f64,
}

impl Default for Prose {
    fn default() -> Self {
        Prose {
            dialect: "american".into(),
            accept: Vec::new(),
            vocab_files: Vec::new(),
            disable: vec!["ExpandConfiguration".into(), "OxfordComma".into()],
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
    pub cache_ttl_hours: u64,
    pub cache_failed_ttl_hours: u64,
    /// Where root-relative links (`/docs/x.md`) resolve. Defaults to project root.
    pub root_dir: Option<PathBuf>,
    /// Allow requests (and redirects) to localhost, private, link-local and unique-local addresses.
    pub allow_private: bool,
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
            cache_ttl_hours: 24,
            cache_failed_ttl_hours: 1,
            root_dir: None,
            allow_private: false,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Slop {
    /// Extra phrase catalogues (same schema as the built-in one).
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
        let mut dir = if start.is_file() {
            start.parent().unwrap_or(&start).to_path_buf()
        } else {
            start.clone()
        };
        loop {
            let candidate = dir.join(CONFIG_FILE);
            if candidate.is_file() {
                return Config::load(&candidate);
            }
            if dir.join(".git").exists() || !dir.pop() {
                break;
            }
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
        cfg.root = path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        cfg.validate()?;
        Ok(cfg)
    }

    fn validate(&self) -> Result<(), String> {
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
        Ok(())
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

    /// Accepted words from config and vocab files; read once per config.
    pub fn accepted_words(&self) -> Arc<Vec<String>> {
        let key = fingerprint(&(&self.root, &self.prose.accept, &self.prose.vocab_files));
        cached(&self.cache.accepted, key, || self.read_accepted_words())
    }

    fn read_accepted_words(&self) -> Vec<String> {
        let mut words = self.prose.accept.clone();
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
