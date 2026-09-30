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

/// All configured [`Phrase`]s, matched by one case-insensitive regex, and their inflected
/// forms (`Kone Oyj:n`, `Rovio Entertainmentissa`) by another.
#[derive(Debug, Default)]
pub struct Phrases {
    pub list: Vec<Phrase>,
    re: Option<Regex>,
    /// Normalized (lowercase, single-spaced) phrase -> index in `list`.
    index: HashMap<String, usize>,
    /// Phrases whose last word runs on into letters or a colon (`Entertainmentissa`).
    inflected_re: Option<Regex>,
    /// Normalized phrase with its last word cut to a stem ([`name_stems`]) -> phrases.
    stems: HashMap<String, Vec<usize>>,
}

/// A [`Phrase`] found in text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhraseMatch {
    pub range: Range<usize>,
    pub phrase: usize,
    /// With an ending on its last word ([`inflected_form`]): `Rovio Entertainmentin`.
    pub inflected: bool,
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
        // Inflected forms: each stem of the last word, then an ending (checked in `find`).
        let mut stems: HashMap<String, Vec<usize>> = HashMap::new();
        let mut infl: Vec<(usize, String)> = Vec::new();
        for &i in &order {
            let words: Vec<&str> = list[i].canonical.split_whitespace().collect();
            let Some((last, head)) = words.split_last() else {
                continue;
            };
            if !last.ends_with(char::is_alphanumeric) {
                continue;
            }
            let head_re: String = head
                .iter()
                .map(|w| format!(r"{}\s+", regex::escape(w)))
                .collect();
            let edge = if list[i].canonical.starts_with(char::is_alphanumeric) {
                r"\b"
            } else {
                ""
            };
            for stem in name_stems(last) {
                let key = normalize(&format!("{} {stem}", head.join(" ")));
                let entry = stems.entry(key).or_default();
                if !entry.contains(&i) {
                    entry.push(i);
                }
                infl.push((
                    list[i].canonical.len(),
                    format!(r"{edge}{head_re}{}", regex::escape(&stem)),
                ));
            }
        }
        infl.sort_by_key(|(len, _)| std::cmp::Reverse(*len));
        infl.dedup_by(|a, b| a.1 == b.1);
        let inflected_re = (!infl.is_empty())
            .then(|| {
                let alts: Vec<&str> = infl.iter().map(|(_, a)| a.as_str()).collect();
                Regex::new(&format!(
                    r"(?i)(?:{})(?::\p{{L}}+|\p{{L}}+)\b",
                    alts.join("|")
                ))
                .ok()
            })
            .flatten();
        Phrases {
            list,
            re,
            index,
            inflected_re,
            stems,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    /// Non-overlapping matches in `text`, leftmost-longest, inflected forms included.
    pub fn find(&self, text: &str) -> Vec<PhraseMatch> {
        let Some(re) = &self.re else {
            return Vec::new();
        };
        let mut all: Vec<PhraseMatch> = re
            .find_iter(text)
            .filter_map(|m| {
                let phrase = *self.index.get(&normalize(m.as_str()))?;
                Some(PhraseMatch {
                    range: m.range(),
                    phrase,
                    inflected: false,
                })
            })
            .collect();
        if let Some(re) = &self.inflected_re {
            all.extend(re.find_iter(text).filter_map(|m| {
                Some(PhraseMatch {
                    range: m.range(),
                    phrase: self.inflected_phrase(m.as_str())?,
                    inflected: true,
                })
            }));
        }
        all.sort_by_key(|m| (m.range.start, std::cmp::Reverse(m.range.end)));
        let mut out: Vec<PhraseMatch> = Vec::with_capacity(all.len());
        for m in all {
            if out.last().is_none_or(|p| p.range.end <= m.range.start) {
                out.push(m);
            }
        }
        out
    }

    /// The phrase `matched` is an inflected form of: its words up to the last as configured,
    /// the last one a stem with a valid ending ([`inflected_form`]).
    fn inflected_phrase(&self, matched: &str) -> Option<usize> {
        let (head, last) = match matched.trim_end().rfind(char::is_whitespace) {
            Some(i) => (&matched[..i], matched[i..].trim_start()),
            None => ("", matched),
        };
        let head = normalize(head);
        // Longest stem first: `Rovio Entertainment|in`.
        for (k, _) in last.char_indices().rev().filter(|&(k, _)| k > 0) {
            let key = normalize(&format!("{head} {}", &last[..k]));
            let Some(ids) = self.stems.get(&key) else {
                continue;
            };
            if let Some(&i) = ids.iter().find(|&&i| {
                self.list[i]
                    .canonical
                    .split_whitespace()
                    .last()
                    .is_some_and(|w| inflected_form(w, last))
            }) {
                return Some(i);
            }
        }
        None
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
                    || casing_ok_in(&text[m.range.clone()], &p.canonical, m.inflected)
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

    /// `m` (matching `matched`) is in a casing `prose/entity-name` reports.
    pub fn wrong_case(&self, m: &PhraseMatch, matched: &str, s: &Speller) -> bool {
        let p = &self.list[m.phrase];
        if !p.case_checked || casing_ok_in(matched, &p.canonical, m.inflected) {
            return false;
        }
        // `apple` for an alias `Apple` is the fruit, `applet` a word of its own.
        !(p.single_word
            && (spell::known(s, &p.canonical.to_lowercase())
                || m.inflected && spell::known(s, &matched.to_lowercase())))
    }
}

/// [`casing_ok`], or for an inflected form (`Rovio Entertainmentin`) its words cased as
/// configured letter by letter ([`recase_inflected`]); ALL CAPS before a colon is fine too.
pub fn casing_ok_in(matched: &str, canonical: &str, inflected: bool) -> bool {
    if !inflected {
        return casing_ok(matched, canonical);
    }
    let head = matched.split(':').next().unwrap_or(matched);
    let letters = || head.chars().filter(|c| c.is_alphabetic());
    (letters().count() > 1 && letters().all(char::is_uppercase))
        || recase_inflected(matched, canonical) == matched
        || canonical.starts_with(char::is_lowercase)
            && recase_inflected(matched, &capitalize(canonical)) == matched
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

/// An inflected form (`rovio entertainmentin`) cased like `canonical`: the words before the
/// last as configured, the last letter by letter as the configured word (`Entertainmentin`;
/// `pekan` for `Pekka` -> `Pekan`), its ending in lowercase.
pub fn recase_inflected(matched: &str, canonical: &str) -> String {
    let canon: Vec<&str> = canonical.split_whitespace().collect();
    let Some((last_canon, _)) = canon.split_last() else {
        return matched.to_string();
    };
    let last_start = matched
        .trim_end()
        .rfind(char::is_whitespace)
        .map_or(0, |i| i + 1);
    let mut out = recase(&matched[..last_start], canonical);
    let mut model = last_canon.chars();
    for ch in matched[last_start..].chars() {
        match model.next() {
            Some(c) if c.is_uppercase() => out.extend(ch.to_uppercase()),
            _ => out.extend(ch.to_lowercase()),
        }
    }
    out
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

// `[[person]]`

/// Built-in placeholder names, accepted everywhere with `[people] placeholders = true`.
pub const PLACEHOLDER_NAMES: &[&str] = &[
    "Alice",
    "Bob",
    "Carol",
    "Dave",
    "Eve",
    "Mallory",
    "Trent",
    "Peggy",
    "Victor",
    "John Doe",
    "Jane Doe",
    "John Smith",
    "Joe Bloggs",
    "Max Mustermann",
    "Erika Mustermann",
    "Matti Meikäläinen",
    "Maija Meikäläinen",
    "Teppo Testaaja",
    "Sven Svensson",
    "Anna Andersson",
    "Kalle Anka",
];

/// Which inflections [`PersonIndex::base`] reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameLang {
    /// English possessive only (`Sami's`).
    English,
    /// Finnish case endings on name parts (`Samin`, `Virtaselle`), and the possessive.
    Finnish,
    /// Swedish genitive `-s` (`Samis`), and the possessive.
    Swedish,
    /// All of them (`prose/ambiguous-person`).
    Any,
}

impl NameLang {
    pub fn from_code(code: &str) -> NameLang {
        match code {
            "fi" => NameLang::Finnish,
            "sv" => NameLang::Swedish,
            _ => NameLang::English,
        }
    }
}

/// One `[[person]]`, split into the words that name them.
#[derive(Debug, Clone)]
pub struct PersonNames {
    pub name: String,
    pub role: String,
    /// Words of the full name (`Sami`, `Virtanen`).
    pub parts: Vec<String>,
    /// Words of each alias, trailing dots dropped (`Sami`, `V`).
    pub aliases: Vec<Vec<String>>,
    /// Handles without a leading `@`.
    pub handles: Vec<String>,
}

/// `[[person]]` names and placeholder names: the words the spell check accepts (case-sensitive,
/// or ALL CAPS) with their inflections, and which persons each word can mean.
#[derive(Debug, Default)]
pub struct PersonIndex {
    pub persons: Vec<PersonNames>,
    /// Accepted word as written -> persons it names (empty for placeholder names).
    words: HashMap<String, Vec<usize>>,
    /// ALL CAPS form -> accepted word.
    upper: HashMap<String, String>,
}

/// Words of a name or alias: split at whitespace, surrounding punctuation dropped.
fn name_words(s: &str) -> Vec<String> {
    s.split_whitespace()
        .map(|w| {
            w.trim_matches(|c: char| !c.is_alphanumeric() && c != '@')
                .to_string()
        })
        .filter(|w| !w.is_empty())
        .collect()
}

impl PersonIndex {
    pub fn new(persons: &[crate::config::Person], placeholders: bool) -> PersonIndex {
        let mut idx = PersonIndex::default();
        for (i, p) in persons.iter().enumerate() {
            let names = PersonNames {
                name: p.name.split_whitespace().collect::<Vec<_>>().join(" "),
                role: p.role.trim().to_string(),
                parts: name_words(&p.name),
                aliases: p.aliases.iter().map(|a| name_words(a)).collect(),
                handles: p
                    .handles
                    .iter()
                    .map(|h| h.trim().trim_start_matches('@').to_string())
                    .filter(|h| !h.is_empty())
                    .collect(),
            };
            let mut words: Vec<String> = names.parts.clone();
            words.extend(names.aliases.iter().flatten().cloned());
            for w in words {
                // Initials (`V.`) name nobody on their own.
                if w.chars().count() >= 2 {
                    idx.add(&w, Some(i));
                }
            }
            for h in &names.handles {
                idx.add(h, Some(i));
                idx.add(&format!("@{h}"), Some(i));
            }
            idx.persons.push(names);
        }
        if placeholders {
            for w in PLACEHOLDER_NAMES.iter().flat_map(|n| n.split_whitespace()) {
                idx.add(w, None);
            }
        }
        idx
    }

    /// Accept `word` (and its hyphenated parts: `Anna-Liisa`), naming `person`.
    fn add(&mut self, word: &str, person: Option<usize>) {
        let mut all = vec![word.to_string()];
        if word.contains('-') {
            all.extend(
                word.split('-')
                    .filter(|p| p.chars().count() >= 2)
                    .map(String::from),
            );
        }
        for w in all {
            let entry = self.words.entry(w.clone()).or_default();
            if let Some(i) = person
                && !entry.contains(&i)
            {
                entry.push(i);
            }
            self.upper.entry(w.to_uppercase()).or_insert(w.clone());
        }
    }

    pub fn is_empty(&self) -> bool {
        self.words.is_empty()
    }

    /// Persons `word` (an accepted word, see [`Self::base`]) can mean.
    pub fn persons_of(&self, word: &str) -> &[usize] {
        self.words.get(word).map_or(&[], Vec::as_slice)
    }

    /// The accepted word `word` is a form of, in `lang`: as written or in ALL CAPS, with an
    /// English possessive (`Sami's`, `Niklas'`), a Swedish genitive (`Samis`) or a Finnish case
    /// ending (`Samin`, `Virtaselle`, `Pekalle`).
    pub fn base<'a>(&'a self, word: &str, lang: NameLang) -> Option<&'a str> {
        if let Some(b) = self.plain(word) {
            return Some(b);
        }
        if let Some(b) = ["'s", "’s", "'", "’"]
            .iter()
            .find_map(|p| word.strip_suffix(p))
            .and_then(|w| self.plain(w))
        {
            return Some(b);
        }
        if matches!(lang, NameLang::Swedish | NameLang::Any)
            && let Some(b) = word.strip_suffix('s').and_then(|w| self.plain(w))
        {
            return Some(b);
        }
        if matches!(lang, NameLang::Finnish | NameLang::Any) && word.starts_with(char::is_uppercase)
        {
            // Same first letter, then a stem and a case ending ([`inflected_form`]).
            let first = word.chars().next();
            return self
                .words
                .keys()
                .filter(|w| w.chars().next() == first && w.chars().count() >= 3)
                .find(|w| inflected_form(w, word))
                .map(String::as_str);
        }
        None
    }

    fn plain<'a>(&'a self, word: &str) -> Option<&'a str> {
        if let Some((k, _)) = self.words.get_key_value(word) {
            return Some(k);
        }
        let letters = || word.chars().filter(|c| c.is_alphabetic());
        if letters().count() > 1 && letters().all(char::is_uppercase) {
            return self.upper.get(word).map(String::as_str);
        }
        None
    }
}

/// Stems a Finnish case ending attaches to on the name `w` (compared against
/// [`crate::rules::spell_lang::finnish_stems`] of a word): the name itself, its weak grade
/// (`Pekka` -> `Peka`, `Lehto` -> `Lehdo`, `Mäki` -> `Mäe`) and the `-nen` / `-s` stems
/// (`Virtanen` -> `Virtase`, `Virtas`; `Markus` -> `Markukse`).
pub fn finnish_name_stems(w: &str) -> Vec<String> {
    let chars: Vec<char> = w.chars().collect();
    if chars.len() < 3 || !chars.iter().all(|c| c.is_alphabetic()) {
        return Vec::new();
    }
    let vowel = |c: char| "aeiouyäöAEIOUYÄÖ".contains(c);
    let text = |v: &[char]| v.iter().collect::<String>();
    let mut out = vec![w.to_string()];
    let n = chars.len();
    if w.ends_with("nen") && n >= 5 {
        out.push(format!("{}se", text(&chars[..n - 3])));
        out.push(format!("{}s", text(&chars[..n - 3])));
    } else if chars[n - 1] == 's' && vowel(chars[n - 2]) {
        out.push(format!("{}kse", text(&chars[..n - 1])));
    }
    // Weak grade of the consonants before the final vowel(s).
    let tail = chars.iter().rev().take_while(|&&c| vowel(c)).count();
    if tail == 0 || tail == n {
        return out;
    }
    let body = &chars[..n - tail];
    let mut finals = vec![text(&chars[n - tail..])];
    // Old `i` stems: `Mäki` -> `Mäe-`, `Lahti` -> `Lahde-`.
    if tail == 1 && chars[n - 1] == 'i' {
        finals.push("e".into());
    }
    const GRADES: &[(&str, &str)] = &[
        ("kk", "k"),
        ("pp", "p"),
        ("tt", "t"),
        ("nk", "ng"),
        ("mp", "mm"),
        ("lt", "ll"),
        ("nt", "nn"),
        ("rt", "rr"),
        ("ht", "hd"),
        ("k", ""),
        ("p", "v"),
        ("t", "d"),
    ];
    let b = text(body);
    for (strong, weak) in GRADES {
        if let Some(head) = b.strip_suffix(strong) {
            // A single consonant grades only after a vowel (`Aho` keeps its `h`).
            if strong.len() == 1 && !head.chars().next_back().is_some_and(vowel) {
                continue;
            }
            for f in &finals {
                out.push(format!("{head}{weak}{f}"));
            }
            break;
        }
    }
    out.retain(|s| s.chars().count() >= 3);
    out
}

// Inflected names, shared by `[[person]]`, `[[entity]]` and `[[vocab]]` terms: Finnish case
// endings (`Rovio Entertainmentin`, `Nokiassa`, `Virtaselle`, `API:ssa`), the Swedish
// genitive `-s`. (The English possessive `'s` is read where the apostrophe is.)

/// Finnish endings after a vowel stem (`Nokia|n`, `Nokia|ssa`), `A` for `a`/`ä`: singular
/// cases, plural `-t`/`-jA`/`-jen`, and the plural `i` stem with a case
/// (`Rovio|issa`). Illatives (`Nokiaan`, `Wikihin`, `Koneeseen`) are built in [`fi_core_ok`].
const FI_VOWEL_CASES: &[&str] = &[
    "n", "A", "tA", "nA", "ssA", "stA", "llA", "ltA", "lle", "ksi", "ttA", "t", "jA", "jen",
    "issA", "istA", "illA", "iltA", "ille", "iksi", "inA", "ihin", "iden", "itten", "itA",
];
/// Endings after the linking `i` of a consonant stem (`Labs|i|ssa`, `Linux|i|in`).
const FI_LINKED_CASES: &[&str] = &[
    "n", "A", "ssA", "stA", "llA", "ltA", "lle", "ksi", "nA", "ttA", "t", "in", "en",
];
/// Endings straight after a consonant stem (`Markus|ta`, `Virtas|ta`, `Virtas|ten`).
const FI_CONSONANT_CASES: &[&str] = &["tA", "ten"];
/// Possessive suffixes after a case ending (`Nokiassa|mme`, `Nokiassa|an`).
const FI_POSSESSIVE: &[&str] = &["ni", "si", "nsA", "mme", "nne", "An"];
/// Clitics, last (`Nokiassa|kin`, `Nokian|han`).
const FI_CLITICS: &[&str] = &["kin", "kAAn", "hAn", "pA", "kO"];

fn fi_vowel(c: char) -> bool {
    "aeiouyäö".contains(c)
}

/// `template` with back (`a`, `o`) or front (`ä`, `ö`) vowels for `A` and `O`.
fn harmonize(template: &str, back: bool) -> String {
    let (a, o) = if back { ("a", "o") } else { ("ä", "ö") };
    template.replace('A', a).replace('O', o)
}

/// Vowel harmony Finnish endings on `name` may take (back, front): back after `a`, `o`, `u`
/// (`Labsissa`), front after `ä`, `ö`, `y` (`Wärtsilässä`), either with neither (`Excel`).
fn fi_harmony(name: &str) -> (bool, bool) {
    let l = name.to_lowercase();
    let back = l.contains(['a', 'o', 'u']);
    let front = l.contains(['ä', 'ö', 'y']);
    if back || front {
        (back, front)
    } else {
        (true, true)
    }
}

fn fi_in(core: &str, templates: &[&str], back: bool, front: bool) -> bool {
    templates
        .iter()
        .any(|t| (back && harmonize(t, true) == core) || (front && harmonize(t, false) == core))
}

/// `ending` (lowercase) without an optional clitic and possessive suffix: the case endings
/// it may be.
fn fi_cores(ending: &str, back: bool, front: bool) -> Vec<String> {
    let optional = |ts: &[&str]| {
        let mut v = vec![String::new()];
        for t in ts {
            for (b, on) in [(true, back), (false, front)] {
                if on && !v.contains(&harmonize(t, b)) {
                    v.push(harmonize(t, b));
                }
            }
        }
        v
    };
    let mut out = Vec::new();
    for c in optional(FI_CLITICS) {
        let Some(rest) = ending.strip_suffix(c.as_str()) else {
            continue;
        };
        for p in optional(FI_POSSESSIVE) {
            if let Some(core) = rest.strip_suffix(p.as_str())
                && !core.is_empty()
            {
                out.push(core.to_string());
            }
        }
    }
    out
}

/// `core` is a Finnish case ending on a stem ending in `last`.
fn fi_core_ok(last: char, core: &str, back: bool, front: bool) -> bool {
    if fi_vowel(last) {
        fi_in(core, FI_VOWEL_CASES, back, front)
            || core == format!("{last}n")
            || core == format!("h{last}n")
            || core == "seen"
            || core == "siin"
    } else {
        core.strip_prefix('i')
            .is_some_and(|r| fi_in(r, FI_LINKED_CASES, back, front))
            || fi_in(core, FI_CONSONANT_CASES, back, front)
    }
}

/// Stems Finnish endings attach to on the name `w`: [`finnish_name_stems`] of its last
/// hyphenated part (`Z-alusta`), and a long `e` stem (`Kone` -> `Konee|ssa`).
pub fn name_stems(w: &str) -> Vec<String> {
    let (head, last) = match w.rsplit_once('-') {
        Some((h, l)) => (format!("{h}-"), l),
        None => (String::new(), w),
    };
    let mut stems = finnish_name_stems(last);
    if stems.is_empty() {
        stems.push(last.to_string());
    }
    if last.ends_with(['e', 'E']) && last.chars().count() >= 3 {
        stems.push(format!("{last}e"));
    }
    stems.into_iter().map(|s| format!("{head}{s}")).collect()
}

/// `form` is the name `name` (a word, case-insensitively) with an ending: a Finnish case
/// ending with vowel harmony, possibly with a possessive suffix and a clitic (`Labsin`,
/// `Labsilla`, `Labsiin`, `Nokiaan`, `Pekalle`, `Virtaselle`, `Z-alustaan`), one after a
/// colon for abbreviations and names ending in a digit (`API:ssa`, `IHP:n`), or the Swedish
/// genitive `-s` (`Nokias`). Not an arbitrary suffix (`Labsxyz`).
pub fn inflected_form(name: &str, form: &str) -> bool {
    let (name_l, form_l) = (name.to_lowercase(), form.to_lowercase());
    if let Some(rest) = form_l.strip_prefix(name_l.as_str()) {
        if let Some(ending) = rest.strip_prefix(':') {
            return !ending.is_empty()
                && ending.chars().all(char::is_lowercase)
                && fi_cores(ending, true, true).iter().any(|c| {
                    "aeiouyäö".chars().any(|v| fi_core_ok(v, c, true, true))
                        || fi_core_ok('x', c, true, true)
                });
        }
        if rest == "s" {
            return !name_l.ends_with(['s', 'x', 'z']);
        }
    }
    // Abbreviations take their endings after a colon.
    let letters = || name.chars().filter(|c| c.is_alphabetic());
    if name.ends_with(|c: char| c.is_ascii_digit())
        || letters().count() > 1 && letters().all(char::is_uppercase)
    {
        return false;
    }
    let (back, front) = fi_harmony(name);
    name_stems(name).iter().any(|stem| {
        let stem = stem.to_lowercase();
        let Some(last) = stem.chars().last() else {
            return false;
        };
        form_l.strip_prefix(stem.as_str()).is_some_and(|ending| {
            !ending.is_empty()
                && fi_cores(ending, back, front)
                    .iter()
                    .any(|c| fi_core_ok(last, c, back, front))
        })
    })
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

/// One suggested `[[vocab]]`, `[[entity]]` or `[[person]]` entry.
#[derive(Debug, Clone, Serialize)]
pub struct Suggestion {
    /// `vocab`, `entity` or `person`.
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
        let mut cues: Vec<(String, Option<&'static str>)> = Vec::new();
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
            cues.push((normalize(&phrase), context_cue(text, range.clone())));
            found.push((range, phrase));
        }
        (
            (
                a.file.rel.clone(),
                found.into_iter().map(|(_, p)| p).collect(),
            ),
            cues,
        )
    });
    // phrase -> (occurrences, with a company cue, with a product cue)
    let mut cue_counts: HashMap<String, (usize, usize, usize)> = HashMap::new();
    for (phrase, cue) in per_file.iter().flat_map(|(_, c)| c) {
        let e = cue_counts.entry(phrase.clone()).or_default();
        e.0 += 1;
        match cue {
            Some("company") => e.1 += 1,
            Some(_) => e.2 += 1,
            None => {}
        }
    }
    let fi = crate::rules::spell_lang::speller("fi", &cfg);
    let sv = crate::rules::spell_lang::speller("sv", &cfg);
    let configured = configured_words(config);
    let dictionary_word = |w: &str| {
        let lower = w.to_lowercase();
        spell::known(speller, &lower)
            || spell::known(other, &lower)
            || [&fi, &sv].into_iter().flatten().any(|sp| sp.check(&lower))
    };
    let names = NameCheck {
        dictionary_word: &dictionary_word,
        english_name: &|w: &str| spell::known(speller, w),
        configured: &|w: &str| configured.contains(&w.to_lowercase()),
    };
    let classify = |term: &str| {
        let (n, company, product) = cue_counts
            .get(&normalize(term))
            .copied()
            .unwrap_or_default();
        // A legal form or product word next to a third of the uses or more.
        if company > 0 && company * 3 >= n {
            return ("entity", "company");
        }
        if product > 0 && product * 3 >= n {
            return ("entity", "product");
        }
        classify_name(term, &names)
    };
    collect(
        per_file.into_iter().map(|(f, _)| f).collect(),
        min_count,
        &classify,
    )
}

/// Rank the phrases found per file into suggestions; single words that are a company name
/// without its suffix become that entity's alias, and name parts count for the person
/// (`classify` gives the section and kind of a multi-word run without a legal suffix).
fn collect(
    per_file: Vec<(PathBuf, Vec<String>)>,
    min_count: usize,
    classify: &dyn Fn(&str) -> (&'static str, &'static str),
) -> Vec<Suggestion> {
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
            let (section, kind) = if !multi {
                ("vocab", String::new())
            } else if LEGAL_SUFFIXES.contains(&last) {
                ("entity", "company".to_string())
            } else {
                let (section, kind) = classify(term);
                (section, kind.to_string())
            };
            let mut files: Vec<FileCount> = files
                .into_iter()
                .map(|(path, count)| FileCount { path, count })
                .collect();
            files.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.path.cmp(&b.path)));
            Some(Suggestion {
                section,
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
    // (target, merged single word): `Telia` becomes an alias of `Telia Oy`; `Nykänen` counts
    // for `Matti Nykänen` (name parts are accepted on their own).
    let mut pairs: Vec<(usize, usize)> = Vec::new();
    for (t, s) in out.iter().enumerate() {
        let parts: Vec<&str> = if s.kind == "company" {
            s.term
                .rsplit_once(' ')
                .map(|(b, _)| b)
                .into_iter()
                .collect()
        } else if s.section == "person" {
            s.term.split(' ').collect()
        } else {
            continue;
        };
        for p in parts {
            if let Some((i, _)) = words.iter().find(|(_, w)| w == p)
                && !pairs.iter().any(|(_, m)| m == i)
            {
                pairs.push((t, *i));
            }
        }
    }
    for &(t, i) in &pairs {
        let (term, n, files) = (out[i].term.clone(), out[i].count, out[i].files.clone());
        let s = &mut out[t];
        if s.kind == "company" {
            s.aliases.push(term);
        }
        s.count += n;
        for f in files {
            match s.files.iter_mut().find(|x| x.path == f.path) {
                Some(x) => x.count += f.count,
                None => s.files.push(f),
            }
        }
        s.files
            .sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.path.cmp(&b.path)));
    }
    let merged: Vec<usize> = pairs.iter().map(|&(_, i)| i).collect();
    let mut out: Vec<Suggestion> = out
        .into_iter()
        .enumerate()
        .filter(|(i, s)| !merged.contains(i) && s.count >= min_count)
        .map(|(_, s)| s)
        .collect();
    let rank = |s: &Suggestion| match s.section {
        "entity" => 0,
        "person" => 1,
        _ => 2,
    };
    out.sort_by(|a, b| {
        (rank(a), std::cmp::Reverse(a.count), &a.term).cmp(&(
            rank(b),
            std::cmp::Reverse(b.count),
            &b.term,
        ))
    });
    out
}

/// What [`classify_name`] asks about a word.
struct NameCheck<'a> {
    /// A word of English, Finnish or Swedish in lowercase (`stora`, `tunnistus`).
    dictionary_word: &'a dyn Fn(&str) -> bool,
    /// Known to the English dictionary as written (`Microsoft`).
    english_name: &'a dyn Fn(&str) -> bool,
    /// A word of a configured `[[entity]]` or `[[vocab]]` entry.
    configured: &'a dyn Fn(&str) -> bool,
}

/// Section and kind for a run of capitalized words: `person` for two or three name-shaped
/// words that start with a common first name (`Matti Virtanen`) or, without one, are words of
/// no dictionary and end in a surname ending (`Sami Blenkiss`); otherwise an entity, a
/// product when a part is a tech or brand word, or has a hyphen or digit (`Microsoft Entra`,
/// `Acme X-alusta`).
fn classify_name(phrase: &str, c: &NameCheck) -> (&'static str, &'static str) {
    let words: Vec<&str> = phrase.split(' ').collect();
    let product_part = |w: &&str| {
        w.contains(|ch: char| ch.is_ascii_digit())
            || (w.contains('-') && !w.split('-').all(|p| FIRST_NAMES.contains(&p)))
            || PRODUCT_WORDS.contains(w)
            || crate::rules::grammar::TECH_WORDS
                .iter()
                .any(|t| t.eq_ignore_ascii_case(w))
    };
    if words.iter().any(product_part) {
        return ("entity", "product");
    }
    let shaped = |w: &&str| {
        w.split('-').all(|part| {
            let mut chars = part.chars();
            chars.next().is_some_and(char::is_uppercase)
                && part.chars().count() >= 2
                && chars.all(|ch| ch.is_lowercase() || matches!(ch, '\'' | '’'))
        })
    };
    let entity = ("entity", "");
    if !(2..=3).contains(&words.len())
        || !words.iter().all(shaped)
        || words
            .iter()
            .any(|w| (c.configured)(w) || LEGAL_SUFFIXES.contains(w))
    {
        return entity;
    }
    let first_name = |w: &&str| w.split('-').all(|p| FIRST_NAMES.contains(&p));
    let (first, rest) = (words[0], &words[1..]);
    let person = if first_name(&first) {
        // `Anna Maria Virtanen`; a surname may be a Finnish or Swedish word (`Laine`), not an
        // English one (`Alexa Skills`).
        rest.iter().all(|w| {
            first_name(w) || !spell::known(spell::dictionary("american"), &w.to_lowercase())
        })
    } else {
        let last = words[words.len() - 1].to_lowercase();
        !(c.english_name)(first)
            && words.iter().all(|w| !(c.dictionary_word)(w))
            && SURNAME_ENDINGS.iter().any(|e| last.ends_with(e))
    };
    if person { ("person", "") } else { entity }
}

/// Lowercase words of configured `[[entity]]` and `[[vocab]]` names and aliases.
fn configured_words(config: &Config) -> std::collections::HashSet<String> {
    let scopes = std::iter::once((&config.vocab, &config.entities))
        .chain(config.overrides.iter().map(|o| (&o.vocab, &o.entities)));
    let mut out = std::collections::HashSet::new();
    for (vocab, entities) in scopes {
        let names = vocab
            .iter()
            .flat_map(|v| std::iter::once(&v.term).chain(&v.aliases))
            .chain(
                entities
                    .iter()
                    .flat_map(|e| std::iter::once(&e.name).chain(&e.aliases)),
            );
        for n in names {
            out.extend(n.split_whitespace().map(str::to_lowercase));
        }
    }
    out
}

/// Words near a found name (one before, two after, same line) that make it a company
/// (`Oyj`, `yhtiö`) or a product (`API`, `app`, `-palvelu`, `alustalla`).
fn context_cue(text: &str, r: Range<usize>) -> Option<&'static str> {
    const COMPANY: &[&str] = &[
        "oy",
        "oyj",
        "ab",
        "ltd",
        "inc",
        "gmbh",
        "llc",
        "corp",
        "plc",
        "company",
        "corporation",
    ];
    const COMPANY_STEMS: &[&str] = &["yhtiö", "yritys", "yrityk", "företag", "bolag", "konserni"];
    const PRODUCT: &[&str] = &[
        "api",
        "apis",
        "app",
        "apps",
        "sdk",
        "service",
        "services",
        "platform",
        "platforms",
        "portal",
        "plugin",
        "product",
        "integration",
        "cloud",
        "suite",
    ];
    const PRODUCT_STEMS: &[&str] = &[
        "palvelu",
        "alusta",
        "sovellu",
        "rajapin",
        "järjestelm",
        "integraati",
        "portaali",
        "tuote",
        "tuotte",
        "työkalu",
        "tjänst",
        "plattform",
        "applikation",
        "verktyg",
    ];
    let line_start = text[..r.start].rfind('\n').map_or(0, |i| i + 1);
    let line_end = text[r.end..].find('\n').map_or(text.len(), |i| r.end + i);
    let words = |s: &str| -> Vec<String> {
        s.split(|ch: char| !ch.is_alphanumeric())
            .filter(|w| !w.is_empty())
            .map(str::to_lowercase)
            .collect()
    };
    let before = words(&text[line_start..r.start]);
    let after = words(&text[r.end..line_end]);
    let near: Vec<&String> = before
        .iter()
        .rev()
        .take(1)
        .chain(after.iter().take(2))
        .collect();
    let is = |w: &str, exact: &[&str], stems: &[&str]| {
        exact.contains(&w) || stems.iter().any(|s| w.starts_with(s))
    };
    if near.iter().any(|w| is(w, COMPANY, COMPANY_STEMS)) {
        Some("company")
    } else if near.iter().any(|w| is(w, PRODUCT, PRODUCT_STEMS)) {
        Some("product")
    } else {
        None
    }
}

/// Brand and product words that lead or end product names (`Microsoft Entra`, `Google Maps`).
const PRODUCT_WORDS: &[&str] = &[
    "Microsoft",
    "Google",
    "Apple",
    "Amazon",
    "AWS",
    "Azure",
    "Entra",
    "Office",
    "Teams",
    "Windows",
    "Android",
    "Meta",
    "Oracle",
    "IBM",
    "SAP",
    "Salesforce",
    "Adobe",
    "Slack",
    "Zoom",
    "Atlassian",
    "Jira",
    "Confluence",
    "GitHub",
    "GitLab",
    "Docker",
    "Kubernetes",
    "Firebase",
    "Cloudflare",
    "Vercel",
    "Netlify",
    "Heroku",
    "Stripe",
    "Twilio",
    "Okta",
    "Auth0",
    "Figma",
    "Notion",
    "Linear",
    "Sentry",
    "Datadog",
    "Grafana",
    "Kela",
    "Kanta",
    "Suomi",
    "Sweden",
    "Finland",
    "Cloud",
    "Hub",
    "Pay",
    "Maps",
    "Drive",
    "Store",
    "Studio",
    "Connect",
    "Suite",
];

/// Common surname endings (Finnish, Swedish, English), for names without a known first name.
const SURNAME_ENDINGS: &[&str] = &[
    "nen", "la", "lä", "sson", "son", "sen", "berg", "ström", "strom", "lund", "qvist", "kvist",
    "gren", "mäki", "niemi", "lahti", "koski", "salo", "vaara", "kangas", "lammi", "lampi",
    "järvi", "joki", "ranta", "harju", "korpi", "aho", "oja", "puro", "ley", "ton", "ford", "man",
    "ez", "ski", "ska", "ov", "ova", "ic", "ić",
];

/// Common given names (Finnish, Swedish, English), about 300: a run starting with one reads as
/// a person (`Sami Virtanen`), even when the name is also a word (`Onni`, `Tuuli`, `Mark`).
pub const FIRST_NAMES: &[&str] = &[
    // Finnish
    "Aada",
    "Aapo",
    "Aarne",
    "Aaro",
    "Aino",
    "Aki",
    "Aleksi",
    "Anna",
    "Anne",
    "Anni",
    "Antero",
    "Antti",
    "Ari",
    "Arja",
    "Arto",
    "Eero",
    "Eetu",
    "Eija",
    "Eila",
    "Eino",
    "Elina",
    "Elisa",
    "Ella",
    "Emilia",
    "Emma",
    "Erkki",
    "Esa",
    "Esko",
    "Hanna",
    "Hannu",
    "Harri",
    "Heikki",
    "Heli",
    "Helena",
    "Helmi",
    "Henna",
    "Henri",
    "Hilkka",
    "Iida",
    "Ilkka",
    "Ilmari",
    "Inkeri",
    "Irma",
    "Jaakko",
    "Jani",
    "Janne",
    "Jari",
    "Jarmo",
    "Jarno",
    "Jenna",
    "Jenni",
    "Jere",
    "Jesse",
    "Joel",
    "Johanna",
    "Joni",
    "Jonna",
    "Jorma",
    "Jouko",
    "Jouni",
    "Juha",
    "Juhani",
    "Jukka",
    "Julia",
    "Jussi",
    "Jutta",
    "Juuso",
    "Jyrki",
    "Kaarina",
    "Kai",
    "Kaisa",
    "Kalle",
    "Kari",
    "Karoliina",
    "Katja",
    "Kati",
    "Kimmo",
    "Kirsi",
    "Kristiina",
    "Kaija",
    "Lasse",
    "Laura",
    "Leena",
    "Leo",
    "Liisa",
    "Lotta",
    "Maarit",
    "Maija",
    "Marja",
    "Marjatta",
    "Marko",
    "Markku",
    "Markus",
    "Matias",
    "Matti",
    "Merja",
    "Mika",
    "Mikael",
    "Mikko",
    "Milla",
    "Minna",
    "Mirja",
    "Mirva",
    "Niina",
    "Niko",
    "Niklas",
    "Noora",
    "Oiva",
    "Olli",
    "Onni",
    "Oona",
    "Oskari",
    "Otto",
    "Paavo",
    "Päivi",
    "Pasi",
    "Pauli",
    "Paula",
    "Pekka",
    "Pentti",
    "Petri",
    "Petteri",
    "Pia",
    "Pirjo",
    "Pirkko",
    "Raija",
    "Raimo",
    "Reijo",
    "Riikka",
    "Riitta",
    "Risto",
    "Ritva",
    "Saara",
    "Sakari",
    "Salla",
    "Sami",
    "Sanna",
    "Sari",
    "Satu",
    "Seppo",
    "Siiri",
    "Simo",
    "Sini",
    "Sirpa",
    "Sofia",
    "Sonja",
    "Suvi",
    "Taina",
    "Tapani",
    "Tapio",
    "Tarja",
    "Teemu",
    "Teija",
    "Tero",
    "Terhi",
    "Timo",
    "Tiina",
    "Tomi",
    "Tommi",
    "Toni",
    "Topi",
    "Tuomas",
    "Tuomo",
    "Tuula",
    "Tuuli",
    "Ulla",
    "Urho",
    "Vappu",
    "Veera",
    "Veikko",
    "Veli",
    "Ville",
    "Vilma",
    "Vesa",
    "Väinö",
    "Ylva",
    // Swedish
    "Agneta",
    "Alva",
    "Anders",
    "Astrid",
    "Axel",
    "Birgitta",
    "Björn",
    "Bo",
    "Carl",
    "Ebba",
    "Elin",
    "Elsa",
    "Emil",
    "Erik",
    "Filip",
    "Fredrik",
    "Gunnar",
    "Gustav",
    "Göran",
    "Hampus",
    "Hans",
    "Hugo",
    "Ingrid",
    "Isak",
    "Johan",
    "Karin",
    "Kerstin",
    "Klara",
    "Lars",
    "Leif",
    "Linnea",
    "Lovisa",
    "Magnus",
    "Malin",
    "Mats",
    "Nils",
    "Olof",
    "Oscar",
    "Per",
    "Sven",
    "Stina",
    "Tove",
    "Ulf",
    "Viktor",
    "Åsa",
    // English
    "Adam",
    "Alan",
    "Alex",
    "Alice",
    "Amanda",
    "Amy",
    "Andrew",
    "Angela",
    "Anthony",
    "Ben",
    "Benjamin",
    "Brian",
    "Charles",
    "Chris",
    "Christopher",
    "Daniel",
    "David",
    "Emily",
    "Eric",
    "Elizabeth",
    "George",
    "Hannah",
    "Henry",
    "Jack",
    "James",
    "Jane",
    "Jason",
    "Jennifer",
    "Jessica",
    "John",
    "Jonathan",
    "Joseph",
    "Joshua",
    "Karen",
    "Kate",
    "Kevin",
    "Linda",
    "Lisa",
    "Mark",
    "Mary",
    "Matthew",
    "Michael",
    "Michelle",
    "Nancy",
    "Nicholas",
    "Olivia",
    "Patricia",
    "Paul",
    "Peter",
    "Rachel",
    "Richard",
    "Robert",
    "Ryan",
    "Sarah",
    "Scott",
    "Sophie",
    "Steven",
    "Susan",
    "Thomas",
    "Timothy",
    "William",
];

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
    let persons = sugs.iter().filter(|s| s.section == "person").count();
    let mut out = format!(
        "# explicit vocab suggest: {entities} entities, {persons} persons, {} terms from \
         {files_checked} files.\n\
         # Fill in each TODO, then paste into explicit.toml.\n",
        sugs.len() - entities - persons
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
        if s.section == "person" {
            out.push_str(&format!(
                "[[person]]\nname = {}\nrole = \"TODO\"\n",
                q(&s.term)
            ));
        } else if s.section == "entity" {
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

/// One configured `[[vocab]]`, `[[entity]]` or `[[person]]` entry, for `explicit vocab list`.
#[derive(Debug, Clone, Serialize)]
pub struct ListRow {
    /// `vocab`, `entity` or `person`.
    pub section: &'static str,
    pub term: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub kind: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
    /// Description (vocab), relationship (entity) or role (person).
    pub note: String,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub case_sensitive: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Glob patterns of the `[[overrides]]` entry that adds it; empty for the top level.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub paths: Vec<String>,
}

/// Every configured entry, entities first, then persons, including those of `[[overrides]]`.
pub fn list_rows(config: &Config) -> Vec<ListRow> {
    let mut rows = Vec::new();
    let scopes = std::iter::once((&config.vocab, &config.entities, &config.persons, Vec::new()))
        .chain(
            config
                .overrides
                .iter()
                .map(|o| (&o.vocab, &o.entities, &o.persons, o.paths.clone())),
        );
    let mut person_rows = Vec::new();
    let mut vocab_rows = Vec::new();
    for (vocab, entities, persons, paths) in scopes {
        person_rows.extend(persons.iter().map(|p| ListRow {
            section: "person",
            term: p.name.clone(),
            kind: String::new(),
            aliases: p.aliases.iter().chain(&p.handles).cloned().collect(),
            note: p.role.clone(),
            case_sensitive: false,
            url: None,
            paths: paths.clone(),
        }));
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
    rows.extend(person_rows);
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
            let mut kind = if r.section == "person" {
                "person".to_string()
            } else if r.section == "entity" {
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
            .filter(|m| p.wrong_case(m, &text[m.range.clone()], s))
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
            &|_| ("entity", ""),
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
        assert_eq!(
            collect(vec![f("a.md", &["Zed"])], 2, &|_| ("entity", "")).len(),
            0
        );
    }

    #[test]
    fn finnish_name_stems_with_gradation() {
        let has = |w: &str, stem: &str| finnish_name_stems(w).iter().any(|s| s == stem);
        assert!(has("Virtanen", "Virtase") && has("Virtanen", "Virtas"));
        assert!(has("Pekka", "Peka") && has("Lehto", "Lehdo") && has("Mäki", "Mäe"));
        assert!(has("Markus", "Markukse") && has("Sami", "Sami"));
        assert!(!has("Aho", "Ao"));
        assert!(finnish_name_stems("Al").is_empty());
    }

    fn person(name: &str, aliases: &[&str], handles: &[&str]) -> crate::config::Person {
        crate::config::Person {
            name: name.into(),
            role: "Role".into(),
            aliases: aliases.iter().map(|a| a.to_string()).collect(),
            handles: handles.iter().map(|a| a.to_string()).collect(),
            ..Default::default()
        }
    }

    #[test]
    fn person_index_forms() {
        let idx = PersonIndex::new(
            &[
                person("Sami Virtanen", &["Sami V."], &["@samiv"]),
                person("Anna-Liisa Virtanen", &[], &[]),
                person("Pekka Lehto", &[], &[]),
            ],
            true,
        );
        let base = |w: &str, l: NameLang| idx.base(w, l);
        assert_eq!(base("Sami", NameLang::English), Some("Sami"));
        assert_eq!(base("SAMI", NameLang::English), Some("Sami"));
        assert_eq!(base("sami", NameLang::English), None);
        assert_eq!(base("Sami's", NameLang::English), Some("Sami"));
        assert_eq!(base("Samin", NameLang::English), None);
        assert_eq!(base("Samin", NameLang::Finnish), Some("Sami"));
        assert_eq!(base("Virtaselle", NameLang::Finnish), Some("Virtanen"));
        assert_eq!(base("Virtasta", NameLang::Finnish), Some("Virtanen"));
        assert_eq!(base("Pekalle", NameLang::Finnish), Some("Pekka"));
        assert_eq!(base("Lehdon", NameLang::Finnish), Some("Lehto"));
        assert_eq!(base("Samis", NameLang::Swedish), Some("Sami"));
        assert_eq!(base("Samis", NameLang::English), None);
        assert_eq!(base("Liisa", NameLang::English), Some("Liisa"));
        assert_eq!(base("samiv", NameLang::English), Some("samiv"));
        assert_eq!(base("Alice", NameLang::English), Some("Alice"));
        assert_eq!(base("Meikäläisen", NameLang::Finnish), Some("Meikäläinen"));
        assert_eq!(idx.persons_of("Virtanen"), [0, 1]);
        assert_eq!(idx.persons_of("Alice"), [] as [usize; 0]);
        // Initials name nobody.
        assert_eq!(base("V", NameLang::English), None);
        let none = PersonIndex::new(&[], false);
        assert!(none.is_empty() && none.base("Alice", NameLang::Any).is_none());
    }

    #[test]
    fn suggest_and_list_persons() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
        let a = root.join("a.md");
        std::fs::write(
            &a,
            "# Team\n\nThe patch is by Sami Blenkiss today. Later we thanked Blenkiss.\n\n\
             The Kvanta Hub syncs.\n",
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
        assert!(got.contains(&("person", "Sami Blenkiss", 2)), "{got:?}");
        assert!(!got.iter().any(|g| g.1 == "Blenkiss"), "{got:?}");
        // A dictionary word makes it an entity.
        assert!(got.contains(&("entity", "Kvanta Hub", 1)), "{got:?}");
        let toml_text = to_toml(&sugs, 1);
        assert!(
            toml_text.contains("[[person]]\nname = \"Sami Blenkiss\"\nrole = \"TODO\"\n"),
            "{toml_text}"
        );
        let parsed: toml::Table = toml::from_str(&toml_text).unwrap();
        assert_eq!(parsed["person"].as_array().unwrap().len(), 1);
        // Configured persons are no longer suggested, and are listed.
        let mut configured = config.clone();
        configured
            .persons
            .push(person("Sami Blenkiss", &["Q. B."], &["@qb"]));
        let sugs = suggest(&[a], &configured, 1);
        assert!(sugs.iter().all(|s| s.section != "person"), "{sugs:?}");
        let rows = list_rows(&configured);
        assert_eq!(rows[0].section, "person");
        assert_eq!(rows[0].aliases, ["Q. B.", "@qb"]);
        assert!(list_table(&rows).contains("person   Sami Blenkiss (Q. B., @qb)  person  Role"));
    }

    #[test]
    fn inflected_forms_of_names() {
        let ok = |n: &str, f: &str| inflected_form(n, f);
        // Consonant stems take a linking `i`, with vowel harmony.
        for f in [
            "Entertainmentin",
            "Entertainmentilla",
            "Entertainmentista",
            "Entertainmentiin",
            "Entertainmentille",
            "Entertainmentissa",
            "Entertainmentina",
            "Entertainmentia",
            "Entertainmentissakin",
            "Entertainmentissamme",
        ] {
            assert!(ok("Entertainment", f), "{f}");
        }
        assert!(!ok("Entertainment", "Entertainmentissä"));
        assert!(!ok("Entertainment", "Entertainmentxyz"));
        assert!(!ok("Entertainment", "Entertainment"));
        assert!(ok("Wärtsilä", "Wärtsilässä") && !ok("Wärtsilä", "Wärtsilässa"));
        // Vowel stems take endings directly; illatives lengthen the vowel.
        for f in [
            "Nokian", "Nokiassa", "Nokiasta", "Nokiaan", "Nokialle", "Nokiaa",
        ] {
            assert!(ok("Nokia", f), "{f}");
        }
        assert!(ok("Rovio", "Rovioon") && ok("Kone", "Koneeseen") && ok("Kone", "Koneen"));
        assert!(ok("Z-alusta", "Z-alustaan") && ok("Z-alusta", "Z-alustan"));
        // Gradation and `-nen` stems (persons).
        assert!(ok("Pekka", "Pekalle") && ok("Virtanen", "Virtaselle"));
        assert!(ok("Virtanen", "Virtasta") && ok("Markus", "Markuksen"));
        // Abbreviations take a colon.
        assert!(ok("API", "API:ssa") && ok("IHP", "IHP:n") && ok("IHP", "IHP:hen"));
        assert!(!ok("IHP", "IHPn") && !ok("IHP", "IHP:xyz"));
        // Swedish genitive.
        assert!(ok("Nokia", "Nokias") && !ok("Labs", "Labss"));
    }

    #[test]
    fn inflected_phrases() {
        let e = [
            entity("Rovio Entertainment", &[]),
            entity("Kone Oyj", &["Kone", "KNX"]),
        ];
        let p = Phrases::new(&[], &e);
        let text = "Rovio Entertainmentin huoli. rovio entertainmentilla, Rovio Entertainmentxyz, \
                    Kone Oyj:n ja Koneessa, KNX:ssa. Kone Oyjxyz.";
        let found: Vec<(&str, bool)> = p
            .find(text)
            .iter()
            .map(|m| (&text[m.range.clone()], m.inflected))
            .collect();
        assert_eq!(
            found,
            [
                ("Rovio Entertainmentin", true),
                ("rovio entertainmentilla", true),
                ("Kone Oyj:n", true),
                ("Koneessa", true),
                ("KNX:ssa", true),
                ("Kone", false),
            ]
        );
        assert_eq!(
            recase_inflected("rovio entertainmentilla", "Rovio Entertainment"),
            "Rovio Entertainmentilla"
        );
        assert_eq!(recase_inflected("knx:ssa", "KNX"), "KNX:ssa");
        assert!(casing_ok_in("KNX:ssa", "KNX", true));
        assert!(!casing_ok_in(
            "rovio entertainmentilla",
            "Rovio Entertainment",
            true
        ));
    }

    fn check_names() -> impl Fn(&str) -> (&'static str, &'static str) {
        let en = spell::dictionary("american");
        let config = Config::default();
        let fi = crate::rules::spell_lang::speller("fi", &config);
        let sv = crate::rules::spell_lang::speller("sv", &config);
        move |phrase: &str| {
            let dictionary_word = |w: &str| {
                let lower = w.to_lowercase();
                spell::known(en, &lower)
                    || [&fi, &sv].into_iter().flatten().any(|s| s.check(&lower))
            };
            let names = NameCheck {
                dictionary_word: &dictionary_word,
                english_name: &|w: &str| spell::known(en, w),
                configured: &|w: &str| w == "Zelkia",
            };
            classify_name(phrase, &names)
        }
    }

    #[test]
    fn products_and_companies_are_no_persons() {
        let classify = check_names();
        for p in [
            "Microsoft Entra",
            "Telia Tunnistus",
            "Pharmaca Fennica",
            "Stora Enso",
            "Acme X-alusta",
            "Zelkia Qorvinen",
            "Google Maps",
            "Qorvo 365",
            "Alexa Skills",
        ] {
            assert_eq!(classify(p).0, "entity", "{p}");
        }
        assert_eq!(classify("Microsoft Entra"), ("entity", "product"));
        assert_eq!(classify("Acme X-alusta"), ("entity", "product"));
        for p in [
            "Sami Qorvalammi",
            "Antti Zelkamo",
            "Jarno Blenkikangas",
            "Matti Laine",
            "Anna-Liisa Qorvanen",
            "Sami Blenkiss",
        ] {
            assert_eq!(classify(p), ("person", ""), "{p}");
        }
    }

    #[test]
    fn suggest_uses_context_cues() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
        let a = root.join("a.md");
        std::fs::write(
            &a,
            "# Notes\n\nWe call the Zorblat Quemm app daily. The code is by Sami Qorvalammi \
             today. Sales moved to Zorbex Quenti company rules.\n",
        )
        .unwrap();
        let config = Config {
            root,
            ..Config::default()
        };
        let sugs = suggest(&[a], &config, 1);
        let got: Vec<(&str, &str, &str)> = sugs
            .iter()
            .map(|s| (s.section, s.term.as_str(), s.kind.as_str()))
            .collect();
        assert!(
            got.contains(&("entity", "Zorblat Quemm", "product")),
            "{got:?}"
        );
        assert!(
            got.contains(&("entity", "Zorbex Quenti", "company")),
            "{got:?}"
        );
        assert!(got.contains(&("person", "Sami Qorvalammi", "")), "{got:?}");
    }
}
