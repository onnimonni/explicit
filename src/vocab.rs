//! Project vocabulary from `explicit.toml`: `[[vocab]]` terms and `[[entity]]` names, how the
//! spell check accepts them ([`accepted_words`], [`Phrases`]), and the `explicit vocab`
//! subcommands ([`suggest`], [`list_rows`]).

use std::collections::{BTreeMap, HashMap};
use std::ops::Range;
use std::path::PathBuf;

use regex::Regex;
use serde::Serialize;

use crate::config::{Config, Entity, Level, Vocab};
use crate::rules::spell::{self, Speller};
use crate::rules::words::Dialect;

/// Single-word entries accepted case-insensitively, like `prose.accept`: `[[vocab]]` terms and
/// aliases that are not case-sensitive, and `[[entity]]` aliases.
pub fn accepted_words(vocab: &[Vocab], entities: &[Entity]) -> Vec<String> {
    let single = |w: &&String| !w.trim().contains(char::is_whitespace);
    vocab
        .iter()
        .filter(|v| !v.case_sensitive)
        .flat_map(|v| std::iter::once(&v.term).chain(&v.aliases))
        .chain(entities.iter().flat_map(|e| &e.aliases))
        .filter(single)
        .map(|w| w.trim().to_string())
        .collect()
}

/// What a [`Phrase`] was configured as.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhraseKind {
    /// An `[[entity]]` name or alias.
    Entity,
    /// A multi-word or case-sensitive `[[vocab]]` term or alias.
    Vocab,
}

/// A configured name matched as a whole phrase, case-insensitively.
#[derive(Debug, Clone)]
pub struct Phrase {
    /// As configured.
    pub canonical: String,
    pub kind: PhraseKind,
    /// The entity name or vocab term this belongs to.
    pub owner: String,
    /// The entity's relationship or the term's description.
    pub note: String,
    /// Other casings are reported (`prose/entity-name`).
    pub case_checked: bool,
    /// A single-word entity name or alias: its casing is checked only when the lowercase form
    /// is not a dictionary word (`Telia` yes, `Apple` no).
    pub single_word: bool,
}

/// All configured [`Phrase`]s, matched by one case-insensitive regex.
#[derive(Debug, Default)]
pub struct Phrases {
    pub list: Vec<Phrase>,
    re: Option<Regex>,
    /// Normalized (lowercase, single-spaced) phrase -> index in `list`.
    index: HashMap<String, usize>,
}

/// A [`Phrase`] found in text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhraseMatch {
    pub range: Range<usize>,
    pub phrase: usize,
}

fn normalize(s: &str) -> String {
    s.split_whitespace()
        .map(str::to_lowercase)
        .collect::<Vec<_>>()
        .join(" ")
}

impl Phrases {
    pub fn new(vocab: &[Vocab], entities: &[Entity]) -> Phrases {
        let mut list = Vec::new();
        for e in entities {
            let note = e.relationship.trim().to_string();
            list.push(Phrase {
                canonical: e.name.trim().to_string(),
                kind: PhraseKind::Entity,
                owner: e.name.trim().to_string(),
                note: note.clone(),
                case_checked: true,
                single_word: !e.name.trim().contains(char::is_whitespace),
            });
            for a in &e.aliases {
                let a = a.trim();
                list.push(Phrase {
                    canonical: a.to_string(),
                    kind: PhraseKind::Entity,
                    owner: e.name.trim().to_string(),
                    note: note.clone(),
                    case_checked: true,
                    single_word: !a.contains(char::is_whitespace),
                });
            }
        }
        for v in vocab {
            for t in std::iter::once(&v.term).chain(&v.aliases) {
                let t = t.trim();
                if v.case_sensitive || t.contains(char::is_whitespace) {
                    list.push(Phrase {
                        canonical: t.to_string(),
                        kind: PhraseKind::Vocab,
                        owner: v.term.trim().to_string(),
                        note: v.description.trim().to_string(),
                        case_checked: v.case_sensitive,
                        single_word: false,
                    });
                }
            }
        }
        let mut index = HashMap::new();
        for (i, p) in list.iter().enumerate() {
            index.entry(normalize(&p.canonical)).or_insert(i);
        }
        // Longest first: the leftmost alternative wins, so `Telia Oy` beats `Telia`.
        let mut order: Vec<usize> = index.values().copied().collect();
        order.sort_by_key(|&i| (std::cmp::Reverse(list[i].canonical.len()), i));
        let alts: Vec<String> = order
            .iter()
            .map(|&i| {
                let c = &list[i].canonical;
                let body = c
                    .split_whitespace()
                    .map(regex::escape)
                    .collect::<Vec<_>>()
                    .join(r"\s+");
                let edge = |ch: Option<char>| {
                    if ch.is_some_and(char::is_alphanumeric) {
                        r"\b"
                    } else {
                        ""
                    }
                };
                format!("{}{body}{}", edge(c.chars().next()), edge(c.chars().last()))
            })
            .collect();
        let re = (!alts.is_empty())
            .then(|| Regex::new(&format!("(?i)(?:{})", alts.join("|"))).ok())
            .flatten();
        Phrases { list, re, index }
    }

    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    /// Non-overlapping matches in `text`, leftmost-longest.
    pub fn find(&self, text: &str) -> Vec<PhraseMatch> {
        let Some(re) = &self.re else {
            return Vec::new();
        };
        re.find_iter(text)
            .filter_map(|m| {
                let phrase = *self.index.get(&normalize(m.as_str()))?;
                Some(PhraseMatch {
                    range: m.range(),
                    phrase,
                })
            })
            .collect()
    }

    /// Ranges of `text` the spell check accepts: every match (with a trailing `'s`), except a
    /// case-sensitive `[[vocab]]` term in another casing when `prose/entity-name`, which would
    /// report it, is off.
    pub fn accepted_ranges(&self, text: &str, case_rule_on: bool) -> Vec<Range<usize>> {
        self.find(text)
            .into_iter()
            .filter(|m| {
                let p = &self.list[m.phrase];
                case_rule_on
                    || p.kind == PhraseKind::Entity
                    || !p.case_checked
                    || casing_ok(&text[m.range.clone()], &p.canonical)
            })
            .map(|m| {
                let rest = &text[m.range.end..];
                let extra = ["'s", "’s"]
                    .iter()
                    .find(|s| rest.starts_with(**s))
                    .map_or(0, |s| s.len());
                m.range.start..m.range.end + extra
            })
            .collect()
    }

    /// `matched` (a match of phrase `i`) is in a casing `prose/entity-name` reports.
    pub fn wrong_case(&self, i: usize, matched: &str, s: &Speller) -> bool {
        let p = &self.list[i];
        if !p.case_checked || casing_ok(matched, &p.canonical) {
            return false;
        }
        // `apple` for an alias `Apple` is the fruit.
        !(p.single_word && spell::known(s, &p.canonical.to_lowercase()))
    }
}

/// `matched` is written as `canonical` (whitespace aside), in ALL CAPS, or capitalized from a
/// lowercase canonical form (a sentence start).
pub fn casing_ok(matched: &str, canonical: &str) -> bool {
    let words: Vec<&str> = matched.split_whitespace().collect();
    let canon: Vec<&str> = canonical.split_whitespace().collect();
    if words == canon {
        return true;
    }
    let letters = || matched.chars().filter(|c| c.is_alphabetic());
    if letters().count() > 1 && letters().all(char::is_uppercase) {
        return true;
    }
    canonical.starts_with(char::is_lowercase) && words.join(" ") == capitalize(&canon.join(" "))
}

fn capitalize(w: &str) -> String {
    let mut c = w.chars();
    c.next()
        .map(|f| f.to_uppercase().chain(c).collect())
        .unwrap_or_default()
}

/// `canonical`'s words with `matched`'s whitespace between them.
pub fn recase(matched: &str, canonical: &str) -> String {
    let mut out = String::new();
    let mut canon = canonical.split_whitespace();
    let mut in_word = false;
    for ch in matched.chars() {
        if ch.is_whitespace() {
            out.push(ch);
            in_word = false;
        } else if !in_word {
            out.push_str(canon.next().unwrap_or(""));
            in_word = true;
        }
    }
    out
}

// `explicit vocab suggest`

/// Legal-form suffixes that make a capitalized phrase a company name.
pub const LEGAL_SUFFIXES: &[&str] = &[
    "Oy", "Oyj", "Ab", "AB", "Ltd", "Inc", "GmbH", "AS", "ASA", "ApS", "LLC", "SA", "BV", "AG",
    "Plc", "PLC", "Corp", "SE", "NV", "Srl", "SpA",
];

/// Lowercase words that may join capitalized words inside a name: `Bank of Finland`.
const CONNECTORS: &[&str] = &[
    "of", "and", "&", "for", "de", "af", "von", "van", "der", "la",
];

/// Capitalized words that open a sentence or clause rather than a name.
const LEADING_NOISE: &[&str] = &[
    "The", "A", "An", "Our", "Their", "This", "That", "These", "Those", "Your", "My", "With",
    "From", "For", "And", "Or", "But", "In", "On", "At", "By", "To", "Via", "See", "Use", "Using",
    "If", "When", "While", "Ask", "Contact", "Call", "Per", "As", "Is", "Are", "Was", "Also",
    "Then", "Each", "Every", "All", "Some", "No", "Not", "Only", "Both",
];

/// One suggested `[[vocab]]` or `[[entity]]` entry.
#[derive(Debug, Clone, Serialize)]
pub struct Suggestion {
    /// `vocab` or `entity`.
    pub section: &'static str,
    /// The term or entity name, as most often written.
    pub term: String,
    /// `company` for names with a legal suffix; empty when unknown.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub kind: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
    /// Occurrences flagged by the spell check.
    pub count: usize,
    /// Files with the most occurrences, most first.
    pub files: Vec<FileCount>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FileCount {
    pub path: PathBuf,
    pub count: usize,
}

/// Candidates for `[[vocab]]` / `[[entity]]` in `files`: capitalized words the spell check
/// flags, and runs of capitalized words around them (`Telia Oy`), most frequent first. Project
/// names (recurring capitalized words the spell check accepts on its own) count too.
pub fn suggest(files: &[PathBuf], config: &Config, min_count: usize) -> Vec<Suggestion> {
    let mut cfg = config.clone();
    // Only spelling: everything else off.
    cfg.rules.insert("*".into(), Level::Off);
    cfg.rules.insert("spelling".into(), Level::Error);
    cfg.overrides = config
        .overrides
        .iter()
        .cloned()
        .map(|mut o| {
            o.rules.retain(|k, _| k == "spelling");
            o
        })
        .collect();
    cfg.links.remote = false;
    let ws = crate::engine::build_workspace(files, &cfg);
    let speller = spell::dictionary(&cfg.prose.dialect);
    // `Optimise` under American spelling is a dialect slip, not a name.
    let other = spell::dictionary(
        if matches!(speller.dialect(), Dialect::American | Dialect::Canadian) {
            "british"
        } else {
            "american"
        },
    );
    let analyzed: Vec<_> = ws.files.values().cloned().collect();
    // Without the workspace's project vocabulary: recurring names are what we look for.
    let per_file = crate::engine::par_map(&analyzed, |a| {
        let effective = cfg.for_path(&a.file.rel);
        let fc: &Config = effective.as_deref().unwrap_or(&cfg);
        let diags = crate::engine::local_diagnostics(a, fc);
        let text = &a.file.text;
        let mut found: Vec<(Range<usize>, String)> = Vec::new();
        for d in diags.iter().filter(|d| d.rule == "spelling") {
            let word = &text[d.range.clone()];
            if !word.starts_with(char::is_uppercase) || spell::known(other, &word.to_lowercase()) {
                continue;
            }
            let (range, phrase) = proper_noun_phrase(text, d.range.clone(), speller);
            if found
                .iter()
                .any(|(r, _)| r.start <= range.start && range.end <= r.end)
            {
                continue;
            }
            found.push((range, phrase));
        }
        (
            a.file.rel.clone(),
            found.into_iter().map(|(_, p)| p).collect(),
        )
    });
    collect(per_file, min_count)
}

/// Rank the phrases found per file into suggestions; single words that are a company name
/// without its suffix become that entity's alias.
fn collect(per_file: Vec<(PathBuf, Vec<String>)>, min_count: usize) -> Vec<Suggestion> {
    // phrase -> (spelling as written -> count), file -> count
    type Tally = (BTreeMap<String, usize>, BTreeMap<PathBuf, usize>);
    let mut tally: BTreeMap<String, Tally> = BTreeMap::new();
    for (path, found) in per_file {
        for phrase in found {
            let t = tally.entry(normalize(&phrase)).or_default();
            *t.0.entry(phrase).or_default() += 1;
            *t.1.entry(path.clone()).or_default() += 1;
        }
    }
    let mut out: Vec<Suggestion> = tally
        .into_values()
        .filter_map(|(forms, files)| {
            let (term, _) = forms
                .iter()
                .max_by_key(|(f, n)| (**n, std::cmp::Reverse((*f).clone())))?;
            let count = forms.values().sum();
            let multi = term.contains(' ');
            if !multi && LEGAL_SUFFIXES.contains(&term.as_str()) {
                return None;
            }
            let last = term.rsplit(' ').next().unwrap_or("");
            let kind = if multi && LEGAL_SUFFIXES.contains(&last) {
                "company".to_string()
            } else {
                String::new()
            };
            let mut files: Vec<FileCount> = files
                .into_iter()
                .map(|(path, count)| FileCount { path, count })
                .collect();
            files.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.path.cmp(&b.path)));
            Some(Suggestion {
                section: if multi { "entity" } else { "vocab" },
                term: term.clone(),
                kind,
                aliases: Vec::new(),
                count,
                files,
            })
        })
        .collect();
    // `Telia` alone next to `Telia Oy`: an alias of the company.
    let words: Vec<(usize, String)> = out
        .iter()
        .enumerate()
        .filter(|(_, s)| s.section == "vocab")
        .map(|(i, s)| (i, s.term.clone()))
        .collect();
    let mut merged = Vec::new();
    for s in out.iter_mut().filter(|s| s.kind == "company") {
        let base = s.term.rsplit_once(' ').map_or("", |(b, _)| b).to_string();
        if let Some((i, w)) = words.iter().find(|(_, w)| *w == base) {
            s.aliases.push(w.clone());
            merged.push(*i);
        }
    }
    let alias_counts: Vec<(String, usize, Vec<FileCount>)> = merged
        .iter()
        .map(|&i| (out[i].term.clone(), out[i].count, out[i].files.clone()))
        .collect();
    for s in out.iter_mut() {
        for (w, n, files) in &alias_counts {
            if s.aliases.contains(w) {
                s.count += n;
                for f in files {
                    match s.files.iter_mut().find(|x| x.path == f.path) {
                        Some(x) => x.count += f.count,
                        None => s.files.push(f.clone()),
                    }
                }
                s.files
                    .sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.path.cmp(&b.path)));
            }
        }
    }
    let mut out: Vec<Suggestion> = out
        .into_iter()
        .enumerate()
        .filter(|(i, s)| !merged.contains(i) && s.count >= min_count)
        .map(|(_, s)| s)
        .collect();
    out.sort_by(|a, b| {
        (a.section != "entity", std::cmp::Reverse(a.count), &a.term).cmp(&(
            b.section != "entity",
            std::cmp::Reverse(b.count),
            &b.term,
        ))
    });
    out
}

/// The run of capitalized words (joined by single spaces, possibly with connectors such as
/// `of`) around `text[r]`, without sentence-opening words before it.
fn proper_noun_phrase(text: &str, r: Range<usize>, s: &Speller) -> (Range<usize>, String) {
    let is_word_char = |c: char| c.is_alphanumeric() || matches!(c, '-' | '\'' | '’' | '&');
    // (start, end) of each word left and right of `r`, separated by exactly one space.
    let word_before = |pos: usize| -> Option<Range<usize>> {
        let head = text[..pos].strip_suffix(' ')?;
        let len: usize = head
            .chars()
            .rev()
            .take_while(|&c| is_word_char(c))
            .map(char::len_utf8)
            .sum();
        (len > 0).then(|| head.len() - len..head.len())
    };
    let word_after = |pos: usize| -> Option<Range<usize>> {
        let tail = text[pos..].strip_prefix(' ')?;
        let len: usize = tail
            .chars()
            .take_while(|&c| is_word_char(c))
            .map(char::len_utf8)
            .sum();
        (len > 0).then(|| pos + 1..pos + 1 + len)
    };
    let capitalized = |w: &str| w.starts_with(char::is_uppercase);
    let connector = |w: &str| CONNECTORS.contains(&w);
    let mut words = vec![r.clone()];
    let mut pos = r.start;
    while let Some(w) = word_before(pos) {
        let t = &text[w.clone()];
        if !(capitalized(t) || connector(t)) {
            break;
        }
        pos = w.start;
        words.insert(0, w);
    }
    let mut pos = r.end;
    while let Some(w) = word_after(pos) {
        let t = &text[w.clone()];
        if !(capitalized(t) || connector(t)) {
            break;
        }
        pos = w.end;
        words.push(w);
    }
    let flagged = words.iter().position(|w| *w == r).unwrap_or(0);
    let mut lo = 0;
    let mut hi = words.len();
    // Trim connectors at the ends, and opening words before the flagged one.
    while hi - 1 > flagged && connector(&text[words[hi - 1].clone()]) {
        hi -= 1;
    }
    while lo < flagged {
        let t = &text[words[lo].clone()];
        let sentence_start = lo == 0 && sentence_start(text, words[lo].start);
        if connector(t)
            || LEADING_NOISE.contains(&t)
            || (sentence_start && spell::known(s, &t.to_lowercase()))
        {
            lo += 1;
        } else {
            break;
        }
    }
    let mut range = words[lo].start..words[hi - 1].end;
    // `Lemire's`: the name without the possessive.
    if let Some(p) = ["'s", "’s"]
        .iter()
        .find(|p| text[range.clone()].ends_with(**p))
    {
        range.end -= p.len();
    }
    let phrase = text[range.clone()].to_string();
    (range, phrase)
}

/// `pos` opens a line, a list item or a sentence.
fn sentence_start(text: &str, pos: usize) -> bool {
    let before = text[..pos].trim_end_matches([' ', '\t']);
    before.is_empty()
        || before.ends_with([
            '\n', '.', '!', '?', ':', '#', '>', '|', '*', '-', '(', '"', '“',
        ])
        || before.ends_with("//")
}

/// The suggestions as TOML stanzas ready to paste into `explicit.toml`.
pub fn to_toml(sugs: &[Suggestion], files_checked: usize) -> String {
    let q = |s: &str| toml::Value::String(s.to_string()).to_string();
    let entities = sugs.iter().filter(|s| s.section == "entity").count();
    let mut out = format!(
        "# explicit vocab suggest: {entities} entities, {} terms from {files_checked} files.\n\
         # Fill in each TODO, then paste into explicit.toml.\n",
        sugs.len() - entities
    );
    for s in sugs {
        let top: Vec<String> = s
            .files
            .iter()
            .take(3)
            .map(|f| format!("{} ({})", f.path.display(), f.count))
            .collect();
        let more = if s.files.len() > 3 {
            format!(", +{} more", s.files.len() - 3)
        } else {
            String::new()
        };
        out.push_str(&format!(
            "\n# {} hit{}: {}{more}\n",
            s.count,
            if s.count == 1 { "" } else { "s" },
            top.join(", ")
        ));
        let aliases = || {
            let a: Vec<String> = s.aliases.iter().map(|a| q(a)).collect();
            format!("aliases = [{}]\n", a.join(", "))
        };
        if s.section == "entity" {
            out.push_str(&format!("[[entity]]\nname = {}\n", q(&s.term)));
            let kind = if s.kind.is_empty() { "TODO" } else { &s.kind };
            out.push_str(&format!("kind = {}\nrelationship = \"TODO\"\n", q(kind)));
            if !s.aliases.is_empty() {
                out.push_str(&aliases());
            }
        } else {
            out.push_str(&format!(
                "[[vocab]]\nterm = {}\ndescription = \"TODO\"\n",
                q(&s.term)
            ));
        }
    }
    out
}

// `explicit vocab list`

/// One configured `[[vocab]]` or `[[entity]]` entry, for `explicit vocab list`.
#[derive(Debug, Clone, Serialize)]
pub struct ListRow {
    /// `vocab` or `entity`.
    pub section: &'static str,
    pub term: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub kind: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
    /// Description (vocab) or relationship (entity).
    pub note: String,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub case_sensitive: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Glob patterns of the `[[overrides]]` entry that adds it; empty for the top level.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub paths: Vec<String>,
}

/// Every configured entry, entities first, including those of `[[overrides]]`.
pub fn list_rows(config: &Config) -> Vec<ListRow> {
    let mut rows = Vec::new();
    let scopes = std::iter::once((&config.vocab, &config.entities, Vec::new())).chain(
        config
            .overrides
            .iter()
            .map(|o| (&o.vocab, &o.entities, o.paths.clone())),
    );
    let mut vocab_rows = Vec::new();
    for (vocab, entities, paths) in scopes {
        rows.extend(entities.iter().map(|e| ListRow {
            section: "entity",
            term: e.name.clone(),
            kind: e.kind.clone(),
            aliases: e.aliases.clone(),
            note: e.relationship.clone(),
            case_sensitive: false,
            url: e.url.clone(),
            paths: paths.clone(),
        }));
        vocab_rows.extend(vocab.iter().map(|v| ListRow {
            section: "vocab",
            term: v.term.clone(),
            kind: String::new(),
            aliases: v.aliases.clone(),
            note: v.description.clone(),
            case_sensitive: v.case_sensitive,
            url: None,
            paths: paths.clone(),
        }));
    }
    rows.extend(vocab_rows);
    rows
}

/// [`list_rows`] as an aligned text table.
pub fn list_table(rows: &[ListRow]) -> String {
    let cells: Vec<[String; 4]> = rows
        .iter()
        .map(|r| {
            let mut term = r.term.clone();
            if !r.aliases.is_empty() {
                term.push_str(&format!(" ({})", r.aliases.join(", ")));
            }
            let mut kind = if r.section == "entity" {
                if r.kind.is_empty() {
                    "entity".to_string()
                } else {
                    r.kind.clone()
                }
            } else if r.case_sensitive {
                "term, exact case".to_string()
            } else {
                "term".to_string()
            };
            if !r.paths.is_empty() {
                kind.push_str(&format!(" [{}]", r.paths.join(", ")));
            }
            let mut note = r.note.clone();
            if let Some(u) = &r.url {
                note.push_str(&format!(" <{u}>"));
            }
            [r.section.to_string(), term, kind, note]
        })
        .collect();
    let header = ["SECTION", "NAME", "KIND", "MEANING"].map(String::from);
    let width = |i: usize| {
        cells
            .iter()
            .chain([&header])
            .map(|c| c[i].chars().count())
            .max()
            .unwrap_or(0)
    };
    let (w0, w1, w2) = (width(0), width(1), width(2));
    let mut out = String::new();
    for c in std::iter::once(&header).chain(&cells) {
        out.push_str(format!("{:<w0$}  {:<w1$}  {:<w2$}  {}", c[0], c[1], c[2], c[3]).trim_end());
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entity(name: &str, aliases: &[&str]) -> Entity {
        Entity {
            name: name.into(),
            kind: "company".into(),
            relationship: "Partner".into(),
            aliases: aliases.iter().map(|a| a.to_string()).collect(),
            url: None,
            ..Default::default()
        }
    }

    fn vocab(term: &str, case_sensitive: bool) -> Vocab {
        Vocab {
            term: term.into(),
            description: "A term".into(),
            case_sensitive,
            aliases: Vec::new(),
            ..Default::default()
        }
    }

    #[test]
    fn accepted_words_skip_phrases_and_exact_case() {
        let v = [
            vocab("Astori", false),
            vocab("Kanta Hub", false),
            vocab("FiMEA", true),
        ];
        let e = [entity("Telia Oy", &["Telia"])];
        assert_eq!(accepted_words(&v, &e), ["Astori", "Telia"]);
    }

    #[test]
    fn phrases_match_longest_first_and_check_case() {
        let e = [
            entity("Telia Oy", &["Telia"]),
            entity("Apple Inc", &["Apple"]),
        ];
        let v = [vocab("FiMEA", true), vocab("Kanta Hub", false)];
        let p = Phrases::new(&v, &e);
        let text = "Ask telia  oy or Telia, TELIA OY, apple, fimea and kanta hub.";
        let found: Vec<(&str, &str)> = p
            .find(text)
            .iter()
            .map(|m| (&text[m.range.clone()], p.list[m.phrase].canonical.as_str()))
            .collect();
        assert_eq!(
            found,
            [
                ("telia  oy", "Telia Oy"),
                ("Telia", "Telia"),
                ("TELIA OY", "Telia Oy"),
                ("apple", "Apple"),
                ("fimea", "FiMEA"),
                ("kanta hub", "Kanta Hub"),
            ]
        );
        let s = spell::dictionary("american");
        let wrong: Vec<&str> = p
            .find(text)
            .iter()
            .filter(|m| p.wrong_case(m.phrase, &text[m.range.clone()], s))
            .map(|m| &text[m.range.clone()])
            .collect();
        // ALL CAPS is fine; `apple` is a word; vocab without case_sensitive is not checked.
        assert_eq!(wrong, ["telia  oy", "fimea"]);
        assert_eq!(recase("telia  oy", "Telia Oy"), "Telia  Oy");
        // Case-sensitive vocab in the wrong case is accepted only when the case rule reports it.
        assert_eq!(p.accepted_ranges("fimea's", true), vec![0..7]);
        assert!(p.accepted_ranges("fimea", false).is_empty());
        assert_eq!(p.accepted_ranges("FiMEA", false), vec![0..5]);
        assert!(casing_ok("Npm", "npm") && !casing_ok("NPm", "npm"));
    }

    #[test]
    fn phrase_around_flagged_word() {
        let s = spell::dictionary("american");
        let t = "Contact Telia Oy today. The Bank of Zorbia said.\nZorbia Oy";
        let at = |w: &str| {
            let i = t.find(w).unwrap();
            proper_noun_phrase(t, i..i + w.len(), s).1
        };
        assert_eq!(at("Telia"), "Telia Oy");
        assert_eq!(at("Oy"), "Telia Oy");
        assert_eq!(at("Zorbia"), "Bank of Zorbia");
        let t = "Use Zorbia's parser.";
        assert_eq!(proper_noun_phrase(t, 4..12, s).1, "Zorbia");
    }

    #[test]
    fn suggest_scans_files() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
        let a = root.join("a.md");
        std::fs::write(
            &a,
            "# Partners\n\nWe ship via Zorbia Oy. Zorbia handles refunds and Zorbia is fast.\n\n\
             Ask Kvelmo, Kvelmo or Kvelmo. The zorbia typo stays out.\n",
        )
        .unwrap();
        let config = Config {
            root: root.clone(),
            ..Config::default()
        };
        let sugs = suggest(std::slice::from_ref(&a), &config, 1);
        let got: Vec<(&str, &str, usize)> = sugs
            .iter()
            .map(|s| (s.section, s.term.as_str(), s.count))
            .collect();
        // Recurring names count although `check` accepts them as project names.
        assert_eq!(got, [("entity", "Zorbia Oy", 3), ("vocab", "Kvelmo", 3)]);
        assert_eq!(sugs[0].aliases, ["Zorbia"]);
        assert_eq!(sugs[0].files[0].path, PathBuf::from("a.md"));
        // Configured names are no longer suggested.
        let mut configured = config.clone();
        configured.entities.push(entity("Zorbia Oy", &["Zorbia"]));
        let sugs = suggest(&[a], &configured, 1);
        assert_eq!(sugs.len(), 1);
        assert_eq!(sugs[0].term, "Kvelmo");
    }

    #[test]
    fn collect_ranks_and_merges_aliases() {
        let f = |p: &str, phrases: &[&str]| {
            (
                PathBuf::from(p),
                phrases.iter().map(|x| x.to_string()).collect(),
            )
        };
        let sugs = collect(
            vec![
                f("a.md", &["Telia Oy", "Telia", "Astori", "Astori", "Oy"]),
                f("b.md", &["Telia", "Zed"]),
            ],
            1,
        );
        let terms: Vec<(&str, usize)> = sugs.iter().map(|s| (s.term.as_str(), s.count)).collect();
        assert_eq!(terms, [("Telia Oy", 3), ("Astori", 2), ("Zed", 1)]);
        assert_eq!(sugs[0].aliases, ["Telia"]);
        assert_eq!(sugs[0].kind, "company");
        let toml_text = to_toml(&sugs, 2);
        assert!(toml_text.contains("[[entity]]\nname = \"Telia Oy\"\nkind = \"company\"\nrelationship = \"TODO\"\naliases = [\"Telia\"]\n"));
        assert!(toml_text.contains("# 3 hits: a.md (2), b.md (1)"));
        assert!(toml_text.contains("[[vocab]]\nterm = \"Astori\"\ndescription = \"TODO\"\n"));
        // The output is valid TOML for the config.
        let parsed: toml::Table = toml::from_str(&toml_text).unwrap();
        assert_eq!(parsed["vocab"].as_array().unwrap().len(), 2);
        assert_eq!(collect(vec![f("a.md", &["Zed"])], 2).len(), 0);
    }
}
