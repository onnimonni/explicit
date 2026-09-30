//! Finnish morphological analysis over `mor.vfst`, ported from libvoikko's
//! `morphology/FinnishVfstAnalyzer.cpp` (MPL 1.1 / GPL 2+ / LGPL 2.1+). Only the attributes
//! spelling and suggestions read are kept: the capitalization pattern (`STRUCTURE`), word class,
//! case, and whether the word may follow a hyphen (`MALAGA_VAPAA_JALKIOSA`).

use super::fst::{Configuration, Transducer};

const MAX_ANALYSIS_COUNT: usize = 100;
pub const MAX_WORD_CHARS: usize = 255;
pub const VOWELS: &str = "aeiouyäö";
pub const CONSONANTS: &str = "bcdfghjklmnpqrstvwxzšž";

#[derive(Clone, Debug, Default)]
pub struct Analysis {
    /// `=` starts a word part, `i`/`p` upper/lower case letter, `j`/`q` the same in an
    /// abbreviation, `-` and `:` themselves.
    pub structure: Vec<char>,
    pub class: Option<&'static str>,
    pub sijamuoto: Option<&'static str>,
    pub comparison: Option<&'static str>,
    pub mood: Option<&'static str>,
    pub participle: Option<&'static str>,
    /// Verb person: `1`, `2`, `3`, `4` (passive).
    pub person: Option<char>,
    /// `singular` or `plural`.
    pub number: Option<&'static str>,
    pub vapaa_jalkiosa: bool,
    /// Base form of the last word part (`[Xp]...[X]`); `None` for a derivation (`[Xj]`).
    pub base: Option<String>,
}

/// Character at `i`, NUL past the end (the C code reads NUL-terminated strings).
fn at(s: &[char], i: usize) -> char {
    s.get(i).copied().unwrap_or('\0')
}

/// `s[i..]` starts with `p` (C `wcsncmp(s + i, p, len(p)) == 0`).
fn starts(s: &[char], i: usize, p: &str) -> bool {
    (i..).zip(p.chars()).all(|(k, c)| at(s, k) == c)
}

pub fn lower(c: char) -> char {
    let mut l = c.to_lowercase();
    match (l.next(), l.next()) {
        (Some(x), None) => x,
        _ => c,
    }
}

pub fn upper(c: char) -> char {
    let mut u = c.to_uppercase();
    match (u.next(), u.next()) {
        (Some(x), None) => x,
        _ => c,
    }
}

pub fn is_upper(c: char) -> bool {
    lower(c) != c
}

pub fn is_lower(c: char) -> bool {
    upper(c) != c
}

fn class_name(key: &[char]) -> Option<&'static str> {
    let k: String = key.iter().collect();
    Some(match k.as_str() {
        "n" => "nimisana",
        "l" => "laatusana",
        "nl" => "nimisana_laatusana",
        "h" => "huudahdussana",
        "ee" => "etunimi",
        "es" => "sukunimi",
        "ep" => "paikannimi",
        "em" => "nimi",
        "t" => "teonsana",
        "a" => "lyhenne",
        "s" => "seikkasana",
        "u" | "ur" => "lukusana",
        "r" => "asemosana",
        "c" => "sidesana",
        "d" => "suhdesana",
        "k" => "kieltosana",
        "p" => "etuliite",
        _ => return None,
    })
}

fn sijamuoto_name(key: &[char]) -> Option<&'static str> {
    let k: String = key.iter().collect();
    Some(match k.as_str() {
        "n" => "nimento",
        "g" => "omanto",
        "p" => "osanto",
        "es" => "olento",
        "tr" => "tulento",
        "ine" => "sisaolento",
        "ela" => "sisaeronto",
        "ill" => "sisatulento",
        "ade" => "ulkoolento",
        "abl" => "ulkoeronto",
        "all" => "ulkotulento",
        "ab" => "vajanto",
        "ko" => "seuranto",
        "in" => "keinonto",
        "sti" => "kerrontosti",
        "ak" => "kohdanto",
        _ => return None,
    })
}

fn comparison_name(key: &[char]) -> Option<&'static str> {
    match key {
        ['c'] => Some("comparative"),
        ['s'] => Some("superlative"),
        _ => None,
    }
}

fn mood_name(key: &[char]) -> Option<&'static str> {
    let k: String = key.iter().collect();
    Some(match k.as_str() {
        "n1" => "A-infinitive",
        "n2" => "E-infinitive",
        "n3" => "MA-infinitive",
        "n4" => "MINEN-infinitive",
        "n5" => "MAINEN-infinitive",
        "t" => "indicative",
        "e" => "conditional",
        "k" => "imperative",
        "m" => "potential",
        _ => return None,
    })
}

fn participle_name(key: &[char]) -> Option<&'static str> {
    Some(match key {
        ['v'] => "present_active",
        ['a'] => "present_passive",
        ['u'] => "past_active",
        ['t'] => "past_passive",
        ['m'] => "agent",
        ['e'] => "negation",
        _ => return None,
    })
}

fn create_default_structure(
    mut missing: usize,
    title_case: &mut bool,
    structure: &mut Vec<char>,
    abbr: bool,
) {
    while missing > 0 {
        if *title_case {
            structure.push(if abbr { 'j' } else { 'i' });
            *title_case = false;
        } else {
            structure.push(if abbr { 'q' } else { 'p' });
        }
        missing -= 1;
    }
}

fn decrease_missing(missing: &mut usize, seen: usize, from_default: usize) {
    let d = seen - from_default;
    if d <= *missing {
        *missing -= d;
    } else {
        *missing = 0;
    }
}

fn parse_structure(o: &[char], wlen: usize) -> Vec<char> {
    let len = o.len();
    let mut s = vec!['='];
    let mut missing = wlen;
    let mut seen = 0usize;
    let mut from_default = 0usize;
    let mut title = false;
    let mut abbr = false;
    let mut i = 0;
    while i < len {
        let c = o[i];
        if c == '[' && i + 2 < len {
            if i + 3 < len && at(o, i + 1) == 'B' && at(o, i + 2) != 'h' && at(o, i + 3) == ']' {
                if i == 1 {
                    s.push('=');
                }
                if seen > from_default {
                    create_default_structure(seen - from_default, &mut title, &mut s, abbr);
                    decrease_missing(&mut missing, seen, from_default);
                }
                if i != 1 && i + 5 < len && s.last() != Some(&'=') {
                    s.push('=');
                }
                i += 3;
                seen = 0;
                from_default = 0;
            } else if i + 3 < len && at(o, i + 1) == 'X' && at(o, i + 3) == ']' {
                if at(o, i + 2) == 'r' {
                    title = false;
                    i += 4;
                    while i < len && o[i] != '[' && missing > 0 {
                        s.push(o[i]);
                        if o[i] != '=' {
                            from_default += 1;
                            if o[i] != '-' {
                                missing -= 1;
                            }
                        }
                        i += 1;
                    }
                    i += 2;
                } else {
                    i += 4;
                    while i < len && o[i] != '[' {
                        i += 1;
                    }
                    i += 2;
                }
            } else if i + 3 < len && at(o, i + 1) == 'L' {
                let c2 = at(o, i + 2);
                if c2 == 'e' {
                    title = true;
                    abbr = false;
                    i += 4;
                } else if c2 == 'n' && at(o, i + 3) == 'l' {
                    abbr = false;
                    i += 4;
                } else if c2 == 'a' {
                    abbr = true;
                    i += 3;
                } else if c2 == 'u'
                    && i + 5 < len
                    && (at(o, i + 3) == 'r' || at(o, i + 4).is_ascii_digit())
                {
                    abbr = true;
                    i += 3;
                    if at(o, i) == 'r' {
                        i += 1;
                    }
                } else {
                    abbr = false;
                    i += 3;
                }
            } else {
                while i < len && o[i] != ']' {
                    i += 1;
                }
            }
        } else if c == '-' {
            if seen > from_default {
                create_default_structure(seen - from_default, &mut title, &mut s, abbr);
                decrease_missing(&mut missing, seen, from_default);
                s.push('-');
                seen = 0;
                from_default = 0;
            } else if i != 0 {
                if seen == from_default {
                    s.push('-');
                } else {
                    seen += 1;
                }
            }
            missing = missing.saturating_sub(1);
            if s.len() == 1 {
                s[0] = '-';
            }
        } else if c == ':' {
            if abbr {
                if seen > from_default {
                    create_default_structure(seen - from_default, &mut title, &mut s, abbr);
                    decrease_missing(&mut missing, seen, from_default);
                    seen = 0;
                    from_default = 0;
                }
                abbr = false;
            }
            s.push(':');
            missing = missing.saturating_sub(1);
        } else {
            seen += 1;
        }
        i += 1;
    }
    create_default_structure(missing, &mut title, &mut s, abbr);
    s
}

fn is_valid_analysis(o: &[char]) -> bool {
    let len = o.len();
    let mut before_last = '\0';
    let mut last = '\0';
    let mut boundary_passed = false;
    let mut hyphen_present = false;
    let mut hyphen_allowed = false;
    let mut hyphen_allowed_just_set = false;
    let mut hyphen_required = false;
    let mut required_hyphen_missing = false;
    let mut starts_with_proper_noun = false;
    let mut ends_with_non_ica_noun = false;
    let mut i = 0;
    while i < len {
        if o[i] == '[' {
            if i + 2 >= len {
                return false;
            }
            if i + 3 < len {
                if o[i + 1] == 'I' {
                    if starts(o, i + 2, "sf") {
                        hyphen_allowed = true;
                        hyphen_allowed_just_set = true;
                    } else if starts(o, i + 2, "cu") {
                        boundary_passed = false;
                        hyphen_allowed = true;
                        hyphen_required = true;
                    } else if starts(o, i + 2, "ca") {
                        required_hyphen_missing = false;
                        ends_with_non_ica_noun = false;
                    }
                } else if o[i + 1] == 'L' {
                    if o[i + 2] == 'e' {
                        starts_with_proper_noun = true;
                        ends_with_non_ica_noun = false;
                    } else if o[i + 2] == 'n' {
                        ends_with_non_ica_noun = true;
                    }
                } else if starts(o, i + 1, "Dg") {
                    starts_with_proper_noun = false;
                }
            }
            if o[i + 1] == 'X' {
                while i + 3 < len {
                    i += 1;
                    if o[i] == '[' && o[i + 1] == 'X' && o[i + 2] == ']' {
                        i += 2;
                        break;
                    }
                }
            } else if starts(o, i + 1, "Bh") {
                i += 3;
                boundary_passed = true;
                hyphen_present = false;
                if required_hyphen_missing {
                    return false;
                }
                if hyphen_required {
                    required_hyphen_missing = true;
                }
            } else {
                i += 1;
                while i < len && o[i] != ']' {
                    i += 1;
                }
            }
        } else if o[i] == '-' {
            starts_with_proper_noun = false;
            ends_with_non_ica_noun = false;
            if i + 5 < len && starts(o, i + 1, "[Bh]") {
                boundary_passed = true;
                hyphen_present = true;
                i += 4;
            }
        } else {
            if boundary_passed {
                if last == '\0' || (before_last == 'i' && last == 's') {
                    hyphen_allowed = true;
                }
                if hyphen_required && hyphen_present {
                    hyphen_required = false;
                }
                if !(hyphen_allowed && hyphen_present) {
                    let l = lower(last);
                    let n = lower(o[i]);
                    let required = (l == n && VOWELS.contains(l)) || l.is_ascii_digit();
                    if required != hyphen_present {
                        return false;
                    }
                }
                boundary_passed = false;
                if hyphen_allowed_just_set {
                    hyphen_allowed_just_set = false;
                } else {
                    hyphen_allowed = false;
                }
            }
            before_last = last;
            last = o[i];
        }
        i += 1;
    }
    !required_hyphen_missing && (!starts_with_proper_noun || !ends_with_non_ica_noun)
}

fn parse_basic_attributes(a: &mut Analysis, o: &[char]) {
    let len = o.len();
    let mut convert_nl = false;
    let mut bc_passed = false;
    let mut class_set = false;
    if len < 3 {
        return;
    }
    let mut i = len - 1;
    while i >= 2 {
        if o[i] == ']' {
            let mut j = i;
            while j >= 1 {
                j -= 1;
                if o[j] != '[' {
                    continue;
                }
                let key = &o[(j + 2).min(i)..i];
                match at(o, j + 1) {
                    'L' => {
                        if !class_set || at(o, j + 2) == ']' {
                            if starts(o, j + 2, "nl") {
                                let comp =
                                    matches!(a.comparison, Some("comparative" | "superlative"));
                                if a.class.is_none() {
                                    a.class = Some(if convert_nl || comp || starts(o, 0, "[Lu]") {
                                        "laatusana"
                                    } else {
                                        "nimisana_laatusana"
                                    });
                                }
                            } else if a.class.is_none() {
                                a.class = class_name(key);
                            }
                            class_set = true;
                        }
                    }
                    'S' => {
                        if !matches!(a.class, Some("etuliite" | "seikkasana")) {
                            if a.sijamuoto.is_none() {
                                a.sijamuoto = sijamuoto_name(key);
                            }
                            if j + 5 < len && starts(o, j + 2, "sti") {
                                convert_nl = true;
                            }
                        }
                    }
                    'P' => {
                        if a.person.is_none() && key.len() == 1 && ('1'..='4').contains(&key[0]) {
                            a.person = Some(key[0]);
                        }
                    }
                    'N' => {
                        if a.number.is_none() {
                            a.number = match key {
                                ['y'] => Some("singular"),
                                ['m'] => Some("plural"),
                                _ => None,
                            };
                        }
                    }
                    'T' => {
                        if a.class.is_none() && a.mood.is_none() {
                            a.mood = mood_name(key);
                        }
                    }
                    'C' => {
                        if a.class.is_none() && a.comparison.is_none() {
                            a.comparison = comparison_name(key);
                        }
                    }
                    'R' => {
                        if !bc_passed
                            && (a.class.is_none()
                                || a.class == Some("laatusana")
                                || o.ends_with(&['[', 'L', 'n', ']']))
                            && a.participle.is_none()
                        {
                            a.participle = participle_name(key);
                        }
                    }
                    'I' => {
                        if starts(o, j + 2, "vj") && at(o, 0) != '-' {
                            a.vapaa_jalkiosa = true;
                        }
                    }
                    'B' if j >= 5 && at(o, j + 2) == 'c' => {
                        if !class_set
                            && a.class.is_none()
                            && (at(o, j - 1) == '-' || starts(o, j - 5, "-[Bh]"))
                        {
                            a.class = Some("etuliite");
                            class_set = true;
                        }
                        bc_passed = true;
                    }
                    _ => {}
                }
                break;
            }
            if j < 3 {
                return;
            }
            i = j;
        }
        i -= 1;
    }
}

fn fix_structure(s: &mut [char], o: &[char]) {
    let len = o.len();
    let mut is_de = false;
    let mut hyphen_count = 0;
    let mut j = 0;
    while j < len {
        if j + 3 < len && o[j] == '[' {
            if o[j + 1] == 'D' {
                if o[j + 2] == 'g' {
                    let mut hyphens = 0;
                    for c in s.iter_mut() {
                        if *c == 'i' {
                            if hyphens == hyphen_count {
                                *c = 'p';
                            }
                        } else if *c == '-' {
                            hyphens += 1;
                        }
                    }
                } else if o[j + 2] == 'e' {
                    is_de = true;
                }
            } else if starts(o, j + 1, "Ln]") {
                is_de = false;
            }
        } else if o[j] == '-' {
            hyphen_count += 1;
            if is_de {
                let mut to_upper = j == len - 1;
                j += 1;
                while !to_upper && j + 4 < len {
                    if starts(o, j, "[Lep]") {
                        to_upper = true;
                    }
                    j += 1;
                }
                if to_upper && let Some(c) = s.iter_mut().find(|c| **c == 'i' || **c == 'p') {
                    *c = 'i';
                    return;
                }
            }
        }
        j += 1;
    }
}

/// The last `[Xp]base[X]` of an analysis, unless a derivation suffix (`[Xj]`) follows it.
fn parse_base(o: &[char]) -> Option<String> {
    let start = (0..o.len()).rev().find(|&i| starts(o, i, "[Xp]"))? + 4;
    let len = (start..o.len()).find(|&i| starts(o, i, "[X]"))? - start;
    let rest = &o[start + len..];
    if (0..rest.len()).any(|i| starts(rest, i, "[Xj]")) {
        return None;
    }
    Some(
        o[start..start + len]
            .iter()
            .filter(|&&c| c != '=')
            .collect(),
    )
}

/// libvoikko's `duplicateOrgName`: a compound ending in an organization suffix (`[Ion]`)
/// is also a proper name.
fn duplicate_org_name(a: &Analysis, o: &[char]) -> Option<Analysis> {
    if a.class != Some("nimisana") {
        return None;
    }
    let len = o.len();
    if len < 13 || o[0] == '-' || starts(o, 0, "[La]") {
        return None;
    }
    let mut i = len - 5;
    while i >= 8 {
        if starts(o, i, "[Bc]") {
            return None;
        }
        if starts(o, i, "[Ion]") {
            let mut j = i - 4;
            while j >= 4 {
                if starts(o, j, "[Bc]") {
                    let mut n = a.clone();
                    n.class = Some("nimi");
                    if n.structure.len() >= 2 {
                        n.structure[1] = 'i';
                    }
                    return Some(n);
                }
                j -= 1;
            }
        }
        i -= 1;
    }
    None
}

/// All analyses of `word` (any case; matched in lowercase).
pub fn analyze(t: &Transducer, c: &mut Configuration, word: &[char]) -> Vec<Analysis> {
    let mut list = Vec::new();
    if word.len() > MAX_WORD_CHARS {
        return list;
    }
    let lowered: Vec<char> = word.iter().map(|&ch| lower(ch)).collect();
    if !t.prepare(c, &lowered) {
        return list;
    }
    let mut out = String::new();
    let mut count = 1;
    while count < MAX_ANALYSIS_COUNT && t.next(c, &mut out) {
        count += 1;
        let o: Vec<char> = out.chars().collect();
        if !is_valid_analysis(&o) {
            continue;
        }
        let mut a = Analysis::default();
        let mut structure = parse_structure(&o, word.len());
        parse_basic_attributes(&mut a, &o);
        fix_structure(&mut structure, &o);
        a.structure = structure;
        a.base = parse_base(&o);
        if a.participle == Some("past_passive") {
            a.class = Some("laatusana");
        }
        if a.comparison.is_none() {
            if matches!(a.class, Some("laatusana" | "nimisana_laatusana")) {
                a.comparison = Some("positive");
            }
        } else if a.class == Some("nimisana") {
            a.comparison = None;
        }
        let dup = duplicate_org_name(&a, &o);
        list.push(a);
        list.extend(dup);
    }
    list
}
