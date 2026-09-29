//! Phrase catalog: built-in `phrases.toml` plus user catalogs from `[slop] extra`.

use std::collections::HashMap;
use std::ops::Range;
use std::path::PathBuf;
use std::sync::{Arc, LazyLock, Mutex};

use aho_corasick::{AhoCorasick, AhoCorasickBuilder, MatchKind};
use regex::{Regex, RegexSet};
use serde::{Deserialize, Deserializer};

use crate::config::Config;
use crate::diagnostic::Severity;

const BUILTIN: &str = include_str!("phrases.toml");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    Boilerplate,
    Filler,
    Hype,
    Hedge,
    Metaphor,
    Vocabulary,
    Sycophancy,
    Closer,
    Opener,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    /// Almost always slop: reported as `slop/phrase`.
    #[default]
    Phrase,
    /// Legitimate in moderation: reported as `slop/word` (info) and counted for density.
    Word,
}

impl Kind {
    pub fn rule(self) -> &'static str {
        match self {
            Kind::Phrase => "slop/phrase",
            Kind::Word => "slop/word",
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub id: String,
    #[serde(default, rename = "match", deserialize_with = "one_or_many")]
    pub literals: Vec<String>,
    #[serde(default)]
    pub pattern: Option<String>,
    pub category: Category,
    #[serde(default)]
    pub kind: Kind,
    pub message: String,
    #[serde(default, deserialize_with = "one_or_many")]
    pub replace: Vec<String>,
    #[serde(default)]
    pub advice: Option<String>,
    /// Overrides the rule's default severity for this entry.
    #[serde(default)]
    pub severity: Option<Severity>,
    /// Regex that must not match right after the hit (after group `nf` when the pattern has
    /// one): a stand-in for a negative lookahead, which the `regex` crate lacks.
    #[serde(default)]
    pub not_followed_by: Option<String>,
    #[serde(default)]
    pub example: Option<String>,
    #[serde(default)]
    pub counterexample: Option<String>,
}

fn one_or_many<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<String>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum OneOrMany {
        One(String),
        Many(Vec<String>),
    }
    Ok(match OneOrMany::deserialize(d)? {
        OneOrMany::One(s) => vec![s],
        OneOrMany::Many(v) => v,
    })
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CatalogueFile {
    #[serde(default)]
    phrase: Vec<Entry>,
}

pub fn parse(src: &str) -> Result<Vec<Entry>, String> {
    toml::from_str::<CatalogueFile>(src)
        .map(|f| f.phrase)
        .map_err(|e| e.to_string())
}

/// A catalogue hit inside a text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit {
    pub entry: usize,
    pub range: Range<usize>,
}

pub struct Catalogue {
    pub entries: Vec<Entry>,
    ac: Option<AhoCorasick>,
    /// Maps an Aho-Corasick pattern index to (entry index, start boundary required, end boundary required).
    ac_map: Vec<(usize, bool, bool)>,
    set: Option<RegexSet>,
    regexes: Vec<(usize, Regex)>,
    /// Per entry: compiled `not_followed_by`, anchored at the start.
    not_followed: Vec<Option<Regex>>,
}

impl std::fmt::Debug for Catalogue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Catalogue")
            .field("entries", &self.entries.len())
            .finish()
    }
}

/// Make `'` in a regex match straight and curly apostrophes.
fn regex_source(pattern: &str) -> String {
    let pattern = if pattern.is_ascii() {
        ascii_word_boundaries(pattern)
    } else {
        pattern.to_string()
    };
    format!("(?i){}", pattern.replace('\'', "['’]"))
}

/// `\b` -> `(?-u:\b)` outside character classes. A Unicode `\b` makes the regex crate's fast
/// DFA give up on any non-ASCII text (`’`, `—`, `é`) and fall back to a far slower engine. For an
/// all-ASCII pattern the two differ only when a hit is glued to a non-ASCII letter (`éthe`).
fn ascii_word_boundaries(pattern: &str) -> String {
    let mut out = String::with_capacity(pattern.len() + 16);
    let mut class = 0usize;
    let mut chars = pattern.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => match chars.next() {
                Some('b') if class == 0 => out.push_str("(?-u:\\b)"),
                Some(n) => {
                    out.push(c);
                    out.push(n);
                }
                None => out.push(c),
            },
            '[' => {
                class += 1;
                out.push(c);
            }
            ']' if class > 0 => {
                class -= 1;
                out.push(c);
            }
            _ => out.push(c),
        }
    }
    out
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

impl Catalogue {
    pub fn build(entries: Vec<Entry>) -> Result<Catalogue, String> {
        let mut needles: Vec<String> = Vec::new();
        let mut ac_map = Vec::new();
        let mut regexes = Vec::new();
        let mut not_followed = Vec::with_capacity(entries.len());
        for (i, e) in entries.iter().enumerate() {
            let nf = match &e.not_followed_by {
                Some(p) => Some(
                    Regex::new(&format!("^(?:{})", regex_source(p)))
                        .map_err(|err| format!("slop entry `{}` not_followed_by: {err}", e.id))?,
                ),
                None => None,
            };
            not_followed.push(nf);
            if e.literals.is_empty() && e.pattern.is_none() {
                return Err(format!(
                    "slop entry `{}` has neither `match` nor `pattern`",
                    e.id
                ));
            }
            for lit in &e.literals {
                let lit = lit.trim();
                if lit.is_empty() {
                    continue;
                }
                let start = lit.chars().next().is_some_and(is_word_char);
                let end = lit.chars().next_back().is_some_and(is_word_char);
                let mut variants = vec![lit.to_string()];
                if lit.contains('\'') {
                    variants.push(lit.replace('\'', "’"));
                }
                for v in variants {
                    needles.push(v);
                    ac_map.push((i, start, end));
                }
            }
            if let Some(p) = &e.pattern {
                let re = Regex::new(&regex_source(p))
                    .map_err(|err| format!("slop entry `{}`: {err}", e.id))?;
                regexes.push((i, re));
            }
        }
        let ac = if needles.is_empty() {
            None
        } else {
            Some(
                AhoCorasickBuilder::new()
                    .ascii_case_insensitive(true)
                    .match_kind(MatchKind::Standard)
                    .build(&needles)
                    .map_err(|e| e.to_string())?,
            )
        };
        let set = if regexes.is_empty() {
            None
        } else {
            Some(
                RegexSet::new(regexes.iter().map(|(_, r)| r.as_str()))
                    .map_err(|e| e.to_string())?,
            )
        };
        Ok(Catalogue {
            entries,
            ac,
            ac_map,
            set,
            regexes,
            not_followed,
        })
    }

    fn followed_by_excluded(&self, entry: usize, text: &str, at: usize) -> bool {
        self.not_followed[entry]
            .as_ref()
            .is_some_and(|re| re.is_match(&text[at..]))
    }

    /// Non-overlapping hits, leftmost-longest across literals and patterns.
    pub fn find(&self, text: &str) -> Vec<Hit> {
        let mut hits: Vec<Hit> = Vec::new();
        if let Some(ac) = &self.ac {
            for m in ac.find_overlapping_iter(text) {
                let (entry, sb, eb) = self.ac_map[m.pattern().as_usize()];
                if sb && !boundary_before(text, m.start()) {
                    continue;
                }
                if eb && !boundary_after(text, m.end()) {
                    continue;
                }
                if self.followed_by_excluded(entry, text, m.end()) {
                    continue;
                }
                hits.push(Hit {
                    entry,
                    range: m.start()..m.end(),
                });
            }
        }
        if let Some(set) = &self.set {
            for idx in set.matches(text).iter() {
                let (entry, re) = &self.regexes[idx];
                for caps in re.captures_iter(text) {
                    let Some(m) = caps.name("m").or_else(|| caps.get(0)) else {
                        continue;
                    };
                    let after = caps
                        .name("nf")
                        .or_else(|| caps.get(0))
                        .map_or(m.end(), |g| g.end());
                    if m.start() < m.end() && !self.followed_by_excluded(*entry, text, after) {
                        hits.push(Hit {
                            entry: *entry,
                            range: m.range(),
                        });
                    }
                }
            }
        }
        hits.sort_by(|a, b| {
            (a.range.start, std::cmp::Reverse(a.range.end), a.entry).cmp(&(
                b.range.start,
                std::cmp::Reverse(b.range.end),
                b.entry,
            ))
        });
        let mut out: Vec<Hit> = Vec::new();
        for h in hits {
            if out.last().is_none_or(|l| h.range.start >= l.range.end) {
                out.push(h);
            }
        }
        out
    }
}

/// A word starts at `i` (no word char before it; `x-` or `x'` count as word chars too).
fn boundary_before(text: &str, i: usize) -> bool {
    let mut it = text[..i].chars().rev();
    match it.next() {
        None => true,
        Some(c) if is_word_char(c) => false,
        Some('-' | '\'' | '’') => !it.next().is_some_and(is_word_char),
        Some(_) => true,
    }
}

fn boundary_after(text: &str, i: usize) -> bool {
    let mut it = text[i..].chars();
    match it.next() {
        None => true,
        Some(c) if is_word_char(c) => false,
        Some('-') => !it.next().is_some_and(is_word_char),
        Some(_) => true,
    }
}

pub fn builtin_entries() -> Vec<Entry> {
    parse(BUILTIN).expect("built-in phrases.toml is valid")
}

static BUILTIN_CAT: LazyLock<Arc<Catalogue>> = LazyLock::new(|| {
    Arc::new(Catalogue::build(builtin_entries()).expect("built-in phrases.toml compiles"))
});

type Key = (PathBuf, Vec<PathBuf>, Vec<String>);
static CUSTOM: LazyLock<Mutex<HashMap<Key, Arc<Catalogue>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// The catalogue for a config: built-in entries, minus `disable`, with `extra` merged by id.
pub fn for_config(config: &Config) -> Arc<Catalogue> {
    let slop = &config.slop;
    if slop.extra.is_empty() && slop.disable.is_empty() {
        return BUILTIN_CAT.clone();
    }
    let key: Key = (
        config.root.clone(),
        slop.extra.clone(),
        slop.disable.clone(),
    );
    let mut cache = CUSTOM.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(c) = cache.get(&key) {
        return c.clone();
    }
    let mut entries = builtin_entries();
    for path in &slop.extra {
        let full = if path.is_absolute() {
            path.clone()
        } else {
            config.root.join(path)
        };
        let parsed = std::fs::read_to_string(&full)
            .map_err(|e| e.to_string())
            .and_then(|s| parse(&s));
        match parsed {
            Ok(extra) => {
                for e in extra {
                    match entries.iter_mut().find(|x| x.id == e.id) {
                        Some(slot) => *slot = e,
                        None => entries.push(e),
                    }
                }
            }
            Err(err) => eprintln!(
                "explicit: ignoring slop catalogue {}: {err}",
                full.display()
            ),
        }
    }
    entries.retain(|e| !slop.disable.contains(&e.id));
    let cat = match Catalogue::build(entries) {
        Ok(c) => Arc::new(c),
        Err(err) => {
            eprintln!("explicit: invalid slop catalogue, using built-in: {err}");
            BUILTIN_CAT.clone()
        }
    };
    cache.insert(key, cat.clone());
    cat
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_entries_match_examples() {
        let entries = builtin_entries();
        assert!(
            entries.len() >= 150,
            "catalogue has {} entries",
            entries.len()
        );
        let mut ids = std::collections::HashSet::new();
        for e in &entries {
            assert!(ids.insert(e.id.clone()), "duplicate id {}", e.id);
            let example = e
                .example
                .as_deref()
                .unwrap_or_else(|| panic!("{} lacks example", e.id));
            let counter = e
                .counterexample
                .as_deref()
                .unwrap_or_else(|| panic!("{} lacks counterexample", e.id));
            let cat = Catalogue::build(vec![e.clone()]).unwrap();
            assert!(
                !cat.find(example).is_empty(),
                "{} does not match its example {example:?}",
                e.id
            );
            assert!(
                cat.find(counter).is_empty(),
                "{} matches its counterexample {counter:?}",
                e.id
            );
        }
    }

    #[test]
    fn ascii_boundaries_outside_classes() {
        assert_eq!(
            ascii_word_boundaries(r"\bfoo[\b\]x]\\b\w\b"),
            r"(?-u:\b)foo[\b\]x]\\b\w(?-u:\b)"
        );
        assert_eq!(regex_source("\\bé\\b"), "(?i)\\bé\\b");
    }

    #[test]
    fn builtin_compiles() {
        assert!(!BUILTIN_CAT.entries.is_empty());
    }

    #[test]
    fn curly_apostrophes_and_boundaries() {
        let cat = &*BUILTIN_CAT;
        let text = "It’s worth noting that it’s robust. Robustness is fine. We delve.";
        let hits = cat.find(text);
        let ids: Vec<&str> = hits
            .iter()
            .map(|h| cat.entries[h.entry].id.as_str())
            .collect();
        assert_eq!(ids, ["worth-noting", "robust", "delve"]);
        assert_eq!(&text[hits[0].range.clone()], "It’s worth noting");
    }

    #[test]
    fn hyphenated_words_are_not_split() {
        let cat = &*BUILTIN_CAT;
        assert!(
            cat.find("an AI-assisted robust-ish tool")
                .iter()
                .all(|h| cat.entries[h.entry].id != "as-an-ai")
        );
        assert!(cat.find("robust-ish").is_empty());
    }

    #[test]
    fn leftmost_longest_wins() {
        let cat = &*BUILTIN_CAT;
        let hits = cat.find("a rich tapestry of");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].range, 2..15);
    }

    #[test]
    fn config_extra_and_disable() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("mine.toml"),
            r#"
[[phrase]]
id = "delve"
match = "delve"
category = "vocabulary"
kind = "word"
message = "custom"

[[phrase]]
id = "frobnicate"
match = ["frobnicate"]
category = "vocabulary"
message = "no"
"#,
        )
        .unwrap();
        let mut config = Config {
            root: dir.path().to_path_buf(),
            ..Default::default()
        };
        config.slop.extra = vec![PathBuf::from("mine.toml")];
        config.slop.disable = vec!["robust".into()];
        let cat = for_config(&config);
        let hits = cat.find("delve and frobnicate robustly");
        let ids: Vec<&str> = hits
            .iter()
            .map(|h| cat.entries[h.entry].id.as_str())
            .collect();
        assert_eq!(ids, ["delve", "frobnicate"]);
        assert_eq!(cat.entries[hits[0].entry].kind, Kind::Word);
    }
}
