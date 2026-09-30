//! Hunspell spelling through `spellbook`, with en_US / en_GB dictionaries from
//! `wooorm/dictionaries` (SCOWL-derived; licenses in `dictionaries/*/license`).
//!
//! Hunspell's lists lack much everyday and developer vocabulary that Harper's curated
//! dictionary ([`super::words`], vendored) has, so a word either knows counts as spelled right
//! ([`harper_known`]).

use std::collections::{HashMap, HashSet};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex, PoisonError};

use spellbook::Dictionary;

use super::lint::{Lint, LintKind, Span};
use super::words::{Dialect, Words, words};

use super::patterns::{Kind, Token};

static EN_US: LazyLock<Dictionary> = LazyLock::new(|| {
    load(
        include_str!("../../dictionaries/en/index.aff"),
        include_str!("../../dictionaries/en/index.dic"),
    )
});
static EN_GB: LazyLock<Dictionary> = LazyLock::new(|| {
    load(
        include_str!("../../dictionaries/en-GB/index.aff"),
        include_str!("../../dictionaries/en-GB/index.dic"),
    )
});

fn load(aff: &str, dic: &str) -> Dictionary {
    let mut d = Dictionary::new(aff, dic).expect("bundled Hunspell dictionary parses");
    for w in super::grammar::TECH_WORDS
        .iter()
        .chain(super::grammar::TECH_COMPOUNDS)
    {
        // Only the word, no affix flags: `add` parses a `.dic` line.
        let _ = d.add(w);
    }
    d
}

/// A Hunspell dictionary plus the dialect for the Harper word-list fallback.
pub struct Speller {
    hunspell: &'static LazyLock<Dictionary>,
    dialect: Dialect,
}

static SPELLERS: [Speller; 5] = [
    Speller {
        hunspell: &EN_US,
        dialect: Dialect::American,
    },
    Speller {
        hunspell: &EN_US,
        dialect: Dialect::Canadian,
    },
    Speller {
        hunspell: &EN_GB,
        dialect: Dialect::British,
    },
    Speller {
        hunspell: &EN_GB,
        dialect: Dialect::Australian,
    },
    Speller {
        hunspell: &EN_GB,
        dialect: Dialect::Indian,
    },
];

impl Speller {
    pub fn dialect(&self) -> Dialect {
        self.dialect
    }
}

/// The speller for a `prose.dialect` value: en_GB for British-style spelling, else en_US.
pub fn dictionary(dialect: &str) -> &'static Speller {
    speller(super::grammar::dialect(dialect))
}

/// The Hunspell dictionary for `d`.
pub fn speller(d: Dialect) -> &'static Speller {
    SPELLERS
        .iter()
        .find(|s| s.dialect == d)
        .unwrap_or(&SPELLERS[0])
}

/// British spelling fragments and their American counterparts: `colour`/`color`,
/// `organise`/`organize`, `analyse`, `catalogue`, `centre`, `licence`, `programme`,
/// `travelled`, `grey`, `anaemia`.
const BRITISH_AMERICAN: &[(&str, &str)] = &[
    ("our", "or"),
    ("is", "iz"),
    ("ys", "yz"),
    ("ogue", "og"),
    ("tre", "ter"),
    ("ence", "ense"),
    ("mme", "m"),
    ("ll", "l"),
    ("grey", "gray"),
    ("cheque", "check"),
    ("tyre", "tire"),
    ("aluminium", "aluminum"),
    ("ae", "e"),
    ("oe", "e"),
];

/// `w` with one occurrence of `from` replaced by `to`, for each occurrence.
fn replacements<'a>(w: &'a str, from: &'a str, to: &'a str) -> impl Iterator<Item = String> + 'a {
    w.match_indices(from)
        .map(move |(i, _)| format!("{}{to}{}", &w[..i], &w[i + from.len()..]))
}

/// British spellings dominate `texts` (prose segments): at least 3 words spelled the British
/// way (`colour`, `behaviour`, `organisation`, `catalogue`) and at least 80% of the words
/// that have a British and an American spelling. Only words en_US lacks and en_GB has count as
/// British, and only words en_GB lacks and en_US has as American, each with its counterpart in
/// the other dictionary. `prose.dialect = "auto"` checks such a file as British.
pub fn british_dominates<'a>(texts: impl IntoIterator<Item = &'a str>) -> bool {
    let mut words: HashMap<String, usize> = HashMap::new();
    for text in texts {
        for w in text.split(|c: char| !c.is_alphabetic()) {
            let n = w.chars().count();
            // Lowercase or capitalized only: `NEIGHBOUR`, `CamelCase` are names or constants.
            if n < 4 || w.chars().skip(1).any(char::is_uppercase) {
                continue;
            }
            *words.entry(w.to_lowercase()).or_default() += 1;
        }
    }
    // Words en_US lacks with a British fragment whose American form it has.
    let candidates: Vec<(&String, usize)> = words
        .iter()
        .filter(|(w, _)| {
            !EN_US.check(w)
                && BRITISH_AMERICAN
                    .iter()
                    .any(|(b, a)| replacements(w, b, a).any(|t| EN_US.check(&t)))
        })
        .map(|(w, &c)| (w, c))
        .collect();
    if candidates.iter().map(|(_, c)| c).sum::<usize>() < 3 {
        return false;
    }
    let british: usize = candidates
        .iter()
        .filter(|(w, _)| EN_GB.check(w))
        .map(|(_, c)| c)
        .sum();
    if british < 3 {
        return false;
    }
    let american: usize = words
        .iter()
        .filter(|(w, _)| {
            BRITISH_AMERICAN.iter().any(|(b, a)| {
                w.contains(a) && replacements(w, a, b).any(|t| EN_GB.check(&t) && !EN_US.check(&t))
            }) && EN_US.check(w)
                && !EN_GB.check(w)
        })
        .map(|(_, &c)| c)
        .sum();
    british * 5 >= (british + american) * 4
}

/// `word` is spelled right: as written, with a straight apostrophe, or as `base's`; failing
/// that, as a tolerated variant ([`doubled_l_variant`]), a regular derivation ([`derived`],
/// [`plural`], [`prefixed`]) or a closed compound of known words ([`compound`]). The fallbacks
/// cover everyday and developer words both dictionaries lack, and each refuses a word one
/// edit from a known one, so typos still show.
pub fn known(s: &Speller, word: &str) -> bool {
    let straight = word.replace('’', "'");
    let base = straight.strip_suffix("'s").filter(|b| !b.is_empty());
    let forms = [
        Some(word),
        (straight != word).then_some(straight.as_str()),
        base,
    ];
    if forms.iter().flatten().any(|w| listed(s, w)) {
        return true;
    }
    // Only lowercase or capitalized words: `HTTPs`, `macOS`-like tokens are names.
    let w = base.unwrap_or(&straight);
    let plain = w.chars().all(char::is_alphabetic) && w.chars().skip(1).all(char::is_lowercase);
    let lower = w.to_lowercase();
    // A word Harper knows only in another case or dialect (`powershell`, `colour`) is wrong as
    // written, not a derivation or compound.
    plain
        && (doubled_l_variant(s, &lower)
            || (!harper_dict().contains(&lower)
                && (derived(s, &lower)
                    || plural(s, &lower)
                    || prefixed(s, &lower)
                    || compound(s, &lower))))
}

/// In a dictionary as written: Hunspell, or Harper's curated list for the dialect.
fn listed(s: &Speller, w: &str) -> bool {
    s.hunspell.check(w) || harper_known(w, s.dialect)
}

/// Harper's curated word list (affix-expanded, dialect-tagged) as a second opinion, accepting
/// what Harper's `SpellCheck` would: the exact or lowercased word in `dialect`.
fn harper_known(word: &str, dialect: Dialect) -> bool {
    let d = harper_dict();
    d.meta(word).is_some_and(|m| m.dialect_enabled(dialect))
        && (d.contains_exact(word) || d.contains_exact(&word.to_lowercase()))
}

/// Harper's curated list has `word` in some capitalization (`CMake` for `cmake`) for
/// `dialect`, as the Harper engines' comment check asks.
pub fn harper_any_case(word: &str, dialect: Dialect) -> bool {
    harper_dict()
        .meta(word)
        .is_some_and(|m| m.dialect_enabled(dialect))
}

/// Harper's curated word list, vendored ([`super::words`]).
fn harper_dict() -> &'static Words {
    words()
}

/// `cancelled`, `labelled`, `signalled`: doubled-L spellings American usage tolerates (listed
/// as variants by Merriam-Webster), when the single-L form is known and Harper has the word in
/// another dialect. `-our` and `-ise` spellings stay dialect errors.
fn doubled_l_variant(s: &Speller, lower: &str) -> bool {
    matches!(s.dialect, Dialect::American)
        && lower.contains("ll")
        && harper_dict().contains_exact(lower)
        && lower.match_indices("ll").any(|(i, _)| {
            let single = format!("{}{}", &lower[..i], &lower[i + 1..]);
            listed(s, &single)
        })
}

/// Suffixes of regular derivations for [`derived`]. No `-ly` or `-ment` (`truely` and
/// `arguement` would pass as `true + ly`, `argue + ment`) and no plural `-s`: Hunspell leaves
/// it off mass nouns on purpose (`feedbacks`, `infos`).
const SUFFIXES: &[&str] = &[
    "ization", "isation", "ities", "ity", "izing", "ising", "ized", "ised", "izes", "ises", "ize",
    "ise", "able", "ible", "ory", "ings", "ing", "ed", "ers", "er", "ors", "or", "ness", "less",
    "ful", "ally",
];

/// Agent-noun suffixes (`approver`, `resolvers`, `validator`): the other spelling (`-er` /
/// `-or`) competes, and the one-edit guard ignores the stem's own inflections (`approves`,
/// `approved` are one edit from `approver`).
const AGENT_SUFFIXES: &[&str] = &["ers", "er", "ors", "or"];

/// Suffixes that make verb forms: the stem needs another verb form in the dictionary, so
/// `serial + ing` or `chose + ing` do not pass.
const VERB_SUFFIXES: &[&str] = &[
    "able", "ible", "ings", "ing", "ed", "ers", "er", "ors", "or",
];

/// `expressivity`, `localizable`, `cloneable`: a known stem of 4+ letters plus a regular
/// suffix, with the usual spelling changes (dropped `e`, doubled consonant, `y` -> `i`). When
/// the dictionary has the derivation in another spelling (`beginning`, `writing`,
/// `organization`) it chose that one, so the word is a misspelling; so is a word one edit away
/// from a known word ([`near_known`]).
pub fn derived(s: &Speller, lower: &str) -> bool {
    let american = matches!(s.dialect, Dialect::American | Dialect::Canadian);
    // Guarded as a whole unless an agent noun matched, which is guarded on its own below.
    let mut agent = false;
    let found = SUFFIXES.iter().any(|&suf| {
        if american && suf.starts_with("is") {
            return false;
        }
        let Some(root) = lower.strip_suffix(suf) else {
            return false;
        };
        let rc: Vec<char> = root.chars().collect();
        let n = rc.len();
        if n < 4 {
            return false;
        }
        let mut stems = vec![root.to_string(), format!("{root}e")];
        if n >= 5 && rc[n - 1] == rc[n - 2] && !is_vowel(rc[n - 1]) {
            stems.push(rc[..n - 1].iter().collect());
        }
        if let Some(r) = root.strip_suffix('i') {
            stems.push(format!("{r}y"));
        }
        let Some(stem) = stems
            .into_iter()
            .find(|st| st.chars().count() >= 4 && listed(s, st))
        else {
            return false;
        };
        // The same suffix, and its other spelling (`-ise` / `-ize`), in every spelling change.
        let swapped = suf
            .strip_prefix("iz")
            .map(|r| format!("is{r}"))
            .or_else(|| suf.strip_prefix("is").map(|r| format!("iz{r}")));
        // `-ally` competes with `-ly`: `extraordinari + ally` misspells `extraordinarily`,
        // and `-er` with `-or`: `adaptor` when the dictionary has `adapter`.
        let ly = (suf == "ally").then(|| "ly".to_string());
        let is_agent = AGENT_SUFFIXES.contains(&suf);
        let agent_alt = is_agent.then(|| {
            suf.strip_prefix('e')
                .map(|r| format!("o{r}"))
                .unwrap_or_else(|| format!("e{}", &suf[1..]))
        });
        let alts: Vec<String> = [Some(suf.to_string()), swapped, ly, agent_alt]
            .into_iter()
            .flatten()
            .flat_map(|x| inflect(&stem, &x))
            .collect();
        if alts.iter().any(|a| a != lower && listed(s, a)) {
            return false;
        }
        let verb = !VERB_SUFFIXES.contains(&suf)
            || ["ed", "ing"]
                .iter()
                .flat_map(|x| inflect(&stem, x))
                .any(|a| a != lower && listed(s, &a));
        if verb && is_agent {
            // One edit from a word other than the stem's inflections: a typo.
            let own: Vec<String> = std::iter::once(stem.clone())
                .chain(
                    ["s", "ed", "ing", "er", "ers", "or", "ors"]
                        .iter()
                        .flat_map(|x| inflect(&stem, x)),
                )
                .collect();
            agent = true;
            return !near_known_except(s, lower, &own);
        }
        verb
    });
    found && (agent || !near_known(s, lower, ""))
}

/// `stem + suf` in each regular spelling: as is, `e` dropped, `y` -> `i`, final consonant
/// doubled (and `es` after a sibilant for `s`).
fn inflect(stem: &str, suf: &str) -> Vec<String> {
    let mut out = vec![format!("{stem}{suf}")];
    if let Some(st) = stem.strip_suffix('e') {
        out.push(format!("{st}{suf}"));
    }
    if let Some(st) = stem.strip_suffix('y') {
        out.push(format!("{st}i{suf}"));
        if suf == "s" {
            out.push(format!("{st}ies"));
        }
    }
    if let Some(last) = stem.chars().last().filter(|&c| !is_vowel(c)) {
        out.push(format!("{stem}{last}{suf}"));
    }
    if suf == "s" {
        out.push(format!("{stem}es"));
    }
    out
}

/// `upstreams`: the plural of a countable noun Harper knows, 4+ letters, when no other plural
/// spelling is known. Mass nouns (`feedback`, `info`) take none.
fn plural(s: &Speller, lower: &str) -> bool {
    let stems = [
        lower.strip_suffix('s').map(str::to_string),
        lower.strip_suffix("es").map(str::to_string),
        lower.strip_suffix("ies").map(|r| format!("{r}y")),
    ];
    // `resolvers`, `implementers`: an agent noun in the dictionary, plural or not, is
    // countable; only its other plural spelling competes.
    let agent = lower.strip_suffix('s').is_some_and(|stem| {
        stem.len() >= 5
            && (stem.ends_with("er") || stem.ends_with("or"))
            && listed(s, stem)
            && !listed(s, &format!("{stem}es"))
    });
    agent
        || stems.into_iter().flatten().any(|stem| {
            stem.len() >= 4
                && harper_dict()
                    .meta(&stem)
                    .is_some_and(|m| m.is_countable_noun() && !m.is_mass_noun_only())
                && harper_known(&stem, s.dialect)
                && inflect(&stem, "s")
                    .iter()
                    .all(|a| a == lower || !listed(s, a))
                && !near_known(s, lower, &stem)
        })
}

/// Prefixes that make regular derivations of any word: `unanchored`, `revalidated`,
/// `nonblocking`, `pregenerated`.
const PREFIXES: &[&str] = &[
    "un", "re", "pre", "non", "de", "sub", "multi", "inter", "over",
];

/// A [`PREFIXES`] prefix on a known word (itself possibly [`derived`]) of 4+ letters, or `un` /
/// `re` on a three-letter verb (`unrun`), not one edit from a known word (`untill` is `until`).
/// A neighbour that is the same word under another prefix (`invalidated` for `unvalidated`,
/// `regenerated` for `pregenerated`) is no typo, nor the prefix on a word one substitution from
/// a listed stem (`unpaused` for `unparsed`), or one insertion from a three-letter verb
/// (`unrung` for `unrun`). Other insertions and deletions are (`restore` for `retore`,
/// `until` for `untill`).
fn prefixed(s: &Speller, lower: &str) -> bool {
    PREFIXES.iter().any(|p| {
        lower.strip_prefix(p).is_some_and(|rest| {
            let short = rest.len() == 3;
            let stem_ok = if short {
                matches!(*p, "un" | "re") && three_letter_verb(s, rest)
            } else {
                rest.len() >= 4 && (listed(s, rest) || derived(s, rest))
            };
            if !stem_ok || !rest.starts_with(|c: char| c.is_ascii_lowercase()) {
                return false;
            }
            // Not insertions into the prefix: `res` + `tore` is `re` + `store`.
            let mut except: Vec<String> = edits(p, true, false)
                .iter()
                .map(|q| format!("{q}{rest}"))
                .collect();
            if short || listed(s, rest) {
                except.extend(edits(rest, false, short).iter().map(|r| format!("{p}{r}")));
            }
            !near_known_except(s, lower, &except)
        })
    })
}

/// `run`, `set`, `cut`: a three-letter verb Harper knows, with an `-ing` form.
fn three_letter_verb(s: &Speller, w: &str) -> bool {
    harper_known(w, s.dialect) && verb(s, w)
}

/// Every string one substitution of a lowercase letter from `w`, plus with `deletions` one
/// deletion and with `insertions` one insertion.
fn edits(w: &str, deletions: bool, insertions: bool) -> Vec<String> {
    let p: Vec<char> = w.chars().collect();
    let mut out = Vec::new();
    for i in 0..=p.len() {
        if deletions && i < p.len() {
            out.push([&p[..i], &p[i + 1..]].concat());
        }
        for l in 'a'..='z' {
            if insertions {
                out.push([&p[..i], &[l], &p[i..]].concat());
            }
            if i < p.len() && p[i] != l {
                let mut v = p.clone();
                v[i] = l;
                out.push(v);
            }
        }
    }
    out.into_iter().map(|v| v.into_iter().collect()).collect()
}

fn is_vowel(c: char) -> bool {
    "aeiouy".contains(c)
}

/// One deletion, insertion, substitution or swap of adjacent letters away from a word the
/// dictionary knows (other than `except`): `realease` (release), `chosing` (choosing),
/// `fallable` (fallible).
fn near_known(s: &Speller, lower: &str, except: &str) -> bool {
    near_known_except(s, lower, &[except.to_string()])
}

/// [`near_known`], ignoring every word in `except`.
fn near_known_except(s: &Speller, lower: &str, except: &[String]) -> bool {
    let c: Vec<char> = lower.chars().collect();
    let n = c.len();
    let known = |v: &[char]| {
        let w: String = v.iter().collect();
        w != lower && !except.contains(&w) && listed(s, &w)
    };
    let mut v: Vec<char> = Vec::with_capacity(n + 1);
    for i in 0..n {
        v.clear();
        v.extend_from_slice(&c[..i]);
        v.extend_from_slice(&c[i + 1..]);
        if known(&v) {
            return true;
        }
        if i + 1 < n && c[i] != c[i + 1] {
            v.clear();
            v.extend_from_slice(&c);
            v.swap(i, i + 1);
            if known(&v) {
                return true;
            }
        }
    }
    for i in 0..=n {
        for l in 'a'..='z' {
            v.clear();
            v.extend_from_slice(&c[..i]);
            v.push(l);
            v.extend_from_slice(&c[i..]);
            if known(&v) {
                return true;
            }
            if i < n && c[i] != l {
                v.clear();
                v.extend_from_slice(&c);
                v[i] = l;
                if known(&v) {
                    return true;
                }
            }
        }
    }
    false
}

/// Never half of a closed compound: `fromthe` and `withthat` miss a space; `everytime`,
/// `eachother` and `thankyou` are two words commonly run together by mistake.
const NOT_COMPOUND_PARTS: &[&str] = &[
    "the", "and", "for", "with", "from", "that", "this", "which", "you", "are", "was", "were",
    "not", "can", "will", "has", "have", "its", "our", "their", "there", "then", "than", "but",
    "all", "any", "each", "every", "other", "some", "into", "onto", "upon", "your", "they", "them",
    "these", "those", "what", "when", "where", "who", "whom", "how", "why", "been", "being",
    "should", "would", "could", "must", "may", "might", "shall", "did", "does", "had", "thank",
    "per", "via", "also", "only", "just", "very", "more", "most", "much", "many",
    // Suffixes, which [`derived`] judges.
    "able", "ible", "less", "ness", "ally", "ings", "ment", "ments", "tion", "tions", "sion",
];

/// `webpage`, `keybindings`, `strikethrough`, `roundtripping`: two known words run together
/// (3+ letters first, 4 to 8 second: not `document + ion`, and a long second word more often
/// makes a product name, `opentelemetry`), 7+ letters in all, neither a function word
/// ([`NOT_COMPOUND_PARTS`]), and not one edit from a known word (`substract` is
/// `subs + tract`). Also, under the same guard: a 4+ letter word plus a 3-letter one
/// (`combobox`), a verb plus a particle (`rollup`, `rollout`), a 3-letter noun plus `less`
/// (`keyless`).
pub fn compound(s: &Speller, lower: &str) -> bool {
    let n = lower.len();
    if n < 6 || !lower.is_ascii() {
        return false;
    }
    let split = (3..=n - 2).any(|i| {
        let (a, b) = lower.split_at(i);
        let free = |p: &str| !NOT_COMPOUND_PARTS.contains(&p);
        match b.len() {
            2 | 3 if PARTICLES.contains(&b) => a.len() >= 4 && free(a) && verb(s, a),
            3 => {
                n >= 7
                    && a.len() >= 4
                    && free(a)
                    && !SUFFIX_LIKE.contains(&b)
                    && harper_known(b, s.dialect)
                    && noun(b)
                    && part_known(s, a)
            }
            4 if b == "less" => a.len() == 3 && harper_known(a, s.dialect) && noun(a),
            4..=8 => n >= 7 && free(a) && free(b) && part_known(s, a) && part_known(s, b),
            _ => false,
        }
    });
    split && !near_known(s, lower, "")
}

/// Three-letter words that are also suffixes: `document + ion` misspells `documentation`.
const SUFFIX_LIKE: &[&str] = &[
    "ion", "ing", "ist", "ism", "ity", "ful", "ize", "ise", "ent", "ant", "ous", "ive", "ers",
    "est", "ary", "ory", "ian", "ish", "ate", "ify", "ics", "ial", "ely", "ess", "ies", "ied",
    "age", "ure", "dom", "ery",
];

/// Particles of phrasal-verb compounds: `rollup`, `rollout`, `dropoff`.
const PARTICLES: &[&str] = &["up", "out", "off"];

/// A verb the dictionary knows in its `-ed` or `-ing` form too.
fn verb(s: &Speller, w: &str) -> bool {
    listed(s, w)
        && ["ed", "ing"]
            .iter()
            .flat_map(|x| inflect(w, x))
            .any(|a| listed(s, &a))
}

/// A countable noun in Harper's list.
fn noun(w: &str) -> bool {
    harper_dict().meta(w).is_some_and(|m| m.is_countable_noun())
}

/// A compound part: a dictionary word, not a three-letter abbreviation only Hunspell lists.
fn part_known(s: &Speller, p: &str) -> bool {
    if p.len() == 3 {
        harper_known(p, s.dialect)
    } else {
        listed(s, p)
    }
}

/// Unknown words among `tokens`, as spelling lints without suggestions.
pub fn misspelled(d: &Speller, chars: &[char], tokens: &[Token]) -> Vec<Lint> {
    let mut out = Vec::new();
    for (i, t) in tokens
        .iter()
        .enumerate()
        .filter(|(_, t)| t.kind == Kind::Word)
    {
        let w = &chars[t.start..t.end];
        if is_informal_laughter(w) {
            continue;
        }
        let word: String = w.iter().collect();
        if known(d, &word) || in_phrase(chars, tokens, i) {
            continue;
        }
        // `(ii)`, `[iv]`, `iii.`: enumerators.
        let prev = t.start.checked_sub(1).map(|i| chars[i]);
        let next = chars.get(t.end).copied();
        if roman_numeral(&word)
            && (matches!(prev, Some('(' | '[')) || matches!(next, Some(')' | ']' | '.')))
        {
            continue;
        }
        out.push(Lint {
            span: Span::new(t.start, t.end),
            lint_kind: LintKind::Spelling,
            suggestions: Vec::new(),
            message: String::new(),
            priority: 63,
        });
    }
    out
}

/// Half of a phrase Harper lists as one entry: `et al.`, `ad hoc`, `per se`.
fn in_phrase(chars: &[char], tokens: &[Token], i: usize) -> bool {
    let t = &tokens[i];
    let text = |t: &Token| chars[t.start..t.end].iter().collect::<String>();
    // The word after (or before) `t` past a single space.
    let neighbour = |fwd: bool| -> Option<String> {
        let (sp, w) = if fwd {
            (tokens.get(i + 1)?, tokens.get(i + 2)?)
        } else {
            (
                tokens.get(i.checked_sub(1)?)?,
                tokens.get(i.checked_sub(2)?)?,
            )
        };
        (sp.kind == Kind::Space && w.kind == Kind::Word).then(|| text(w))
    };
    let me = text(t);
    let listed = |p: String| {
        let d = harper_dict();
        d.contains_exact(&p) || d.contains_exact(&format!("{p}."))
    };
    neighbour(true).is_some_and(|n| listed(format!("{me} {n}")))
        || neighbour(false).is_some_and(|p| listed(format!("{p} {me}")))
}

/// Up to three corrections from Hunspell's suggester, without its n-gram pass (5x slower
/// overall on large docs, and its extra guesses are for names and jargon).
pub fn suggest(d: &Speller, word: &str) -> Vec<String> {
    let mut out = Vec::new();
    d.hunspell
        .suggester()
        .with_ngram_suggestions(false)
        .suggest(word, &mut out);
    out.truncate(3);
    out
}

fn is_informal_laughter(chars: &[char]) -> bool {
    chars.len() >= 2
        && chars.iter().enumerate().all(|(i, c)| {
            if i % 2 == 0 {
                matches!(c, 'h' | 'H')
            } else {
                matches!(c, 'a' | 'A')
            }
        })
}

/// A roman numeral (`ii`, `xiv`): an enumerator, not a word.
pub fn roman_numeral(word: &str) -> bool {
    !word.is_empty()
        && word.len() <= 6
        && (word.chars().all(|c| "ivxlc".contains(c)) || word.chars().all(|c| "IVXLC".contains(c)))
}

/// Names of a project's dependencies (and its own package names), from the manifests in the
/// file's directory and each directory above it up to the root: `Cargo.toml`, `Cargo.lock`,
/// `package.json`, `go.mod`, `pyproject.toml`, `requirements*.txt`, `Gemfile`, `mix.exs`.
/// Lowercase, with `-` / `_` variants, parts and `rust-` / `-rs`-style affixes stripped.
#[derive(Default)]
pub struct DepNames {
    /// Per directory with manifests, nearest first.
    dirs: Vec<Arc<DirDeps>>,
    /// Hash of the manifests' paths and contents, 0 without any.
    pub fingerprint: u64,
}

impl DepNames {
    pub fn contains(&self, lower: &str) -> bool {
        self.dirs.iter().any(|d| d.names.contains(lower))
    }
}

/// The manifests of one directory.
struct DirDeps {
    names: HashSet<String>,
    fingerprint: u64,
}

type DirCache = Mutex<HashMap<PathBuf, Option<Arc<DirDeps>>>>;
static DIR_DEPS: LazyLock<DirCache> = LazyLock::new(Default::default);
/// (root, directory) -> names for the files in it.
type ChainCache = Mutex<HashMap<(PathBuf, PathBuf), Arc<DepNames>>>;
static FILE_DIR_DEPS: LazyLock<ChainCache> = LazyLock::new(Default::default);

const MANIFESTS: &[&str] = &[
    "Cargo.toml",
    "Cargo.lock",
    "package.json",
    "go.mod",
    "pyproject.toml",
    "Gemfile",
    "mix.exs",
];

/// A manifest file name ([`dep_names`] reads), so a change can drop the cached names.
pub fn is_manifest(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| MANIFESTS.contains(&n) || requirements_txt(n))
}

fn requirements_txt(name: &str) -> bool {
    name.starts_with("requirements") && name.ends_with(".txt")
}

/// Forget cached dependency names (a manifest changed in watch mode).
pub fn forget_dep_names() {
    DIR_DEPS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clear();
    FILE_DIR_DEPS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clear();
}

/// [`DepNames`] for a file at `file` in the project at `root`, read once per directory.
pub fn dep_names(root: &Path, file: &Path) -> Arc<DepNames> {
    // Relative paths (tests, stdin) would resolve against the working directory.
    let Some(start) = file.parent().filter(|p| p.is_absolute()) else {
        return Arc::default();
    };
    let key = (root.to_path_buf(), start.to_path_buf());
    if let Some(d) = FILE_DIR_DEPS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get(&key)
    {
        return d.clone();
    }
    let mut dirs = Vec::new();
    let mut dir = start;
    loop {
        dirs.extend(dir_deps(dir));
        if dir == root || !dir.starts_with(root) {
            break;
        }
        match dir.parent() {
            Some(p) => dir = p,
            None => break,
        }
    }
    let fingerprint = if dirs.is_empty() {
        0
    } else {
        let mut h = DefaultHasher::new();
        for d in &dirs {
            d.fingerprint.hash(&mut h);
        }
        h.finish()
    };
    let names = Arc::new(DepNames { dirs, fingerprint });
    FILE_DIR_DEPS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .insert(key, names.clone());
    names
}

/// Project-wide names for the spell check, built once per workspace: dependency names from
/// the manifests of every directory with a checked file (so a README sees the crates of
/// sub-projects), and capitalized names (`Quux`, `Zellij`) that recur across the checked files.
#[derive(Default)]
pub struct ProjectVocab {
    /// Lowercase dependency names, as in [`DepNames`].
    deps: HashSet<String>,
    /// Capitalized names as written; only that exact form is accepted.
    names: HashSet<String>,
    /// Hash of the manifests and the accepted names, for the results-cache key.
    pub fingerprint: u64,
}

/// Times a capitalized name must occur in the checked files to count as a name.
pub const NAME_MIN_COUNT: usize = 3;

impl ProjectVocab {
    /// `dirs`: directories of the checked files. `counts`: capitalized, unknown words with
    /// their number of occurrences. Names one edit from a lowercase dictionary word (`Teh`,
    /// `Recieve`) stay typos.
    pub fn new<'a>(
        dirs: impl IntoIterator<Item = &'a Path>,
        counts: &HashMap<String, usize>,
        s: &Speller,
    ) -> ProjectVocab {
        let mut h = DefaultHasher::new();
        let mut deps = HashSet::new();
        let mut seen: Vec<&Path> = dirs.into_iter().collect();
        seen.sort();
        seen.dedup();
        for dir in seen {
            if let Some(d) = dir_deps(dir) {
                d.fingerprint.hash(&mut h);
                deps.extend(d.names.iter().cloned());
            }
        }
        let mut names: Vec<&String> = counts
            .iter()
            .filter(|(w, n)| **n >= NAME_MIN_COUNT && project_name(s, w))
            .map(|(w, _)| w)
            .collect();
        names.sort();
        names.hash(&mut h);
        ProjectVocab {
            deps,
            names: names.into_iter().cloned().collect(),
            fingerprint: h.finish(),
        }
    }

    /// A dependency name of some sub-project (lowercase).
    pub fn has_dep(&self, lower: &str) -> bool {
        self.deps.contains(lower)
    }

    /// A recurring capitalized name, exactly as written.
    pub fn has_name(&self, word: &str) -> bool {
        self.names.contains(word)
    }
}

/// `word` may be a product or proper name: capitalized with the rest lowercase (other shapes
/// are identifiers already), 3+ letters, unknown, and not one edit from a lowercase
/// dictionary word.
/// `word` (any case) is one edit from a dictionary word: likely a typo, not a name.
pub fn near_typo(s: &Speller, word: &str) -> bool {
    near_known(s, &word.to_lowercase(), "")
}

pub fn project_name(s: &Speller, word: &str) -> bool {
    let mut chars = word.chars();
    let first_upper = chars.next().is_some_and(char::is_uppercase);
    first_upper
        && word.chars().count() >= 3
        && chars.all(char::is_lowercase)
        && !known(s, word)
        && !near_known(s, &word.to_lowercase(), "")
}

/// Surname and given-name endings of Finnish, Nordic, Slavic and Indian names: `Virtanen`,
/// `Andersson`, `Lindström`, `Kowalski`, `Ramesh`, `Rajkumar`.
const NAME_ENDINGS: &[&str] = &[
    // Finnish.
    "nen", "koski", "järvi", "jarvi", "salo", "lahti", "niemi", "mäki", "maki", "vaara", "harju",
    "korpi", "kangas", "joki", "oja", "maa", "linna", "saari", "vuori", "lampi", "ranta",
    // Swedish, Norwegian, Danish.
    "sson", "berg", "ström", "strom", "qvist", "kvist", "lund", "gren", "dahl", "holm", "stad",
    "vik", "heim", "gaard", "gård", "sen", "rud", "fors", "blad", "stedt",
    // Slavic, Baltic.
    "ski", "ska", "wicz", "czyk", "enko", "vić", "vic", "ova", "evich", "ovich", "aitis",
    // Indian.
    "kumar", "esh", "endra", "ndra", "jeet", "jit", "preet", "deep", "appa", "amma", "swamy",
    "swami", "raj", "nath", "rao", "reddy", "anth", "lakshmi", "priya", "eswar", "ananda",
];

/// Titles before a name: `Dr. Korhonen`, `Prof Lindqvist`.
const NAME_TITLES: &[&str] = &[
    "Mr",
    "Mrs",
    "Ms",
    "Mx",
    "Dr",
    "Prof",
    "Professor",
    "Sir",
    "Dame",
    "Madam",
    "Rev",
    "Saint",
    "St",
    "Mister",
    "Miss",
    "Doctor",
];

/// The word right before byte `s` of `text` across a single space (and the period of a title):
/// `Dr. Nieminen`, `Anna Korhonen`.
fn word_before(text: &str, s: usize) -> Option<&str> {
    let head = text[..s].strip_suffix(' ')?;
    let head = head.strip_suffix('.').unwrap_or(head);
    let start = head
        .char_indices()
        .rev()
        .take_while(|(_, c)| c.is_alphabetic())
        .last()
        .map(|(i, _)| i)?;
    Some(&head[start..])
}

/// The word right after byte `e` of `text` across a single space.
fn word_after(text: &str, e: usize) -> Option<&str> {
    let tail = text[e..].strip_prefix(' ')?;
    let end = tail
        .char_indices()
        .find(|(_, c)| !c.is_alphabetic())
        .map_or(tail.len(), |(i, _)| i);
    (end > 0).then(|| &tail[..end])
}

/// Byte `s` of `text` starts a sentence: nothing or sentence punctuation before it.
fn sentence_initial(text: &str, s: usize) -> bool {
    text[..s]
        .chars()
        .rev()
        .find(|c| !c.is_whitespace())
        .is_none_or(|c| {
            matches!(
                c,
                '.' | '!'
                    | '?'
                    | ':'
                    | ';'
                    | '"'
                    | '“'
                    | '('
                    | '['
                    | '*'
                    | '>'
                    | '|'
                    | '•'
                    | '-'
                    | '–'
                    | '—'
                    | '#'
            )
        })
}

/// Name-shaped: a capital, then 2+ lowercase letters.
fn name_shaped(w: &str) -> bool {
    let mut c = w.chars();
    c.next().is_some_and(char::is_uppercase)
        && w.chars().count() >= 3
        && c.all(|c| c.is_alphabetic() && c.is_lowercase())
}

/// How surely `w` (name-shaped) is a Finnish, Nordic, Slavic or Indian name by its letters:
/// 2 for letters English lacks (`ä`, `ö`, `å`, `ø`) or a doubled vowel English rarely has
/// (`Aalto`, `Tuulia`, `Siiri`), 1 for a name ending (`Virtanen`, `Andersson`, `Ramesh`) or a
/// Finnish letter pair (`Kuopio`, `Mikko`), else 0.
fn name_letters(w: &str) -> u8 {
    let lower = w.to_lowercase();
    if lower.chars().any(|c| c.is_alphabetic() && !c.is_ascii()) {
        return 2;
    }
    if ["aa", "ii", "uu", "yy"].iter().any(|d| lower.contains(d)) {
        return 2;
    }
    // `Kuopio`, `Mikko`: Finnish letter pairs, rare in English words.
    if ["uo", "kk", "yö"].iter().any(|d| lower.contains(d)) {
        return 1;
    }
    let n = lower.chars().count();
    u8::from(NAME_ENDINGS.iter().any(|e| {
        let k = e.chars().count();
        lower.ends_with(e) && (n >= k + 2 || k >= 5)
    }))
}

/// `text[s..e]`, unknown to the English speller `sp`, is a person or place name from another
/// language, not a typo: name-shaped, not one edit from an English word, and either spelled
/// like such a name ([`name_letters`]: letters like `ä` count even at a sentence start, a
/// doubled vowel or a name ending only inside a sentence) or right after a title or another
/// name (`Dr. Xu`, `Anna Korhonen`), or right before a surname that qualifies (`Mikko
/// Virtanen`).
pub fn foreign_name(sp: &Speller, text: &str, s: usize, e: usize) -> bool {
    let w = &text[s..e];
    if !name_shaped(w) {
        return false;
    }
    let letters = name_letters(w);
    let initial = sentence_initial(text, s);
    let lower = w.to_lowercase();
    let near = || lower.is_ascii() && near_known(sp, &lower, "");
    // `Jyväskylä` anywhere; `Aalto` inside a sentence, though one edit from `alto`, but not
    // `Defiinition`: a long word with a vowel typed twice.
    let doubled_typo = ["aa", "ii", "uu", "yy"]
        .iter()
        .any(|d| replacements(&lower, d, &d[..1]).any(|t| t.len() >= 6 && listed(sp, &t)));
    if letters == 2 && (!lower.is_ascii() || (!initial && !doubled_typo)) {
        return true;
    }
    // A given name before a surname spelled like one: `Sanna Nieminen`, though `Sanna` is
    // one edit from `sauna`.
    let surname_after = word_after(text, e).is_some_and(|a| {
        name_shaped(a) && name_letters(a) > 0 && !known(sp, a) && {
            let al = a.to_lowercase();
            !(al.is_ascii() && near_known(sp, &al, ""))
        }
    });
    if surname_after && !initial {
        return true;
    }
    if near() {
        return false;
    }
    if letters >= 1 && !initial {
        return true;
    }
    // A title or given name before it.
    let before = word_before(text, s);
    let titled = before.is_some_and(|b| NAME_TITLES.contains(&b));
    // `Anna`, `John`: a name the dictionary has only capitalized, or one spelled like a name.
    let after_name = before.is_some_and(|b| {
        name_shaped(b) && (name_letters(b) > 0 || (known(sp, b) && !known(sp, &b.to_lowercase())))
    });
    titled || after_name || surname_after
}

thread_local! {
    static PROJECT: std::cell::RefCell<Option<Arc<ProjectVocab>>> =
        const { std::cell::RefCell::new(None) };
}

/// Run `f` with `vocab` as the project vocabulary [`project_vocab`] returns on this thread.
pub fn with_project_vocab<R>(vocab: Option<Arc<ProjectVocab>>, f: impl FnOnce() -> R) -> R {
    let prev = PROJECT.with(|p| p.replace(vocab));
    let r = f();
    PROJECT.with(|p| p.replace(prev));
    r
}

/// The project vocabulary of the workspace being checked on this thread, if any.
pub fn project_vocab() -> Option<Arc<ProjectVocab>> {
    PROJECT.with(|p| p.borrow().clone())
}

/// The manifests of `dir`, read once per process (until [`forget_dep_names`]).
fn dir_deps(dir: &Path) -> Option<Arc<DirDeps>> {
    let cached = DIR_DEPS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get(dir)
        .cloned();
    cached.unwrap_or_else(|| {
        let d = read_dir_deps(dir).map(Arc::new);
        DIR_DEPS
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(dir.to_path_buf(), d.clone());
        d
    })
}

fn read_dir_deps(dir: &Path) -> Option<DirDeps> {
    let mut files: Vec<String> = MANIFESTS.iter().map(|m| (*m).to_string()).collect();
    if let Ok(rd) = std::fs::read_dir(dir) {
        let mut reqs: Vec<String> = rd
            .flatten()
            .filter_map(|e| e.file_name().into_string().ok())
            .filter(|n| requirements_txt(n))
            .collect();
        reqs.sort();
        files.extend(reqs);
    }
    let mut raw = Vec::new();
    let mut h = DefaultHasher::new();
    for f in files {
        if let Ok(text) = std::fs::read_to_string(dir.join(&f)) {
            f.hash(&mut h);
            text.hash(&mut h);
            raw.extend(manifest_names(&f, &text));
        }
    }
    if raw.is_empty() {
        return None;
    }
    let mut names = HashSet::new();
    for r in raw {
        add_dep_name(&mut names, &r);
    }
    Some(DirDeps {
        names,
        fingerprint: h.finish(),
    })
}

/// Package names in one manifest, as written.
fn manifest_names(file: &str, text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let quoted = |l: &str| -> Option<String> {
        let l = l.trim_start();
        let q = l.chars().next().filter(|c| matches!(c, '"' | '\''))?;
        l[1..].split(q).next().map(str::to_string)
    };
    match file {
        "Cargo.lock" => {
            out.extend(
                text.lines()
                    .filter_map(|l| l.strip_prefix("name = ").and_then(&quoted)),
            );
        }
        "Cargo.toml" => {
            let Ok(t) = text.parse::<toml::Table>() else {
                return out;
            };
            fn deps(t: &toml::Table, out: &mut Vec<String>) {
                for k in ["dependencies", "dev-dependencies", "build-dependencies"] {
                    if let Some(d) = t.get(k).and_then(|v| v.as_table()) {
                        for (name, spec) in d {
                            out.push(name.clone());
                            if let Some(p) = spec.get("package").and_then(|p| p.as_str()) {
                                out.push(p.to_string());
                            }
                        }
                    }
                }
            }
            deps(&t, &mut out);
            if let Some(w) = t.get("workspace").and_then(|v| v.as_table()) {
                deps(w, &mut out);
            }
            if let Some(targets) = t.get("target").and_then(|v| v.as_table()) {
                for spec in targets.values().filter_map(|v| v.as_table()) {
                    deps(spec, &mut out);
                }
            }
            for sec in ["package", "lib"] {
                if let Some(n) = t
                    .get(sec)
                    .and_then(|p| p.get("name"))
                    .and_then(|n| n.as_str())
                {
                    out.push(n.to_string());
                }
            }
        }
        "package.json" => {
            let Ok(v) = serde_json::from_str::<serde_json::Value>(text) else {
                return out;
            };
            if let Some(n) = v.get("name").and_then(|n| n.as_str()) {
                out.push(n.to_string());
            }
            for k in [
                "dependencies",
                "devDependencies",
                "peerDependencies",
                "optionalDependencies",
            ] {
                if let Some(d) = v.get(k).and_then(|d| d.as_object()) {
                    out.extend(d.keys().cloned());
                }
            }
        }
        "go.mod" => {
            let mut in_require = false;
            for l in text.lines().map(str::trim) {
                let l = l.split("//").next().unwrap_or("").trim();
                if in_require {
                    if l.starts_with(')') {
                        in_require = false;
                    } else if let Some(m) = l.split_whitespace().next() {
                        out.push(m.to_string());
                    }
                } else if l == "require (" {
                    in_require = true;
                } else if let Some(r) = l
                    .strip_prefix("require ")
                    .or_else(|| l.strip_prefix("module "))
                {
                    out.extend(r.split_whitespace().next().map(str::to_string));
                }
            }
        }
        "pyproject.toml" => {
            let Ok(t) = text.parse::<toml::Table>() else {
                return out;
            };
            let strings = |v: &toml::Value| -> Vec<String> {
                v.as_array()
                    .map(|a| {
                        a.iter()
                            .filter_map(|s| s.as_str())
                            .map(str::to_string)
                            .collect()
                    })
                    .unwrap_or_default()
            };
            let mut specs: Vec<String> = Vec::new();
            if let Some(p) = t.get("project") {
                if let Some(n) = p.get("name").and_then(|n| n.as_str()) {
                    out.push(n.to_string());
                }
                if let Some(d) = p.get("dependencies") {
                    specs.extend(strings(d));
                }
                if let Some(o) = p.get("optional-dependencies").and_then(|o| o.as_table()) {
                    specs.extend(o.values().flat_map(strings));
                }
            }
            if let Some(g) = t.get("dependency-groups").and_then(|o| o.as_table()) {
                specs.extend(g.values().flat_map(strings));
            }
            if let Some(poetry) = t.get("tool").and_then(|t| t.get("poetry")) {
                let mut tables = vec![poetry.get("dependencies"), poetry.get("dev-dependencies")];
                if let Some(groups) = poetry.get("group").and_then(|g| g.as_table()) {
                    tables.extend(groups.values().map(|g| g.get("dependencies")));
                }
                for d in tables.into_iter().flatten().filter_map(|d| d.as_table()) {
                    out.extend(d.keys().cloned());
                }
            }
            out.extend(specs.iter().filter_map(|r| requirement_name(r)));
        }
        "Gemfile" => {
            out.extend(
                text.lines()
                    .filter_map(|l| l.trim_start().strip_prefix("gem ").and_then(&quoted)),
            );
        }
        "mix.exs" => {
            static DEP: LazyLock<regex::Regex> = LazyLock::new(|| {
                regex::Regex::new(r"\{\s*:([a-z][a-z0-9_]*)\s*,|app:\s*:([a-z][a-z0-9_]*)")
                    .expect("hardcoded regex is valid")
            });
            out.extend(DEP.captures_iter(text).filter_map(|c| {
                c.get(1)
                    .or_else(|| c.get(2))
                    .map(|m| m.as_str().to_string())
            }));
        }
        _ if requirements_txt(file) => {
            out.extend(text.lines().filter_map(requirement_name));
        }
        _ => {}
    }
    out
}

/// The package name of a PEP 508 requirement line (`requests[socks]>=2; python_version...`).
fn requirement_name(line: &str) -> Option<String> {
    let l = line.trim();
    if l.starts_with(['#', '-']) {
        return None;
    }
    let name: String = l
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
        .collect();
    (!name.is_empty()).then_some(name)
}

/// Ecosystem affixes a name drops in prose: `rust-phf` is `phf`, `cc-rs` is `cc`.
const DEP_PREFIXES: &[&str] = &["rust-", "python-", "py-", "node-", "go-", "ruby-"];
const DEP_SUFFIXES: &[&str] = &["-rs", "-py", "-js", ".js", "-go", "-rb", "-sys"];

/// `name` and the forms prose writes it in: `-` / `_` swapped, affixes dropped, and its
/// parts of 3+ letters (`tokio` of `tokio-util`, `cobra` of `github.com/spf13/cobra`).
fn add_dep_name(set: &mut HashSet<String>, name: &str) {
    let lower = name.to_lowercase();
    for seg in lower.split('/') {
        let seg = seg.trim_start_matches('@');
        // Go major-version suffixes (`/v2`).
        if seg.is_empty() || (seg.starts_with('v') && seg[1..].chars().all(|c| c.is_ascii_digit()))
        {
            continue;
        }
        let mut forms = vec![
            seg.to_string(),
            seg.replace('-', "_"),
            seg.replace('_', "-"),
        ];
        let dashed = seg.replace('_', "-");
        for p in DEP_PREFIXES {
            if let Some(r) = dashed.strip_prefix(p) {
                forms.push(r.to_string());
            }
        }
        for sfx in DEP_SUFFIXES {
            if let Some(r) = dashed.strip_suffix(sfx) {
                forms.push(r.to_string());
            }
        }
        for f in forms.iter().filter(|f| !f.is_empty()) {
            set.insert(f.clone());
            set.insert(f.replace('-', "_"));
        }
        set.extend(
            seg.split(['-', '_', '.'])
                .filter(|p| p.len() >= 3)
                .map(str::to_string),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn foreign_name_shapes() {
        let us = dictionary("american");
        let name = |text: &str, w: &str| {
            let s = text.find(w).unwrap();
            foreign_name(us, text, s, s + w.len())
        };
        for (text, w) in [
            ("We met Korhonen there.", "Korhonen"),
            ("Thanks to Andersson for it.", "Andersson"),
            ("Ask Ramesh about it.", "Ramesh"),
            ("Ask Dr. Xiaoyu.", "Xiaoyu"),
            ("Jyväskylä is far.", "Jyväskylä"),
            ("Ask Mikko Virtanen.", "Mikko"),
            ("Near Aalto campus.", "Aalto"),
        ] {
            assert!(name(text, w), "{w} in {text}");
        }
        for (text, w) in [
            ("Ramesh wrote it.", "Ramesh"),
            ("See the Wrold map.", "Wrold"),
            ("The Language Defiinition.", "Defiinition"),
            ("The recieve call.", "recieve"),
            ("Ask Jonh.", "Jonh"),
            ("Tampere is nice.", "Tampere"),
        ] {
            assert!(!name(text, w), "{w} in {text}");
        }
    }

    #[test]
    fn british_spelling_dominance() {
        assert!(british_dominates([
            "The colour reflects its behaviour.",
            "We organise the catalogue by centre and licence."
        ]));
        assert!(british_dominates([
            "Initialising the programme; analyse the travelled grey tyre."
        ]));
        // Too few British words, or American ones alongside.
        assert!(!british_dominates(["The colour is fine."]));
        assert!(!british_dominates([
            "The colour, behaviour and catalogue.",
            "The color, behavior, catalog, center and license. We organize and analyze."
        ]));
        // Words both dictionaries know or neither knows do not count; nor do constants.
        assert!(!british_dominates([
            "Four hours of dialogue. Colourz NEIGHBOUR ColourPicker."
        ]));
    }

    #[test]
    fn derivations_compounds_and_variants() {
        let us = dictionary("american");
        for w in [
            "expressivity",
            "localizable",
            "focusable",
            "webpage",
            "keybindings",
            "strikethrough",
            "roundtripping",
            "codepoints",
            "unanchored",
            "revalidated",
            "upstreams",
            "cancelled",
            "labelled",
            "signalled",
            // Agent nouns of known verbs, closed compounds.
            "approver",
            "resolvers",
            "implementers",
            "inflater",
            "combobox",
            "rollup",
            "keyless",
        ] {
            assert!(known(us, w), "{w}");
        }
        for w in [
            // Misspellings of known words, one edit away.
            "begining",
            "writting",
            "chosing",
            "fallable",
            "realease",
            "substract",
            "untill",
            "implementes",
            // Other spelling of the same derivation, dialect, case, run-on words.
            "organisation",
            "specialised",
            "behaviour",
            "colour",
            "powershell",
            "everytime",
            "eachother",
            "thankyou",
            "fromthat",
            // Mass nouns take no plural; `-ly` and `serial + ing` are not derivations.
            "feedbacks",
            "truely",
            "serialing",
            "documention",
            // Agent nouns one edit from another word, or spelled the other way.
            "writter",
            "commiter",
            "reciever",
            "adaptor",
        ] {
            assert!(!known(us, w), "{w}");
        }
        let gb = dictionary("british");
        assert!(known(gb, "organisation") && known(gb, "behaviour"));
    }

    #[test]
    fn un_prefixed_participles() {
        let us = dictionary("american");
        for w in [
            "unvalidated",
            "unrun",
            "unmerged",
            "unreviewed",
            "untested",
            "unsanitized",
            "unversioned",
            "unparsed",
            "rerun",
        ] {
            assert!(known(us, w), "{w}");
        }
        for w in [
            "untill",
            "unecessary",
            "unrecieved",
            "unvalidatd",
            "unrn",
            "retore",
            "unallowed",
        ] {
            assert!(!known(us, w), "{w}");
        }
    }

    #[test]
    fn developer_words() {
        let us = dictionary("american");
        for w in [
            "cron",
            "crontab",
            "pnpm",
            "npx",
            "bunx",
            "argv",
            "argc",
            "keyset",
            "fanout",
            "prerender",
            "gitignored",
            "nixpkgs",
            "devenv",
            "stdlib",
            "upsert",
            "idempotency",
            "webhooks",
            "healthchecks",
            "backoff",
            "failover",
            "runbooks",
            "linter",
            "monorepo",
            "codegen",
            "subcommand",
            "tokenizer",
        ] {
            assert!(known(us, w), "{w}");
        }
    }

    #[test]
    fn phrases_harper_lists() {
        let us = dictionary("american");
        let lints = |text: &str| {
            let chars: Vec<char> = text.chars().collect();
            let tokens = super::super::patterns::tokenize(&chars);
            misspelled(us, &chars, &tokens)
                .iter()
                .map(|l| chars[l.span.start..l.span.end].iter().collect::<String>())
                .collect::<Vec<_>>()
        };
        assert!(lints("As Smith et al. showed.").is_empty());
        assert_eq!(lints("Smith et al say sentance."), ["sentance"]);
        assert_eq!(lints("The et word."), ["et"]);
    }

    #[test]
    fn dependency_names_from_manifests() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
        let sub = root.join("web");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(
            root.join("Cargo.toml"),
            "[package]\nname = \"my-tool\"\n[dependencies]\ntokio-util = \"1\"\nfoo = { package = \"rust-phf\", version = \"1\" }\n[target.'cfg(unix)'.dependencies]\nnix = \"0.1\"\n",
        )
        .unwrap();
        std::fs::write(
            sub.join("package.json"),
            r#"{"name": "web", "dependencies": {"@tanstack/react-query": "5"}}"#,
        )
        .unwrap();
        std::fs::write(
            sub.join("requirements-dev.txt"),
            "# dev\nruff>=0.1\n-r base.txt\n",
        )
        .unwrap();
        std::fs::write(
            sub.join("go.mod"),
            "module x\nrequire (\n\tgithub.com/spf13/cobra/v2 v2.0.0\n)\n",
        )
        .unwrap();
        let top = dep_names(&root, &root.join("README.md"));
        for w in [
            "my-tool",
            "my_tool",
            "tokio_util",
            "tokio",
            "phf",
            "rust-phf",
            "nix",
        ] {
            assert!(top.contains(w), "{w}");
        }
        assert!(!top.contains("cobra") && top.fingerprint != 0);
        let deep = dep_names(&root, &sub.join("docs.md"));
        for w in ["tanstack", "react-query", "ruff", "cobra", "spf13", "tokio"] {
            assert!(deep.contains(w), "{w}");
        }
        assert!(!deep.contains("v2"));
        assert_ne!(top.fingerprint, deep.fingerprint);
        assert!(dep_names(&root, Path::new("rel.md")).dirs.is_empty());
    }
}
