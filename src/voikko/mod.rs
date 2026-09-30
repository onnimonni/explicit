//! Finnish spelling and suggestions: a pure-Rust port of libvoikko's VFST speller
//! (<https://github.com/voikko/corevoikko>, MPL 1.1 / GPL 2+ / LGPL 2.1+) reading the
//! `voikko-fi` morphology (`mor.vfst`, GPL 2+). The dictionary is embedded (zlib) under the
//! `voikko` feature; `[languages.fi] dictionary_path` loads another `mor.vfst`.
//!
//! Ported: `FinnishVfstAnalyzer` (the parts spelling reads), `AnalyzerToSpellerAdapter`,
//! `FinnishSpellerTweaksWrapper`, `voikkoSpellUcs4` with default options, `SpellWithPriority`
//! and the typing suggestion strategy. Not ported: soft-hyphen position checks (soft hyphens
//! are dropped), hyphenation, grammar checking.

mod analyzer;
mod fst;

use std::collections::HashSet;
use std::path::Path;
use std::sync::OnceLock;

use analyzer::{CONSONANTS, VOWELS, analyze, is_lower, is_upper, lower, upper};
use fst::{Configuration, Transducer};

const FAILED: u8 = 0;
const OK: u8 = 1;
const CAP_FIRST: u8 = 2;
const CAP_ERROR: u8 = 3;

/// Suggestions returned per word.
const MAX_SUGGESTIONS: usize = 5;
/// Speller calls a suggestion search may make (libvoikko's typing strategy).
const MAX_COST: usize = 800;

pub struct Voikko {
    t: Transducer,
}

/// One morphological reading of a word (Finnish class and case names, as libvoikko's).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reading {
    /// `nimisana` (noun), `laatusana` (adjective), `teonsana` (verb), `seikkasana` (adverb),
    /// `etunimi`, `sukunimi`, `paikannimi`, `nimi` (names), ...
    pub class: Option<&'static str>,
    /// `nimento` (nominative), `omanto` (genitive), `osanto` (partitive), ...
    pub case: Option<&'static str>,
    /// `positive`, `comparative`, `superlative` for adjectives.
    pub comparison: Option<&'static str>,
    /// Verb mood: `indicative`, `imperative`, `A-infinitive`, ...
    pub mood: Option<&'static str>,
    /// Verb person: `1`, `2`, `3`, or `4` for the passive.
    pub person: Option<char>,
    /// `singular` or `plural` (nouns, adjectives, pronouns and finite verbs).
    pub number: Option<&'static str>,
    /// Compound parts.
    pub parts: usize,
    /// Base form of the (last part of the) word: `pitää` for `pidän`, `se` for `sitä`;
    /// `None` for derivations.
    pub base: Option<String>,
    /// Starts with a capital letter (a name).
    pub proper: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Case {
    NoLetters,
    AllLower,
    FirstUpper,
    Complex,
    AllUpper,
}

fn casetype(w: &[char]) -> Case {
    let Some(&first) = w.first() else {
        return Case::NoLetters;
    };
    let (mut first_uc, mut rest_lc, mut all_uc, mut no_letters) = (false, true, true, true);
    if is_upper(first) {
        first_uc = true;
        no_letters = false;
    }
    if is_lower(first) {
        all_uc = false;
        no_letters = false;
    }
    for &c in &w[1..] {
        if is_upper(c) {
            no_letters = false;
            rest_lc = false;
        }
        if is_lower(c) {
            all_uc = false;
            no_letters = false;
        }
    }
    if no_letters {
        Case::NoLetters
    } else if all_uc {
        Case::AllUpper
    } else if !rest_lc {
        Case::Complex
    } else if first_uc {
        Case::FirstUpper
    } else {
        Case::AllLower
    }
}

/// URLs and email addresses (libvoikko's `voikko_is_nonword`).
fn is_nonword(w: &[char]) -> bool {
    let n = w.len();
    if n < 4 {
        return false;
    }
    if let Some(i) = w[..n - 3].iter().position(|&c| c == '/')
        && w[i + 1] == '/'
        && w[i + 1..].contains(&'.')
    {
        return true;
    }
    if let Some(i) = w[..n - 3].iter().position(|&c| c == '@')
        && w[i + 1] != '.'
        && w[i + 1..].contains(&'.')
    {
        return true;
    }
    n >= 7 && w.starts_with(&['w', 'w', 'w', '.']) && w[4] != '.' && w[5..].contains(&'.')
}

/// Character normalization (libvoikko's `voikko_normalise`, common cases): combining
/// diacritics composed, typographic apostrophe and hyphens made ASCII, ligatures split.
fn normalise(word: &str) -> Vec<char> {
    let mut out = Vec::with_capacity(word.len());
    let mut it = word.chars().peekable();
    while let Some(c) = it.next() {
        let composed = match (c, it.peek()) {
            (base, Some('\u{308}')) => match base {
                'a' => Some('ä'),
                'o' => Some('ö'),
                'u' => Some('ü'),
                'A' => Some('Ä'),
                'O' => Some('Ö'),
                'U' => Some('Ü'),
                _ => None,
            },
            (base, Some('\u{30A}')) => match base {
                'a' => Some('å'),
                'A' => Some('Å'),
                _ => None,
            },
            (base, Some('\u{30C}')) => match base {
                's' => Some('š'),
                'S' => Some('Š'),
                'z' => Some('ž'),
                'Z' => Some('Ž'),
                _ => None,
            },
            _ => None,
        };
        if let Some(x) = composed {
            it.next();
            out.push(x);
            continue;
        }
        match c {
            '\u{2019}' => out.push('\''),
            '\u{2010}' | '\u{2011}' => out.push('-'),
            '\u{FB00}' => out.extend(['f', 'f']),
            '\u{FB01}' => out.extend(['f', 'i']),
            '\u{FB02}' => out.extend(['f', 'l']),
            '\u{FB03}' => out.extend(['f', 'f', 'i']),
            '\u{FB04}' => out.extend(['f', 'f', 'l']),
            _ => out.push(c),
        }
    }
    out
}

/// How `word` matches the capitalization pattern `structure` (libvoikko's
/// `SpellUtils::matchWordAndAnalysis`).
fn match_word_and_analysis(word: &[char], structure: &[char]) -> u8 {
    let at = |j: usize| structure.get(j).copied().unwrap_or('\0');
    let mut result = OK;
    let mut j = 0;
    for (i, &c) in word.iter().enumerate() {
        while at(j) == '=' {
            j += 1;
        }
        if at(j) == '\0' {
            break;
        }
        let s = at(j);
        let up = is_upper(c);
        if !up && is_lower(c) && (s == 'i' || s == 'j') {
            result = if i == 0 { CAP_FIRST } else { CAP_ERROR };
        }
        if up && (s == 'p' || s == 'q') {
            result = CAP_ERROR;
        }
        if result == CAP_ERROR {
            break;
        }
        j += 1;
    }
    result
}

fn better(best: u8, r: u8) -> bool {
    best == FAILED || best > r
}

fn noun_priority(sijamuoto: Option<&str>) -> i32 {
    match sijamuoto {
        Some("nimento") => 2,
        Some("omanto") => 3,
        Some("osanto") => 5,
        Some("sisaolento" | "sisatulento") => 8,
        Some("sisaeronto" | "ulkoolento") => 12,
        Some("ulkoeronto") => 30,
        Some("ulkotulento" | "olento" | "tulento" | "keinonto") => 20,
        Some("vajanto" | "seuranto") => 60,
        _ => 4,
    }
}

fn analysis_priority(a: &analyzer::Analysis, result: u8) -> i32 {
    let class = match a.class {
        Some(
            "nimisana" | "laatusana" | "nimisana_laatusana" | "asemosana" | "etunimi" | "sukunimi"
            | "paikannimi" | "nimi",
        ) => noun_priority(a.sijamuoto),
        _ => 4,
    };
    let parts = a.structure.iter().filter(|&&c| c == '=').take(5).count() as u32;
    let structure = if parts == 0 {
        1
    } else {
        1 << (3 * (parts - 1))
    };
    let spell = match result {
        CAP_FIRST => 2,
        CAP_ERROR => 3,
        _ => 1,
    };
    class * structure * spell
}

impl Voikko {
    pub fn new(data: Box<[u8]>) -> Result<Voikko, String> {
        Ok(Voikko {
            t: Transducer::new(data)?,
        })
    }

    /// Load `mor.vfst` from `dir`: the file itself, a `mor-*` directory, or a Voikko
    /// dictionary root (`5/mor-standard/mor.vfst`).
    pub fn from_path(path: &Path) -> Result<Voikko, String> {
        let candidates = [
            path.to_path_buf(),
            path.join("mor.vfst"),
            path.join("mor-standard/mor.vfst"),
            path.join("5/mor-standard/mor.vfst"),
        ];
        let file = candidates
            .iter()
            .find(|p| p.is_file())
            .ok_or_else(|| format!("no mor.vfst under {}", path.display()))?;
        let data = std::fs::read(file).map_err(|e| format!("{}: {e}", file.display()))?;
        Voikko::new(data.into_boxed_slice()).map_err(|e| format!("{}: {e}", file.display()))
    }

    fn analyze(&self, c: &mut Configuration, w: &[char]) -> Vec<analyzer::Analysis> {
        analyze(&self.t, c, w)
    }

    /// Morphological readings of `word` (any case).
    pub fn analyses(&self, word: &str) -> Vec<Reading> {
        let w = normalise(word);
        self.with_configuration(|c| self.analyze(c, &w))
            .into_iter()
            .map(|a| Reading {
                class: a.class,
                case: a.sijamuoto,
                comparison: a.comparison,
                mood: a.mood,
                person: a.person,
                number: a.number,
                parts: a.structure.iter().filter(|&&ch| ch == '=').count(),
                base: a.base,
                proper: a.structure.iter().find(|ch| !matches!(ch, '=' | '-')) == Some(&'i'),
            })
            .collect()
    }

    /// `AnalyzerToSpellerAdapter::spell`.
    fn adapter_spell(&self, c: &mut Configuration, w: &[char]) -> u8 {
        let mut best = FAILED;
        for a in self.analyze(c, w) {
            let r = match_word_and_analysis(w, &a.structure);
            if better(best, r) {
                best = r;
            }
            if best == OK {
                break;
            }
        }
        best
    }

    /// `FinnishSpellerTweaksWrapper::spell`; soft hyphens are dropped without checking their
    /// positions.
    fn tweaks_spell(&self, c: &mut Configuration, w: &[char]) -> u8 {
        if w.contains(&'\u{AD}') {
            let stripped: Vec<char> = w.iter().copied().filter(|&ch| ch != '\u{AD}').collect();
            if stripped.is_empty() || w.first() == Some(&'\u{AD}') || w.last() == Some(&'\u{AD}') {
                return FAILED;
            }
            return self.spell_without_soft_hyphen(c, &stripped);
        }
        self.spell_without_soft_hyphen(c, w)
    }

    fn spell_without_soft_hyphen(&self, c: &mut Configuration, w: &[char]) -> u8 {
        let wlen = w.len();
        let result = self.adapter_spell(c, w);
        if result == OK || wlen <= 3 {
            return result;
        }
        let Some(ll) = w[1..wlen - 1]
            .iter()
            .position(|&ch| ch == '-')
            .map(|p| p + 1)
        else {
            return result;
        };
        let mut buffer: Vec<char> = w[..ll].to_vec();
        buffer.extend_from_slice(&w[ll + 1..]);
        // Leading part ends with the vowel-consonant pair the trailing part starts with
        // (`pop-opisto`).
        if ll >= 2 && wlen - ll >= 3 {
            let v1 = lower(w[ll - 2]);
            let v2 = lower(w[ll - 1]);
            if VOWELS.contains(v1)
                && CONSONANTS.contains(v2)
                && lower(w[ll + 1]) == v1
                && lower(w[ll + 2]) == v2
            {
                let r = self.adapter_spell(c, &buffer);
                if r != FAILED && better(result, r) {
                    return r;
                }
            }
        }
        // `ja-sana`: any valid word, then a word that may follow a hyphen freely.
        for i in (1..=wlen - 2).rev() {
            if w[i] == '-' {
                let leading = self.tweaks_spell(c, &w[..i]);
                if leading != FAILED
                    && self
                        .analyze(c, &w[i + 1..])
                        .iter()
                        .any(|a| a.vapaa_jalkiosa)
                {
                    return leading;
                }
                break;
            }
        }
        // Ambiguous compound (`syy-silta`, `syys-ilta`).
        let analyses = self.analyze(c, &buffer);
        if analyses.is_empty() {
            return result;
        }
        let (mut with_border, mut without_border) = (FAILED, FAILED);
        for a in &analyses {
            let s = &a.structure;
            let at = |j: usize| s.get(j).copied().unwrap_or('\0');
            let mut j = 0;
            let mut i = 0;
            while i < ll {
                while at(j) == '=' {
                    j += 1;
                }
                if at(j) == '\0' {
                    break;
                }
                j += 1;
                i += 1;
            }
            if i == ll {
                let r = match_word_and_analysis(&buffer, s);
                if at(j) == '=' && better(with_border, r) {
                    with_border = r;
                }
                if at(j) != '=' && better(without_border, r) {
                    without_border = r;
                }
            }
        }
        if with_border != FAILED && without_border != FAILED && better(result, with_border) {
            return with_border;
        }
        result
    }

    /// `word` is spelled right (libvoikko's `voikkoSpellUcs4` with default options).
    pub fn spell(&self, word: &str) -> bool {
        self.with_configuration(|c| self.spell_with(c, word))
    }

    /// Run `f` with this thread's traversal buffers (about 32 KB, reused across calls).
    fn with_configuration<R>(&self, f: impl FnOnce(&mut Configuration) -> R) -> R {
        thread_local! {
            static CONF: std::cell::RefCell<Option<(usize, Configuration)>> =
                const { std::cell::RefCell::new(None) };
        }
        let id = std::ptr::from_ref(self) as usize;
        let mut c = CONF
            .with(|cell| cell.borrow_mut().take())
            .filter(|(owner, _)| *owner == id)
            .map_or_else(|| Configuration::new(&self.t), |(_, c)| c);
        let r = f(&mut c);
        CONF.with(|cell| *cell.borrow_mut() = Some((id, c)));
        r
    }

    fn spell_with(&self, c: &mut Configuration, word: &str) -> bool {
        if word.is_empty() {
            return true;
        }
        let w = normalise(word);
        if w.len() > analyzer::MAX_WORD_CHARS {
            return false;
        }
        let caps = casetype(&w);
        if is_nonword(&w) {
            return true;
        }
        if matches!(caps, Case::Complex | Case::NoLetters) {
            let mut buffer = w.clone();
            buffer[0] = lower(buffer[0]);
            let r = self.tweaks_spell(c, &buffer);
            return r == OK || (r == CAP_FIRST && is_upper(w[0]));
        }
        let buffer: Vec<char> = w.iter().map(|&ch| lower(ch)).collect();
        let r = self.tweaks_spell(c, &buffer);
        match caps {
            Case::AllLower => r == OK,
            Case::FirstUpper => r == OK || r == CAP_FIRST,
            _ => r != FAILED,
        }
    }

    /// Up to five corrections, best first (libvoikko's typing suggestion strategy).
    pub fn suggest(&self, word: &str) -> Vec<String> {
        let w = normalise(word);
        if w.len() <= 1 || w.len() > analyzer::MAX_WORD_CHARS {
            return Vec::new();
        }
        let found = self.with_configuration(|c| {
            let mut s = Suggest {
                v: self,
                c,
                word: w.clone(),
                found: Vec::new(),
                cost: 0,
                max: MAX_SUGGESTIONS * 3,
            };
            s.generate();
            s.found
        });
        let mut s = Suggestions { found };
        // Stable sort by priority, as libvoikko's insertion sort.
        s.found.sort_by_key(|(_, p)| *p);
        let orig = casetype(&w);
        let mut seen = HashSet::new();
        let mut out = Vec::new();
        for (sugg, _) in s.found {
            let mut sugg = sugg;
            if orig == Case::FirstUpper || (orig == Case::Complex && is_upper(w[0])) {
                if casetype(&sugg) == Case::AllLower {
                    sugg[0] = upper(sugg[0]);
                }
            } else if orig == Case::AllUpper {
                sugg.iter_mut().for_each(|ch| *ch = upper(*ch));
            }
            let text: String = sugg.into_iter().collect();
            if seen.insert(text.clone()) {
                out.push(text);
            }
            if out.len() == MAX_SUGGESTIONS {
                break;
            }
        }
        out
    }
}

struct Suggestions {
    found: Vec<(Vec<char>, i32)>,
}

/// libvoikko's `SuggestionStatus` plus the generators of `SuggestionStrategyTyping`.
struct Suggest<'a> {
    v: &'a Voikko,
    c: &'a mut Configuration,
    word: Vec<char>,
    found: Vec<(Vec<char>, i32)>,
    cost: usize,
    max: usize,
}

const BACK_VOWELS: [char; 6] = ['a', 'o', 'u', 'A', 'O', 'U'];
const FRONT_VOWELS: [char; 6] = ['ä', 'ö', 'y', 'Ä', 'Ö', 'Y'];

const REPLACEMENTS_1: &str = ".,asiuiotrtdersšsanmuilkklkgoiäömnrertvbpbpoythjjhjkdtdsdföägfghgkfgfdbpbncvcswewvxczžzxqaåoåpåäåöaeiktyea";
const REPLACEMENTS_2: &str = "1q2q2w3w3e4e4r5r5t6t6y7y7u8u8i9i9o0o0p+pie";
const REPLACEMENTS_3: &str = "essdnhujlökjopäpmkrdvgplyhhujideölgtfvbvckwaxszaqkåaaåeéaâkcscijxz";
const REPLACEMENTS_4: &str = "qwqswqwswdedefrfrgtftgthygyjuhukilokolpöpäsesxdrbgfefrftfcgygbgvhyhnhbhgjujmjnkikokmlolpöpöåäåzsxdcdcfcxvfbhnjnbmjewpåaqswszdwdcdxvcawazsq";
const REPLACEMENTS_5: &str =
    "aooaoutlsraieääeuvvuoddokqpvvpqeeqaddarsetteryyrtuutyiiyuoippioåhvvhhmmh";

fn pairs(table: &str) -> Vec<(char, char)> {
    let chars: Vec<char> = table.chars().collect();
    chars.chunks(2).map(|p| (p[0], p[1])).collect()
}

impl Suggest<'_> {
    fn should_abort(&self) -> bool {
        if self.found.len() >= self.max {
            return true;
        }
        if self.cost < MAX_COST {
            return false;
        }
        !(self.found.is_empty() && self.cost < 2 * MAX_COST)
    }

    fn add(&mut self, s: Vec<char>, priority: i32) {
        if self.found.len() < self.max {
            let n = self.found.len() as i32;
            self.found.push((s, priority * (n + 5)));
        }
    }

    /// `SpellWithPriority::spellWithPriority`.
    fn spell_with_priority(&mut self, w: &[char]) -> (u8, i32, Option<Vec<char>>) {
        let analyses = self.v.analyze(self.c, w);
        let mut best = FAILED;
        let mut best_prio = 0;
        let first_structure = analyses.first().map(|a| a.structure.clone());
        for a in &analyses {
            let r = match_word_and_analysis(w, &a.structure);
            let p = analysis_priority(a, r);
            if better(best, r) {
                best = r;
                best_prio = p;
            } else if best == r && p < best_prio {
                best_prio = p;
            }
        }
        (best, best_prio, first_structure)
    }

    /// `SuggestionGeneratorCaseChange::suggestForBuffer`.
    fn try_word(&mut self, w: &[char]) {
        if self.should_abort() || w.is_empty() {
            return;
        }
        let (r, prio, structure) = self.spell_with_priority(w);
        self.cost += 1;
        match r {
            OK => self.add(w.to_vec(), prio),
            CAP_FIRST => {
                let mut s = w.to_vec();
                s[0] = upper(s[0]);
                self.add(s, prio);
            }
            CAP_ERROR => {
                self.cost += 1;
                let Some(structure) = structure else {
                    return;
                };
                let at = |j: usize| structure.get(j).copied().unwrap_or('\0');
                let mut s = w.to_vec();
                let mut j = 0;
                for ch in s.iter_mut() {
                    while at(j) == '=' {
                        j += 1;
                    }
                    match at(j) {
                        '\0' => break,
                        'i' | 'j' => *ch = upper(*ch),
                        'p' | 'q' => *ch = lower(*ch),
                        _ => {}
                    }
                    j += 1;
                }
                self.add(s, prio);
            }
            _ => {}
        }
    }

    fn generate(&mut self) {
        let word = self.word.clone();
        // Primary generators: case change, soft hyphens.
        self.try_word(&word);
        if !self.should_abort() && word.contains(&'\u{AD}') {
            let stripped: Vec<char> = word.iter().copied().filter(|&c| c != '\u{AD}').collect();
            self.try_word(&stripped);
        }
        if !self.found.is_empty() {
            return;
        }
        let r1 = pairs(REPLACEMENTS_1);
        let r2 = pairs(REPLACEMENTS_2);
        let r3 = pairs(REPLACEMENTS_3);
        let r4 = pairs(REPLACEMENTS_4);
        let r5 = pairs(REPLACEMENTS_5);
        // Each generator stops early once the search budget is spent.
        let generators: [&dyn Fn(&mut Self); 17] = [
            &|s| s.vowel_change(),
            &|s| s.replacement(&r1),
            &|s| s.deletion(),
            &|s| s.insert_special(),
            &|s| s.split_word(),
            &|s| s.replace_two(&r1),
            &|s| s.replacement(&r2),
            &|s| s.insertion("aitesn"),
            &|s| s.swap(),
            &|s| s.replacement(&r3),
            &|s| s.insertion("ulkoämrvpyhjdögfbcw:xzqå'."),
            &|s| s.replacement(&r4),
            &|s| s.replace_two(&r2),
            &|s| s.replace_two(&r3),
            &|s| s.replace_two(&r4),
            &|s| s.delete_two(),
            &|s| s.replacement(&r5),
        ];
        for g in generators {
            if self.should_abort() {
                break;
            }
            g(self);
        }
    }

    fn vowel_change(&mut self) {
        let w = self.word.clone();
        let is_vowel = |c: char| BACK_VOWELS.contains(&c) || FRONT_VOWELS.contains(&c);
        let vcount = w.iter().filter(|&&c| is_vowel(c)).count();
        if vcount == 0 || vcount > 7 {
            return;
        }
        let mask: u32 = (1 << vcount) - 1;
        let mut pat: u32 = 1;
        while pat & mask != 0 {
            let mut buffer = w.clone();
            let mut i = 0;
            for j in 0..vcount {
                while !is_vowel(buffer[i]) {
                    i += 1;
                }
                if pat & (1 << j) != 0 {
                    if let Some(k) = BACK_VOWELS.iter().position(|&b| b == buffer[i]) {
                        buffer[i] = FRONT_VOWELS[k];
                    } else if let Some(k) = FRONT_VOWELS.iter().position(|&f| f == buffer[i]) {
                        buffer[i] = BACK_VOWELS[k];
                    }
                }
                i += 1;
            }
            if self.should_abort() {
                return;
            }
            self.try_word(&buffer);
            pat += 1;
        }
    }

    fn replacement(&mut self, table: &[(char, char)]) {
        let mut buffer = self.word.clone();
        for &(from, to) in table {
            let mut variants = vec![(from, to)];
            // Upper-case occurrences too, when the letter has case.
            if upper(from) != from {
                variants.push((upper(from), upper(to)));
            }
            for (f, t) in variants {
                for pos in 0..buffer.len() {
                    if buffer[pos] != f {
                        continue;
                    }
                    buffer[pos] = t;
                    self.try_word(&buffer);
                    buffer[pos] = f;
                    if self.should_abort() {
                        return;
                    }
                }
            }
        }
    }

    fn deletion(&mut self) {
        let w = self.word.clone();
        for i in 0..w.len() {
            if self.should_abort() {
                return;
            }
            if i == 0 || lower(w[i]) != lower(w[i - 1]) {
                let mut b = w[..i].to_vec();
                b.extend_from_slice(&w[i + 1..]);
                self.try_word(&b);
            }
        }
    }

    fn insert_special(&mut self) {
        let w = self.word.clone();
        let n = w.len();
        if n >= 4 {
            for j in 2..=n - 2 {
                if self.should_abort() {
                    break;
                }
                if w[j - 2] == '-' || w[j - 1] == '-' || w[j] == '-' || w[j + 1] == '-' {
                    continue;
                }
                let mut b = w[..j].to_vec();
                b.push('-');
                b.extend_from_slice(&w[j..]);
                self.try_word(&b);
            }
        }
        let mut j = 0;
        while j < n && !self.should_abort() {
            if j + 1 < n && w[j] == w[j + 1] {
                j += 2;
                continue;
            }
            if w[j] != '-' && w[j] != '\'' {
                let mut b = w[..=j].to_vec();
                b.extend_from_slice(&w[j..]);
                self.try_word(&b);
            }
            j += 1;
        }
    }

    /// `SplitWord::spellOk`: capitalizes the first letter of `w` when needed.
    fn split_ok(&mut self, w: &mut [char]) -> (bool, i32) {
        let first_upper = is_upper(w[0]);
        if first_upper {
            w[0] = lower(w[0]);
        }
        let (r, prio, _) = self.spell_with_priority(w);
        self.cost += 1;
        if first_upper || r == CAP_FIRST {
            w[0] = upper(w[0]);
        }
        (r == OK || r == CAP_FIRST, prio)
    }

    fn split_word(&mut self) {
        let w = self.word.clone();
        let n = w.len();
        if n < 4 {
            return;
        }
        let mut part1 = w.clone();
        for split in (2..=n - 2).rev() {
            if w[split - 2] == '-' || w[split - 1] == '-' || w[split + 1] == '-' {
                continue;
            }
            let strip = usize::from(w[split] == '-');
            let (mut ok, mut prio_total) = self.split_ok(&mut part1[..split]);
            if !ok && part1[split - 1] == '.' {
                (ok, prio_total) = self.split_ok(&mut part1[..split - 1]);
            }
            if ok {
                let mut part2 = w[split + strip..].to_vec();
                let (ok2, prio_part) = self.split_ok(&mut part2);
                let prio = (prio_total + prio_part) * (1 + strip as i32 * 5);
                if ok2 {
                    let mut s = part1[..split].to_vec();
                    s.push(' ');
                    s.extend(part2);
                    self.add(s, prio);
                }
            }
            if self.should_abort() {
                break;
            }
        }
    }

    fn replace_two(&mut self, table: &[(char, char)]) {
        let mut b: Vec<char> = self.word.iter().map(|&c| lower(c)).collect();
        let n = b.len();
        let mut i = 0;
        while i + 1 < n {
            let replaced = b[i];
            if replaced != b[i + 1] {
                i += 1;
                continue;
            }
            for &(from, to) in table {
                if from != replaced {
                    continue;
                }
                b[i] = to;
                b[i + 1] = to;
                self.try_word(&b);
                if self.should_abort() {
                    break;
                }
            }
            b[i] = replaced;
            b[i + 1] = replaced;
            if self.should_abort() {
                break;
            }
            i += 2;
        }
    }

    fn insertion(&mut self, chars: &str) {
        let w = self.word.clone();
        let n = w.len();
        for ins in chars.chars() {
            for j in 0..n {
                if self.should_abort() {
                    break;
                }
                if ins == lower(w[j]) || (j > 0 && ins == lower(w[j - 1])) {
                    continue;
                }
                let mut b = w[..j].to_vec();
                b.push(ins);
                b.extend_from_slice(&w[j..]);
                self.try_word(&b);
            }
            if self.should_abort() {
                break;
            }
            if ins == w[n - 1] {
                continue;
            }
            let mut b = w.clone();
            b.push(ins);
            self.try_word(&b);
        }
    }

    fn swap(&mut self) {
        let w = self.word.clone();
        let n = w.len();
        let max_distance = if n <= 8 { 10 } else { 50 / n };
        if max_distance == 0 {
            return;
        }
        let mut b = w.clone();
        for i in 0..n {
            if self.should_abort() {
                return;
            }
            for j in i + 1..n {
                if self.should_abort() {
                    return;
                }
                if j - i > max_distance {
                    break;
                }
                let (li, lj) = (lower(b[i]), lower(b[j]));
                if li == lj {
                    continue;
                }
                if (0..3).any(|k| {
                    (li == BACK_VOWELS[k] && lj == FRONT_VOWELS[k])
                        || (li == FRONT_VOWELS[k] && lj == BACK_VOWELS[k])
                }) {
                    continue;
                }
                b[i] = w[j];
                b[j] = w[i];
                self.try_word(&b);
                b[i] = w[i];
                b[j] = w[j];
            }
        }
    }

    fn delete_two(&mut self) {
        let w = self.word.clone();
        let n = w.len();
        if n < 6 {
            return;
        }
        let mut attempts = HashSet::new();
        for i in 0..n - 3 {
            if self.should_abort() {
                return;
            }
            if w[i..i + 2] == w[i + 2..i + 4] {
                let mut b = w[..i].to_vec();
                b.extend_from_slice(&w[i + 2..]);
                if attempts.insert(b.clone()) {
                    self.try_word(&b);
                }
            }
        }
    }
}

/// The embedded `voikko-fi` morphology (GPL 2+), inflated on first use.
#[cfg(feature = "voikko")]
pub fn embedded() -> &'static Voikko {
    static DATA: &[u8] = include_bytes!("../../dictionaries/fi/mor.vfst.zlib");
    static V: OnceLock<Voikko> = OnceLock::new();
    V.get_or_init(|| {
        let data =
            miniz_oxide::inflate::decompress_to_vec_zlib(DATA).expect("mor.vfst.zlib inflates");
        Voikko::new(data.into_boxed_slice()).expect("bundled mor.vfst parses")
    })
}

/// A `mor.vfst` loaded from a configured path, kept for the process; `hash` (of the file's
/// contents) tells a changed file from the one loaded before.
pub fn from_path_cached(path: &Path, hash: u64) -> Result<&'static Voikko, String> {
    use std::collections::HashMap;
    use std::sync::{Mutex, PoisonError};
    type Loaded = HashMap<(std::path::PathBuf, u64), &'static Voikko>;
    static LOADED: OnceLock<Mutex<Loaded>> = OnceLock::new();
    let map = LOADED.get_or_init(Default::default);
    let mut map = map.lock().unwrap_or_else(PoisonError::into_inner);
    let key = (path.to_path_buf(), hash);
    if let Some(v) = map.get(&key) {
        return Ok(v);
    }
    let v: &'static Voikko = Box::leak(Box::new(Voikko::from_path(path)?));
    map.insert(key, v);
    Ok(v)
}

#[cfg(all(test, feature = "voikko"))]
mod tests;
