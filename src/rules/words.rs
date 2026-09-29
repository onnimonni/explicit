//! Harper's curated English dictionary, vendored: every word (affix-expanded, as Harper's
//! `MutableDictionary::curated()` holds it) with its dialects and the word-class bits our spell
//! check ([`super::spell`]) and pattern rules ([`super::patterns`]) ask about.
//!
//! `dictionaries/harper/words.tsv.zlib` (zlib-compressed TSV) is generated from harper-core by
//! `scripts/update-harper-words.sh` (Apache-2.0, see `NOTICE`), so every build gives the same
//! answers with or without the `harper` feature. Lookups follow Harper's: words are keyed by
//! their lowercased form with curly quotes and dashes normalized, and an *exact* match also
//! needs the dictionary's own capitalization.

use std::collections::HashMap;
use std::sync::LazyLock;

/// The dialects of `crate::config` / Harper, in bit order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Dialect {
    American,
    Canadian,
    Australian,
    British,
    Indian,
}

impl Dialect {
    pub const ALL: [Dialect; 5] = [
        Dialect::American,
        Dialect::Canadian,
        Dialect::Australian,
        Dialect::British,
        Dialect::Indian,
    ];

    fn bit(self) -> u32 {
        1 << self as u32
    }

    #[cfg(feature = "harper")]
    pub fn to_harper(self) -> harper_core::Dialect {
        match self {
            Dialect::American => harper_core::Dialect::American,
            Dialect::Canadian => harper_core::Dialect::Canadian,
            Dialect::Australian => harper_core::Dialect::Australian,
            Dialect::British => harper_core::Dialect::British,
            Dialect::Indian => harper_core::Dialect::Indian,
        }
    }
}

/// Word-class bits after the five dialect bits, each one of Harper's `DictWordMetadata`
/// queries (`is_noun`, `is_countable_noun`, ...). Names in the file header follow this order.
#[cfg(feature = "harper")]
const CLASSES: &[&str] = &[
    "noun",
    "verb",
    "adjective",
    "adverb",
    "pronoun",
    "determiner",
    "preposition",
    "conjunction",
    "verb_progressive",
    "verb_past",
    "verb_past_participle",
    "verb_third_person_singular_present",
    "verb_simple_past",
    "plural_noun_only",
    "non_singular_noun",
    "countable_noun",
    "mass_noun_only",
];

const NOUN: u32 = 1 << 5;
const VERB: u32 = 1 << 6;
const ADJECTIVE: u32 = 1 << 7;
const ADVERB: u32 = 1 << 8;
const PRONOUN: u32 = 1 << 9;
const DETERMINER: u32 = 1 << 10;
const PREPOSITION: u32 = 1 << 11;
const CONJUNCTION: u32 = 1 << 12;
const VERB_PROGRESSIVE: u32 = 1 << 13;
const VERB_PAST: u32 = 1 << 14;
const VERB_PAST_PARTICIPLE: u32 = 1 << 15;
const VERB_THIRD_PERSON_SINGULAR_PRESENT: u32 = 1 << 16;
const VERB_SIMPLE_PAST: u32 = 1 << 17;
const PLURAL_NOUN_ONLY: u32 = 1 << 18;
const NON_SINGULAR_NOUN: u32 = 1 << 19;
const COUNTABLE_NOUN: u32 = 1 << 20;
const MASS_NOUN_ONLY: u32 = 1 << 21;

/// What the dictionary says about one word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Meta(u32);

impl Meta {
    pub fn dialect_enabled(self, d: Dialect) -> bool {
        self.0 & d.bit() != 0
    }
    fn has(self, bit: u32) -> bool {
        self.0 & bit != 0
    }
    pub fn is_noun(self) -> bool {
        self.has(NOUN)
    }
    pub fn is_verb(self) -> bool {
        self.has(VERB)
    }
    pub fn is_adjective(self) -> bool {
        self.has(ADJECTIVE)
    }
    pub fn is_adverb(self) -> bool {
        self.has(ADVERB)
    }
    pub fn is_pronoun(self) -> bool {
        self.has(PRONOUN)
    }
    pub fn is_determiner(self) -> bool {
        self.has(DETERMINER)
    }
    pub fn is_preposition(self) -> bool {
        self.has(PREPOSITION)
    }
    pub fn is_conjunction(self) -> bool {
        self.has(CONJUNCTION)
    }
    pub fn is_verb_progressive_form(self) -> bool {
        self.has(VERB_PROGRESSIVE)
    }
    pub fn is_verb_past_form(self) -> bool {
        self.has(VERB_PAST)
    }
    pub fn is_verb_past_participle_form(self) -> bool {
        self.has(VERB_PAST_PARTICIPLE)
    }
    pub fn is_verb_third_person_singular_present_form(self) -> bool {
        self.has(VERB_THIRD_PERSON_SINGULAR_PRESENT)
    }
    pub fn is_verb_simple_past_form(self) -> bool {
        self.has(VERB_SIMPLE_PAST)
    }
    pub fn is_plural_noun_only(self) -> bool {
        self.has(PLURAL_NOUN_ONLY)
    }
    pub fn is_non_singular_noun(self) -> bool {
        self.has(NON_SINGULAR_NOUN)
    }
    pub fn is_countable_noun(self) -> bool {
        self.has(COUNTABLE_NOUN)
    }
    pub fn is_mass_noun_only(self) -> bool {
        self.has(MASS_NOUN_ONLY)
    }

    /// The bits of Harper's metadata for one word.
    #[cfg(feature = "harper")]
    pub fn from_harper(m: &harper_core::DictWordMetadata) -> Meta {
        let dialects = Dialect::ALL
            .iter()
            .filter(|d| m.dialects.is_dialect_enabled(d.to_harper()))
            .fold(0, |acc, d| acc | d.bit());
        let classes = [
            m.is_noun(),
            m.is_verb(),
            m.is_adjective(),
            m.is_adverb(),
            m.is_pronoun(),
            m.is_determiner(),
            m.preposition,
            m.is_conjunction(),
            m.is_verb_progressive_form(),
            m.is_verb_past_form(),
            m.is_verb_past_participle_form(),
            m.is_verb_third_person_singular_present_form(),
            m.is_verb_simple_past_form(),
            m.is_plural_noun_only(),
            m.is_non_singular_noun(),
            m.is_countable_noun(),
            m.is_mass_noun_only(),
        ];
        debug_assert_eq!(classes.len(), CLASSES.len());
        let bits = classes
            .iter()
            .enumerate()
            .filter(|(_, on)| **on)
            .fold(0, |acc, (i, _)| acc | 1 << (5 + i));
        Meta(dialects | bits)
    }
}

/// Harper's dictionary key: curly quotes and dashes normalized, then lowercased.
fn key(word: &str) -> String {
    word.chars()
        .map(normalized)
        .flat_map(char::to_lowercase)
        .collect()
}

/// Harper's `CharExt::normalized`.
fn normalized(c: char) -> char {
    match c {
        '\u{2018}' | '\u{2019}' | '\u{02BC}' | '\u{FF07}' => '\'',
        '\u{201C}' | '\u{201D}' | '\u{FF02}' => '"',
        '\u{2013}' | '\u{2014}' | '\u{2212}' | '\u{FF0D}' => '-',
        _ => c,
    }
}

/// The vendored word list.
pub struct Words {
    /// Key -> (canonical spelling, metadata).
    map: HashMap<Box<str>, (&'static str, Meta)>,
}

/// `words.tsv`, zlib-compressed (2.1 MB -> 0.4 MB in the binary).
static DATA: &[u8] = include_bytes!("../../dictionaries/harper/words.tsv.zlib");

/// The TSV text of [`DATA`].
static TEXT: LazyLock<String> = LazyLock::new(|| {
    let bytes =
        miniz_oxide::inflate::decompress_to_vec_zlib(DATA).expect("words.tsv.zlib inflates");
    String::from_utf8(bytes).expect("words.tsv is UTF-8")
});

/// The vendored dictionary, parsed on first use.
pub fn words() -> &'static Words {
    static WORDS: LazyLock<Words> = LazyLock::new(|| Words::parse(&TEXT));
    &WORDS
}

impl Words {
    fn parse(data: &'static str) -> Words {
        let lines = data
            .lines()
            .filter(|l| !l.starts_with('#') && !l.is_empty());
        let mut map = HashMap::with_capacity(data.len() / 12);
        for line in lines {
            let (word, bits) = line.split_once('\t').expect("words.tsv: word<TAB>hex bits");
            let bits = u32::from_str_radix(bits, 16).expect("words.tsv: hex bits");
            map.insert(key(word).into_boxed_str(), (word, Meta(bits)));
        }
        Words { map }
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Metadata of `word` in any capitalization (Harper's `get_word_metadata_str`).
    pub fn meta(&self, word: &str) -> Option<Meta> {
        self.map.get(key(word).as_str()).map(|e| e.1)
    }

    /// `word` in any capitalization (Harper's `contains_word_str`).
    pub fn contains(&self, word: &str) -> bool {
        self.map.contains_key(key(word).as_str())
    }

    /// `word` as the dictionary writes it (Harper's `contains_exact_word_str`).
    pub fn contains_exact(&self, word: &str) -> bool {
        self.map
            .get(key(word).as_str())
            .is_some_and(|(canon, _)| canon.chars().eq(word.chars().map(normalized)))
    }
}

/// `words.tsv` (uncompressed) from Harper's live curated dictionary: a header naming the
/// harper-core version and the bit order, then `word<TAB>hex bits` lines sorted by word.
#[cfg(feature = "harper")]
pub fn export(harper_version: &str) -> String {
    use harper_core::spell::{Dictionary as _, MutableDictionary};
    use std::fmt::Write as _;
    let dict = MutableDictionary::curated();
    let mut rows: Vec<(String, Meta)> = dict
        .words_iter()
        .map(|w| {
            let meta = dict.get_word_metadata(w).expect("listed word has metadata");
            (w.iter().collect(), Meta::from_harper(&meta))
        })
        .collect();
    rows.sort_by(|a, b| a.0.cmp(&b.0));
    let mut out = String::new();
    let _ = writeln!(
        out,
        "# Harper curated dictionary from harper-core {harper_version} (Apache-2.0, https://github.com/Automattic/harper)."
    );
    out.push_str("# Generated by scripts/update-harper-words.sh; do not edit.\n");
    let names: Vec<String> = Dialect::ALL
        .iter()
        .map(|d| format!("{d:?}").to_lowercase())
        .chain(CLASSES.iter().map(|c| c.to_string()))
        .collect();
    let _ = writeln!(out, "# word<TAB>hex bits, bit 0 first: {}", names.join(" "));
    for (w, m) in rows {
        let _ = writeln!(out, "{w}\t{:x}", m.0);
    }
    out
}

#[cfg(all(test, feature = "harper"))]
mod tests {
    use super::*;
    use harper_core::spell::{Dictionary as _, MutableDictionary};

    /// The vendored list is Harper's: same words, spellings and metadata bits.
    #[test]
    fn vendored_words_match_harper() {
        let live = MutableDictionary::curated();
        let w = words();
        assert_eq!(w.len(), live.word_count());
        for (i, word) in live.words_iter().enumerate() {
            if i % 7 != 0 {
                continue;
            }
            let s: String = word.iter().collect();
            let m = live.get_word_metadata(word).unwrap();
            assert_eq!(w.meta(&s), Some(Meta::from_harper(&m)), "{s}");
            assert!(w.contains_exact(&s), "{s}");
        }
        for s in [
            "colour",
            "Colour",
            "color",
            "CMake",
            "cmake",
            "et al",
            "et al.",
            "don’t",
            "don't",
            "running",
            "ran",
            "feedback",
            "sentance",
            "powershell",
            "PowerShell",
            "résumé",
        ] {
            assert_eq!(w.contains(s), live.contains_word_str(s), "{s}");
            assert_eq!(w.contains_exact(s), live.contains_exact_word_str(s), "{s}");
            assert_eq!(
                w.meta(s),
                live.get_word_metadata_str(s).map(|m| Meta::from_harper(&m)),
                "{s}"
            );
        }
    }
}
