//! Swedish grammar rules that need only the speller: split compounds (särskrivning),
//! capitalized months, weekdays and language adjectives, and the capital at a sentence start.

use std::collections::BTreeMap;

use super::lint::{Lint, LintKind, Span, Suggestion};
use super::spell_lang::{LangSpeller, tokens};

/// Rule names (`grammar/<name>`) with their descriptions for `explicit rules --all`.
pub const RULES: &[(&str, &str)] = &[
    (
        "SwedishCompoundSplit",
        "Swedish: a compound written as two words, särskrivning (`kund tjänst` -> `kundtjänst`)",
    ),
    (
        "SwedishCapitalization",
        "Swedish: months, weekdays and language adjectives are lowercase inside a sentence",
    ),
    (
        "SwedishSentenceStart",
        "Swedish: a sentence starts with a capital letter",
    ),
    (
        "SwedishDomDem",
        "Swedish: `dom` or `dem` as a subject (before a finite verb) is `de`",
    ),
];

/// Language and nationality adjectives (with `-a`, `-t`), lowercase in Swedish.
const NATIONALITY_STEMS: &[&str] = &[
    "finsk",
    "svensk",
    "engelsk",
    "tysk",
    "fransk",
    "spansk",
    "norsk",
    "dansk",
    "isländsk",
    "rysk",
    "estnisk",
    "lettisk",
    "litauisk",
    "arabisk",
    "kinesisk",
    "japansk",
    "italiensk",
    "polsk",
    "somalisk",
    "samisk",
    "nederländsk",
    "holländsk",
    "portugisisk",
    "ukrainsk",
    "turkisk",
    "persisk",
    "grekisk",
    "ungersk",
    "finlandssvensk",
    "rikssvensk",
    "amerikansk",
    "brittisk",
    "europeisk",
    "nordisk",
];

/// Nationality nouns and everyday day words, lowercase in Swedish.
const LOWERCASE_NOUNS: &[&str] = &[
    "finländare",
    "finländaren",
    "finländarna",
    "finne",
    "finnar",
    "finnarna",
    "svenskar",
    "svenskarna",
    "tyskar",
    "tyskarna",
    "danskar",
    "danskarna",
    "norrmän",
    "norrmännen",
    "engelsmän",
    "engelsmännen",
    "vardag",
    "vardagar",
    "vardagarna",
    "helgdag",
    "helgdagar",
    "helgdagarna",
    "veckodag",
    "veckodagar",
];

/// Lowercase words after a nationality adjective in proper names (`Finska viken`, `Svenska
/// kyrkan`).
const NAME_HEADS: &[&str] = &[
    "viken",
    "kyrkan",
    "akademien",
    "institutet",
    "handelshögskolan",
    "litteratursällskapet",
    "dagbladet",
    "folkpartiet",
    "kulturfonden",
    "yle",
    "teatern",
];

/// Function words and other words that do not start a compound with the next word.
const STOP: &[&str] = &[
    "och", "eller", "men", "utan", "att", "som", "om", "när", "då", "där", "hur", "vad", "vem",
    "vilken", "vilket", "vilka", "den", "det", "de", "dem", "dom", "en", "ett", "denna", "detta",
    "dessa", "min", "mitt", "mina", "din", "ditt", "dina", "sin", "sitt", "sina", "hans", "hennes",
    "dess", "deras", "vår", "vårt", "våra", "er", "ert", "era", "jag", "du", "han", "hon", "vi",
    "ni", "man", "sig", "mig", "dig", "oss", "är", "var", "vara", "blir", "bli", "blev", "har",
    "hade", "ha", "kan", "kunde", "ska", "skall", "skulle", "vill", "ville", "måste", "får",
    "fick", "bör", "borde", "inte", "också", "även", "bara", "redan", "alltid", "aldrig", "ofta",
    "sedan", "nu", "här", "så", "för", "med", "av", "på", "till", "från", "i", "vid", "under",
    "över", "efter", "före", "mellan", "genom", "mot", "utom", "hos", "inom", "enligt", "per",
    "samt", "alla", "allt", "all", "varje", "ingen", "inget", "inga", "någon", "något", "några",
    "många", "mycket", "flera", "fler", "mer", "mest", "mindre", "annan", "annat", "andra",
    "samma", "hela", "helt", "egen", "eget", "egna", "ny", "nytt", "nya", "stor", "stort", "stora",
    "liten", "litet", "små", "god", "gott", "goda", "bra", "dålig", "första", "sista", "nästa",
    "senaste", "vissa", "viss", "visst", "båda", "både", "varken", "antingen", "ju", "väl", "nog",
    "dock", "alltså", "därför", "t.ex", "ca", "cirka", "ungefär", "minst", "högst", "nya", "gamla",
    "gammal", "hög", "låg", "lång", "kort", "snabb", "enkel", "två", "tre", "fyra", "fem", "sex",
    "sju", "åtta", "nio", "tio", "elva", "tolv", "tjugo", "hundra", "tusen", "kom", "ihåg", "gör",
    "går", "tar", "ger", "ser", "vet", "sa", "del", "fel", "rätt", "sätt", "tills", "finns",
    "fanns", "anges", "gäller", "enligt",
];

const MONTHS_WEEKDAYS: &[&str] = &[
    "januari",
    "februari",
    "mars",
    "april",
    "maj",
    "juni",
    "juli",
    "augusti",
    "september",
    "oktober",
    "november",
    "december",
    "måndag",
    "tisdag",
    "onsdag",
    "torsdag",
    "fredag",
    "lördag",
    "söndag",
];

struct Word {
    start: usize,
    end: usize,
    text: String,
    lower: String,
}

fn words(chars: &[char]) -> Vec<Word> {
    tokens(chars)
        .into_iter()
        .map(|(start, end)| {
            let text: String = chars[start..end].iter().collect();
            let lower = text.to_lowercase();
            Word {
                start,
                end,
                text,
                lower,
            }
        })
        .collect()
}

fn plain_gap(chars: &[char], a: &Word, b: &Word) -> bool {
    let gap = &chars[a.end..b.start];
    !gap.is_empty()
        && gap.iter().all(|&c| c == ' ' || c == '\n')
        && gap.iter().filter(|&&c| c == '\n').count() < 2
}

fn prev_char(chars: &[char], start: usize) -> Option<char> {
    chars[..start]
        .iter()
        .rev()
        .copied()
        .find(|c| !c.is_whitespace())
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    c.next()
        .map(|f| f.to_uppercase().chain(c).collect())
        .unwrap_or_default()
}

fn lint(start: usize, end: usize, kind: LintKind, message: String, fix: &str) -> Lint {
    Lint {
        span: Span::new(start, end),
        lint_kind: kind,
        suggestions: vec![Suggestion::ReplaceWith(fix.chars().collect())],
        message,
        priority: 31,
    }
}

/// Adjective endings (`dubbel`, `digital`... are caught by their `-a` / `-t` forms).
const ADJECTIVE_ENDINGS: &[&str] = &["ig", "isk", "lig", "bar", "sam"];

/// Words before a noun that make a following `-s` a genitive (`en patients`, `två veckors`).
const GENITIVE_BEFORE: &[&str] = &[
    "en", "ett", "varje", "någon", "något", "denna", "detta", "den", "det", "de", "min", "din",
    "sin", "vår", "er", "hans", "hennes", "deras", "dess", "två", "tre", "fyra", "fem", "sex",
    "sju", "åtta", "nio", "tio", "några", "många", "flera", "alla", "båda", "hela", "samma",
    "nästa", "förra", "första", "sista", "egen", "eget", "egna",
];

/// A noun in its base form: the dictionary knows it with a singular and a plural definite
/// form of one declension (`tid`: `tiden`, `tiderna`; `resultat`: `resultatet`, `resultaten`;
/// `flicka`: `flickan`, `flickorna`; `möte`: `mötet`, `mötena`), and it has no adjective
/// ending. One form alone proves little: the compound rules accept `direkt|en`.
fn noun_base(sp: &dyn LangSpeller, w: &str) -> bool {
    if !sp.check(w)
        || w.chars().count() < 2
        || ADJECTIVE_ENDINGS
            .iter()
            .any(|e| w.ends_with(e) && w.len() > e.len() + 2)
    {
        return false;
    }
    let ok = |s: String| sp.check(&s);
    let last = w.chars().last().unwrap_or(' ');
    if let Some(stem) = w.strip_suffix('a') {
        return ok(format!("{w}n")) && (ok(format!("{stem}orna")) || ok(format!("{w}rna")));
    }
    if let Some(stem) = w.strip_suffix('e') {
        return ok(format!("{w}t")) && (ok(format!("{w}na")) || ok(format!("{w}n")))
            || ok(format!("{w}n")) && ["arna", "na"].iter().any(|p| ok(format!("{stem}{p}")))
            || ok(format!("{w}rna"));
    }
    if "iouyåäö".contains(last) {
        return (ok(format!("{w}n")) || ok(format!("{w}t")))
            && ["r", "er", "rna", "erna", "na"]
                .iter()
                .any(|p| ok(format!("{w}{p}")));
    }
    // `program` -> `programmet`: a doubled final consonant.
    [w.to_string(), format!("{w}{last}")].iter().any(|b| {
        ok(format!("{b}et")) && ok(format!("{b}en"))
            || ok(format!("{b}en"))
                && ["arna", "erna", "orna"]
                    .iter()
                    .any(|p| ok(format!("{b}{p}")))
    })
}

/// A participle, supine or s-passive of a first-conjugation verb (`sparad`, `bekräftat`) or
/// an s-passive (`ansluts`, `skickas`).
fn verb_form(sp: &dyn LangSpeller, w: &str) -> bool {
    let participle = ["d", "t"].iter().any(|e| {
        w.strip_suffix(e)
            .is_some_and(|b| b.ends_with('a') && b.chars().count() >= 4 && sp.check(b))
    });
    let passive = w.strip_suffix('s').is_some_and(|b| {
        b.chars().count() >= 3
            && sp.check(&format!("{b}a"))
            && (!noun_base(sp, b) || sp.check(&format!("{b}er")))
    });
    participle || passive
}

/// A noun by a weaker test for a first part: [`noun_base`], or a definite form alone
/// (`frånvaron`, `sökningen`) when it is no adjective (`direkt`, `fast`: `direkta`, `fastare`)
/// and, opening a clause, no verb stem (`Skriv svaret`).
fn first_noun(sp: &dyn LangSpeller, w: &str, clause_start: bool) -> bool {
    let ok = |s: String| sp.check(&s);
    if noun_base(sp, w) {
        return !(clause_start && verb_stem(sp, w));
    }
    let vowel = w.ends_with(|c: char| "aeiouyåäö".contains(c));
    let definite = if vowel {
        !w.ends_with('a') && (ok(format!("{w}n")) || ok(format!("{w}t")))
    } else {
        ok(format!("{w}en")) || ok(format!("{w}et"))
    };
    let adjective = ok(format!("{w}a"))
        && (w.ends_with('t') || ["t", "are", "ast"].iter().any(|e| ok(format!("{w}{e}"))));
    sp.check(w) && definite && !adjective && !verb_stem(sp, w)
}

/// A verb stem or imperative (`skriv`: `skriva`, `skriver`; `spara`: `sparar`, `sparade`).
fn verb_stem(sp: &dyn LangSpeller, w: &str) -> bool {
    let ok = |s: String| sp.check(&s);
    ok(format!("{w}a")) && ok(format!("{w}er"))
        || w.ends_with('a') && ok(format!("{w}r")) && (ok(format!("{w}de")) || ok(format!("{w}t")))
}

/// A noun form: a base noun, or a base noun with a definite, plural or genitive ending
/// (`tjänsten`, `uppgifter`, `anmärkningarna`); not a verb form.
fn noun_form(sp: &dyn LangSpeller, w: &str) -> bool {
    if !sp.check(w) || verb_form(sp, w) {
        return false;
    }
    if noun_base(sp, w) {
        return true;
    }
    const ENDINGS: &[&str] = &[
        "arnas", "ernas", "ornas", "arna", "erna", "orna", "ens", "ets", "en", "et", "ar", "er",
        "or", "na", "n", "t", "r", "s",
    ];
    ENDINGS.iter().any(|e| {
        w.strip_suffix(e).is_some_and(|base| {
            base.chars().count() >= 2
                && (noun_base(sp, base)
                    // `-e` / `-a` dropped before a plural (`anmärkning-ar`, `flick-or`).
                    || ["e", "a"].iter().any(|v| noun_base(sp, &format!("{base}{v}"))))
        })
    })
}

fn compound_split(sp: &dyn LangSpeller, chars: &[char], ws: &[Word], out: &mut Vec<Lint>) {
    for (k, pair) in ws.windows(2).enumerate() {
        let (a, b) = (&pair[0], &pair[1]);
        if !plain_gap(chars, a, b)
            || a.text.chars().count() < 3
            || b.text.chars().count() < 3
            || !a
                .text
                .chars()
                .chain(b.text.chars())
                .all(char::is_alphabetic)
            || !b.text.starts_with(char::is_lowercase)
            || STOP.contains(&a.lower.as_str())
            || STOP.contains(&b.lower.as_str())
            || MONTHS_WEEKDAYS.iter().any(|m| b.lower.starts_with(m))
        {
            continue;
        }
        // A capitalized first word only at a sentence start (else a name).
        if a.text.starts_with(char::is_uppercase)
            && prev_char(chars, a.start).is_some_and(|c| !".!?:".contains(c))
        {
            continue;
        }
        // Word classes from the speller: a present-tense verb is its infinitive plus `r`
        // (`visar`, `granskar`), an s-passive its infinitive plus `s` (`skickas`), an
        // adjective has `-t` and `-a` forms (`intern`, `digital`), a neuter adjective or
        // adverb is its base plus `t` (`långt`), a definite noun ends `-et`/`-ets`/`-ens`.
        let prev_word = k.checked_sub(1).map(|i| ws[i].lower.as_str());
        let ok = |w: String| sp.check(&w);
        let strip = |w: &str, suf: &str| w.strip_suffix(suf).map(str::to_string);
        let present = |w: &str| {
            (w.ends_with("ar") || w.ends_with("er") || w.ends_with("ir"))
                && strip(w, "r").is_some_and(ok)
        };
        let passive = |w: &str| {
            (w.ends_with("as") || w.ends_with("es") || w.ends_with("ds"))
                && strip(w, "s").is_some_and(ok)
        };
        let adjective = |w: &str| {
            ok(format!("{w}a")) && (ok(format!("{w}t")) || w.ends_with("at") || w.ends_with("tt"))
        };
        let neuter = |w: &str| {
            w.ends_with('t') && strip(w, "t").is_some_and(|b| b.chars().count() >= 3 && ok(b))
                || w.ends_with('a')
                    && strip(w, "a").is_some_and(|b| ok(b.clone()) && ok(format!("{b}t")))
        };
        let stem = a.lower.strip_suffix('s').unwrap_or(&a.lower);
        // Suffixes that derive nouns always link with `s` (`behörighets-`, `bokningssystem`).
        let suffix_link = a.lower.ends_with('s')
            && [
                "ning", "het", "tion", "sion", "itet", "skap", "dom", "else", "ing",
            ]
            .iter()
            .any(|x| stem.ends_with(x));
        let definite = !suffix_link
            && ["et", "ets", "ens", "arnas", "ernas"]
                .iter()
                .any(|d| a.lower.ends_with(d));
        if present(&a.lower)
            || present(&b.lower)
            || passive(&b.lower)
            || adjective(&a.lower)
            || neuter(&b.lower)
            || definite
            || a.lower.ends_with("iga")
            || a.lower.ends_with("ade")
        {
            continue;
        }
        if prev_word.is_some_and(|p| matches!(p, "att" | "kan" | "ska" | "vill" | "måste" | "bör"))
        {
            continue;
        }
        let one = format!("{}{}", a.lower, b.lower);
        // Both words nouns, the second in any form (`kund tjänsten`, `patient uppgifter`).
        // A first word that is also a verb stem opening the sentence is an imperative (`Skriv
        // svaret`).
        let clause_start = prev_char(chars, a.start).is_none_or(|c| ".!?:".contains(c));
        // `för säkerhets skull`; a definite genitive owns the next noun
        // (`form läkarens underskrift`).
        let genitive_b = ["ens", "ets", "arnas", "ernas", "ornas"]
            .iter()
            .any(|e| b.lower.ends_with(e));
        if !noun_form(sp, &b.lower) || b.lower == "skull" || genitive_b {
            continue;
        }
        // A first word with a linking `s` (`behörighets hanteringen`, `tids bokningar`).
        // Not an s-passive (`skickas`, `raderades`, `stängs`: the stem plus `a` is a verb)
        // or a participle (`jourhavandes`).
        let verb_like = a.lower.ends_with("as")
            || a.lower.ends_with("des")
            || a.lower.ends_with("tes")
            || stem.ends_with("ande")
            || stem.ends_with("ende")
            || ok(format!("{stem}a")) && !noun_base(sp, stem)
            || ok(format!("{stem}r")) && stem.ends_with('e');
        // `arbets-` from `arbete`.
        let s_form = a.lower.ends_with('s')
            && !a.lower.ends_with("ss")
            && !verb_like
            && ["", "e"]
                .iter()
                .any(|v| first_noun(sp, &format!("{stem}{v}"), false));
        // After any other noun the `s` is a genitive unless the compound is a dictionary word,
        // and an article, attribute or number before it makes it one (`föregående mötes
        // protokoll`, `två veckors semester`, `en patients journal`).
        let genitive_context = prev_word.is_some_and(|p| {
            GENITIVE_BEFORE.contains(&p)
                || p.ends_with("nde")
                || p.ends_with("ade")
                || p.chars().all(|c| c.is_ascii_digit())
        });
        let linking = s_form
            && (suffix_link && a.lower.chars().count() >= 5 || sp.check(&one) && !genitive_context);
        let plain = !s_form && first_noun(sp, &a.lower, clause_start) && sp.check(&one);
        if !(linking || plain) {
            continue;
        }
        let fix = if a.text.starts_with(char::is_uppercase) {
            capitalize(&one)
        } else {
            one
        };
        out.push(lint(
            a.start,
            b.end,
            LintKind::Spelling,
            format!("Swedish compounds are written as one word: `{fix}`."),
            &fix,
        ));
    }
}

fn capitalization(chars: &[char], ws: &[Word], out: &mut Vec<Lint>) {
    for (k, w) in ws.iter().enumerate() {
        let mut it = w.text.chars();
        if !it.next().is_some_and(char::is_uppercase) || it.any(|c| !c.is_lowercase()) {
            continue;
        }
        let inside = k > 0
            && prev_char(chars, w.start).is_some_and(|c| c.is_alphanumeric() || c == ',')
            && !chars[ws[k - 1].end..w.start].contains(&'\n');
        if !inside {
            continue;
        }
        let calendar = MONTHS_WEEKDAYS.iter().any(|m| w.lower.starts_with(m));
        let next = ws.get(k + 1);
        let in_name = next.is_some_and(|n| {
            n.text.starts_with(char::is_uppercase) || NAME_HEADS.contains(&n.lower.as_str())
        });
        let nationality = NATIONALITY_STEMS.iter().any(|s| {
            w.lower
                .strip_prefix(s)
                .is_some_and(|rest| matches!(rest, "" | "a" | "t" | "an" | "ans"))
        }) || LOWERCASE_NOUNS.contains(&w.lower.as_str());
        let language = w.lower.contains("språkig") || nationality && !in_name;
        if calendar || language {
            out.push(lint(
                w.start,
                w.end,
                LintKind::Capitalization,
                format!(
                    "Swedish writes `{}` in lowercase inside a sentence.",
                    w.lower
                ),
                &w.lower,
            ));
        }
    }
}

/// Frequent finite verbs, for `dem` / `dom` as a subject.
const FINITE_VERBS: &[&str] = &[
    "är", "var", "har", "hade", "kan", "kunde", "ska", "skall", "skulle", "vill", "ville", "måste",
    "får", "fick", "bör", "borde", "blir", "blev", "går", "gick", "kommer", "kom", "gör", "gjorde",
    "finns", "fanns", "ser", "såg", "tar", "tog", "vet", "visste", "säger", "sa", "sade", "brukar",
    "behöver", "verkar", "tycker", "tror",
];

/// Words after which a clause starts (its subject follows).
const CLAUSE_OPENERS: &[&str] = &[
    "och", "men", "om", "när", "eftersom", "då", "att", "så", "medan", "innan", "tills", "fast",
    "ifall", "därför", "sedan", "nu", "där",
];

/// `word` is a finite verb: a frequent one, or an s-passive or present tense of a verb the
/// dictionary knows (`debiteras`, `svarar`).
fn finite(sp: &dyn LangSpeller, w: &str) -> bool {
    FINITE_VERBS.contains(&w)
        || w.ends_with("as") && w.strip_suffix('s').is_some_and(|b| sp.check(b))
        || (w.ends_with("ar") || w.ends_with("er"))
            && w.strip_suffix('r')
                .is_some_and(|b| b.ends_with('a') && sp.check(b))
}

/// `dom` or `dem` as a subject: at a clause start (sentence start, after punctuation or a
/// conjunction) before a finite verb (`om dem är utdelade`, `dom kommer i morgon`), which is
/// `de`. Elsewhere `dem` is the object form and `dom` also the noun (`en fällande dom`) or
/// spoken style whose written form depends on the reading, so they are left alone. Not in
/// quotations.
fn dom_dem(sp: &dyn LangSpeller, chars: &[char], ws: &[Word], out: &mut Vec<Lint>) {
    let toks: Vec<(usize, usize)> = ws.iter().map(|w| (w.start, w.end)).collect();
    let quotes = super::spell_lang::quotations(chars, &toks);
    for (k, w) in ws.iter().enumerate() {
        if !matches!(w.lower.as_str(), "dom" | "dem")
            || w.text.chars().skip(1).any(char::is_uppercase)
            || quotes.iter().any(|q| q.contains(&w.start))
        {
            continue;
        }
        let prev = k
            .checked_sub(1)
            .map(|i| &ws[i])
            .filter(|p| plain_gap(chars, p, w));
        let next = ws.get(k + 1).filter(|n| plain_gap(chars, w, n));
        let opens_clause = k == 0
            || prev_char(chars, w.start).is_some_and(|c| ".!?:;,".contains(c))
            || prev.is_some_and(|p| CLAUSE_OPENERS.contains(&p.lower.as_str()));
        if !opens_clause || !next.is_some_and(|n| finite(sp, &n.lower)) {
            continue;
        }
        let de = if w.text.starts_with(char::is_uppercase) {
            "De"
        } else {
            "de"
        };
        let message = if w.lower == "dem" {
            "`dem` is an object; as a subject write `de`.".to_string()
        } else {
            "`dom` is spoken style; as a subject write `de`.".to_string()
        };
        let kind = if w.lower == "dem" {
            LintKind::WordChoice
        } else {
            LintKind::Style
        };
        out.push(lint(w.start, w.end, kind, message, de));
    }
}

fn sentence_start(sp: &dyn LangSpeller, chars: &[char], ws: &[Word], out: &mut Vec<Lint>) {
    for k in 1..ws.len() {
        let (a, b) = (&ws[k - 1], &ws[k]);
        if !b.text.starts_with(char::is_lowercase) {
            continue;
        }
        let gap: String = chars[a.end..b.start].iter().collect();
        let t = gap.trim();
        if !matches!(t, "." | "!" | "?") || !gap.ends_with(' ') {
            continue;
        }
        let abbreviation = a.text.chars().any(|c| c.is_ascii_digit())
            || a.text.chars().count() <= 2
            || a.text.chars().all(|c| !c.is_lowercase())
            || super::spell_lang::is_abbreviation("sv", &a.lower)
            || (sp.check(&format!("{}.", a.text)) && !sp.check(&a.text));
        if t == "." && abbreviation {
            continue;
        }
        let fix = capitalize(&b.text);
        out.push(lint(
            b.start,
            b.end,
            LintKind::Capitalization,
            "A sentence starts with a capital letter.".to_string(),
            &fix,
        ));
    }
}

/// Lints of the Swedish rules in `on` for one segment text.
pub fn lints(sp: &dyn LangSpeller, chars: &[char], on: &[&str]) -> BTreeMap<String, Vec<Lint>> {
    let ws = words(chars);
    let mut out = BTreeMap::new();
    for &name in on {
        let mut l = Vec::new();
        match name {
            "SwedishCompoundSplit" => compound_split(sp, chars, &ws, &mut l),
            "SwedishCapitalization" => capitalization(chars, &ws, &mut l),
            "SwedishSentenceStart" => sentence_start(sp, chars, &ws, &mut l),
            "SwedishDomDem" => dom_dem(sp, chars, &ws, &mut l),
            _ => {}
        }
        if !l.is_empty() {
            out.insert(name.to_string(), l);
        }
    }
    out
}

#[cfg(all(test, feature = "swedish"))]
mod tests {
    use super::*;

    fn run(text: &str) -> Vec<String> {
        let sp = crate::rules::spell_lang::speller("sv", &crate::config::Config::default())
            .expect("bundled Swedish");
        let chars: Vec<char> = text.chars().collect();
        let names: Vec<&str> = RULES.iter().map(|(n, _)| *n).collect();
        lints(&*sp, &chars, &names).into_keys().collect()
    }

    #[test]
    fn swedish_rules() {
        let got = run("Ring kund tjänsten på Tisdag. sedan loggar du in.");
        assert_eq!(
            got,
            [
                "SwedishCapitalization",
                "SwedishCompoundSplit",
                "SwedishSentenceStart"
            ]
        );
    }

    #[test]
    fn noun_phrases_are_no_split_compounds() {
        for text in [
            "Den tidigare åtgärd som gjordes räckte.",
            "Föregående mötes protokoll godkändes.",
            "Patienten fick en dubbel dos.",
            "Hon tog två veckors semester.",
            "Vi besökte Malms hälsostation.",
            "När en domän ansluts syns den.",
            "Skriv svaret här.",
            "Delade postlådor flyttas sist.",
            "Kunden sparar för säkerhets skull.",
        ] {
            assert!(run(text).is_empty(), "{text}: {:?}", run(text));
        }
        assert_eq!(run("Ring kund tjänsten."), ["SwedishCompoundSplit"]);
        assert_eq!(
            run("Se över behörighets hanteringen."),
            ["SwedishCompoundSplit"]
        );
    }

    #[test]
    fn nationalities_and_dom_dem() {
        assert_eq!(
            run("Vi talar Finska och Svenska."),
            ["SwedishCapitalization"]
        );
        assert!(run("Vi läser Svenska Dagbladet vid Finska viken.").is_empty());
        assert_eq!(
            run("Om dem är utdelade får de användas."),
            ["SwedishDomDem"]
        );
        assert_eq!(run("Vi väntar, dom kommer i morgon."), ["SwedishDomDem"]);
        assert_eq!(run("Dem har redan fått beskedet."), ["SwedishDomDem"]);
        // Only subjects: objects, the noun and articles are left alone.
        for text in [
            "Han fick en fällande dom. Vi ringer dem i morgon.",
            "Vi rekommenderar att du tillåter dom.",
            "Rapporten skickas till dem som är ansvariga.",
            "Domstolen meddelade dom i målet. Dom som överklagas prövas igen.",
        ] {
            assert!(run(text).is_empty(), "{text}: {:?}", run(text));
        }
    }

    #[test]
    fn correct_swedish_is_quiet() {
        assert!(
            run("Vi ringer kundtjänsten på tisdag, t.ex. kl. 10. Den nya versionen finns.")
                .is_empty()
        );
    }
}
