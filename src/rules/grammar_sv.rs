//! Swedish grammar rules that need only the speller (and, for noun gender, the bundled
//! dictionary's inflection flags): split compounds (särskrivning), capitalized months,
//! weekdays and language adjectives, the capital at a sentence start, `de` / `dem`, article
//! and adjective gender, and verb forms after a subject, `har` and `att`.

use std::collections::{BTreeMap, HashMap};
use std::sync::OnceLock;

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
    (
        "SwedishDeDem",
        "Swedish: `de` as an object is `dem` (`med de.`, `jag såg de.`); `dem` before a noun phrase is `de` (`med dem nya reglerna`)",
    ),
    (
        "SwedishArticleGender",
        "Swedish: the article agrees with the noun's gender (`en hus` -> `ett hus`, `ett bil` -> `en bil`, `den huset` -> `det huset`)",
    ),
    (
        "SwedishAdjectiveGender",
        "Swedish: an adjective after `ett` takes the neuter `-t` form (`ett stor hus` -> `ett stort hus`)",
    ),
    (
        "SwedishPresentTense",
        "Swedish: a subject at a clause start takes the present tense, not the infinitive (`han skriva` -> `han skriver`)",
    ),
    (
        "SwedishSupine",
        "Swedish: `har` / `hade` takes the supine, not the participle (`har skriven` -> `har skrivit`)",
    ),
    (
        "SwedishAttInfinitive",
        "Swedish: `att` takes the infinitive, not the present tense (`att skriver` -> `att skriva`)",
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
            || a.lower.ends_with('a') && adjective_like(sp, &a.lower)
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

/// Grammatical gender of a Swedish noun.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Gender {
    Utrum,
    Neutrum,
}

/// Noun genders from the bundled dictionary, filled by [`index_genders`]; `None` for nouns
/// of both genders, which also end a compound's head search.
static GENDERS: OnceLock<HashMap<Box<str>, Option<Gender>>> = OnceLock::new();

/// Nouns of both genders (`en/ett test`, `ett/en sms`) and homographs of two (`ett lag` a
/// team, `en lag` a law; `ett byte` a swap, `en byte` of data): no article is wrong.
const BOTH_GENDERS: &[&str] = &[
    "test", "mejl", "mail", "email", "e-mejl", "e-mail", "sms", "mms", "virus", "paraply",
    "pyjamas", "vad", "fax", "kex", "id", "apropå", "gem", "kalas", "byte", "tag", "block", "lock",
    "lag", "plan", "val", "ton", "led", "mod", "gift", "chip", "rim",
];

/// Index noun genders from Hunspell `.dic` texts of Göran Andersson's Swedish dictionary,
/// whose affix flags encode the definite singular: `B` (`-et`) and `C` (`-t`) are neuter,
/// `D` (`-en`) and `E` (`-n`) utrum. A neuter noun also carries `D` (`husen`), so neuter wins
/// within an entry; `B` and `D` on a verb in `-a` derive `-ande` forms, and `E` on `-um`
/// gives `fakta`, so those say nothing; nor does `E` beside `F` on a vowel (`ärenden`,
/// `ärendena`). Homographs whose entries disagree are dropped.
pub fn index_genders(dics: &[&str]) {
    GENDERS.get_or_init(|| {
        let mut seen: HashMap<&str, Option<Gender>> = HashMap::new();
        for line in dics.iter().flat_map(|d| d.lines()) {
            let entry = line.split_whitespace().next().unwrap_or("");
            let Some((w, flags)) = entry.split_once('/') else {
                continue;
            };
            if !w.starts_with(char::is_lowercase) {
                continue;
            }
            let a = w.ends_with('a');
            let neutrum = flags.contains('B') && !a || flags.contains('C');
            // `E` on `ärende`, `meddelande` with `F` (`-na`) is the neuter plural `-n`.
            let neuter_plural = flags.contains('F') && w.ends_with(['e', 'i', 'o', 'ä']);
            let utrum = flags.contains('D') && !a
                || flags.contains('E') && !w.ends_with("um") && !neuter_plural;
            // An utrum plural (`G` `-ar`, `H` `-er`, `I` `-r`) on a consonant: a homograph of
            // both genders (`grund`: `ett grund`, `en grund`, `grunder`).
            let utrum_plural =
                flags.contains(['G', 'H', 'I']) && !w.ends_with(|c: char| "aeiouyåäöé".contains(c));
            let g = if neutrum && utrum_plural {
                None
            } else if neutrum {
                Some(Gender::Neutrum)
            } else if utrum {
                Some(Gender::Utrum)
            } else {
                continue;
            };
            seen.entry(w)
                .and_modify(|old| {
                    if *old != g {
                        *old = None;
                    }
                })
                .or_insert(g);
        }
        seen.into_iter().map(|(w, g)| (Box::from(w), g)).collect()
    });
}

/// Gender of the indefinite singular noun `w`: a dictionary noun of one gender, or a compound
/// whose longest dictionary head follows a known first part (`kund|nummer`, `säkerhets|system`).
/// Not an adjective or participle (`omfattande`, `stort`) and not a noun of both genders.
fn gender(sp: &dyn LangSpeller, w: &str) -> Option<Gender> {
    let map = GENDERS.get()?;
    // `en sökande` (an applicant), `sökandet` (the search): both genders.
    if BOTH_GENDERS.contains(&w) || STOP.contains(&w) || w.ends_with("nde") {
        return None;
    }
    if let Some(&g) = map.get(w) {
        let g = g?;
        // A verb in `-a` with a stray `E` (`krascha`) is no noun without a plural
        // (`flickor`, `frågor`, `historier`).
        let noun = w.strip_suffix('a').is_none_or(|stem| {
            sp.check(&format!("{stem}or"))
                || stem.ends_with('i') && sp.check(&format!("{stem}er"))
                || !sp.check(&format!("{w}r"))
        });
        return noun.then_some(g);
    }
    if !sp.check(w) || w.ends_with("nde") || adjective_like(sp, w) || verb_form(sp, w) {
        return None;
    }
    let first_part = |p: &str| {
        let ok = |s: &str| s.chars().count() >= 3 && sp.check(s);
        ok(p)
            || p.strip_suffix('s').is_some_and(ok)
            || p.strip_suffix(['a', 'e', 'o', 'u'])
                .is_some_and(|b| ok(&format!("{b}a")) || ok(&format!("{b}e")))
    };
    let (i, head) = w
        .char_indices()
        .skip(3)
        .map(|(i, _)| (i, &w[i..]))
        .find(|(_, h)| h.chars().count() >= 3 && map.contains_key(*h))?;
    if BOTH_GENDERS.contains(&head) || !first_part(&w[..i]) {
        return None;
    }
    map.get(head).copied().flatten()
}

/// The neuter `-t` form of the adjective base `a` when it differs (`stor` -> `stort`, `ny` ->
/// `nytt`, `röd` -> `rött`, `sparad` -> `sparat`, `liten` -> `litet`), and `a` is a known
/// adjective with a weak or plural form (`stora`, `nya`, `sparade`, `öppna`, `enkla`).
fn neuter_adjective(sp: &dyn LangSpeller, a: &str) -> Option<String> {
    let ok = |s: &str| sp.check(s);
    let n = a.chars().count();
    if n < 2 || !ok(a) || a.ends_with(['a', 'e', 'o', 'u']) {
        return None;
    }
    let vowel = |c: char| "aeiouyåäö".contains(c);
    let chars: Vec<char> = a.chars().collect();
    let last = chars[n - 1];
    let before = if n >= 2 { chars[n - 2] } else { ' ' };
    let cut = |k: usize| chars[..n - k].iter().collect::<String>();
    let neuter = if a.ends_with("ad") || a.ends_with("en") {
        format!("{}t", cut(1))
    } else if last == 'd' && vowel(before) {
        format!("{}tt", cut(1))
    } else if last == 'd' {
        format!("{}t", cut(1))
    } else if last == 't' {
        // `skrivet`, `sparat` are neuter already (`skriven`, `sparad`).
        let neuter_of = |e: char| ok(&format!("{}{e}", cut(1)));
        if matches!(before, 'e' | 'a') && (neuter_of('n') || neuter_of('d')) {
            return None;
        }
        if vowel(before) {
            format!("{a}t")
        } else {
            return None;
        }
    } else if vowel(last) {
        format!("{a}tt")
    } else {
        format!("{a}t")
    };
    let weak = [
        format!("{a}a"),
        format!("{a}e"),
        // `öppen` -> `öppna`, `enkel` -> `enkla`, `vacker` -> `vackra`.
        format!("{}{last}a", cut(2)),
    ];
    let known = weak.iter().any(|w| ok(w)) || a == "liten";
    (known && neuter != a && ok(&neuter)).then_some(neuter)
}

/// `w` is an adjective form: a base with a neuter form, or a weak, neuter, comparative or
/// superlative form of one (`stora`, `svenska`, `nytt`, `öppna`, `större`, `bästa`), or a
/// present participle (`omfattande`).
fn adjective_like(sp: &dyn LangSpeller, w: &str) -> bool {
    if !sp.check(w) {
        return false;
    }
    if neuter_adjective(sp, w).is_some() || w.ends_with("nde") {
        return true;
    }
    let base = |b: &str| b.chars().count() >= 2 && neuter_adjective(sp, b).is_some();
    let chars: Vec<char> = w.chars().collect();
    let n = chars.len();
    let cut = |k: usize| chars[..n.saturating_sub(k)].iter().collect::<String>();
    if let Some(b) = w.strip_suffix('a') {
        // `stora`, `nya`; `öppna` from `öppen`, `enkla` from `enkel`.
        let inserted = if n >= 3 {
            format!("{}e{}", cut(2), chars[n - 2])
        } else {
            String::new()
        };
        return base(b) || base(&inserted) || sp.check(&format!("{b}t")) && !noun_base(sp, b);
    }
    if let Some(b) = w.strip_suffix("tt").or_else(|| w.strip_suffix('t')) {
        return base(b) || base(&format!("{b}d"));
    }
    ["are", "ast", "aste", "ste"]
        .iter()
        .any(|e| w.strip_suffix(e).is_some_and(base))
}

/// A noun, adjective or participle form: what may follow an article or be the head of a noun
/// phrase.
fn nounish(sp: &dyn LangSpeller, w: &str) -> bool {
    !STOP.contains(&w)
        && !finite_or_past(sp, w)
        && (noun_form(sp, w) || adjective_like(sp, w) || gender(sp, w).is_some())
}

/// A noun by its gender or a plural / definite plural ending on a gendered base (`kunder`,
/// `bilar`, `reglerna`); stricter than [`noun_form`], which also takes `inte`, `skriver`.
fn noun_strict(sp: &dyn LangSpeller, w: &str) -> bool {
    if STOP.contains(&w) || !sp.check(w) {
        return false;
    }
    if gender(sp, w).is_some() || definite_singular(sp, w).is_some() {
        return true;
    }
    if w.chars().count() > 5 && ["arna", "erna", "orna"].iter().any(|e| w.ends_with(e)) {
        return true;
    }
    ["ar", "er", "or", "r", "n"].iter().any(|e| {
        w.strip_suffix(e).is_some_and(|b| {
            b.chars().count() >= 2
                && [b.to_string(), format!("{b}e"), format!("{b}a")]
                    .iter()
                    .any(|x| gender(sp, x).is_some())
        })
    })
}

/// A finite verb: [`finite`], a present tense (`ringer`) or a past tense (`ringde`,
/// `sparade`, `köpte`).
fn finite_or_past(sp: &dyn LangSpeller, w: &str) -> bool {
    finite(sp, w)
        || infinitive_of(sp, w).is_some()
        || ["de", "te", "dde", "tte"].iter().any(|e| {
            w.strip_suffix(e).is_some_and(|stem| {
                stem.chars().count() >= 2
                    && sp.check(w)
                    && (present_of(sp, &format!("{stem}a")).is_some()
                        || stem.ends_with('a') && present_of(sp, stem).is_some())
            })
        })
}

/// Hyphens and dashes that join a word to a compound (`volym- eller underhållsavtal`,
/// `dator‑till‑dator`).
const DASHES: &[char] = &['-', '\u{2010}', '\u{2011}', '\u{2013}'];

/// A plain word: letters only, no inner capitals.
fn plain_word(w: &Word) -> bool {
    w.text.chars().all(char::is_alphabetic) && w.text.chars().skip(1).all(char::is_lowercase)
}

/// A plain lowercase word.
fn lower_word(w: &Word) -> bool {
    plain_word(w) && w.text == w.lower
}

/// The word after `ws[k]` with only spaces between.
fn next_of<'a>(chars: &[char], ws: &'a [Word], k: usize) -> Option<&'a Word> {
    ws.get(k + 1).filter(|n| plain_gap(chars, &ws[k], n))
}

/// The word before `ws[k]` with only spaces between.
fn prev_of<'a>(chars: &[char], ws: &'a [Word], k: usize) -> Option<&'a Word> {
    k.checked_sub(1)
        .map(|i| &ws[i])
        .filter(|p| plain_gap(chars, p, &ws[k]))
}

/// `w` ends a clause: the text after it is empty or starts with punctuation.
fn clause_end(chars: &[char], w: &Word) -> bool {
    chars[w.end..]
        .iter()
        .find(|c| !c.is_whitespace() || **c == '\n')
        .is_none_or(|c| ".,;:!?)\n".contains(*c))
}

/// Conjunctions after which a clause (its subject) starts.
const SUBJECT_AFTER: &[&str] = &[
    "att", "om", "när", "eftersom", "medan", "innan", "tills", "ifall", "fast", "fastän", "men",
    "så", "därför", "då", "där", "huruvida",
];

/// `ws[k]` opens a clause: a segment or sentence start, after `,;:` or a conjunction.
fn opens_clause(chars: &[char], ws: &[Word], k: usize) -> bool {
    k == 0
        || prev_char(chars, ws[k].start).is_none_or(|c| ".!?:;,(".contains(c))
        || prev_of(chars, ws, k).is_some_and(|p| SUBJECT_AFTER.contains(&p.lower.as_str()))
}

fn with_case(like: &str, fix: &str) -> String {
    if like.starts_with(char::is_uppercase) {
        capitalize(fix)
    } else {
        fix.to_string()
    }
}

fn in_quotes(chars: &[char], ws: &[Word]) -> Vec<std::ops::Range<usize>> {
    let toks: Vec<(usize, usize)> = ws.iter().map(|w| (w.start, w.end)).collect();
    super::spell_lang::quotations(chars, &toks)
}

/// Prepositions whose object is `dem` (`med dem`, `till dem`).
const PREPOSITIONS: &[&str] = &[
    "med", "till", "från", "av", "på", "i", "åt", "hos", "mot", "bland", "bakom", "framför",
    "bredvid", "kring", "runt", "genom", "vid", "mellan", "inför", "förbi", "utan", "för", "om",
    "efter", "under", "över", "enligt", "inom", "utom", "ur",
];

/// Prepositions that are also conjunctions (`för de kom`, `om de vill`, `efter de gått`).
const CONJUNCTION_PREPOSITIONS: &[&str] = &["för", "om", "efter", "utan", "under"];

/// Words after an object `de` that show it closes its phrase (`med de och`).
const AFTER_OBJECT: &[&str] = &["och", "eller", "men", "igen", "heller"];

/// Subject pronouns.
const SUBJECT_PRONOUNS: &[&str] = &["jag", "du", "han", "hon", "vi", "ni", "man", "de"];

/// `de` as an object (`med de.`, `vi ringde de.`) is `dem`; `dem` before an attributive
/// adjective and a noun, or before a definite plural, after a preposition or at a clause start
/// (`med dem nya reglerna`), is `de`. `de` before a noun phrase is its article (`med de nya
/// reglerna`) and a verb may take two objects (`vi gav dem nya regler`), so both need the
/// clear cases.
fn de_dem(sp: &dyn LangSpeller, chars: &[char], ws: &[Word], out: &mut Vec<Lint>) {
    let quotes = in_quotes(chars, ws);
    let finite_verb = |w: &Word| lower_word(w) && finite_or_past(sp, &w.lower);
    for (k, w) in ws.iter().enumerate() {
        if !matches!(w.lower.as_str(), "de" | "dem")
            || !plain_word(w)
            || quotes.iter().any(|q| q.contains(&w.start))
        {
            continue;
        }
        let prev = prev_of(chars, ws, k);
        let next = next_of(chars, ws, k);
        if w.lower == "de" {
            let end = clause_end(chars, w);
            let after_prep = prev.is_some_and(|p| {
                PREPOSITIONS.contains(&p.lower.as_str())
                    && (end
                        || !CONJUNCTION_PREPOSITIONS.contains(&p.lower.as_str())
                            && next.is_some_and(|n| AFTER_OBJECT.contains(&n.lower.as_str())))
            });
            // `vi ringde de.`, `då ringer vi de.`: subject and verb in either order before.
            let pron = |x: &Word| SUBJECT_PRONOUNS.contains(&x.lower.as_str()) && x.lower != "de";
            let after_verb = end
                && prev.is_some_and(|v| {
                    prev_of(chars, ws, k - 1)
                        .is_some_and(|pp| pron(pp) && finite_verb(v) || finite_verb(pp) && pron(v))
                });
            if after_prep || after_verb {
                out.push(lint(
                    w.start,
                    w.end,
                    LintKind::WordChoice,
                    "`de` is a subject or an article; as an object write `dem`.".to_string(),
                    &with_case(&w.text, "dem"),
                ));
            }
            continue;
        }
        let Some(n) = next.filter(|n| lower_word(n)) else {
            continue;
        };
        let definite_plural = n.lower.chars().count() > 5
            && ["arna", "erna", "orna"]
                .iter()
                .any(|e| n.lower.ends_with(e))
            && sp.check(&n.lower);
        let attributive = n.lower.ends_with('a')
            && !matches!(
                n.lower.as_str(),
                "alla" | "båda" | "andra" | "sina" | "egna"
            )
            && adjective_like(sp, &n.lower)
            && next_of(chars, ws, k + 1)
                .is_some_and(|h| lower_word(h) && noun_strict(sp, &h.lower) && !finite_verb(h));
        let position = prev.is_some_and(|p| PREPOSITIONS.contains(&p.lower.as_str()))
            || opens_clause(chars, ws, k);
        if position && (definite_plural || attributive) {
            out.push(lint(
                w.start,
                w.end,
                LintKind::WordChoice,
                "`dem` is an object; before a noun phrase the article is `de`.".to_string(),
                &with_case(&w.text, "de"),
            ));
        }
    }
}

/// Gender of the definite singular `w`: `huset` (`hus`, neuter), `äpplet` (`äpple`), `bilen`
/// (`bil`, utrum), `flickan` (`flicka`).
fn definite_singular(sp: &dyn LangSpeller, w: &str) -> Option<Gender> {
    let map = GENDERS.get()?;
    if !sp.check(w) {
        return None;
    }
    let vowel_end = |b: &str| b.ends_with(|c: char| "aeiouyåäö".contains(c));
    let is = |b: &str, g: Gender| {
        (map.get(b) == Some(&Some(g)) || !map.contains_key(b) && gender(sp, b) == Some(g))
            && !BOTH_GENDERS.contains(&b)
    };
    // `fönstret`, `vattnet`, `exemplet`: `-er`, `-en`, `-el` drop their `e`.
    let dropped_e = |b: &str| {
        let c: Vec<char> = b.chars().collect();
        c.len() >= 3
            && matches!(c[c.len() - 1], 'r' | 'n' | 'l')
            && !"aeiouyåäö".contains(c[c.len() - 2])
            && {
                let base: String = c[..c.len() - 1]
                    .iter()
                    .chain(['e', c[c.len() - 1]].iter())
                    .collect();
                is(&base, Gender::Neutrum)
            }
    };
    let neutrum = w
        .strip_suffix("et")
        .is_some_and(|b| is(b, Gender::Neutrum) || dropped_e(b))
        || w.strip_suffix('t')
            .is_some_and(|b| vowel_end(b) && is(b, Gender::Neutrum));
    // `flickan`, `datorn`, `servern`.
    let utrum = w.strip_suffix("en").is_some_and(|b| is(b, Gender::Utrum))
        || w.strip_suffix('n').is_some_and(|b| is(b, Gender::Utrum));
    match (neutrum, utrum) {
        (true, false) => Some(Gender::Neutrum),
        (false, true) => Some(Gender::Utrum),
        _ => None,
    }
}

/// `en` / `ett` before a noun of the other gender (`en hus`, `ett bil`), and `den` / `det`
/// before a definite noun of the other gender (`den huset`). The noun must end the phrase: a
/// following noun or adjective may be the real head (`en fel uppfattning`) or a split
/// compound. `det` + a definite utrum noun is also a pronoun and a subject (`det kunden vill
/// ha`), so it counts only after a preposition at a clause end.
fn article_gender(sp: &dyn LangSpeller, chars: &[char], ws: &[Word], out: &mut Vec<Lint>) {
    let quotes = in_quotes(chars, ws);
    for (k, w) in ws.iter().enumerate() {
        let article = w.lower.as_str();
        if !matches!(article, "en" | "ett" | "den" | "det")
            || !plain_word(w)
            || quotes.iter().any(|q| q.contains(&w.start))
        {
            continue;
        }
        let Some(n) = next_of(chars, ws, k).filter(|n| lower_word(n)) else {
            continue;
        };
        let prev = prev_of(chars, ws, k);
        // `var och en`, `en och en`, `mer än en`: pronoun and numeral readings.
        let pronoun = prev.is_some_and(|p| match p.lower.as_str() {
            "än" => true,
            "och" => prev_of(chars, ws, k - 1)
                .is_some_and(|pp| matches!(pp.lower.as_str(), "var" | "en" | "ett")),
            _ => false,
        });
        if pronoun {
            continue;
        }
        let after = next_of(chars, ws, k + 1);
        // `en köp nu, betala senare-tjänst`: a phrase compound ends in a hyphenated head.
        let phrase_compound = ws[k + 1..]
            .iter()
            .take(6)
            .take_while(|x| !chars[w.end..x.start].iter().any(|c| ".!?\n".contains(*c)))
            .any(|x| x.text.contains(DASHES));
        if after.is_some_and(|a| nounish(sp, &a.lower))
            || phrase_compound
            || chars.get(n.end).is_some_and(|c| DASHES.contains(c))
        {
            continue;
        }
        let fix = match article {
            "en" | "ett" => match gender(sp, &n.lower) {
                Some(Gender::Neutrum) if article == "en" => "ett",
                Some(Gender::Utrum) if article == "ett" => "en",
                _ => continue,
            },
            _ => match definite_singular(sp, &n.lower) {
                // `den barnet leker med` is rare; a finite verb after it reads so.
                Some(Gender::Neutrum)
                    if article == "den" && !after.is_some_and(|a| finite(sp, &a.lower)) =>
                {
                    "det"
                }
                Some(Gender::Utrum)
                    if article == "det"
                        && clause_end(chars, n)
                        && prev.is_some_and(|p| {
                            PREPOSITIONS.contains(&p.lower.as_str())
                                && !CONJUNCTION_PREPOSITIONS.contains(&p.lower.as_str())
                        }) =>
                {
                    "den"
                }
                _ => continue,
            },
        };
        let gender_name = if matches!(fix, "ett" | "det") {
            "neuter (ett-ord)"
        } else {
            "common gender (en-ord)"
        };
        let fix = with_case(&w.text, fix);
        out.push(lint(
            w.start,
            w.end,
            LintKind::Grammar,
            format!("`{}` is {gender_name}: write `{fix} {}`.", n.text, n.text),
            &fix,
        ));
    }
}

/// Adverbs and quantifiers before an adjective, and indeclinable words.
const DEGREE_WORDS: &[&str] = &[
    "mycket",
    "ganska",
    "väldigt",
    "helt",
    "så",
    "för",
    "lite",
    "mer",
    "mest",
    "mindre",
    "allt",
    "inte",
    "bara",
    "också",
    "extra",
    "rätt",
    "riktigt",
    "särskilt",
    "alltför",
    "tämligen",
    "fel",
    "annat",
    "eget",
    "sådant",
    "visst",
];

/// `ett` + an adjective base + a neuter noun (`ett stor hus`): the adjective takes `-t`.
fn adjective_gender(sp: &dyn LangSpeller, chars: &[char], ws: &[Word], out: &mut Vec<Lint>) {
    let quotes = in_quotes(chars, ws);
    for (k, w) in ws.iter().enumerate() {
        if w.lower != "ett" || !plain_word(w) || quotes.iter().any(|q| q.contains(&w.start)) {
            continue;
        }
        let Some(a) = next_of(chars, ws, k).filter(|a| lower_word(a)) else {
            continue;
        };
        let Some(n) = next_of(chars, ws, k + 1).filter(|n| lower_word(n)) else {
            continue;
        };
        if DEGREE_WORDS.contains(&a.lower.as_str())
            || GENDERS
                .get()
                .is_some_and(|m| m.contains_key(a.lower.as_str()))
            || gender(sp, &n.lower) != Some(Gender::Neutrum)
            || next_of(chars, ws, k + 2).is_some_and(|x| nounish(sp, &x.lower))
            || chars.get(n.end).is_some_and(|c| DASHES.contains(c))
        {
            continue;
        }
        let Some(fix) = neuter_adjective(sp, &a.lower) else {
            continue;
        };
        out.push(lint(
            a.start,
            a.end,
            LintKind::Grammar,
            format!(
                "`{}` is neuter (ett-ord): the adjective is `{fix}`.",
                n.text
            ),
            &fix,
        ));
    }
}

/// Irregular infinitives and their present tense.
const IRREGULAR_PRESENT: &[(&str, &str)] = &[
    ("vara", "är"),
    ("ha", "har"),
    ("kunna", "kan"),
    ("vilja", "vill"),
    ("veta", "vet"),
    ("göra", "gör"),
    ("bära", "bär"),
    ("fara", "far"),
    ("skära", "skär"),
    ("svära", "svär"),
    ("ta", "tar"),
    ("dra", "drar"),
    ("bli", "blir"),
    ("ge", "ger"),
    ("se", "ser"),
    ("le", "ler"),
    ("få", "får"),
    ("gå", "går"),
    ("stå", "står"),
    ("dö", "dör"),
    ("ske", "sker"),
    ("säga", "säger"),
    ("lägga", "lägger"),
    ("heta", "heter"),
    ("böra", "bör"),
    ("komma", "kommer"),
];

/// The present tense of the infinitive `v` (`fungera` -> `fungerar`, `skriva` -> `skriver`,
/// `köra` -> `kör`, `bo` -> `bor`) when the dictionary knows the verb's forms.
fn present_of(sp: &dyn LangSpeller, v: &str) -> Option<String> {
    presents_of(sp, v).into_iter().next()
}

/// Every present tense [`present_of`] finds, for verbs of two conjugations (`ringa`:
/// `ringar in`, `ringer upp`).
fn presents_of(sp: &dyn LangSpeller, v: &str) -> Vec<String> {
    if let Some((_, p)) = IRREGULAR_PRESENT.iter().find(|(i, _)| *i == v) {
        return vec![p.to_string()];
    }
    let ok = |s: String| sp.check(&s);
    let mut out = Vec::new();
    if !sp.check(v) || v.chars().count() < 3 {
        return out;
    }
    if let Some(stem) = v.strip_suffix('a') {
        // First conjugation: `fungerar`, `fungerade`, `fungerat`.
        if ok(format!("{v}r")) && ok(format!("{v}de")) && ok(format!("{v}t")) {
            out.push(format!("{v}r"));
        }
        // Second and fourth: `skriver` with `skrivit`, `leker` with `lekte`.
        if ok(format!("{stem}er")) && ["it", "te", "de"].iter().any(|e| ok(format!("{stem}{e}"))) {
            out.push(format!("{stem}er"));
        }
        // Stems in `-r`: `kör`, `körde`, `kört`.
        if stem.ends_with('r')
            && sp.check(stem)
            && ok(format!("{stem}de"))
            && ok(format!("{stem}t"))
        {
            out.push(stem.to_string());
        }
        // Strong verbs, whose past and supine change the vowel: `bryter` with the imperative
        // `bryt` and the participle `brytande`.
        if out.is_empty()
            && !stem.ends_with('r')
            && sp.check(stem)
            && ok(format!("{stem}er"))
            && ok(format!("{stem}ande"))
        {
            out.push(format!("{stem}er"));
        }
        return out;
    }
    // Third conjugation: `bor`, `bodde`.
    if v.ends_with(|c: char| "eiouyåäö".contains(c)) && ok(format!("{v}r")) && ok(format!("{v}dde"))
    {
        out.push(format!("{v}r"));
    }
    out
}

/// The infinitive of the present tense `p` (`skriver` -> `skriva`), the inverse of
/// [`present_of`].
fn infinitive_of(sp: &dyn LangSpeller, p: &str) -> Option<String> {
    if let Some((i, _)) = IRREGULAR_PRESENT.iter().find(|(_, x)| *x == p) {
        return Some(i.to_string());
    }
    let base = p.strip_suffix('r').filter(|_| p.chars().count() >= 4)?;
    let mut candidates = vec![base.to_string()];
    if let Some(stem) = p.strip_suffix("er") {
        candidates.push(format!("{stem}a"));
    }
    candidates.push(format!("{p}a"));
    candidates
        .into_iter()
        .find(|i| presents_of(sp, i).iter().any(|x| x == p))
}

/// A definite noun opening a clause as its subject: `systemet`, `kunden`, `användarna`.
fn definite_subject(sp: &dyn LangSpeller, w: &str) -> bool {
    if STOP.contains(&w) || adjective_like(sp, w) {
        return false;
    }
    if definite_singular(sp, w).is_some() {
        return true;
    }
    // Definite plurals: `användarna`, `kunderna`, `frågorna`.
    w.chars().count() > 5 && ["arna", "erna", "orna"].iter().any(|e| w.ends_with(e)) && sp.check(w)
}

/// Words ending in `-a` that are no infinitives after a subject.
const NOT_INFINITIVE: &[&str] = &[
    "bara", "gärna", "sedan", "kanske", "alltså", "också", "ofta", "sällan", "nästan", "enda",
    "själva", "båda", "alla", "andra", "flesta", "vilka", "dessa", "sina", "egna", "hela",
    "första", "sista", "samma", "nya", "gamla", "stora", "små", "sa", "tidigare", "senare",
];

/// A subject at a clause start followed by an infinitive (`han skriva`, `systemet fungera`)
/// is missing the present tense. Inverted clauses (`kan han skriva`) do not open with the
/// subject. `de`, `den`, `det` are also articles (`de kalla vindarna`) and a noun subject may
/// head a split compound, so there the verb must be no adjective or noun and no noun may
/// follow it.
fn present_tense(sp: &dyn LangSpeller, chars: &[char], ws: &[Word], out: &mut Vec<Lint>) {
    let quotes = in_quotes(chars, ws);
    for (k, s) in ws.iter().enumerate() {
        if !plain_word(s)
            || !opens_clause(chars, ws, k)
            || quotes.iter().any(|q| q.contains(&s.start))
        {
            continue;
        }
        let Some(v) = next_of(chars, ws, k).filter(|v| lower_word(v)) else {
            continue;
        };
        let subject = s.lower.as_str();
        let verb = v.lower.as_str();
        let article = matches!(subject, "de" | "den" | "det");
        let pronoun = SUBJECT_PRONOUNS.contains(&subject) || article;
        let irregular = IRREGULAR_PRESENT.iter().any(|(i, _)| *i == verb);
        if !(pronoun || definite_subject(sp, subject))
            || NOT_INFINITIVE.contains(&verb)
            || STOP.contains(&verb) && !irregular
        {
            continue;
        }
        let next = next_of(chars, ws, k + 1);
        // `de kalla vindarna`, `de kalla.`: an attribute or a nominalized adjective; a
        // present participle after it is the verb's object (`det bryta hängande anslutningar`).
        let attribute = adjective_like(sp, verb)
            && next.is_none_or(|n| !n.lower.ends_with("nde") && nounish(sp, &n.lower));
        if (article || !pronoun)
            && (article && attribute
                || gender(sp, verb).is_some()
                || next.is_some_and(|n| {
                    !STOP.contains(&n.lower.as_str())
                        && (noun_strict(sp, &n.lower)
                            || article && noun_form(sp, &n.lower) && !adjective_like(sp, &n.lower))
                        && !finite(sp, &n.lower)
                }))
        {
            continue;
        }
        let Some(fix) = present_of(sp, verb) else {
            continue;
        };
        out.push(lint(
            v.start,
            v.end,
            LintKind::Grammar,
            format!(
                "After the subject `{}` the verb takes the present tense: `{fix}`.",
                s.text
            ),
            &fix,
        ));
    }
}

/// Adverbs between `har` and its supine (`har inte skrivit`).
const SUPINE_ADVERBS: &[&str] = &[
    "inte",
    "redan",
    "aldrig",
    "också",
    "alltid",
    "just",
    "nu",
    "ännu",
    "äntligen",
    "precis",
    "bara",
    "även",
    "dock",
    "nog",
    "ju",
    "väl",
    "ofta",
    "sällan",
    "tidigare",
];

/// Function words after a participle that show no noun follows it (`har skriven i`).
const AFTER_PARTICIPLE: &[&str] = &[
    "i", "på", "av", "till", "för", "med", "om", "och", "men", "att", "som", "under", "efter",
    "från", "hos", "vid", "genom", "sedan", "igen", "där", "här", "nu", "redan", "också",
    "tidigare", "innan", "när", "eller",
];

/// `har` / `hade` / `ha` + a past participle (`har skriven`, `hade sparad`) is the supine
/// (`skrivit`, `sparat`). Only when no noun follows: `vi har sparad data` is an object with
/// an attribute.
fn supine(sp: &dyn LangSpeller, chars: &[char], ws: &[Word], out: &mut Vec<Lint>) {
    for (k, h) in ws.iter().enumerate() {
        if !matches!(h.lower.as_str(), "har" | "hade" | "ha") || !plain_word(h) {
            continue;
        }
        let mut j = k;
        while next_of(chars, ws, j).is_some_and(|x| SUPINE_ADVERBS.contains(&x.lower.as_str())) {
            j += 1;
        }
        let Some(x) = next_of(chars, ws, j).filter(|x| lower_word(x)) else {
            continue;
        };
        let w = x.lower.as_str();
        let ends = clause_end(chars, x)
            || next_of(chars, ws, j + 1)
                .is_some_and(|n| AFTER_PARTICIPLE.contains(&n.lower.as_str()));
        if !ends || !sp.check(w) || definite_singular(sp, w).is_some() || STOP.contains(&w) {
            continue;
        }
        let ok = |s: &str| sp.check(s);
        let fix = if let Some(b) = w.strip_suffix("ad").or_else(|| w.strip_suffix("ade")) {
            // First conjugation: `sparad`, `sparade` -> `sparat`.
            let inf = format!("{b}a");
            let sup = format!("{b}at");
            (ok(&inf) && ok(&format!("{inf}r")) && ok(&sup)).then_some(sup)
        } else if let Some(stem) = w
            .strip_suffix("en")
            .or_else(|| w.strip_suffix("et"))
            .or_else(|| w.strip_suffix("na"))
        {
            // Strong verbs: `skriven`, `skrivet`, `skrivna` -> `skrivit`.
            let sup = format!("{stem}it");
            (stem.chars().count() >= 2 && ok(&sup) && gender(sp, w).is_none()).then_some(sup)
        } else if let Some(stem) = w.strip_suffix('d') {
            // Second conjugation: `stängd` -> `stängt` (`stänga`, `stänger`).
            let sup = format!("{stem}t");
            (!stem.ends_with(|c: char| "aeiouyåäöd".contains(c))
                && ok(&sup)
                && present_of(sp, &format!("{stem}a")).is_some())
            .then_some(sup)
        } else {
            None
        };
        let Some(fix) = fix.filter(|f| f != w) else {
            continue;
        };
        out.push(lint(
            x.start,
            x.end,
            LintKind::Grammar,
            format!("`{}` takes the supine: `{fix}`.", h.text),
            &fix,
        ));
    }
}

/// `att` + a present-tense verb (`att skriver`) is the infinitive (`att skriva`). A plural
/// noun (`att kunder ...`, the subject of an `att` clause) or adjective is left alone.
fn att_infinitive(sp: &dyn LangSpeller, chars: &[char], ws: &[Word], out: &mut Vec<Lint>) {
    let quotes = in_quotes(chars, ws);
    for (k, a) in ws.iter().enumerate() {
        if a.lower != "att" || !plain_word(a) || quotes.iter().any(|q| q.contains(&a.start)) {
            continue;
        }
        let Some(v) = next_of(chars, ws, k).filter(|v| lower_word(v)) else {
            continue;
        };
        let w = v.lower.as_str();
        let irregular = IRREGULAR_PRESENT.iter().any(|(_, p)| *p == w);
        if !irregular && (STOP.contains(&w) || noun_strict(sp, w) || adjective_like(sp, w)) {
            continue;
        }
        let Some(fix) = infinitive_of(sp, w) else {
            continue;
        };
        out.push(lint(
            v.start,
            v.end,
            LintKind::Grammar,
            format!("After `att` the verb is an infinitive: `{fix}`."),
            &fix,
        ));
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
            "SwedishDeDem" => de_dem(sp, chars, &ws, &mut l),
            "SwedishArticleGender" => article_gender(sp, chars, &ws, &mut l),
            "SwedishAdjectiveGender" => adjective_gender(sp, chars, &ws, &mut l),
            "SwedishPresentTense" => present_tense(sp, chars, &ws, &mut l),
            "SwedishSupine" => supine(sp, chars, &ws, &mut l),
            "SwedishAttInfinitive" => att_infinitive(sp, chars, &ws, &mut l),
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

    /// `rule: word -> fix` for each finding of `rule` in `text`.
    fn fixes(rule: &str, text: &str) -> Vec<String> {
        let sp = crate::rules::spell_lang::speller("sv", &crate::config::Config::default())
            .expect("bundled Swedish");
        let chars: Vec<char> = text.chars().collect();
        lints(&*sp, &chars, &[rule])
            .into_values()
            .flatten()
            .map(|l| {
                let w: String = chars[l.span.start..l.span.end].iter().collect();
                let fix = match &l.suggestions[0] {
                    Suggestion::ReplaceWith(f) => f.iter().collect::<String>(),
                    _ => String::new(),
                };
                format!("{w} -> {fix}")
            })
            .collect()
    }

    fn quiet(rule: &str, texts: &[&str]) {
        for text in texts {
            assert!(
                fixes(rule, text).is_empty(),
                "{rule} {text}: {:?}",
                fixes(rule, text)
            );
        }
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

    #[test]
    fn noun_gender_from_dictionary_flags() {
        let sp = crate::rules::spell_lang::speller("sv", &crate::config::Config::default())
            .expect("bundled Swedish");
        let g = |w: &str| gender(&*sp, w);
        assert_eq!(g("hus"), Some(Gender::Neutrum));
        assert_eq!(g("äpple"), Some(Gender::Neutrum));
        assert_eq!(g("bil"), Some(Gender::Utrum));
        assert_eq!(g("flicka"), Some(Gender::Utrum));
        assert_eq!(g("server"), Some(Gender::Utrum));
        // Compounds by their head; not adjectives, participles or two-gender nouns.
        assert_eq!(g("kundnummer"), Some(Gender::Neutrum));
        assert_eq!(g("säkerhetsfunktion"), Some(Gender::Utrum));
        for w in [
            "stort",
            "omfattande",
            "meddelande",
            "sökande",
            "test",
            "byte",
            "krascha",
        ] {
            assert_eq!(g(w), None, "{w}");
        }
        assert_eq!(definite_singular(&*sp, "huset"), Some(Gender::Neutrum));
        assert_eq!(definite_singular(&*sp, "datorn"), Some(Gender::Utrum));
    }

    #[test]
    fn de_dem() {
        let r = "SwedishDeDem";
        assert_eq!(fixes(r, "Jag såg de."), ["de -> dem"]);
        assert_eq!(fixes(r, "Då ringer vi de."), ["de -> dem"]);
        assert_eq!(fixes(r, "Ge boken till de."), ["de -> dem"]);
        assert_eq!(fixes(r, "Vi pratade med de och deras chef."), ["de -> dem"]);
        assert_eq!(
            fixes(r, "Med dem nya reglerna blir det enklare."),
            ["dem -> de"]
        );
        assert_eq!(fixes(r, "Dem nya reglerna gäller."), ["Dem -> De"]);
        assert_eq!(fixes(r, "Enligt dem reglerna gäller det."), ["dem -> de"]);
        quiet(
            r,
            &[
                "Med de nya reglerna blir det bättre.",
                "Hon kom med de två barnen.",
                "Vi pratade med de andra.",
                "Med de flesta kunderna går det bra.",
                "Vi gav dem nya regler.",
                "Vi visar dem reglerna.",
                "Jag tror de kommer i morgon.",
                "Jag hoppas de inte blir sena.",
                "Vad gör de?",
                "Hur mår de?",
                "Sedan såg de filmen.",
                "För de som vill finns kaffe.",
                "Om de vill kan de stanna.",
                "Rapporten skickas till dem som är ansvariga.",
                "Vi ringde dem igår.",
                "Det var en av dem.",
            ],
        );
    }

    #[test]
    fn article_and_adjective_gender() {
        let r = "SwedishArticleGender";
        assert_eq!(fixes(r, "Vi köpte en hus."), ["en -> ett"]);
        assert_eq!(fixes(r, "Han har ett dator."), ["ett -> en"]);
        assert_eq!(fixes(r, "Vi har en kundnummer."), ["en -> ett"]);
        assert_eq!(
            fixes(r, "Vi fick ett svar och ett bekräftelse."),
            ["ett -> en"]
        );
        assert_eq!(fixes(r, "Ett fråga kom in."), ["Ett -> En"]);
        assert_eq!(fixes(r, "Jag bor i den huset."), ["den -> det"]);
        assert_eq!(fixes(r, "Han såg på det bilen."), ["det -> den"]);
        quiet(
            r,
            &[
                "Vi köpte ett hus och en bil.",
                "Det kunden vill ha är snabbhet.",
                "Till det kunden efterfrågar hör snabbhet.",
                "Han fick en fel uppfattning.",
                "Han tog en kund nummer.",
                "Ett omfattande arbete gjordes.",
                "Det kom ett meddelande och en sökande.",
                "En och en gick de in. Var och en fick en.",
                "Det är ett bra sätt att arbeta.",
                "Ett par dagar senare kom svaret.",
                "Det tog ett tag.",
                "Du får ett mejl eller ett sms. Varje fil har en byte per tecken.",
                "Det finns ett test och en test.",
                "Använd en köp nu, betala senare-tjänst.",
                "Ett volym- eller underhållsavtal gäller.",
                "Den boken läste jag. Det huset är gammalt.",
                "Den svenska versionen finns.",
                "Det utgör en grund för beslutet.",
            ],
        );
        let r = "SwedishAdjectiveGender";
        assert_eq!(fixes(r, "Det var ett stor hus."), ["stor -> stort"]);
        assert_eq!(fixes(r, "Han har ett ny jobb."), ["ny -> nytt"]);
        assert_eq!(fixes(r, "Det är ett viktig beslut."), ["viktig -> viktigt"]);
        quiet(
            r,
            &[
                "Det var ett stort hus.",
                "Ett skrivet avtal och ett sparat utkast finns.",
                "Han bor i ett litet hus.",
                "Det är ett mycket stort hus.",
                "Han har ett extra rum och ett eget kontor.",
                "Ett par skor och ett bra sätt.",
                "Det blev ett långsam process.",
            ],
        );
    }

    #[test]
    fn verb_forms() {
        let r = "SwedishPresentTense";
        assert_eq!(fixes(r, "Han skriva ett brev."), ["skriva -> skriver"]);
        assert_eq!(fixes(r, "Systemet fungera inte."), ["fungera -> fungerar"]);
        assert_eq!(fixes(r, "Vi gå nu."), ["gå -> går"]);
        assert_eq!(
            fixes(r, "Det bryta hängande anslutningar."),
            ["bryta -> bryter"]
        );
        assert_eq!(
            fixes(r, "När du logga in visas menyn."),
            ["logga -> loggar"]
        );
        assert_eq!(
            fixes(r, "Om servern krascha startas den om."),
            ["krascha -> kraschar"]
        );
        quiet(
            r,
            &[
                "Kan han skriva brevet?",
                "Varför låter ni bli att svara?",
                "De kalla vindarna kom tidigt.",
                "De öppna frågorna besvaras senare.",
                "Det svenska språket är vackert.",
                "Att det äkta värdet ligger utanför.",
                "Det äkta populationsvärdet ligger utanför.",
                "Vi svenskar dricker kaffe. Ni andra kan gå.",
                "De båda kom sent. Han själv visste inget.",
                "Du bara går. Han nästan föll.",
                "Kunden kan skriva. Tjänsten kostar pengar.",
                "Om vi ska hinna måste vi gå nu.",
            ],
        );
        let r = "SwedishSupine";
        assert_eq!(fixes(r, "Vi har skriven."), ["skriven -> skrivit"]);
        assert_eq!(fixes(r, "Hon hade sparad i tid."), ["sparad -> sparat"]);
        assert_eq!(
            fixes(r, "Han har inte stängd av datorn."),
            ["stängd -> stängt"]
        );
        quiet(
            r,
            &[
                "Vi har sparad data i molnet.",
                "Han har en skriven överenskommelse.",
                "Butiken har öppet till nio.",
                "Vi har stängt för dagen. Hon har ledigt i dag.",
                "Jag har skrivit brevet. Vi har sparat filen.",
                "Han har rätt. Vi har bråttom. Jag har ont.",
                "Har du tiden?",
                "Den är skriven på svenska.",
            ],
        );
        let r = "SwedishAttInfinitive";
        assert_eq!(
            fixes(r, "Det är viktigt att skriver tydligt."),
            ["skriver -> skriva"]
        );
        assert_eq!(fixes(r, "Kom ihåg att sparar."), ["sparar -> spara"]);
        assert_eq!(fixes(r, "Det är dags att går."), ["går -> gå"]);
        quiet(
            r,
            &[
                "Det är viktigt att skriva tydligt.",
                "Vi vet att kunder vill ha snabbhet.",
                "Att bilar kostar pengar vet alla.",
                "Han tror att fler kommer.",
                "Vi hoppas att över hälften svarar.",
                "Du ser att under tiden händer mycket.",
                "Jag vet att de bor här.",
            ],
        );
    }
}
