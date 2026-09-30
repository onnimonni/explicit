//! Spelling for non-English prose: Finnish through Voikko, Swedish and bundled German,
//! French, Spanish and Portuguese through Hunspell, and configured Hunspell dictionaries.

use std::borrow::Cow;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex, PoisonError};

use spellbook::Dictionary;
use unicode_normalization::char::{compose, is_combining_mark};
use unicode_normalization::{IsNormalized, UnicodeNormalization, is_nfc_quick};

use super::lint::{Lint, LintKind, Span};
use super::words::Dialect;
use crate::config::Config;

fn nfc(word: &str) -> Cow<'_, str> {
    if word.is_ascii() || is_nfc_quick(word.chars()) == IsNormalized::Yes {
        Cow::Borrowed(word)
    } else {
        Cow::Owned(word.nfc().collect())
    }
}

/// A spell checker for one language.
pub trait LangSpeller: Send + Sync {
    /// `word` is spelled right as written (case as the language requires).
    fn check(&self, word: &str) -> bool;
    /// Corrections, best first.
    fn suggest(&self, word: &str) -> Vec<String>;
    /// The Finnish morphology, for Finnish grammar rules.
    fn voikko(&self) -> Option<&'static crate::voikko::Voikko> {
        None
    }
}

thread_local! {
    static ENGLISH_DIALECT: std::cell::Cell<Dialect> = const { std::cell::Cell::new(Dialect::American) };
}

/// Run `f` with `d` as the dialect of English words inside text of another language
/// (`prose.dialect`; American when unset).
pub fn with_english_dialect<R>(d: Dialect, f: impl FnOnce() -> R) -> R {
    let old = ENGLISH_DIALECT.with(|c| c.replace(d));
    let r = f();
    ENGLISH_DIALECT.with(|c| c.set(old));
    r
}

/// The project's English speller, for English words inside Finnish or Swedish text.
fn english() -> &'static super::spell::Speller {
    super::spell::speller(ENGLISH_DIALECT.with(std::cell::Cell::get))
}

/// [`LangSpeller::suggest`] shared by all threads; suggestions cost milliseconds per word.
pub fn suggestions(sp: &dyn LangSpeller, word: &str) -> Arc<[String]> {
    type Words = HashMap<Box<str>, Arc<[String]>>;
    #[derive(Default)]
    struct Cache {
        by_speller: HashMap<usize, Words>,
        len: usize,
    }
    static CACHE: LazyLock<Mutex<Cache>> = LazyLock::new(Default::default);
    let key = std::ptr::from_ref(sp).cast::<()>() as usize;
    if let Some(hit) = CACHE
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .by_speller
        .get(&key)
        .and_then(|words| words.get(word))
    {
        return hit.clone();
    }
    let found: Arc<[String]> = sp.suggest(word).into();
    let mut cache = CACHE.lock().unwrap_or_else(PoisonError::into_inner);
    if cache.len >= 100_000 {
        cache.by_speller.clear();
        cache.len = 0;
    }
    let new_word = cache
        .by_speller
        .entry(key)
        .or_default()
        .insert(word.into(), found.clone())
        .is_none();
    cache.len += usize::from(new_word);
    found
}

/// Primary language subtag of a BCP 47 tag or language name, lowercase: `fi-FI` -> `fi`,
/// `Swedish` -> `sv`; empty and `english` -> `en`.
pub fn primary(tag: &str) -> String {
    let t = tag.trim().to_ascii_lowercase();
    let p = t.split(['-', '_']).next().unwrap_or("");
    match p {
        "" | "english" => "en",
        "fin" | "finnish" | "suomi" => "fi",
        "swe" | "swedish" | "svenska" => "sv",
        _ => p,
    }
    .to_string()
}

struct Finnish(&'static crate::voikko::Voikko);

impl LangSpeller for Finnish {
    fn check(&self, word: &str) -> bool {
        self.0.spell(&nfc(word))
    }
    fn suggest(&self, word: &str) -> Vec<String> {
        self.0.suggest(&nfc(word))
    }
    fn voikko(&self) -> Option<&'static crate::voikko::Voikko> {
        Some(self.0)
    }
}

struct Hunspell(Dictionary);

impl LangSpeller for Hunspell {
    fn check(&self, word: &str) -> bool {
        self.0.check(&nfc(word))
    }
    fn suggest(&self, word: &str) -> Vec<String> {
        let mut out = Vec::new();
        self.0
            .suggester()
            .with_ngram_suggestions(false)
            .suggest(&nfc(word), &mut out);
        // Compound fragments (`sjukvårdsystem-`) are no corrections.
        out.retain(|s| !s.starts_with('-') && !s.ends_with('-'));
        out
    }
}

/// Portuguese variants use incompatible affix flags; never merge their word lists.
struct Portuguese(Hunspell, Hunspell);

impl LangSpeller for Portuguese {
    fn check(&self, word: &str) -> bool {
        self.0.check(word) || self.1.check(word)
    }

    fn suggest(&self, word: &str) -> Vec<String> {
        let mut out = self.0.suggest(word);
        for suggestion in self.1.suggest(word) {
            if !out.contains(&suggestion) {
                out.push(suggestion);
            }
        }
        out
    }
}

/// Decode a bundled Hunspell dictionary only when its language is used.
fn bundled_hunspell(code: &str) -> Result<Dictionary, String> {
    let (aff, compressed): (&str, &[u8]) = match code {
        "de" => (
            include_str!("../../dictionaries/de/index.aff"),
            include_bytes!("../../dictionaries/de/index.dic.zlib"),
        ),
        "fr" => (
            include_str!("../../dictionaries/fr/index.aff"),
            include_bytes!("../../dictionaries/fr/index.dic.zlib"),
        ),
        "es" => (
            include_str!("../../dictionaries/es/index.aff"),
            include_bytes!("../../dictionaries/es/index.dic.zlib"),
        ),
        "pt" => (
            include_str!("../../dictionaries/pt/index.aff"),
            include_bytes!("../../dictionaries/pt/index.dic.zlib"),
        ),
        "pt-BR" => (
            include_str!("../../dictionaries/pt/brazil.aff"),
            include_bytes!("../../dictionaries/pt/brazil.dic.zlib"),
        ),
        _ => return Err(format!("no bundled Hunspell dictionary for {code}")),
    };
    let dic = miniz_oxide::inflate::decompress_to_vec_zlib(compressed)
        .map_err(|e| format!("bundled {code} dictionary: {e:?}"))?;
    let dic = std::str::from_utf8(&dic).map_err(|e| e.to_string())?;
    Dictionary::new(aff, dic).map_err(|e| format!("bundled {code} dictionary: {e}"))
}

/// Hunspell dictionary at `path`: an `.aff` file with its `.dic` beside it, or a directory
/// with `index.aff` / `index.dic` (or a single `*.aff`).
fn load_hunspell(path: &Path) -> Result<Dictionary, String> {
    let aff = if path.is_dir() {
        let index = path.join("index.aff");
        if index.is_file() {
            index
        } else {
            std::fs::read_dir(path)
                .map_err(|e| format!("{}: {e}", path.display()))?
                .filter_map(Result::ok)
                .map(|e| e.path())
                .find(|p| p.extension().is_some_and(|x| x == "aff"))
                .ok_or_else(|| format!("{}: no .aff file", path.display()))?
        }
    } else {
        path.to_path_buf()
    };
    let dic = aff.with_extension("dic");
    let read = |p: &Path| std::fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()));
    Dictionary::new(&read(&aff)?, &read(&dic)?).map_err(|e| format!("{}: {e}", aff.display()))
}

/// The bundled Swedish dictionary (sv_SE plus the sv_FI-only entries).
#[cfg(feature = "swedish")]
fn swedish() -> Result<Dictionary, String> {
    static DIC: &[u8] = include_bytes!("../../dictionaries/sv/index.dic.zlib");
    let dic = miniz_oxide::inflate::decompress_to_vec_zlib(DIC)
        .map_err(|e| format!("bundled Swedish dictionary: {e:?}"))?;
    let dic = String::from_utf8(dic).map_err(|e| e.to_string())?;
    let mut d = Dictionary::new(include_str!("../../dictionaries/sv/index.aff"), &dic)
        .map_err(|e| e.to_string())?;
    let fi = include_str!("../../dictionaries/sv/sv-FI.dic");
    for line in fi.lines() {
        let _ = d.add(line);
    }
    // Noun genders for the Swedish article and adjective rules, from the same flags.
    super::grammar_sv::index_genders(&[&dic, fi]);
    Ok(d)
}

#[cfg(not(feature = "swedish"))]
fn swedish() -> Result<Dictionary, String> {
    Err("built without the `swedish` feature".into())
}

/// Configured dictionary path for `code`, resolved against the root.
fn configured_path(code: &str, config: &Config) -> Option<PathBuf> {
    let p = config.languages.get(code)?.dictionary_path.as_ref()?;
    Some(if p.is_absolute() {
        p.clone()
    } else {
        config.root.join(p)
    })
}

/// Dictionary files `config` points at, for the results cache key.
pub fn dictionary_files(config: &Config) -> Vec<PathBuf> {
    config
        .languages
        .keys()
        .filter_map(|code| configured_path(code, config))
        .flat_map(|p| files_of(&p))
        .collect()
}

/// The files a configured dictionary path reads: an `.aff` and its `.dic`, a `mor.vfst`, or
/// those inside a dictionary directory.
fn files_of(p: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if p.is_file() {
        if p.extension().is_some_and(|x| x == "aff") {
            out.push(p.with_extension("dic"));
        }
        out.push(p.to_path_buf());
    } else {
        for f in [
            "mor.vfst",
            "mor-standard/mor.vfst",
            "5/mor-standard/mor.vfst",
            "index.aff",
            "index.dic",
        ] {
            let f = p.join(f);
            if f.is_file() {
                out.push(f);
            }
        }
    }
    out
}

/// Content hashes of dictionary files, read once until [`refresh_dictionaries`].
static FILE_HASHES: LazyLock<Mutex<HashMap<PathBuf, u64>>> = LazyLock::new(Default::default);

/// Hash of the contents of the files of dictionary path `p` (0 when there are none).
fn dictionary_hash(p: &Path) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    for f in files_of(p) {
        let mut hashes = FILE_HASHES.lock().unwrap_or_else(PoisonError::into_inner);
        let fh = *hashes.entry(f.clone()).or_insert_with(|| {
            let mut fh = std::collections::hash_map::DefaultHasher::new();
            std::fs::read(&f).unwrap_or_default().hash(&mut fh);
            fh.finish()
        });
        (f, fh).hash(&mut h);
    }
    h.finish()
}

/// Forget the dictionary file hashes: the next [`speller`] call re-reads the configured
/// dictionaries and rebuilds any whose contents changed (watch mode, after a file event).
pub fn refresh_dictionaries() {
    FILE_HASHES
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clear();
}

/// Spellers by language, configured path and content hash of its files. Replaced spellers
/// stay in the map, so the address of a live speller (a cache key elsewhere) is never reused.
type Registry = HashMap<String, HashMap<(Option<PathBuf>, u64), Option<Arc<dyn LangSpeller>>>>;

/// The speller for language `code` under `config`, built once per process and dictionary
/// contents; `None` when the language has no dictionary in this build or config. English has
/// none here (see `spell`).
pub fn speller(code: &str, config: &Config) -> Option<Arc<dyn LangSpeller>> {
    static REGISTRY: LazyLock<Mutex<Registry>> = LazyLock::new(Default::default);
    if code == "en" {
        return None;
    }
    let path = configured_path(code, config);
    let hash = path.as_deref().map_or(0, dictionary_hash);
    let key = (path, hash);
    let mut reg = REGISTRY.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(s) = reg.get(code).and_then(|variants| variants.get(&key)) {
        return s.clone();
    }
    let built: Result<Option<Arc<dyn LangSpeller>>, String> = match (code, &key.0) {
        ("fi", Some(p)) => crate::voikko::from_path_cached(p, hash)
            .map(|v| Some(Arc::new(Finnish(v)) as Arc<dyn LangSpeller>)),
        ("fi", None) => Ok(finnish_embedded()),
        (_, Some(p)) => load_hunspell(p).map(|d| Some(Arc::new(Hunspell(d)) as _)),
        ("sv", None) => match swedish() {
            Ok(d) => Ok(Some(Arc::new(Hunspell(d)) as _)),
            Err(_) if cfg!(not(feature = "swedish")) => Ok(None),
            Err(e) => Err(e),
        },
        ("pt", None) => bundled_hunspell("pt").and_then(|european| {
            bundled_hunspell("pt-BR").map(|brazilian| {
                Some(Arc::new(Portuguese(Hunspell(european), Hunspell(brazilian))) as _)
            })
        }),
        ("de" | "fr" | "es", None) => {
            bundled_hunspell(code).map(|d| Some(Arc::new(Hunspell(d)) as _))
        }
        _ => Ok(None),
    };
    let s = built.unwrap_or_else(|e| {
        eprintln!("explicit: languages.{code}: {e}");
        None
    });
    reg.entry(code.to_owned())
        .or_default()
        .insert(key, s.clone());
    s
}

#[cfg(feature = "voikko")]
fn finnish_embedded() -> Option<Arc<dyn LangSpeller>> {
    Some(Arc::new(Finnish(crate::voikko::embedded())))
}

#[cfg(not(feature = "voikko"))]
fn finnish_embedded() -> Option<Arc<dyn LangSpeller>> {
    None
}

/// Abbreviations written with a final dot, lowercase without it; accepted before a `.`.
pub fn is_abbreviation(code: &str, lower: &str) -> bool {
    const FI: &[&str] = &[
        "esim", "ks", "jne", "mm", "yms", "ym", "ns", "vrt", "ts", "tms", "huom", "ko", "em", "ao",
        "klo", "nk", "ml", "mrd", "milj", "ed", "yl", "os", "puh", "sis", "kpl", "pvm", "yo", "lk",
        "n", "s", "v", "vs", "eaa", "jaa", "jkr", "ekr", "tri", "prof", "dos", "ht", "mt", "ry",
        "oy", "ab", "oyj", "vk", "kk", "vrk", "min", "sek", "t", "tod", "läh",
    ];
    const SV: &[&str] = &[
        "t.ex", "bl.a", "osv", "dvs", "ca", "m.m", "o.s.v", "d.v.s", "fr.o.m", "t.o.m", "s.k",
        "resp", "enl", "jfr", "kl", "nr", "st", "ang", "p.g.a", "m.fl", "etc", "obs", "tel", "f.d",
        "sekr", "ordf", "adj", "bitr", "avd", "tfn", "fd", "dir", "leg", "uppl", "t.ex", "bl.a",
        "e.d", "o.d", "fr", "tf", "vd", "ev", "inkl", "exkl", "kap", "s", "sid", "tim", "min",
        "sek", "mån", "tis", "ons", "tors", "fre", "lör", "sön",
    ];
    match code {
        "fi" => FI.contains(&lower),
        "sv" => SV.contains(&lower),
        _ => false,
    }
}

/// Single letters and acronyms (`EU`, `API`, `HTTP2`): names and codes in any language.
fn neutral(w: &str) -> bool {
    w.chars().nth(1).is_none()
        || (!w.chars().any(char::is_lowercase) && w.chars().any(char::is_uppercase))
        || w.chars().all(|c| c.is_ascii_digit())
}

/// Replacing plain vowels by `ä ö å` gives a valid word: `for` -> `för`, `tama` -> `tämä`.
fn umlaut_variant(sp: &dyn LangSpeller, w: &str) -> bool {
    let chars: Vec<char> = w.chars().collect();
    let slots: Vec<usize> = (0..chars.len())
        .filter(|&i| matches!(chars[i].to_ascii_lowercase(), 'a' | 'o'))
        .take(4)
        .collect();
    if slots.is_empty() {
        return false;
    }
    let options = |c: char| -> &'static [char] {
        match c {
            'a' => &['a', 'ä', 'å'],
            'o' => &['o', 'ö'],
            'A' => &['A', 'Ä', 'Å'],
            'O' => &['O', 'Ö'],
            _ => &[],
        }
    };
    let total: usize = slots.iter().map(|&i| options(chars[i]).len()).product();
    for n in 1..total {
        let mut v = chars.clone();
        let mut k = n;
        for &i in &slots {
            let o = options(chars[i]);
            v[i] = o[k % o.len()];
            k /= o.len();
        }
        if sp.check(&v.iter().collect::<String>()) {
            return true;
        }
    }
    false
}

/// A word English knows as written (developer vocabulary inside Finnish or Swedish prose),
/// unless it is a short word or a plain-vowel spelling of a word of the language.
fn english_word(sp: &dyn LangSpeller, w: &str) -> bool {
    w.chars().count() >= 4 && super::spell::known(english(), w) && !umlaut_variant(sp, w)
}

/// Do not let an English homograph hide a native spelling with one missing accent.
/// Work is bounded and stack-only; words already accepted by the native dictionary never enter.
fn accent_variant(sp: &dyn LangSpeller, code: &str, word: &str) -> bool {
    let marks: &[char] = match code {
        "de" => &['\u{308}'],
        "fr" => &['\u{301}', '\u{300}', '\u{302}', '\u{308}', '\u{327}'],
        "es" => &['\u{301}', '\u{308}', '\u{303}'],
        "pt" => &['\u{301}', '\u{300}', '\u{302}', '\u{303}', '\u{327}'],
        _ => return false,
    };
    // Capitalized English terms may be names (a paper format or product), not native words.
    if !word.is_ascii() || word.len() > 64 || word.starts_with(|c: char| c.is_ascii_uppercase()) {
        return false;
    }
    let source = word.as_bytes();
    let mut candidate = [0u8; 128];
    for (i, &letter) in source.iter().enumerate() {
        candidate[..i].copy_from_slice(&source[..i]);
        for &mark in marks {
            let Some(accented) = compose(char::from(letter), mark) else {
                continue;
            };
            let end = i + accented.len_utf8();
            accented.encode_utf8(&mut candidate[i..end]);
            let len = end + source.len() - i - 1;
            candidate[end..len].copy_from_slice(&source[i + 1..]);
            let text =
                std::str::from_utf8(&candidate[..len]).expect("composed text is valid UTF-8");
            if sp.check(text) {
                return true;
            }
        }
    }
    false
}

/// `word` (a token of [`tokens`], followed by `next`) is spelled right in `code`: as written,
/// as an abbreviation before a dot, as an English word, or as a hyphenated compound whose
/// leading parts are names, acronyms or English words and whose last part is right.
pub fn word_ok(sp: &dyn LangSpeller, code: &str, word: &str, next: Option<char>) -> bool {
    if neutral(word) || sp.check(word) {
        return true;
    }
    let lower = word.to_lowercase();
    if next == Some('.') && (is_abbreviation(code, &lower) || sp.check(&format!("{word}."))) {
        return true;
    }
    // `Huom:`
    if next == Some(':') && is_abbreviation(code, &lower) {
        return true;
    }
    // Weekday abbreviations in calendars and tables (`ti 12.3.`, `Tis`).
    let weekday: &[&str] = match code {
        "fi" => &["ma", "ti", "ke", "to", "pe", "la", "su"],
        "sv" => &["mån", "tis", "ons", "tors", "fre", "lör", "sön"],
        _ => &[],
    };
    if weekday.contains(&lower.as_str()) {
        return true;
    }
    // `Jira:an`, `Vitest:llä`: a name or code with a case ending after a colon.
    if code == "fi"
        && let Some((stem, _)) = word.rsplit_once(':')
        && !finnish_stems(word).is_empty()
    {
        let stem_ok = |p: &str| {
            neutral(p) || p.starts_with(char::is_uppercase) || sp.check(p) || foreign_stem(sp, p)
        };
        return stem.split('-').all(stem_ok);
    }
    let loanword = |w: &str| english_word(sp, w) && !accent_variant(sp, code, w);
    if !word.contains('-') {
        return loanword(word)
            || (code == "fi"
                && (finnish_inflection(sp, word, &|_| false) || finnish_extra(sp, word)))
            || (code == "sv"
                && (swedish_extra(sp, word)
                    || swedish_compound(sp, word)
                    || SV_MISSING.contains(&lower.as_str())));
    }
    let parts: Vec<&str> = word.split('-').filter(|p| !p.is_empty()).collect();
    let Some((last, lead)) = parts.split_last() else {
        return true;
    };
    let extra = |p: &str| code == "fi" && finnish_extra(sp, p);
    if !(neutral(last) || sp.check(last) || loanword(last) || extra(last)) {
        return false;
    }
    // A hyphen may mark the main boundary of a long compound (`yksityisyyssuoja-kalvo`), and
    // leading parts may be names, acronyms, numbers or English (`Docker-kontti`).
    lead.iter().all(|p| {
        neutral(p)
            || sp.check(p)
            || p.starts_with(char::is_uppercase)
            || loanword(p)
            || foreign_stem(sp, p)
            || extra(p)
            || p.chars().any(|c| c.is_ascii_digit())
    })
}

/// `dictionaries/fi/extra.txt`: general medical and IT loanwords voikko-fi lacks, base forms.
static FI_EXTRA: LazyLock<Vec<Vec<char>>> = LazyLock::new(|| {
    include_str!("../../dictionaries/fi/extra.txt")
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| l.chars().collect())
        .collect()
});

/// Words voikko-fi inflects, covering the inflection types of [`FI_EXTRA`]: an extra word
/// inflects like the one sharing its longest ending (and vowel harmony), so `lokituksen` is
/// right because `kirjoituksen` is, `idempotentin` because `patentin` is.
const FI_ANALOGS: &[&str] = &[
    "radio",
    "studio",
    "kopio",
    "versio",
    "fuusio",
    "illuusio",
    "depressio",
    "operaatio",
    "laboratorio",
    "tekniikka",
    "paprika",
    "ohjelmointi",
    "ohjelmoida",
    "analysoida",
    "kirjoitus",
    "kirjoittaa",
    "pakkaus",
    "virus",
    "variantti",
    "konsonantti",
    "patentti",
    "magneetti",
    "konferenssi",
    "tendenssi",
    "generaattori",
    "paperi",
    "tiikeri",
    "prosessori",
    "kulttuuri",
    "katalogi",
    "motiivi",
    "perspektiivi",
    "teleskooppi",
    "terapia",
    "anemia",
    "historia",
    "geometria",
    "biologia",
    "diagnoosi",
    "analyysi",
    "fantasia",
    "idea",
    "geeni",
    "tavallinen",
    "yleinen",
    "barometri",
    "metri",
    "seminaari",
    "pelata",
    "hypätä",
    "budjetti",
    "insinööri",
    "moduuli",
    "metodi",
    "koodi",
    "systeemi",
    "mineraali",
    "sessio",
    "kamera",
    "hissi",
    "tiimi",
    "linkki",
    "mittari",
    "sinetti",
    "resepti",
    "kerätä",
    "kehittää",
    "määrittää",
];

fn back_harmony(w: &[char]) -> bool {
    w.iter().any(|c| matches!(c, 'a' | 'o' | 'u'))
}

/// `w` (lowercase) is an inflection of an extra word: it keeps all but the last three letters
/// of the base (at least four), and swapping the base for its analog ([`FI_ANALOGS`]) gives a
/// form voikko-fi knows.
fn extra_inflection(sp: &dyn LangSpeller, w: &[char]) -> Option<&'static [char]> {
    FI_EXTRA.iter().map(Vec::as_slice).find(|&b| {
        let keep = b.len().saturating_sub(3).max(b.len().min(4));
        if w.len() <= keep || w[..keep] != b[..keep] || w == b {
            return false;
        }
        let harmony = back_harmony(b);
        let mut analogs: Vec<(usize, Vec<char>)> = FI_ANALOGS
            .iter()
            .map(|a| a.chars().collect::<Vec<char>>())
            .filter(|a| back_harmony(a) == harmony)
            .map(|a| {
                let s = b
                    .iter()
                    .rev()
                    .zip(a.iter().rev())
                    .take_while(|(x, y)| x == y)
                    .count();
                (s, a)
            })
            .filter(|(s, a)| *s >= 2 && *s < b.len() && *s < a.len())
            .collect();
        analogs.sort_by_key(|(s, _)| std::cmp::Reverse(*s));
        let best = analogs.first().map_or(0, |(s, _)| *s);
        analogs
            .iter()
            .take_while(|(s, _)| *s == best)
            .any(|(s, a)| {
                let cut = b.len() - s;
                if w.len() <= cut || w[..cut] != b[..cut] {
                    return false;
                }
                let cand: String = a[..a.len() - s].iter().chain(&w[cut..]).collect();
                sp.check(&cand)
            })
    })
}

/// One letter among the first `upto` replaced or two neighbors there swapped gives a word
/// voikko-fi knows, and not only as an ad hoc compound (`klusterionnin`): then `w` is more
/// likely a typo of it than a form of an extra word.
fn near_known(sp: &dyn LangSpeller, w: &[char], upto: usize) -> bool {
    const LETTERS: &str = "abcdefghijklmnopqrstuvwxyzäö";
    let known = |v: &[char]| {
        let s: String = v.iter().collect();
        match sp.voikko() {
            Some(vk) => vk.analyses(&s).iter().any(|r| r.parts <= 1),
            None => sp.check(&s),
        }
    };
    let mut v = w.to_vec();
    for i in 0..w.len().min(upto) {
        for c in LETTERS.chars().filter(|&c| c != w[i]) {
            v[i] = c;
            if known(&v) {
                return true;
            }
        }
        v[i] = w[i];
        if i + 1 < w.len() && w[i] != w[i + 1] {
            v.swap(i, i + 1);
            let hit = known(&v);
            v.swap(i, i + 1);
            if hit {
                return true;
            }
        }
    }
    false
}

/// [`near_known`] for an extra base, once per speller and base.
fn base_near_known(sp: &dyn LangSpeller, base: &[char]) -> bool {
    type Cache = HashMap<(usize, Box<str>), bool>;
    static CACHE: LazyLock<Mutex<Cache>> = LazyLock::new(Default::default);
    let key = (
        std::ptr::from_ref(sp).cast::<()>() as usize,
        base.iter().collect::<String>().into_boxed_str(),
    );
    if let Some(&hit) = CACHE
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get(&key)
    {
        return hit;
    }
    let near = near_known(sp, base, base.len());
    CACHE
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .insert(key, near);
    near
}

/// A word of the supplementary Finnish list ([`FI_EXTRA`]), an inflection of one
/// (`infuusiota`, `lokituksen`, `idempotentit`) or a compound ending or starting with one
/// (`asennusskripti`, `infuusiopumpun`): a first part voikko-fi takes before `tieto`, a last
/// part it knows. An inflection or compound one edit from a word voikko-fi knows stays wrong
/// (a typo of that word), unless the base itself is (`lokitus` next to `lukitus`).
pub fn finnish_extra(sp: &dyn LangSpeller, word: &str) -> bool {
    if word.chars().skip(1).any(char::is_uppercase) || !word.chars().all(char::is_alphabetic) {
        return false;
    }
    let w: Vec<char> = word.to_lowercase().chars().collect();
    if w.len() < 4 {
        return false;
    }
    let exact = |c: &[char]| FI_EXTRA.iter().map(Vec::as_slice).find(|b| *b == c);
    if exact(&w).is_some() {
        return true;
    }
    let text = |c: &[char]| c.iter().collect::<String>();
    let extra_form = |c: &[char]| exact(c).or_else(|| extra_inflection(sp, c));
    // An inflection: edits in the ending are the analog's to judge (`skriptissa`); a
    // compound: anywhere.
    let (base, upto) = match extra_inflection(sp, &w) {
        Some(b) => (Some(b), b.len().saturating_sub(3).max(b.len().min(4))),
        None => (
            (4..w.len().saturating_sub(3)).find_map(|i| {
                let (head, rest) = (&w[..i], &w[i..]);
                extra_form(rest)
                    .filter(|_| sp.check(&format!("{}tieto", text(head))))
                    .or_else(|| exact(head).filter(|_| rest.len() >= 3 && sp.check(&text(rest))))
            }),
            w.len(),
        ),
    };
    // Any base the word may inflect (`välimuistitus`, `välimuistittaa`) next to a known word
    // accepts the typos of that word anyway.
    let base_near = || {
        FI_EXTRA.iter().any(|b| {
            let keep = b.len().saturating_sub(3).max(b.len().min(4));
            w.len() >= keep && w[..keep] == b[..keep] && base_near_known(sp, b)
        })
    };
    base.is_some() && (base_near() || !near_known(sp, &w, upto))
}

/// English words with a Swedish ending (`headern`, `runbooken`), which Swedish tech prose
/// uses freely; not when the stem is Swedish (`systemt` is a typo). (Closed compounds are
/// Hunspell's own job: the bundled dictionary accepts them through its compound rules;
/// splitting unknown words into known parts accepted too many typos, `sårbar|eter`.)
fn swedish_extra(sp: &dyn LangSpeller, word: &str) -> bool {
    if word.chars().count() < 6
        || !word.chars().all(char::is_alphabetic)
        || word.chars().skip(1).any(char::is_uppercase)
    {
        return false;
    }
    let lower = word.to_lowercase();
    let english = english();
    [
        "erna", "arna", "en", "et", "er", "ar", "n", "ns", "ens", "ets",
    ]
    .iter()
    .any(|suf| {
        lower.strip_suffix(suf).is_some_and(|stem| {
            stem.chars().count() >= 4 && super::spell::known(english, stem) && !sp.check(stem)
        })
    })
}

/// Finnish case endings and plural markers of loanwords and names, and possessive suffixes.
const FI_ENDINGS: &[&str] = &[
    "ineen", "itten", "iden", "ssa", "ssä", "sta", "stä", "lla", "llä", "lta", "ltä", "lle", "ksi",
    "tta", "ttä", "jen", "ien", "den", "ta", "tä", "na", "nä", "ja", "jä", "n", "a", "ä", "t",
];
const FI_POSSESSIVES: &[&str] = &["nsa", "nsä", "mme", "nne", "ni", "si"];
/// Verb and action-noun derivations of loanwords: `mentorointi`, `pseudonymisoidaan`.
const FI_DERIVATIONS: &[&str] = &[
    "isoidaan", "isointi", "isoinnin", "isoituna", "isoitu", "isoida", "oidaan", "ointi", "oinnin",
    "ointia", "oituna", "oitu", "oida",
];

/// Candidate stems of `word` read as a Finnish inflection of a loanword or name: `Jiraan` ->
/// `Jira`, `nginxistä` -> `nginx`, `committeja` -> `commit`, `Vitestillä` -> `Vitest`,
/// `Mentorini` -> `Mentor`, `Jira:an` -> `Jira` (the part before a colon only).
pub fn finnish_stems(word: &str) -> Vec<String> {
    let vowel = |c: char| "aeiouyäö".contains(c.to_ascii_lowercase());
    if let Some((stem, ending)) = word.rsplit_once(':') {
        let ok = !stem.is_empty()
            && (1..=6).contains(&ending.chars().count())
            && ending.chars().all(char::is_lowercase);
        return if ok {
            vec![stem.to_string()]
        } else {
            Vec::new()
        };
    }
    let chars: Vec<char> = word.chars().collect();
    let cut = |w: &[char], n: usize| -> Option<Vec<char>> {
        (w.len() >= n + 2).then(|| w[..w.len() - n].to_vec())
    };
    let ends = |w: &[char], e: &str| {
        let e: Vec<char> = e.chars().collect();
        w.len() > e.len() && w[w.len() - e.len()..] == e[..]
    };
    let mut bodies = vec![chars.clone()];
    for p in FI_POSSESSIVES {
        if ends(&chars, p)
            && let Some(b) = cut(&chars, p.chars().count())
        {
            bodies.push(b);
        }
    }
    // Possessive forms without a case ending (`Mentori-ni`) are bases themselves.
    let mut bases: Vec<Vec<char>> = bodies[1..].to_vec();
    for b in &bodies {
        for e in FI_ENDINGS.iter().chain(FI_DERIVATIONS) {
            if ends(b, e)
                && let Some(x) = cut(b, e.chars().count())
            {
                bases.push(x);
            }
        }
        let n = b.len();
        // Illative forms: `Jiraan`, `Linuxiin`, `Wikihin`, `kanavaseen`.
        if n >= 5 && b[n - 1] == 'n' && vowel(b[n - 2]) && b[n - 3] == b[n - 2] {
            bases.push(b[..n - 2].to_vec());
        }
        if n >= 6 && b[n - 1] == 'n' && b[n - 3] == 'h' && vowel(b[n - 2]) {
            bases.push(b[..n - 3].to_vec());
        }
        if n >= 7 && (ends(b, "seen") || ends(b, "siin")) {
            bases.push(b[..n - 4].to_vec());
        }
    }
    let mut out: Vec<String> = Vec::new();
    let mut push = |w: &[char]| {
        let s: String = w.iter().collect();
        if !out.contains(&s) {
            out.push(s);
        }
    };
    for b in bases {
        push(&b);
        let n = b.len();
        // An `i` or `e` Finnish adds after a consonant (`Vitest-i-llä`), and a doubled final
        // consonant (`commit-t-iin`).
        let mut stem = b.clone();
        if n >= 4 && matches!(b[n - 1], 'i' | 'e') && !vowel(b[n - 2]) {
            stem.truncate(n - 1);
            push(&stem);
        }
        let m = stem.len();
        if m >= 4 && stem[m - 1] == stem[m - 2] && !vowel(stem[m - 1]) {
            push(&stem[..m - 1]);
        }
    }
    out
}

/// A stem of a Finnish inflection that is a loanword: English (the project's dialect) or developer
/// vocabulary, at least three letters, and no plain-vowel spelling of a Finnish word.
fn foreign_stem(sp: &dyn LangSpeller, stem: &str) -> bool {
    if stem.chars().count() < 3 || !stem.chars().all(char::is_alphabetic) {
        return false;
    }
    let lower = stem.to_lowercase();
    let tech = super::grammar::TECH_WORDS
        .iter()
        .any(|t| t.to_lowercase() == lower);
    (tech || super::spell::known(english(), stem)) && !umlaut_variant(sp, stem)
}

/// A Finnish spelling of the loanword `word` exists (`clusterin` -> `klusterin`, `logiin` ->
/// `lokiin`, `ibuprofenia` -> `ibuprofeenia`): one foreign letter replaced or a vowel doubled.
fn nativized(sp: &dyn LangSpeller, word: &str) -> bool {
    const SUBS: &[(&str, &str)] = &[
        ("ck", "kk"),
        ("ph", "f"),
        ("th", "t"),
        ("c", "k"),
        ("c", "s"),
        ("g", "k"),
        ("x", "ks"),
        ("w", "v"),
        ("z", "ts"),
        ("q", "k"),
    ];
    let lower: Vec<char> = word.to_lowercase().chars().collect();
    let vowel = |c: char| "aeiouyäö".contains(c);
    let text = |v: &[char]| v.iter().collect::<String>();
    for i in 0..lower.len() {
        for (from, to) in SUBS {
            let f: Vec<char> = from.chars().collect();
            if lower[i..].starts_with(&f) {
                let mut v = lower[..i].to_vec();
                v.extend(to.chars());
                v.extend_from_slice(&lower[i + f.len()..]);
                if sp.check(&text(&v)) {
                    return true;
                }
            }
        }
        let c = lower[i];
        if vowel(c) && lower.get(i + 1) != Some(&c) && (i == 0 || lower[i - 1] != c) {
            let mut v = lower.clone();
            v.insert(i, c);
            if sp.check(&text(&v)) {
                return true;
            }
        }
    }
    false
}

/// `word` is a Finnish inflection of a loanword or name the speller does not know
/// (`nginxistä`, `commitin`, `App Storesta`), judged by its stems ([`finnish_stems`]): one is
/// foreign ([`foreign_stem`]) or `known` says yes, and none is a Finnish word (then the ending
/// is wrong: `testissa`).
pub fn finnish_inflection(sp: &dyn LangSpeller, word: &str, known: &dyn Fn(&str) -> bool) -> bool {
    let stems = finnish_stems(word);
    if nativized(sp, word)
        || stems
            .iter()
            .any(|s| s.chars().count() >= 4 && (sp.check(s) || sp.check(&s.to_lowercase())))
    {
        return false;
    }
    stems.iter().any(|s| foreign_stem(sp, s) || known(s))
}

/// A Swedish closed compound led by a configured or product term (`Kubernetes|klustret`,
/// `Jira|ärendet`): `head_ok` takes the first part (three or more letters), the dictionary
/// knows the rest (three or more letters, a definite or inflected form too).
pub fn swedish_led(sp: &dyn LangSpeller, word: &str, head_ok: &dyn Fn(&str) -> bool) -> bool {
    let chars: Vec<char> = word.chars().collect();
    if chars.len() < 7 || !chars.iter().all(|c| c.is_alphabetic()) {
        return false;
    }
    (3..=chars.len() - 3).any(|i| {
        let rest: String = chars[i..].iter().collect();
        rest.chars().all(char::is_lowercase)
            && sp.check(&rest)
            && head_ok(&chars[..i].iter().collect::<String>())
    })
}

/// Swedish closed compounds the bundled dictionary misses because its last part lacks the
/// compound-end flag (`kalendervy`, `meddelandekö`, `redigeringsvyn`) or because a middle part
/// lacks the middle flag (`befolkningsdatasystemet`). The first part must be a compound
/// beginning by the dictionary's own flags (`COMPOUNDBEGIN`, linking forms such as
/// `befolknings` from its affix classes): the dictionary accepts it before a probe word
/// (`kalendersystem`), which is how Hunspell's flags decide it; an English word of four or more
/// letters also begins one (`backendutveckling`). The rest is a word or compound the
/// dictionary knows (a two-letter noun when its definite form is known: `vy`, `kö`). Parts of
/// three or more letters (the first), and nothing one edit from a known word (`sårbar|eter`
/// fails the first test, `säkerhetskopa` the last).
fn swedish_compound(sp: &dyn LangSpeller, word: &str) -> bool {
    const PROBES: &[&str] = &["system", "tjänst"];
    // Prefixes that begin compounds with verbs and action nouns (`för|registrering`,
    // `om|planering`), which the dictionary flags do not always allow.
    const PREFIXES: &[&str] = &["om", "åter", "sam", "för", "under", "över", "efter"];
    let lower = word.to_lowercase();
    let chars: Vec<char> = lower.chars().collect();
    if chars.len() < 7
        || !chars.iter().all(|c| c.is_alphabetic())
        || word.chars().skip(1).any(char::is_uppercase)
    {
        return false;
    }
    let text = |c: &[char]| c.iter().collect::<String>();
    let english = english();
    // An English or developer word that is not Swedish (`pull|förfrågan`, `docker|bilden`).
    let foreign = |head: &str| {
        !sp.check(head)
            && (head.chars().count() >= 4 && super::spell::known(english, head)
                || head.chars().count() >= 3
                    && super::grammar::TECH_WORDS
                        .iter()
                        .any(|t| t.eq_ignore_ascii_case(head)))
    };
    let split = (2..=chars.len() - 2).find(|&i| {
        let (head, rest) = (text(&chars[..i]), text(&chars[i..]));
        let n = rest.chars().count();
        if PREFIXES.contains(&head.as_str()) {
            return n >= 5 && sp.check(&rest);
        }
        let rest_ok = if n == 2 {
            sp.check(&rest) && sp.check(&format!("{rest}n"))
        } else {
            sp.check(&rest) || swedish_head_form(sp, &rest)
        };
        i >= 3
            && rest_ok
            && (PROBES.iter().any(|p| sp.check(&format!("{head}{p}"))) || foreign(&head))
    });
    let Some(i) = split else {
        return false;
    };
    // A missing linking `s` (`lösenordbyte` for `lösenordsbyte`).
    let (head, rest) = (text(&chars[..i]), text(&chars[i..]));
    if !head.ends_with('s') && sp.check(&format!("{head}s{rest}")) {
        return false;
    }
    let foreign_head = foreign(&head);
    // An agent noun whose compound verb the dictionary knows (`lastbalanserare`:
    // `lastbalansera`).
    if !sp.check(&rest)
        && swedish_head_form(sp, &rest)
        && rest
            .find("erar")
            .is_some_and(|j| sp.check(&format!("{head}{}a", &rest[..j + 2])))
    {
        return true;
    }
    // A first part that is a word or prefix of its own: a suggestion changing only it is
    // another compound (`kort|vyn`, `kart|vyn`; `för|registrering`, `dör|registrering`).
    let head_word =
        PREFIXES.contains(&head.as_str()) || head.chars().count() >= 4 && sp.check(&head);
    // A linking `s` absorbed into the next part (`månad|syn` for `månads|vyn`).
    let bare_head = head
        .strip_suffix('s')
        .filter(|h| h.chars().count() >= 3 && sp.check(h));
    // A suggestion one edit away is the word meant, unless it is another compound of the same
    // first part (`start|syn` for `startvyn`) or glues a scrap in at the seam (`kalender|e|vyns`),
    // or it only changes an English first part (`pull|förfrågan`, not `full|förfrågan`).
    // A doubled or undoubled letter is a typo still (`ändrings|loggen` for `ändringslogen`).
    !suggestions(sp, word).iter().any(|s| {
        let s = s.to_lowercase();
        let c: Vec<char> = s.chars().collect();
        if s.contains([' ', '-']) || !within_one_edit(&chars, &c) {
            return false;
        }
        if foreign_head && rest.chars().count() >= 5 && s.ends_with(&rest) && !s.starts_with(&head)
        {
            return false;
        }
        let doubled = c.len().abs_diff(chars.len()) == 1 && {
            let (long, short) = if c.len() > chars.len() {
                (&c, &chars)
            } else {
                (&chars, &c)
            };
            (1..long.len()).any(|j| {
                long[j] == long[j - 1] && long[..j] == short[..j] && long[j + 1..] == short[j..]
            })
        };
        if doubled {
            return true;
        }
        if head_word && s.ends_with(&rest) && c.len() == chars.len() {
            return false;
        }
        if c.len() + 1 == chars.len()
            && bare_head.is_some_and(|h| {
                s.strip_prefix(h)
                    .is_some_and(|t| t.chars().count() >= 3 && sp.check(t))
            })
        {
            return false;
        }
        let Some(tail) = s.strip_prefix(&head) else {
            return true;
        };
        let seam = c.len() == chars.len() + 1 && tail.ends_with(&rest);
        // Another form of the same last part (`sök|vy` for `sökvyn`).
        let inflection = tail.chars().count() >= 2
            && (rest.starts_with(tail) || tail.starts_with(rest.as_str()))
            && (sp.check(tail) || sp.check(&format!("{tail}n")));
        // After a prefix another word is the typo's target (`om|startas` for `omstratas`).
        let other_word =
            !PREFIXES.contains(&head.as_str()) && tail.chars().count() >= 3 && sp.check(tail);
        !(seam || inflection || other_word)
    })
}

/// Inflections the bundled Swedish dictionary lacks.
const SV_MISSING: &[&str] = &["begäranden", "begärandena", "begärandens"];

/// A last part of a Swedish compound the dictionary lacks as a word: an agent noun of a verb
/// in `-era` (`balanserare`, `balanseraren` from `balansera`), or the plural `begäranden`.
fn swedish_head_form(sp: &dyn LangSpeller, rest: &str) -> bool {
    if SV_MISSING.contains(&rest) {
        return true;
    }
    ["are", "aren", "arna", "ares", "arens", "arnas"]
        .iter()
        .any(|e| {
            rest.strip_suffix(e).is_some_and(|stem| {
                stem.ends_with("er") && stem.chars().count() >= 5 && sp.check(&format!("{stem}a"))
            })
        })
}

/// Words of `chars`: letter and digit runs joined by `-`, `'`, `’`, `:` (`EU:n`) and `.`
/// (`t.ex`) between two of them. Char index ranges; tokens without a letter are dropped.
pub fn tokens(chars: &[char]) -> Vec<(usize, usize)> {
    let joiner = |c: char| matches!(c, '-' | '\'' | '’' | ':' | '.');
    let mut out = Vec::new();
    let mut i = 0;
    let n = chars.len();
    while i < n {
        // Email addresses, URLs and key chords (`Ctrl+Q`) are no words.
        if (i == 0 || chars[i - 1].is_whitespace()) && !chars[i].is_whitespace() {
            let end = (i..n).find(|&j| chars[j].is_whitespace()).unwrap_or(n);
            let chunk = &chars[i..end];
            let chord = chunk.contains(&'+')
                && chunk.split(|&c| c == '+').all(|p| {
                    let Some(first) = p.iter().position(|c| c.is_alphanumeric()) else {
                        return false;
                    };
                    p.iter()
                        .rposition(|c| c.is_alphanumeric())
                        .is_some_and(|last| last - first < 6)
                });
            if (chunk.contains(&'@') && chunk.contains(&'.'))
                || chunk.windows(3).any(|s| s == [':', '/', '/'])
                || chunk.starts_with(&['w', 'w', 'w', '.'])
                || chord
            {
                i = end;
                continue;
            }
        }
        if !chars[i].is_alphanumeric() {
            i += 1;
            continue;
        }
        let start = i;
        while i < n
            && (chars[i].is_alphanumeric()
                || (i > start && is_combining_mark(chars[i]))
                || (joiner(chars[i])
                    && i > start
                    && chars.get(i + 1).is_some_and(|c| c.is_alphanumeric())))
        {
            i += 1;
        }
        if chars[start..i].iter().any(|c| c.is_alphabetic()) {
            out.push((start, i));
        }
    }
    out
}

/// At most one insertion, deletion, substitution or adjacent transposition.
fn within_one_edit(a: &[char], b: &[char]) -> bool {
    if a.len().abs_diff(b.len()) > 1 {
        return false;
    }
    let prefix = a.iter().zip(b).take_while(|(x, y)| x == y).count();
    if prefix == a.len().min(b.len()) {
        return true;
    }
    if a.len() > b.len() {
        a[prefix + 1..] == b[prefix..]
    } else if b.len() > a.len() {
        a[prefix..] == b[prefix + 1..]
    } else {
        a[prefix + 1..] == b[prefix + 1..]
            || (prefix + 1 < a.len()
                && a[prefix] == b[prefix + 1]
                && a[prefix + 1] == b[prefix]
                && a[prefix + 2..] == b[prefix + 2..])
    }
}

/// A capitalized word inside a sentence (not after `. ! ? :`, a quote, bracket or list
/// marker, and not first in its segment): in Finnish and Swedish only names are capitalized
/// there.
fn mid_sentence(chars: &[char], start: usize) -> bool {
    let prev = chars[..start].iter().rev().find(|c| !c.is_whitespace());
    prev.is_some_and(|&c| c.is_alphanumeric() || matches!(c, ',' | '%' | ')' | '(' | '&' | '/'))
}

/// An unknown capitalized word inside a sentence is a name (product, organization, person)
/// unless the speller knows a name one edit away (`Helsingisä`).
fn unknown_name(sp: &dyn LangSpeller, word: &str) -> bool {
    let w: Vec<char> = word.chars().collect();
    // In `Kong-yhdyskäytvän` the name part passed; the rest is what failed.
    if !w[0].is_uppercase() || w[1..].iter().any(|c| c.is_uppercase()) || word.contains('-') {
        return false;
    }
    // Suggestions come capitalized like the word; a name is one the speller rejects in
    // lowercase.
    // Split suggestions (`Vi testillä`, `ha-Ahti`) say nothing about names.
    !suggestions(sp, word).iter().any(|s| {
        if s.contains([' ', '-']) {
            return false;
        }
        let c: Vec<char> = s.chars().collect();
        within_one_edit(&w, &c) && !sp.check(&s.to_lowercase())
    })
}

/// Short English function words between English words (`Squash and merge`).
fn english_glue(sp: &dyn LangSpeller, chars: &[char], toks: &[(usize, usize)], k: usize) -> bool {
    const GLUE: &[&str] = &[
        "and", "or", "of", "the", "to", "for", "in", "on", "a", "an", "with",
    ];
    let text = |(s, e): (usize, usize)| chars[s..e].iter().collect::<String>();
    let w = text(toks[k]).to_lowercase();
    if !GLUE.contains(&w.as_str()) {
        return false;
    }
    let english_at = |i: Option<usize>| {
        i.and_then(|i| toks.get(i)).is_some_and(|&t| {
            let t = text(t);
            super::spell::known(english(), &t) && !sp.check(&t.to_lowercase())
        })
    };
    english_at(k.checked_sub(1)) || english_at(Some(k + 1))
}

/// Capitalized words `chars` uses as names inside sentences (see [`unknown_name`]).
/// Token `k` is a capitalized word right after a given name the speller knows: a surname
/// (`Kjell Westö`, `Hilja Haahti`).
fn after_given_name(
    sp: &dyn LangSpeller,
    chars: &[char],
    toks: &[(usize, usize)],
    k: usize,
) -> bool {
    let text = |(s, e): (usize, usize)| chars[s..e].iter().collect::<String>();
    let word = text(toks[k]);
    if k == 0
        || !word.starts_with(char::is_uppercase)
        || word.chars().skip(1).any(|c| !c.is_lowercase())
    {
        return false;
    }
    let (ps, pe) = toks[k - 1];
    let prev = text((ps, pe));
    chars[pe..toks[k].0].iter().all(|&c| c == ' ')
        && prev.starts_with(char::is_uppercase)
        && prev.chars().skip(1).all(char::is_lowercase)
        && prev.chars().count() >= 2
        && sp.check(&prev)
        && !sp.check(&prev.to_lowercase())
}

pub fn names(sp: &dyn LangSpeller, code: &str, chars: &[char], out: &mut Vec<String>) {
    // German capitalizes ordinary nouns too; capitalization is not name evidence.
    if code == "de" {
        return;
    }
    let toks = tokens(chars);
    for (k, &(s, e)) in toks.iter().enumerate() {
        let word: String = chars[s..e].iter().collect();
        if word.starts_with(char::is_uppercase)
            && !word_ok(sp, code, &word, chars.get(e).copied())
            && (mid_sentence(chars, s) && unknown_name(sp, &word)
                || after_given_name(sp, chars, &toks, k))
        {
            out.push(word);
        }
    }
}

/// `word` is an inflected or plain form of one of `names`: they share all but the last two
/// letters of the shorter one, and at least four.
pub fn is_name_form(word: &str, names: &[String]) -> bool {
    let w: Vec<char> = word.chars().collect();
    names.iter().any(|n| {
        let n: Vec<char> = n.chars().collect();
        let common = w.iter().zip(&n).take_while(|(a, b)| a == b).count();
        common >= 4 && common + 2 >= w.len().min(n.len())
    })
}

/// Quoted passages of three or more words (char ranges): verbatim speech, often colloquial
/// (`"Mä vaan pushasin"`), which a dictionary of the standard language does not cover.
pub fn quotations(chars: &[char], toks: &[(usize, usize)]) -> Vec<std::ops::Range<usize>> {
    let mut out = Vec::new();
    let mut open: Option<(usize, char)> = None;
    for (i, &c) in chars.iter().enumerate() {
        let closes = |o: char| match o {
            '"' => c == '"',
            '“' | '”' => c == '”' || c == '“',
            '«' => c == '»',
            _ => false,
        };
        match open {
            Some((start, o)) if closes(o) => {
                let words = toks.iter().filter(|(s, e)| start < *s && *e <= i).count();
                if words >= 3 {
                    out.push(start..i);
                }
                open = None;
            }
            None if matches!(c, '"' | '“' | '”' | '«') => open = Some((i, c)),
            _ => {}
        }
        if c == '\n' && chars.get(i + 1) == Some(&'\n') {
            open = None;
        }
    }
    out
}

/// Misspelled words of `chars` in language `code`, as spelling lints without suggestions.
pub fn misspelled(sp: &dyn LangSpeller, code: &str, chars: &[char]) -> Vec<Lint> {
    let mut out = Vec::new();
    let toks = tokens(chars);
    let quotes = quotations(chars, &toks);
    for (k, &(s, e)) in toks.iter().enumerate() {
        if quotes.iter().any(|q| q.contains(&s)) {
            continue;
        }
        // `EU:n`, `v2:ssa` after a code span: a case ending of something unchecked.
        // `#tagi`, `@kayttaja`: hashtags and handles.
        if s > 0 && matches!(chars[s - 1], ':' | '#' | '@') {
            continue;
        }
        let word: String = chars[s..e].iter().collect();
        let next = chars.get(e).copied();
        if word_ok(sp, code, &word, next) {
            continue;
        }
        // `sosiaali- ja terveyspalvelut`: a compound prefix.
        if next == Some('-')
            && !chars.get(e + 1).is_some_and(|c| c.is_alphanumeric())
            && sp.check(&format!("{word}-"))
        {
            continue;
        }
        if english_glue(sp, chars, &toks, k) {
            continue;
        }
        // `App Storesta`: an English word capitalized inside a sentence is part of a name.
        if code != "de"
            && (mid_sentence(chars, s)
                && (unknown_name(sp, &word)
                    || word.starts_with(char::is_uppercase) && foreign_stem(sp, &word))
                || after_given_name(sp, chars, &toks, k))
        {
            continue;
        }
        out.push(Lint {
            span: Span::new(s, e),
            lint_kind: LintKind::Spelling,
            suggestions: Vec::new(),
            message: String::new(),
            priority: 63,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primary_subtags() {
        assert_eq!(primary("fi-FI"), "fi");
        assert_eq!(primary("sv_FI"), "sv");
        assert_eq!(primary(""), "en");
        assert_eq!(primary("Swedish"), "sv");
        assert_eq!(primary("de"), "de");
    }

    #[test]
    fn tokenizer() {
        let text: Vec<char> = "EU:n t.ex. maa-alue -kirjasto 2024 x".chars().collect();
        let got: Vec<String> = tokens(&text)
            .into_iter()
            .map(|(s, e)| text[s..e].iter().collect())
            .collect();
        assert_eq!(got, ["EU:n", "t.ex", "maa-alue", "kirjasto", "x"]);
    }

    #[cfg(feature = "swedish")]
    #[test]
    fn swedish_words() {
        let sp = speller("sv", &Config::default()).expect("bundled Swedish");
        for w in [
            "sjukvårdssystem",
            "användargränssnitt",
            "Uppgifterna",
            "IT-avdelningen",
        ] {
            assert!(word_ok(&*sp, "sv", w, None), "{w}");
        }
        assert!(word_ok(&*sp, "sv", "t.ex", Some('.')));
        assert!(word_ok(&*sp, "sv", "osv", Some('.')));
        for w in ["sjukvårdsystem", "anvädargränssnitt", "behandlinngen"] {
            assert!(!word_ok(&*sp, "sv", w, None), "{w}");
        }
        assert!(
            sp.suggest("anvädargränssnitt")
                .contains(&"användargränssnitt".to_string())
        );
    }

    #[cfg(feature = "voikko")]
    #[test]
    fn finnish_words() {
        let sp = speller("fi", &Config::default()).expect("bundled Finnish");
        for w in [
            "potilasasiakirjojen",
            "EU-maissa",
            "Kanta-palvelut",
            "Docker-kontti",
            "EU:n",
        ] {
            assert!(word_ok(&*sp, "fi", w, None), "{w}");
        }
        assert!(word_ok(&*sp, "fi", "esim", Some('.')));
        // A hyphen may mark a compound's main seam.
        assert!(word_ok(&*sp, "fi", "yksityisyyssuoja-kalvo", None));
        for w in ["potilasasiakrja", "Docker-kontii", "Kong-yhdyskäytvän"] {
            assert!(!word_ok(&*sp, "fi", w, None), "{w}");
        }
    }

    #[cfg(feature = "voikko")]
    #[test]
    fn finnish_inflected_loanwords_and_names() {
        let sp = speller("fi", &Config::default()).expect("bundled Finnish");
        for w in [
            "nginxistä",
            "commitin",
            "committeja",
            "Jiraan",
            "Jira:an",
            "Vitest:llä",
            "Mentorini",
            "podit",
        ] {
            assert!(word_ok(&*sp, "fi", w, None), "{w}");
        }
        // English stems follow the project's dialect (`axe` is British).
        assert!(!word_ok(&*sp, "fi", "axe-core", None));
        let british =
            with_english_dialect(Dialect::British, || word_ok(&*sp, "fi", "axe-core", None));
        assert!(british);
        // A Finnish word with a wrong ending, and loanwords with a Finnish spelling.
        for w in ["testissa", "clusterin", "logiin", "ibuprofenia"] {
            assert!(!word_ok(&*sp, "fi", w, None), "{w}");
        }
        let text: Vec<char> = "Sovellus on saatavilla App Storesta. Hoitaja: Hilja Haahti. \
            Yksikkötestit ajetaan Vitestillä. Hän kirjoitti helsinkiä väärin #tagi."
            .chars()
            .collect();
        let found: Vec<String> = misspelled(&*sp, "fi", &text)
            .iter()
            .map(|l| text[l.span.start..l.span.end].iter().collect())
            .collect();
        assert_eq!(found, ["helsinkiä"]);
    }

    /// The supplementary Finnish list: base forms, inflections by analogy and compounds; typos
    /// and wrong vowel harmony stay wrong.
    #[cfg(feature = "voikko")]
    #[test]
    fn finnish_extra_words() {
        let sp = speller("fi", &Config::default()).expect("bundled Finnish");
        for a in FI_ANALOGS {
            assert!(sp.check(a), "analog {a} must be a voikko-fi word");
        }
        let known: Vec<String> = FI_EXTRA
            .iter()
            .map(|b| b.iter().collect::<String>())
            .filter(|b| sp.check(b))
            .collect();
        assert!(
            known.is_empty(),
            "voikko-fi knows these, drop them: {known:?}"
        );
        let good = [
            "infuusio",
            "infuusiota",
            "infuusion",
            "infuusioita",
            "Okkluusio",
            "repositorioon",
            "repositoriossa",
            "replikan",
            "replikoita",
            "klusterointi",
            "klusteroinnin",
            "lokitus",
            "lokituksen",
            "lokitusta",
            "lokitukset",
            "skriptin",
            "skriptejä",
            "skriptissä",
            "mentoroinnin",
            "pseudonymisoinnin",
            "antikoagulantin",
            "antikoagulantteja",
            "välimuistituksen",
            "idempotentti",
            "idempotentin",
            "idempotentteja",
            "stentin",
            "intuboitiin",
            "palliatiivisen",
            "asennusskripti",
            "asennusskriptin",
            "infuusiopumppu",
            "infuusiopumpun",
            "skripti-tiedosto",
        ];
        let rejected: Vec<&str> = good
            .into_iter()
            .filter(|w| !word_ok(&*sp, "fi", w, None))
            .collect();
        assert!(rejected.is_empty(), "{rejected:?}");
        let bad = [
            "skriptissa",
            "infuusiossä",
            "lokitusksen",
            "skripiti",
            "infuusoita",
            "antikoagulanttin",
            "replikkaa",
        ];
        let accepted: Vec<&str> = bad
            .into_iter()
            .filter(|w| word_ok(&*sp, "fi", w, None))
            .collect();
        assert!(accepted.is_empty(), "{accepted:?}");
    }

    #[cfg(feature = "swedish")]
    #[test]
    fn swedish_compounds_by_flags() {
        let sp = speller("sv", &Config::default()).expect("bundled Swedish");
        for w in [
            "kalendervy",
            "meddelandekö",
            "Startvyn",
            "befolkningsdatasystemet",
            "backendutveckling",
            // An English or developer first part; a suggestion changing only it (`full`) is
            // no evidence of a typo.
            "pullförfrågan",
            "dockerbilden",
            // Another form of the last part, a first part changed into another word or an
            // absorbed linking `s` are no typo evidence; prefixes; agent nouns; `begäranden`.
            "redigeringsvyn",
            "månadsvyn",
            "kortvyn",
            "förregistrering",
            "lastbalanseraren",
            "ändringsbegäranden",
            "begäranden",
        ] {
            assert!(word_ok(&*sp, "sv", w, None), "{w}");
        }
        // The first part is no compound beginning, a linking `s` is missing, or a known word
        // is one edit away (`lugnande`, even with an English first part).
        for w in [
            "sårbareter",
            "lösenordbyte",
            "händelselog",
            "debiterras",
            "lungande",
            "underhålet",
            "omdirgering",
            "säkerhetstset",
            "åtkomstbegärean",
            "överföringn",
            "gränssen",
            "omstratas",
            "integrationsest",
        ] {
            assert!(!word_ok(&*sp, "sv", w, None), "{w}");
        }
        // Led by a configured term.
        let term = |h: &str| h.eq_ignore_ascii_case("Telia");
        assert!(swedish_led(&*sp, "Teliaärendet", &term));
        assert!(!swedish_led(&*sp, "Teliaärendte", &term));
        assert!(!swedish_led(&*sp, "Kalevalaärendet", &term));
    }
}
