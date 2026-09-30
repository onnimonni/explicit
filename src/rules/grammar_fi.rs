//! Finnish grammar rules on top of the Voikko morphology ([`crate::voikko`]): split and
//! wrongly joined compounds, capitalization, `kun` / `kuin`, `vaan` / `vain`, `sitä` for
//! `siitä`, relative pronoun agreement, and the comma before a subordinate or relative
//! clause. Each rule is conservative: it fires only where Finnish orthography leaves no
//! choice.

use std::collections::BTreeMap;

use super::lint::{Lint, LintKind, Span, Suggestion};
use super::spell_lang::{LangSpeller, tokens};
use crate::voikko::{Reading, Voikko};

/// Rule names (`grammar/<name>`) with their descriptions for `explicit rules --all`.
pub const RULES: &[(&str, &str)] = &[
    (
        "FinnishCompoundSplit",
        "Finnish: a compound written as two words (`tieto kanta` -> `tietokanta`)",
    ),
    (
        "FinnishCompoundJoined",
        "Finnish: a genitive and a postposition written as one word (`kirjautumisenjälkeen`)",
    ),
    (
        "FinnishCapitalization",
        "Finnish: weekdays, months, nationalities and adverbs are lowercase inside a sentence",
    ),
    (
        "FinnishSentenceStart",
        "Finnish: a sentence starts with a capital letter",
    ),
    (
        "FinnishKuin",
        "Finnish: `kuin`, not `kun`, in a comparison (`parempi kuin`, `yhtä hyvä kuin`, `sama \
         kuin`), and `kun`, not `kuin`, opening a time clause (`soita, kun olet valmis`)",
    ),
    (
        "FinnishComma",
        "Finnish: a comma before a subordinate or relative clause (`jos`, `että`, `kun`, \
         `ennen kuin`, `joka`, `jotka`, ...)",
    ),
    (
        "FinnishVaanVain",
        "Finnish: `vain` (only) and `vaan` (but, after a negation) confused",
    ),
    (
        "FinnishAgreement",
        "Finnish: a plural subject with a singular verb or the reverse (`tiedot siirtyy`)",
    ),
    (
        "FinnishElative",
        "Finnish: a verb that governs the elative takes `siitä`, not `sitä` (`tykkään siitä`, \
         `huolehdin siitä`)",
    ),
    (
        "FinnishRelative",
        "Finnish: a relative pronoun agrees with its antecedent (`tiedostot, jotka`), and a \
         whole clause takes `mikä` (`palvelu kaatui, mikä`)",
    ),
];

struct Word {
    start: usize,
    end: usize,
    text: String,
    lower: String,
}

/// Words of `chars` and whether the gap before each is a plain space (same clause).
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

/// Only spaces (and single line breaks) between `a` and `b`.
fn plain_gap(chars: &[char], a: &Word, b: &Word) -> bool {
    let gap = &chars[a.end..b.start];
    !gap.is_empty()
        && gap.iter().all(|&c| c == ' ' || c == '\n')
        && gap.iter().filter(|&&c| c == '\n').count() < 2
}

/// The last non-space character before `start`.
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

const WEEKDAYS_MONTHS: &[&str] = &[
    "maanantai",
    "tiistai",
    "keskiviiko",
    "keskiviikko",
    "torstai",
    "perjantai",
    "lauantai",
    "sunnuntai",
    "tammikuu",
    "helmikuu",
    "maaliskuu",
    "huhtikuu",
    "toukokuu",
    "kesäkuu",
    "heinäkuu",
    "elokuu",
    "syyskuu",
    "lokakuu",
    "marraskuu",
    "joulukuu",
];

/// Nouns that head a phrase with the next word rather than compound with it (`kaikki
/// kirjoitukset`, `osa potilaista`, `kiitos tiimille`).
const SPLIT_STOP: &[&str] = &[
    "kaikki",
    "koko",
    "sama",
    "muu",
    "moni",
    "jokainen",
    "osa",
    "pari",
    "puoli",
    "aika",
    "kiitos",
    "määrä",
    "joukko",
    "enemmistö",
    "vähemmistö",
    "loput",
    "kukin",
    "kumpikin",
    "itse",
];

/// Classes never capitalized inside a Finnish sentence.
const LOWERCASE_CLASSES: &[&str] = &[
    "seikkasana",
    "lukusana",
    "teonsana",
    "asemosana",
    "sidesana",
    "suhdesana",
    "kieltosana",
    "huudahdussana",
];

/// Postpositions and adverbs that follow a genitive as a separate word.
const POSTPOSITIONS: &[&str] = &[
    "jälkeen",
    "yhteydessä",
    "aikana",
    "kanssa",
    "takia",
    "vuoksi",
    "avulla",
    "mukaan",
    "sijaan",
    "sijasta",
    "lisäksi",
    "kautta",
    "ansiosta",
    "perusteella",
    "osalta",
    "puolesta",
    "johdosta",
    "myötä",
    "ohella",
    "välillä",
    "aikaan",
    "edessä",
    "takana",
    "jäljessä",
    "ympärillä",
    "vieressä",
    "alaisena",
    "ajaksi",
    "päätteeksi",
    "tähden",
];

/// Subordinating conjunctions and question words that open a clause needing a comma.
const CLAUSE_WORDS: &[&str] = &[
    "että", "jotta", "koska", "jos", "vaikka", "mikäli", "kunnes", "jolloin", "joten", "kun",
    "ettei", "jottei", "ellei", "vaikkei",
];

/// Forms of the relative pronoun `joka`, singular then plural.
const RELATIVES: &[(&str, &str)] = &[
    ("joka", "jotka"),
    ("jonka", "joiden"),
    ("jota", "joita"),
    ("jossa", "joissa"),
    ("josta", "joista"),
    ("johon", "joihin"),
    ("jolla", "joilla"),
    ("jolta", "joilta"),
    ("jolle", "joille"),
    ("jona", "joina"),
    ("joksi", "joiksi"),
];

fn is_relative(w: &str) -> bool {
    RELATIVES.iter().any(|(s, p)| *s == w || *p == w)
}

/// `mikä` forms that open a relative clause after a pronoun (`kaikki, mitä`, `se, mikä`).
const MIKA_FORMS: &[&str] = &[
    "mikä", "minkä", "mitä", "missä", "mistä", "mihin", "millä", "mitkä",
];
const MIKA_ANTECEDENTS: &[&str] = &[
    "kaikki", "kaikkea", "se", "sen", "sitä", "siitä", "siihen", "siinä", "tämä", "tätä", "sama",
    "samaa",
];

/// One-word units before a conjunction that take the comma before or after them (`aina kun`,
/// `aina, kun`; `siksi että`).
const FLEX_ONE: &[&str] = &["siksi", "samalla", "siten", "nyt", "aina", "sitten"];
/// Two-word units (`sen jälkeen kun`, `sillä aikaa kun`): a pronoun, then its head.
const FLEX_DET: &[&str] = &[
    "sen", "sillä", "sitä", "siitä", "siihen", "siinä", "niin", "samaan", "yhtä",
];
const FLEX_HEAD: &[&str] = &[
    "jälkeen",
    "sijaan",
    "takia",
    "vuoksi",
    "aikaa",
    "mukaa",
    "huolimatta",
    "lisäksi",
    "asti",
    "saakka",
    "verran",
    "kauan",
    "tavalla",
    "tavoin",
    "määrin",
    "aikaan",
];

/// Nominative personal pronouns that start the clause after the conjunction `sillä`.
const SUBJECT_PRONOUNS: &[&str] = &["minä", "sinä", "hän", "me", "te", "he", "se", "ne"];
const QUESTION_WORDS: &[&str] = &[
    "mitä", "miten", "miksi", "kuka", "mikä", "missä", "milloin", "kuinka", "mihin", "mistä",
];

/// Words that form a unit with the following conjunction; the comma goes before them.
const UNIT_BEFORE: &[&str] = &[
    "niin", "paitsi", "heti", "vasta", "silloin", "sitä", "ikään",
];

/// Words after which no comma comes before a conjunction.
const NO_COMMA_AFTER: &[&str] = &[
    "ja",
    "tai",
    "sekä",
    "eli",
    "mutta",
    "vaan",
    "sillä",
    "kuin",
    "entä",
    "myös",
    "jopa",
    "etenkin",
    "varsinkin",
    "erityisesti",
    "esimerkiksi",
    "esim",
    "mm",
    "lähinnä",
    "ainakin",
    "vaikkapa",
    "juuri",
    "aivan",
    "vain",
    "ei",
    "en",
    "et",
    "emme",
    "ette",
    "eivät",
    "että",
    "jos",
    "kun",
    "koska",
    "jotta",
    "vaikka",
    "tai",
    "joko",
];

fn readings(v: &Voikko, w: &str) -> Vec<Reading> {
    v.analyses(w)
}

/// `w1 w2` as one compound: `tietokanta`, `tietosuoja-asetus` (same vowel at the seam).
fn joined(w1: &str, w2: &str) -> String {
    let (a, b) = (w1.chars().last(), w2.chars().next());
    if a.is_some() && a == b && a.is_some_and(|c| "aeiouyäö".contains(c)) {
        format!("{w1}-{w2}")
    } else {
        format!("{w1}{w2}")
    }
}

fn is_nominative_noun(rs: &[Reading]) -> bool {
    !rs.is_empty()
        && rs
            .iter()
            .all(|r| r.class == Some("nimisana") && r.case == Some("nimento"))
}

/// Nouns in a case that works as an adverb or postposition after a noun phrase (`ottaa lääke
/// kerran`, `jättää annos väliin`, `ottaa järjestelmä käyttöön`): never a compound's head.
const ADVERBIAL_FORMS: &[&str] = &[
    "kerran",
    "kertaa",
    "väliin",
    "välissä",
    "välistä",
    "käyttöön",
    "käytössä",
    "käytöstä",
    "asti",
    "saakka",
    "lähtien",
    "päälle",
    "päällä",
    "päältä",
    "pois",
    "esiin",
    "esille",
    "ulos",
    "sisään",
    "mukana",
    "mukaan",
    "voimaan",
    "voimassa",
    "taakse",
    "eteen",
    "alle",
    "yli",
    "läpi",
    "ympäri",
    "vastaan",
    "kohti",
    "luo",
    "luona",
    "luota",
    "päin",
    "aikaan",
    "ajan",
    "ajaksi",
    "ajoin",
    "kuluessa",
    "kuluttua",
    "puolesta",
    "kello",
    "klo",
    "alusta",
    "lopusta",
    "lähtien",
    "osaksi",
    "osin",
    "kesken",
    "loppuun",
    "alkuun",
    "lopussa",
    "alussa",
];

/// The word before `a` is a nominative attribute (`uusi`, `unohtunut`, `tekemä`) while `b` is
/// in another case: `a` heads its own phrase (`unohtunut annos väliin`), for in a compound the
/// attribute would agree with `b`.
fn attribute_before(v: &Voikko, chars: &[char], ws: &[Word], k: usize, b_nominative: bool) -> bool {
    if b_nominative || k == 0 || !plain_gap(chars, &ws[k - 1], &ws[k]) {
        return false;
    }
    let p = &ws[k - 1];
    // `on salattava levy tilalla`: a `-tava` predicate (must be done), no attribute.
    if k >= 2
        && (p.lower.ends_with("va") || p.lower.ends_with("vä"))
        && matches!(
            ws[k - 2].lower.as_str(),
            "on" | "ovat" | "oli" | "olivat" | "ole" | "olla" | "ollut" | "olisi" | "olisivat"
        )
    {
        return false;
    }
    let rs: Vec<Reading> = readings(v, &p.lower)
        .into_iter()
        .filter(|r| !r.proper)
        .collect();
    let nominal = |r: &Reading| {
        matches!(
            r.class,
            Some("laatusana" | "nimisana_laatusana" | "nimisana" | "lukusana" | "asemosana")
        ) && r.case == Some("nimento")
    };
    if rs.is_empty() {
        // An unknown participle: `anonymisoitu`, `päivitetty`, `loppunut`.
        return ["tu", "ty", "nut", "nyt"]
            .iter()
            .any(|e| p.lower.ends_with(e))
            && p.text.chars().all(char::is_alphabetic);
    }
    rs.iter().any(nominal) && !rs.iter().any(|r| r.class == Some("teonsana"))
}

/// Words after which a new clause (and so a possible subject) starts.
const CLAUSE_OPENERS: &[&str] = &[
    "ja", "tai", "sekä", "mutta", "vaan", "eli", "että", "jotta", "koska", "jos", "vaikka",
    "mikäli", "kunnes", "kun", "joten", "sillä",
];

/// Forms of `olla` (to be) before a predicative or existential nominative.
const OLLA: &[&str] = &[
    "on", "ovat", "oli", "olivat", "ole", "olla", "ollut", "olleet", "olisi", "olisivat", "onko",
    "oliko", "olen", "olet", "olemme", "olette",
];

/// Some reading of `w` is a finite verb, active or passive (`kestää`, `tehdään`).
fn finite_or_passive(v: &Voikko, w: &Word) -> bool {
    readings(v, &w.lower).iter().any(|r| {
        r.class == Some("teonsana")
            && matches!(r.mood, Some("indicative" | "conditional"))
            && r.person.is_some()
    })
}

/// The word before pair `k` is an attribute (adjective, pronoun, numeral or participle, not a
/// noun) in the same non-nominative case as the second word's readings `nouns`.
fn agreeing_attribute(
    v: &Voikko,
    chars: &[char],
    ws: &[Word],
    k: usize,
    nouns: &[&Reading],
) -> bool {
    if k == 0 || !plain_gap(chars, &ws[k - 1], &ws[k]) {
        return false;
    }
    let p = &ws[k - 1];
    if !p.text.chars().all(char::is_alphabetic) {
        return false;
    }
    let rs: Vec<Reading> = readings(v, &p.lower)
        .into_iter()
        .filter(|r| !r.proper)
        .collect();
    let attribute = |r: &Reading| {
        matches!(
            r.class,
            Some("laatusana" | "nimisana_laatusana" | "asemosana" | "lukusana")
        )
    };
    !rs.is_empty()
        && rs
            .iter()
            .all(|r| attribute(r) || r.class == Some("nimisana"))
        && rs.iter().any(|r| {
            attribute(r)
                && r.case.is_some_and(|c| c != "nimento")
                && nouns
                    .iter()
                    .any(|n| n.case == r.case && (n.number == r.number || r.number.is_none()))
        })
}

fn compound_split(
    v: &Voikko,
    sp: &dyn LangSpeller,
    chars: &[char],
    ws: &[Word],
    out: &mut Vec<Lint>,
) {
    // Readings of a lowercase word as a common word (not a same-looking name: `Päivä`).
    let common =
        |w: &str| -> Vec<Reading> { readings(v, w).into_iter().filter(|r| !r.proper).collect() };
    for (k, pair) in ws.windows(2).enumerate() {
        let (a, b) = (&pair[0], &pair[1]);
        // `levy tila-osio`: the second word's first part joins.
        let b_head = b.lower.split('-').next().unwrap_or("");
        if !plain_gap(chars, a, b)
            || a.text.chars().count() < 3
            || b_head.chars().count() < 3
            || !a.text.chars().all(char::is_alphabetic)
            || !b_head.chars().all(char::is_alphabetic)
            || !b.text.starts_with(char::is_lowercase)
        {
            continue;
        }
        // A capitalized first word only at a sentence start (else a name).
        if a.text.starts_with(char::is_uppercase)
            && prev_char(chars, a.start).is_some_and(|c| !".!?:".contains(c))
        {
            continue;
        }
        if SPLIT_STOP.contains(&a.lower.as_str())
            || ADVERBIAL_FORMS.contains(&b.lower.as_str())
            || POSTPOSITIONS.contains(&b.lower.as_str())
            || WEEKDAYS_MONTHS.iter().any(|d| b_head.starts_with(d))
        {
            continue;
        }
        let ra = common(&a.lower);
        let rb = common(b_head);
        // Before a nominative noun, a noun that is also a past verb form (`tuki palvelu`,
        // `muisti vuoto`): a verb takes no nominative object there. Not an imperative,
        // which does (`katko yhteys`).
        let noun_or_past = |r: &Reading| {
            r.class == Some("nimisana") && r.case == Some("nimento")
                || r.class == Some("teonsana") && r.mood == Some("indicative")
        };
        let b_nominative_only = !rb.is_empty()
            && rb
                .iter()
                .all(|r| r.class == Some("nimisana") && r.case == Some("nimento"));
        if !(is_nominative_noun(&ra)
            || b_nominative_only
                && ra.iter().any(|r| r.class == Some("nimisana"))
                && ra.iter().all(noun_or_past))
        {
            continue;
        }
        // The second word a noun, or a first or second person verb form no nominative
        // subject takes (`tieto kannan`); not `voi`, `osaa`, `kerran`.
        let noun = |r: &Reading| r.class == Some("nimisana");
        let odd_verb =
            |r: &Reading| r.class == Some("teonsana") && matches!(r.person, Some('1' | '2'));
        if rb.is_empty() || !rb.iter().any(noun) || !rb.iter().all(|r| noun(r) || odd_verb(r)) {
            continue;
        }
        let nouns: Vec<&Reading> = rb.iter().filter(|r| noun(r)).collect();
        // Not itself a compound (`markkinaoikeuteen`), nor in a case compounds rarely split
        // into; a compound first word only before a nominative (`tekstiviesti operaattori`).
        let nominative = nouns.iter().all(|r| r.case == Some("nimento"));
        if nouns.iter().any(|r| {
            r.parts > 1
                || matches!(
                    r.case,
                    Some(
                        "tulento" | "olento" | "vajanto" | "seuranto" | "keinonto" | "kerrontosti"
                    )
                )
        }) {
            continue;
        }
        // An attribute before the pair agreeing with the second word (`yleisen tietosuoja
        // asetuksen`, `omasta terveys asemasta`): the pair is one noun, whatever its first part.
        let agrees = !nominative && agreeing_attribute(v, chars, ws, k, &nouns);
        if ra.iter().any(|r| r.parts > 1) && !nominative && !agrees {
            continue;
        }
        // An action or quality noun with a complement or adverbial in another case: `ilmoitus
        // esihenkilölle`, `päätös hankinnasta`, `käsittely valtuutuksella`, `salaus levossa`.
        let action = [
            "us", "ys", "nti", "lu", "ly", "nta", "ntä", "minen", "elmä", "elma", "ntö", "nto",
            "isto", "istö", "sy", "os", "ös",
        ]
        .iter()
        .any(|s| a.lower.ends_with(s));
        // A nominative subject before its finite verb (`hoito sairaalassa kestää`, `maksu
        // kortilla onnistuu`) or a predicative after `olla` (`on pääsy järjestelmään`).
        let clause_start = k == 0
            || sentence_start_of(chars, ws, k) == k
            || !plain_gap(chars, &ws[k - 1], a)
            || CLAUSE_OPENERS.contains(&ws[k - 1].lower.as_str());
        let subject = !nominative
            && clause_start
            && ws
                .get(k + 2)
                .is_some_and(|n| plain_gap(chars, b, n) && finite_or_passive(v, n));
        let predicative = !nominative
            && k > 0
            && plain_gap(chars, &ws[k - 1], a)
            && OLLA.contains(&ws[k - 1].lower.as_str());
        // A genitive before a postposition (`keskustelu henkilön kanssa`, `kulku alusta
        // lähtien`), a title before a name (`lääkäri Matti Meikäläinen`).
        let next = ws.get(k + 2).filter(|n| plain_gap(chars, b, n));
        let before_postposition = next.is_some_and(|n| {
            POSTPOSITIONS.contains(&n.lower.as_str())
                || ADVERBIAL_FORMS.contains(&n.lower.as_str())
                || n.text.starts_with(char::is_uppercase)
        });
        // `asiakas- ja potilasturvallisuus`: the second word starts a coordinated compound.
        let prefix = chars.get(b.end) == Some(&'-');
        // An imperative before: `aseta kortti lukijaan`, `lisää tieto lääkärille`.
        let imperative = k > 0
            && plain_gap(chars, &ws[k - 1], a)
            && readings(v, &ws[k - 1].lower)
                .iter()
                .any(|r| r.mood == Some("imperative"));
        if action && !nominative && !agrees
            || subject
            || predicative
            || before_postposition
            || prefix
            || imperative && !nominative
            || attribute_before(v, chars, ws, k, nominative)
        {
            continue;
        }
        let one = joined(&a.lower, b_head);
        if !sp.check(&one) {
            continue;
        }
        let whole = joined(&a.lower, &b.lower);
        let fix = if a.text.starts_with(char::is_uppercase) {
            capitalize(&whole)
        } else {
            whole
        };
        out.push(lint(
            a.start,
            b.end,
            LintKind::Spelling,
            format!("Finnish compounds are written as one word: `{fix}`."),
            &fix,
        ));
    }
}

fn compound_joined(v: &Voikko, ws: &[Word], out: &mut Vec<Lint>) {
    for w in ws {
        if !w.text.chars().all(char::is_alphabetic) {
            continue;
        }
        for p in POSTPOSITIONS {
            let Some(head) = w.lower.strip_suffix(p) else {
                continue;
            };
            if head.chars().count() < 4 || !head.ends_with('n') {
                continue;
            }
            if !readings(v, head).iter().any(|r| r.case == Some("omanto")) {
                continue;
            }
            let head_written: String = w.text.chars().take(head.chars().count()).collect();
            let fix = format!("{head_written} {p}");
            out.push(lint(
                w.start,
                w.end,
                LintKind::Spelling,
                format!("Write the postposition apart: `{fix}`."),
                &fix,
            ));
            break;
        }
    }
}

fn capitalization(v: &Voikko, chars: &[char], ws: &[Word], out: &mut Vec<Lint>) {
    for (k, w) in ws.iter().enumerate() {
        let mut it = w.text.chars();
        let first_upper = it.next().is_some_and(char::is_uppercase);
        if !first_upper || it.any(|c| !c.is_lowercase()) {
            continue;
        }
        // Inside a sentence: after a word and only spaces or a comma.
        let inside = k > 0
            && prev_char(chars, w.start).is_some_and(|c| c.is_alphanumeric() || c == ',')
            && !chars[ws[k - 1].end..w.start].contains(&'\n');
        if !inside {
            continue;
        }
        let rs = readings(v, &w.lower);
        // Names, and imperatives naming a button (`painamalla Lähetä palaute`).
        if rs.is_empty() || rs.iter().any(|r| r.proper || r.mood == Some("imperative")) {
            continue;
        }
        // A multiword name before a hyphenated suffix: `Ilmoita tietojenkalastelusta -painike`.
        let rest: String = chars[w.end..].iter().take(60).collect();
        let clause = rest
            .split(['.', ',', ';', ':', '!', '?'])
            .next()
            .unwrap_or("");
        if clause.contains(" -") {
            continue;
        }
        let closed_class = rs
            .iter()
            .all(|r| r.class.is_some_and(|c| LOWERCASE_CLASSES.contains(&c)));
        let calendar = WEEKDAYS_MONTHS.iter().any(|d| w.lower.starts_with(d));
        let nationality = (w.lower.ends_with("lainen")
            || w.lower.ends_with("läinen")
            || w.lower.ends_with("laiset")
            || w.lower.ends_with("läiset")
            || w.lower.contains("kieli"))
            && rs
                .iter()
                .all(|r| r.class == Some("laatusana") || r.class == Some("nimisana_laatusana"));
        if closed_class || calendar || nationality {
            out.push(lint(
                w.start,
                w.end,
                LintKind::Capitalization,
                format!(
                    "Finnish writes `{}` in lowercase inside a sentence.",
                    w.lower
                ),
                &w.lower,
            ));
        }
    }
}

/// Abbreviations and numbers before a dot that do not end a sentence (`esim.`, `15.`).
fn non_final_dot(v: &Voikko, w: &Word) -> bool {
    w.text.chars().any(|c| c.is_ascii_digit())
        || w.text.chars().count() <= 2
        || w.text.chars().all(|c| !c.is_lowercase())
        || v.spell(&format!("{}.", w.text)) && !v.spell(&w.text)
        || super::spell_lang::is_abbreviation("fi", &w.lower)
}

fn sentence_start(v: &Voikko, chars: &[char], ws: &[Word], out: &mut Vec<Lint>) {
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
        if t == "." && non_final_dot(v, a) {
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

fn is_comparative(v: &Voikko, w: &Word) -> bool {
    matches!(
        w.lower.as_str(),
        "enemmän" | "vähemmän" | "muu" | "muuta" | "muut" | "toisin" | "mieluummin" | "ennemmin"
    ) || w.lower.ends_with("mmin")
        || readings(v, &w.lower)
            .iter()
            .any(|r| r.comparison == Some("comparative"))
}

/// Some reading is a finite verb (a mood with a person) or the negation verb; not only an
/// infinitive that doubles as the passive after `ei` (`mitä tehdä`).
fn has_finite(v: &Voikko, w: &Word) -> bool {
    let rs = readings(v, &w.lower);
    let infinitive = rs.iter().any(|r| r.mood == Some("A-infinitive"));
    rs.iter().any(|r| {
        r.class == Some("kieltosana")
            || r.class == Some("teonsana")
                && r.mood.is_some()
                && r.person.is_some_and(|p| p != '4' || !infinitive)
    })
}

/// Clause punctuation or a paragraph break between words `a` and `b`.
fn clause_break(chars: &[char], a: &Word, b: &Word) -> bool {
    let gap = &chars[a.end..b.start];
    gap.iter().any(|c| ".,;:!?()[]\"“”".contains(*c))
        || gap.iter().filter(|&&c| c == '\n').count() >= 2
}

/// A finite verb among the words from `from` up to the next clause break (12 words at most).
fn clause_has_finite(v: &Voikko, chars: &[char], ws: &[Word], from: usize) -> bool {
    for i in from..ws.len().min(from + 12) {
        if i > from && clause_break(chars, &ws[i - 1], &ws[i]) {
            return false;
        }
        if has_finite(v, &ws[i]) {
            return true;
        }
    }
    false
}

/// Some reading of `w` has one of `bases` as its base form, in one of `cases` (any when empty).
fn has_base(v: &Voikko, w: &Word, bases: &[&str], cases: &[&str]) -> bool {
    readings(v, &w.lower).iter().any(|r| {
        r.base.as_deref().is_some_and(|b| bases.contains(&b))
            && (cases.is_empty() || r.case.is_some_and(|c| cases.contains(&c)))
    })
}

/// `sama` and `samanlainen` in the nominative or partitive (`sama kuin`, `samaa kuin`); not
/// `samalla kun` or `samaan aikaan kun`, which are temporal.
fn same(v: &Voikko, w: &Word) -> bool {
    has_base(
        v,
        w,
        &["sama", "samanlainen", "samankaltainen"],
        &["nimento", "osanto"],
    )
}

/// Word `k` is `kun` where a comparison needs `kuin`: after a comparative (`parempi kun`),
/// `yhtä X kun`, `sama (X) kun`, `samanlainen kun` or `ikään kun`.
fn kun_for_kuin(v: &Voikko, chars: &[char], ws: &[Word], k: usize) -> bool {
    if ws[k].lower != "kun" || k == 0 || !plain_gap(chars, &ws[k - 1], &ws[k]) {
        return false;
    }
    let prev = &ws[k - 1];
    if is_comparative(v, prev) || prev.lower == "ikään" || same(v, prev) {
        return true;
    }
    let Some(before) = k
        .checked_sub(2)
        .map(|i| &ws[i])
        .filter(|b| plain_gap(chars, b, prev))
    else {
        return false;
    };
    let rs: Vec<Reading> = readings(v, &prev.lower)
        .into_iter()
        .filter(|r| !r.proper)
        .collect();
    // `yhtä hyvä kun`, `yhtä paljon kun`; not `yhtä aikaa kun`.
    if before.lower == "yhtä" && prev.lower != "aikaa" {
        return rs.iter().any(|r| {
            matches!(
                r.class,
                Some("laatusana" | "nimisana_laatusana" | "seikkasana")
            )
        });
    }
    // `sama tulos kun`, `samaa mieltä kun`: a noun in the nominative or partitive between.
    same(v, before)
        && !rs.is_empty()
        && rs
            .iter()
            .all(|r| r.class == Some("nimisana") && matches!(r.case, Some("nimento" | "osanto")))
}

/// Words and base forms that make a following `, kuin` comparative or `as if` (`toisin, kuin`,
/// `niin, kuin`, `tuntui, kuin`).
const COMPARISON_WORDS: &[&str] = &[
    "yhtä", "niin", "siltä", "toisin", "ikään", "aivan", "ihan", "kuten", "juuri", "mitä", "sitä",
    "kertaa", "ennen", "vasta", "muu", "muuta", "muut", "muita", "muuhun", "muualla", "muualle",
    "muualta", "muulloin", "muuten", "muun", "muussa", "muusta", "eri", "paitsi", "enää",
];
const COMPARISON_BASES: &[&str] = &[
    "sama",
    "samanlainen",
    "samankaltainen",
    "toinen",
    "toisenlainen",
    "erilainen",
    "sellainen",
    "tällainen",
    "tuollainen",
    "muunlainen",
    "tuntua",
    "näyttää",
    "kuulostaa",
    "vaikuttaa",
    "tuntea",
    "kuvitella",
];

/// Word `k` is `kuin` opening a time clause after a comma (`soita, kuin olet valmis`): no
/// comparison or `as if` word earlier in the sentence, and an indicative verb (after an
/// optional subject pronoun) right after it.
fn kuin_for_kun(v: &Voikko, chars: &[char], ws: &[Word], k: usize) -> bool {
    if ws[k].text != "kuin" || k == 0 {
        return false;
    }
    let gap: String = chars[ws[k - 1].end..ws[k].start].iter().collect();
    if gap.trim() != "," {
        return false;
    }
    let from = sentence_start_of(chars, ws, k);
    if ws[from..k].iter().any(|w| {
        COMPARISON_WORDS.contains(&w.lower.as_str())
            || is_comparative(v, w)
            || has_base(v, w, COMPARISON_BASES, &[])
    }) {
        return false;
    }
    let mut i = k + 1;
    if ws
        .get(i)
        .is_some_and(|w| SUBJECT_PRONOUNS.contains(&w.lower.as_str()))
    {
        i += 1;
    }
    let Some(verb) = ws.get(i) else {
        return false;
    };
    if (k + 1..=i).any(|j| !plain_gap(chars, &ws[j - 1], &ws[j])) {
        return false;
    }
    let rs = readings(v, &verb.lower);
    !rs.is_empty()
        && rs.iter().all(|r| {
            r.class == Some("teonsana")
                && r.mood == Some("indicative")
                && matches!(r.person, Some('1' | '2' | '3'))
        })
}

fn kuin(v: &Voikko, chars: &[char], ws: &[Word], out: &mut Vec<Lint>) {
    for (k, w) in ws.iter().enumerate() {
        let (fix, message) = if kun_for_kuin(v, chars, ws, k) {
            ("kuin", "In a comparison, write `kuin`.")
        } else if kuin_for_kun(v, chars, ws, k) {
            (
                "kun",
                "A time clause starts with `kun`; `kuin` compares (`parempi kuin`).",
            )
        } else {
            continue;
        };
        let fix = if w.text.starts_with(char::is_uppercase) {
            capitalize(fix)
        } else {
            fix.to_string()
        };
        out.push(lint(
            w.start,
            w.end,
            LintKind::WordChoice,
            message.to_string(),
            &fix,
        ));
    }
}

/// Word `k` of `ws` starts with a lowercase letter and follows word `k - 1` after spaces only.
fn joined_lower(chars: &[char], ws: &[Word], k: usize) -> bool {
    k > 0 && plain_gap(chars, &ws[k - 1], &ws[k]) && ws[k].text.starts_with(char::is_lowercase)
}

/// Readings of `w` as a noun, adjective, pronoun or name: an antecedent of a relative clause.
fn nominal(v: &Voikko, w: &Word) -> bool {
    let rs = readings(v, &w.lower);
    !rs.is_empty()
        && w.text.chars().all(char::is_alphabetic)
        && rs.iter().any(|r| {
            matches!(
                r.class,
                Some(
                    "nimisana"
                        | "nimisana_laatusana"
                        | "laatusana"
                        | "asemosana"
                        | "etunimi"
                        | "sukunimi"
                        | "paikannimi"
                        | "nimi"
                )
            ) && r.case.is_some()
        })
        && !rs.iter().all(|r| r.class == Some("teonsana"))
}

/// Word `k` is a verb or an adverb followed by one: the relative `joka`, not the determiner
/// of `joka päivä`, `joka tapauksessa`.
fn verb_follows(v: &Voikko, chars: &[char], ws: &[Word], k: usize) -> bool {
    let Some(n) = ws.get(k + 1).filter(|n| plain_gap(chars, &ws[k], n)) else {
        return false;
    };
    let rs = readings(v, &n.lower);
    if has_finite(v, n) && !rs.iter().any(|r| r.class == Some("nimisana")) {
        return true;
    }
    !rs.is_empty()
        && rs.iter().all(|r| r.class == Some("seikkasana"))
        && ws
            .get(k + 2)
            .is_some_and(|m| plain_gap(chars, n, m) && has_finite(v, m))
}

fn comma(v: &Voikko, chars: &[char], ws: &[Word], out: &mut Vec<Lint>) {
    for k in 1..ws.len() {
        if !joined_lower(chars, ws, k) {
            continue;
        }
        let c = &ws[k];
        let prev = &ws[k - 1];
        let word = c.lower.as_str();
        let prev_verb = || {
            let rs = readings(v, &prev.lower);
            !rs.is_empty() && rs.iter().all(|r| r.class == Some("teonsana"))
        };
        let question = QUESTION_WORDS.contains(&word) && prev_verb();
        let conjunction = CLAUSE_WORDS.contains(&word);
        let relative = is_relative(word)
            && nominal(v, prev)
            && (word != "joka" || verb_follows(v, chars, ws, k));
        let mika = MIKA_FORMS.contains(&word) && MIKA_ANTECEDENTS.contains(&prev.lower.as_str());
        let ennen_kuin = word == "kuin" && prev.lower == "ennen";
        // `sillä` (for, because) before a subject pronoun and its verb; else `sillä` is `with it`.
        let silla = word == "sillä"
            && ws.get(k + 1).is_some_and(|n| {
                plain_gap(chars, c, n) && SUBJECT_PRONOUNS.contains(&n.lower.as_str())
            })
            && ws
                .get(k + 2)
                .is_some_and(|n| plain_gap(chars, &ws[k + 1], n) && has_finite(v, n));
        if !(question || conjunction || relative || mika || ennen_kuin || silla) {
            continue;
        }
        // `mitä tahansa`, `mitä enemmän`: no clause.
        if (question || mika)
            && ws.get(k + 1).is_some_and(|n| {
                matches!(
                    n.lower.as_str(),
                    "tahansa" | "hyvänsä" | "enemmän" | "pikemmin"
                )
            })
        {
            continue;
        }
        if kun_for_kuin(v, chars, ws, k) {
            continue;
        }
        let sentence_start = sentence_start_of(chars, ws, k);
        // `sekä ... että`: correlative, no comma.
        if word == "että" && ws[sentence_start..k].iter().any(|w| w.lower == "sekä") {
            continue;
        }
        // The first word of the unit the comma goes before: `ennen kuin`, `niin että`, `sen
        // jälkeen kun`. Flexible units also take it after them (`siksi, että`).
        let pronoun_head = k >= 2
            && FLEX_HEAD.contains(&prev.lower.as_str())
            && FLEX_DET.contains(&ws[k - 2].lower.as_str())
            && plain_gap(chars, &ws[k - 2], prev);
        let unit_word = conjunction || question;
        let (unit, comma_at_unit) =
            if ennen_kuin || unit_word && UNIT_BEFORE.contains(&prev.lower.as_str()) {
                (k - 1, true)
            } else if unit_word && FLEX_ONE.contains(&prev.lower.as_str()) {
                (k - 1, false)
            } else if unit_word && pronoun_head {
                (k - 2, false)
            } else {
                (k, false)
            };
        if unit <= sentence_start || !plain_gap(chars, &ws[unit - 1], &ws[unit]) {
            continue;
        }
        let before = &ws[unit - 1];
        if NO_COMMA_AFTER.contains(&before.lower.as_str()) {
            continue;
        }
        if !silla && !clause_has_finite(v, chars, ws, k + 1) {
            continue;
        }
        let (before, first) = if comma_at_unit {
            (before, &ws[unit])
        } else {
            (prev, c)
        };
        let fix = format!(
            "{}, {}",
            before.text,
            chars[first.start..c.end].iter().collect::<String>()
        );
        out.push(lint(
            before.start,
            c.end,
            LintKind::Grammar,
            format!(
                "Add a comma before `{}`: a subordinate clause starts here.",
                first.lower
            ),
            &fix,
        ));
    }
}

/// Negations after which `vaan` (but) may follow in the same sentence.
const NEGATIONS: &[&str] = &[
    "ei",
    "en",
    "et",
    "emme",
    "ette",
    "eivät",
    "älä",
    "älkää",
    "älköön",
    "eikä",
    "ettei",
    "ellei",
    "jottei",
    "ilman",
    "tuskin",
    "ainoastaan",
    "pelkästään",
    "yksinomaan",
];

/// Index of the first word of the sentence holding word `k`.
fn sentence_start_of(chars: &[char], ws: &[Word], k: usize) -> usize {
    ws[..k]
        .iter()
        .rposition(|w| {
            let g: String = chars[w.end..].iter().take(3).collect();
            g.trim_start().starts_with(['.', '!', '?'])
        })
        .map_or(0, |i| i + 1)
}

/// A finite verb only: every reading an indicative or conditional form of persons 1-3.
fn finite_verb(v: &Voikko, w: &Word) -> bool {
    let rs = readings(v, &w.lower);
    !rs.is_empty()
        && rs.iter().all(|r| {
            r.class == Some("teonsana")
                && matches!(r.mood, Some("indicative" | "conditional"))
                && matches!(r.person, Some('1' | '2' | '3'))
        })
}

/// `vaan` (but) needs a negation before it in its sentence; without one it is colloquial for
/// `vain` (`on vaan kaksi`). And `vain` after a negated clause, right before a finite verb,
/// is `vaan` (`ei sisällä logiikkaa vain kutsuu rajapintaa`). Not in quotations.
fn vaan_vain(v: &Voikko, chars: &[char], ws: &[Word], out: &mut Vec<Lint>) {
    let toks: Vec<(usize, usize)> = ws.iter().map(|w| (w.start, w.end)).collect();
    let quotes = super::spell_lang::quotations(chars, &toks);
    for (k, w) in ws.iter().enumerate() {
        if !matches!(w.lower.as_str(), "vaan" | "vain")
            || quotes.iter().any(|q| q.contains(&w.start))
        {
            continue;
        }
        let from = sentence_start_of(chars, ws, k);
        if k == from || !w.text.starts_with(char::is_lowercase) {
            continue;
        }
        let negation = ws[from..k]
            .iter()
            .position(|p| NEGATIONS.contains(&p.lower.as_str()))
            .map(|i| from + i);
        if w.lower == "vaan" {
            if negation.is_none() {
                out.push(lint(
                    w.start,
                    w.end,
                    LintKind::WordChoice,
                    "Without a negation before it, `vaan` means `vain` (only) in colloquial \
                     Finnish; standard Finnish writes `vain` (or `mutta` for but)."
                        .to_string(),
                    "vain",
                ));
            }
            continue;
        }
        // `ei vain ... vaan myös`, `ei ole vain`: `vain` right after the negation is right.
        let Some(n) = negation else { continue };
        // `ei ainoastaan hidas vain myös kallis`: `not only ... but also`.
        if k >= n + 2
            && ws
                .get(k + 1)
                .is_some_and(|next| plain_gap(chars, w, next) && next.lower == "myös")
        {
            out.push(lint(
                w.start,
                w.end,
                LintKind::WordChoice,
                "`Not only ... but also` is `ei vain ... vaan myös`.".to_string(),
                "vaan",
            ));
            continue;
        }
        if k < n + 3 || ws[n..k].iter().any(|p| p.lower == "vaan") {
            continue;
        }
        if ws.get(k + 1).is_some_and(|next| {
            plain_gap(chars, w, next)
                && next.text.starts_with(char::is_lowercase)
                && finite_verb(v, next)
        }) {
            out.push(lint(
                w.start,
                w.end,
                LintKind::WordChoice,
                "After a negated clause, `but` is `vaan`: `ei X vaan Y`.".to_string(),
                "vaan",
            ));
        }
    }
}

/// Nominatives that are time adverbials after these (`joka päivä tulevat`).
const TIME_ATTRIBUTES: &[&str] = &["joka", "koko", "ensi", "viime", "kuluva"];

/// Subject-verb agreement in two safe patterns: a plural nominative subject (`tiedot`, `ne`)
/// right before a singular intransitive verb in `-uu`, `-yy` or `-nee` (`siirtyy`, `lievenee`:
/// no zero-person object reading), and a singular nominative noun right before a third person
/// plural verb (`katselija pystyivät`) with no coordination before it.
fn agreement(v: &Voikko, chars: &[char], ws: &[Word], out: &mut Vec<Lint>) {
    let nominal = |r: &Reading| {
        matches!(
            r.class,
            Some("nimisana" | "asemosana" | "laatusana" | "nimisana_laatusana")
        ) && r.case == Some("nimento")
            && !r.proper
    };
    for k in 0..ws.len().saturating_sub(1) {
        let (a, b) = (&ws[k], &ws[k + 1]);
        if !plain_gap(chars, a, b)
            || !b.text.starts_with(char::is_lowercase)
            || !a.text.chars().all(char::is_alphabetic)
            || (a.text.starts_with(char::is_uppercase) && k != sentence_start_of(chars, ws, k))
        {
            continue;
        }
        let ra = readings(v, &a.lower);
        let rb = readings(v, &b.lower);
        if ra.is_empty() || rb.is_empty() || !ra.iter().all(nominal) {
            continue;
        }
        // `palvelu, jotka käynnistyy`: the pronoun disagrees with its antecedent instead
        // (`FinnishRelative`).
        if is_relative(&a.lower)
            && k > 0
            && number_of(v, &ws[k - 1]) == Some("singular")
            && chars[ws[k - 1].end..a.start].contains(&',')
        {
            continue;
        }
        let verb3 = |n: &str| {
            rb.iter().all(|r| {
                r.class == Some("teonsana")
                    && matches!(r.mood, Some("indicative" | "conditional"))
                    && r.person == Some('3')
                    && r.number == Some(n)
            })
        };
        let plural_subject = ra.iter().all(|r| r.number == Some("plural"));
        if plural_subject && verb3("singular") {
            let fix = if let Some(s) = b.lower.strip_suffix("uu") {
                format!("{s}uvat")
            } else if let Some(s) = b.lower.strip_suffix("yy") {
                format!("{s}yvät")
            } else if let Some(s) = b.lower.strip_suffix("nee") {
                format!("{s}nevät")
            } else {
                continue;
            };
            if !v.spell(&fix) {
                continue;
            }
            out.push(lint(
                b.start,
                b.end,
                LintKind::Grammar,
                format!("`{}` is plural: the verb is `{fix}`.", a.text),
                &fix,
            ));
            continue;
        }
        let singular_noun = ra
            .iter()
            .all(|r| r.class == Some("nimisana") && r.number == Some("singular"));
        if !singular_noun
            || !verb3("plural")
            || WEEKDAYS_MONTHS.iter().any(|d| a.lower.starts_with(d))
        {
            continue;
        }
        // `potilas ja omainen voivat`, `joka päivä tulevat`, a list before the noun.
        let from = sentence_start_of(chars, ws, k);
        let lo = k.saturating_sub(4).max(from);
        let coordinated = ws[lo..k]
            .iter()
            .any(|p| matches!(p.lower.as_str(), "ja" | "sekä" | "tai" | "eli" | "kuin"))
            || (lo..=k).any(|i| i > from && chars[ws[i - 1].end..ws[i].start].contains(&','));
        if coordinated || k > 0 && TIME_ATTRIBUTES.contains(&ws[k - 1].lower.as_str()) {
            continue;
        }
        out.push(lint(
            a.start,
            b.end,
            LintKind::Grammar,
            format!(
                "`{}` is singular but `{}` plural: make them agree.",
                a.text, b.text
            ),
            &format!("{} {}", a.text, b.text),
        ));
    }
}

/// Verbs whose complement is in the elative (`tykätä jostakin`): a partitive `sitä` after
/// them is `siitä`. Not verbs that also take a partitive object (`puhua sitä kieltä`, `kertoa`,
/// `nauttia`, `kärsiä`, `välittää`).
const ELATIVE_VERBS: &[&str] = &[
    "tykätä",
    "huolehtia",
    "johtua",
    "riippua",
    "hyötyä",
    "luopua",
    "keskustella",
    "neuvotella",
    "haaveilla",
    "unelmoida",
    "kieltäytyä",
    "iloita",
    "koostua",
    "selviytyä",
    "pitää",
];
/// Stems of derived elative verbs Voikko gives no base form for (`kiinnostuin`).
const ELATIVE_STEMS: &[&str] = &[
    "kiinnostu",
    "kiinnostui",
    "innostu",
    "innostui",
    "huolestu",
    "huolestui",
    "ilahdu",
    "ilahtu",
    "ilahdui",
];
/// Partitive demonstratives and their elatives.
const PARTITIVE_ELATIVE: &[(&str, &str)] = &[
    ("sitä", "siitä"),
    ("tätä", "tästä"),
    ("niitä", "niistä"),
    ("näitä", "näistä"),
    ("tuota", "tuosta"),
    ("noita", "noista"),
];
/// Adverbs that may stand between the verb and its complement (`pidän todella siitä`).
const DEGREE_ADVERBS: &[&str] = &[
    "todella",
    "kovasti",
    "paljon",
    "erityisesti",
    "myös",
    "hyvin",
    "aina",
    "eniten",
    "kyllä",
    "vielä",
    "yhä",
    "ehdottomasti",
    "jo",
    "nyt",
    "enemmän",
    "kovin",
    "ihan",
    "aivan",
    "erittäin",
    "usein",
];

/// `sitä` for `siitä` after a verb that governs the elative: `tykkään sitä`, `huolehdi
/// tätä`, `pidän sitä.` (`pitää` only with nothing after the pronoun in its clause, since
/// `pidän sitä tärkeänä` and `pidä sitä kädessä` are right).
fn elative(v: &Voikko, chars: &[char], ws: &[Word], out: &mut Vec<Lint>) {
    for k in 1..ws.len() {
        let w = &ws[k];
        let Some((_, fix)) = PARTITIVE_ELATIVE.iter().find(|(p, _)| *p == w.lower) else {
            continue;
        };
        if !joined_lower(chars, ws, k) {
            continue;
        }
        let mut vi = k - 1;
        if DEGREE_ADVERBS.contains(&ws[vi].lower.as_str()) {
            if vi == 0 || !plain_gap(chars, &ws[vi - 1], &ws[vi]) {
                continue;
            }
            vi -= 1;
        }
        let verb = &ws[vi];
        let rs = readings(v, &verb.lower);
        let elative_verb = |r: &Reading| {
            r.class == Some("teonsana")
                && match r.base.as_deref() {
                    Some("pitää") => false,
                    Some(b) => ELATIVE_VERBS.contains(&b),
                    None => ELATIVE_STEMS.iter().any(|s| verb.lower.starts_with(s)),
                }
        };
        // `pidän`, `pidin`, `pitäisin`, `en pidä`: liking, not keeping. Imperative and passive
        // forms are excluded.
        let negated = vi > 0
            && NEGATIONS[..6].contains(&ws[vi - 1].lower.as_str())
            && plain_gap(chars, &ws[vi - 1], verb);
        let likes = |r: &Reading| {
            r.base.as_deref() == Some("pitää")
                && (matches!(r.mood, Some("indicative" | "conditional"))
                    && matches!(r.person, Some('1' | '2' | '3'))
                    || negated && verb.lower == "pidä")
        };
        let next = ws.get(k + 1).filter(|n| plain_gap(chars, w, n));
        let clause_end = match ws.get(k + 1) {
            Some(n) => clause_break(chars, w, n),
            None => true,
        };
        let hit = if rs.iter().any(elative_verb) {
            // `tykkään sitä kirjaa` (a determiner: the fix is more than one word), `puhuttiin
            // sitä sun tätä`.
            !next.is_some_and(|n| {
                matches!(n.lower.as_str(), "ja" | "sun" | "tai")
                    || readings(v, &n.lower)
                        .iter()
                        .any(|r| r.case == Some("osanto"))
            })
        } else {
            clause_end && rs.iter().any(likes)
        };
        if !hit {
            continue;
        }
        out.push(lint(
            w.start,
            w.end,
            LintKind::Grammar,
            format!(
                "`{}` takes the elative: `{fix}`, not the partitive `{}`.",
                verb.lower, w.lower
            ),
            fix,
        ));
    }
}

/// Antecedents that are singular in form but plural in sense (`kaikki, jotka`, `osa, jotka`).
const PLURAL_SENSE: &[&str] = &[
    "kaikki",
    "moni",
    "harva",
    "useampi",
    "muutama",
    "pari",
    "osa",
    "joukko",
    "enemmistö",
    "vähemmistö",
    "loput",
    "kukin",
    "jokainen",
    "kumpikin",
    "kukaan",
    "ryhmä",
    "väki",
    "henkilöstö",
];

/// The number of every nominal reading of `w`, when they agree.
fn number_of(v: &Voikko, w: &Word) -> Option<&'static str> {
    let rs = readings(v, &w.lower);
    let first = rs.first()?.number?;
    rs.iter().all(|r| r.number == Some(first)).then_some(first)
}

/// Relative pronouns: `joka` forms agree in number with the noun before the comma
/// (`tiedostot, joka` -> `jotka`, `palvelu, jotka` -> `joka`), a clause antecedent after its
/// finite verb takes `mikä` (`palvelu kaatui, joka` -> `mikä`), and `mitkä` after a plural
/// noun is `jotka`.
fn relative(v: &Voikko, chars: &[char], ws: &[Word], out: &mut Vec<Lint>) {
    for k in 1..ws.len() {
        let w = &ws[k];
        let (a, b) = (&ws[k - 1], w);
        let gap: String = chars[a.end..b.start].iter().collect();
        if gap.trim() != ","
            || gap.contains('\n')
            || !b.text.starts_with(char::is_lowercase)
            || !a.text.chars().all(char::is_alphabetic)
        {
            continue;
        }
        let from = sentence_start_of(chars, ws, k);
        let push = |out: &mut Vec<Lint>, fix: &str, message: String| {
            out.push(lint(b.start, b.end, LintKind::Grammar, message, fix));
        };
        // `tiedostot, mitkä`: a plural noun, so the relative `jotka`.
        if b.lower == "mitkä" {
            if number_of(v, a) == Some("plural")
                && readings(v, &a.lower)
                    .iter()
                    .all(|r| r.class == Some("nimisana") && !r.proper)
            {
                push(
                    out,
                    "jotka",
                    format!(
                        "After the noun `{}`, the relative pronoun is `jotka`.",
                        a.text
                    ),
                );
            }
            continue;
        }
        let Some(&(sg, pl)) = RELATIVES
            .iter()
            .find(|(s, p)| *s == b.lower || *p == b.lower)
        else {
            continue;
        };
        let rel_plural = pl == b.lower;
        let ra = readings(v, &a.lower);
        if ra.is_empty() {
            continue;
        }
        // A whole clause before: `palvelu kaatui, joka aiheutti` -> `mikä`.
        let finite_only = ra.iter().all(|r| {
            r.class == Some("teonsana")
                && matches!(r.mood, Some("indicative" | "conditional"))
                && r.person.is_some()
        });
        if finite_only && !rel_plural {
            let fix = match sg {
                "joka" if verb_follows(v, chars, ws, k) => "mikä",
                "jonka" => "minkä",
                "jota" => "mitä",
                _ => continue,
            };
            push(
                out,
                fix,
                format!(
                    "`{}` refers to the whole clause before it: write `{fix}`.",
                    b.lower
                ),
            );
            continue;
        }
        let noun_classes = |r: &Reading| {
            matches!(
                r.class,
                Some(
                    "nimisana"
                        | "nimisana_laatusana"
                        | "laatusana"
                        | "asemosana"
                        | "etunimi"
                        | "sukunimi"
                        | "paikannimi"
                        | "nimi"
                )
            ) && r.case.is_some()
        };
        if !ra.iter().all(noun_classes) || PLURAL_SENSE.contains(&a.lower.as_str()) {
            continue;
        }
        let Some(number) = number_of(v, a) else {
            continue;
        };
        // `seitsemän kehittäjää, joista`: a partitive after a numeral or quantifier.
        if (number == "plural") == rel_plural
            || rel_plural && ra.iter().any(|r| r.case == Some("osanto"))
        {
            continue;
        }
        // Another noun of the pronoun's number earlier in the sentence may be the antecedent
        // (`tiedot palvelusta, jotka`), and so may a coordination (`palvelin ja tietokanta,
        // jotka`).
        let want = if rel_plural { "plural" } else { "singular" };
        let lo = k.saturating_sub(7).max(from);
        let other = ws[from..k - 1].iter().any(|p| {
            readings(v, &p.lower)
                .iter()
                .any(|r| noun_classes(r) && r.number == Some(want))
        });
        let coordinated = ws[lo..k - 1]
            .iter()
            .any(|p| matches!(p.lower.as_str(), "ja" | "sekä" | "tai" | "eli" | "kuin"))
            || (lo + 1..k - 1).any(|i| chars[ws[i - 1].end..ws[i].start].contains(&','));
        if other || coordinated {
            continue;
        }
        let next = ws.get(k + 1).filter(|n| plain_gap(chars, b, n));
        let fix = match (b.lower.as_str(), rel_plural) {
            // `tiedostot, jonka nimet` (genitive) or `tiedostot, jonka tallensin` (object).
            ("jonka", false) => {
                if next.is_some_and(|n| {
                    POSTPOSITIONS.contains(&n.lower.as_str())
                        || readings(v, &n.lower)
                            .iter()
                            .all(|r| r.class == Some("nimisana"))
                            && !readings(v, &n.lower).is_empty()
                }) {
                    "joiden"
                } else {
                    "jotka"
                }
            }
            // `palvelu, jotka asensimme` (object) or `palvelu, jotka toimii` (subject).
            ("jotka", true) => {
                if next.is_some_and(|n| {
                    readings(v, &n.lower).iter().any(|r| {
                        r.class == Some("teonsana") && matches!(r.person, Some('1' | '2' | '4'))
                    })
                }) {
                    "jonka"
                } else {
                    "joka"
                }
            }
            (_, true) => sg,
            (_, false) => pl,
        };
        push(
            out,
            fix,
            format!("`{}` is {number}: the relative pronoun is `{fix}`.", a.text),
        );
    }
}

/// Lints of the Finnish rules in `on` for one segment text.
pub fn lints(
    v: &Voikko,
    sp: &dyn LangSpeller,
    chars: &[char],
    on: &[&str],
) -> BTreeMap<String, Vec<Lint>> {
    let ws = words(chars);
    let mut out = BTreeMap::new();
    for &name in on {
        let mut l = Vec::new();
        match name {
            "FinnishCompoundSplit" => compound_split(v, sp, chars, &ws, &mut l),
            "FinnishCompoundJoined" => compound_joined(v, &ws, &mut l),
            "FinnishCapitalization" => capitalization(v, chars, &ws, &mut l),
            "FinnishSentenceStart" => sentence_start(v, chars, &ws, &mut l),
            "FinnishKuin" => kuin(v, chars, &ws, &mut l),
            "FinnishComma" => comma(v, chars, &ws, &mut l),
            "FinnishVaanVain" => vaan_vain(v, chars, &ws, &mut l),
            "FinnishAgreement" => agreement(v, chars, &ws, &mut l),
            "FinnishElative" => elative(v, chars, &ws, &mut l),
            "FinnishRelative" => relative(v, chars, &ws, &mut l),
            _ => {}
        }
        if !l.is_empty() {
            out.insert(name.to_string(), l);
        }
    }
    out
}

#[cfg(all(test, feature = "voikko"))]
mod tests {
    use super::*;

    fn run(text: &str) -> Vec<(String, String)> {
        let v = crate::voikko::embedded();
        let sp = crate::rules::spell_lang::speller("fi", &crate::config::Config::default())
            .expect("bundled Finnish");
        let chars: Vec<char> = text.chars().collect();
        let names: Vec<&str> = RULES.iter().map(|(n, _)| *n).collect();
        lints(v, &*sp, &chars, &names)
            .into_iter()
            .flat_map(|(n, ls)| {
                ls.into_iter().map(move |l| {
                    (
                        n.clone(),
                        text.chars()
                            .skip(l.span.start)
                            .take(l.span.end - l.span.start)
                            .collect(),
                    )
                })
            })
            .collect()
    }

    #[test]
    fn finnish_rules() {
        let got = run(
            "Tallennamme potilas tietoja. Kokous on Tiistaina. tämä on parempi kun ennen. \
            Kerro jos tarvitset apua. Käynnistyksenyhteydessä näkyy ikkuna.",
        );
        let rules: Vec<&str> = got.iter().map(|(r, _)| r.as_str()).collect();
        assert!(rules.contains(&"FinnishCompoundSplit"), "{got:?}");
        assert!(rules.contains(&"FinnishCapitalization"), "{got:?}");
        assert!(rules.contains(&"FinnishSentenceStart"), "{got:?}");
        assert!(rules.contains(&"FinnishKuin"), "{got:?}");
        assert!(rules.contains(&"FinnishComma"), "{got:?}");
        assert!(rules.contains(&"FinnishCompoundJoined"), "{got:?}");
    }

    #[test]
    fn vaan_vain_and_agreement() {
        let got = run(
            "Palvelussa on vaan kaksi palvelinta. Käyttöliittymä ei sisällä logiikkaa vain \
            kutsuu rajapintaa. Oireet lievenee yleensä. Katselija pystyivät avaamaan näkymän.",
        );
        let rules: Vec<&str> = got.iter().map(|(r, _)| r.as_str()).collect();
        assert_eq!(
            rules.iter().filter(|r| **r == "FinnishVaanVain").count(),
            2,
            "{got:?}"
        );
        assert_eq!(
            rules.iter().filter(|r| **r == "FinnishAgreement").count(),
            2,
            "{got:?}"
        );
        let quiet = run(
            "Se ei ole vain työkalu vaan tapa toimia. Hän ei tullut, vaan soitti. Potilas ja \
            omainen voivat tulla. Tiedot tallennetaan. Tulokset näkee sovelluksesta. \
            \"Mä vaan pushasin koodin\" on tuttu lause.",
        );
        assert!(quiet.is_empty(), "{quiet:?}");
    }

    #[test]
    fn noun_phrases_are_no_split_compounds() {
        let got = run(
            "Ottakaa lääke kerran päivässä. Jättäkää unohtunut annos väliin. Uusi järjestelmä \
            käyttöön. Data on anonymisoitu kopio tuotannosta. Viestin sisältö on salattava levy \
            tilalla.",
        );
        let split: Vec<&str> = got
            .iter()
            .filter(|(r, _)| r == "FinnishCompoundSplit")
            .map(|(_, t)| t.as_str())
            .collect();
        assert_eq!(split, ["levy tilalla"], "{got:?}");
    }

    /// Split compounds with an inflected second word: an agreeing attribute before the pair
    /// marks one noun; a subject before its verb or a predicative after `olla` is a phrase.
    #[test]
    fn inflected_split_compounds() {
        let split = |text: &str| -> Vec<String> {
            run(text)
                .into_iter()
                .filter(|(r, _)| r == "FinnishCompoundSplit")
                .map(|(_, t)| t)
                .collect()
        };
        assert_eq!(
            split("Ajat näytetään asiakkaan omasta terveys asemasta."),
            ["terveys asemasta"]
        );
        assert_eq!(
            split("Seloste noudattaa yleisen tietosuoja asetuksen vaatimuksia."),
            ["tietosuoja asetuksen"]
        );
        assert_eq!(split("Lähetä yhteys tiedot meille."), ["yhteys tiedot"]);
        for text in [
            "Hoito sairaalassa kestää yleensä kolme päivää.",
            "Maksu kortilla onnistuu kaikissa toimipisteissä.",
            "Raportti johdolle laaditaan kuukausittain.",
            "Meillä on pääsy järjestelmään.",
            "Päätös hankinnasta tehdään kokouksessa.",
        ] {
            assert!(split(text).is_empty(), "{text}: {:?}", split(text));
        }
    }

    #[test]
    fn correct_finnish_is_quiet() {
        let got = run(
            "Osa potilaista saapuu 15. syyskuuta klo 12. Presidentti Niinistö puhui. \
            Kerro, jos tarvitset apua. Hän on sekä nopea että tarkka. Voit valita mitä tahansa.",
        );
        assert!(got.is_empty(), "{got:?}");
    }

    /// Texts of the lints of rule `rule` in `text`.
    fn of(rule: &str, text: &str) -> Vec<String> {
        run(text)
            .into_iter()
            .filter(|(r, _)| r == rule)
            .map(|(_, t)| t)
            .collect()
    }

    fn quiet(rule: &str, texts: &[&str]) {
        for text in texts {
            let got = of(rule, text);
            assert!(got.is_empty(), "{rule}: {text}: {got:?}");
        }
    }

    #[test]
    fn elative_after_elative_verbs() {
        assert_eq!(
            of(
                "FinnishElative",
                "Tykkään sitä todella paljon. Pidän sitä. En pidä sitä. Huolehdi tätä ennen \
                 lähtöä. Virhe johtuu sitä. Kiinnostuin niitä heti."
            ),
            ["sitä", "sitä", "sitä", "tätä", "sitä", "niitä"]
        );
        quiet(
            "FinnishElative",
            &[
                "Pidän sitä tärkeänä.",
                "Pidä sitä kädessä.",
                "Pidä sitä.",
                "Puhun suomea ja puhun sitä hyvin.",
                "Kerroin sitä kaikille.",
                "Tykkään siitä.",
                "Puhuttiin sitä sun tätä.",
                "Minun pitää sitä miettiä.",
                "En pidä sitä hyvänä ratkaisuna.",
                "Tykkään sitä kirjaa lukea.",
                "Sitä pidetään yleisesti hyvänä.",
                "Pidetään sitä.",
            ],
        );
    }

    #[test]
    fn kuin_and_kun() {
        assert_eq!(
            of(
                "FinnishKuin",
                "Tämä on yhtä hyvä kun edellinen. Tulos on sama kun viime vuonna. Se toimii \
                 ikään kun ennenkin. Soita, kuin olet valmis. Tämä on samanlainen kun se. Hinta \
                 on sama summa kun ennen."
            ),
            ["kun", "kun", "kun", "kuin", "kun", "kun"]
        );
        quiet(
            "FinnishKuin",
            &[
                "Samalla kun asennus etenee, lokit tallentuvat.",
                "Samaan aikaan kun palvelu päivittyy, varmuuskopio otetaan.",
                "Samana päivänä kun järjestelmä avattiin, tuli vika.",
                "Yhtä aikaa kun hälytys tuli, valot sammuivat.",
                "Tee se, kun ehdit.",
                "Se on parempi kuin ennen.",
                "Tuntui, kuin olisin nähnyt aaveen.",
                "Hän toimi niin, kuin oli sovittu.",
                "Tilanne on toinen, kuin luulit.",
                "En tehnyt muuta, kuin istuin koneella.",
                "Hän juoksi nopeammin, kuin odotin.",
                "Näytti siltä, kuin se toimii.",
                "Kaikki sujui, kuin olisi ollut tapana.",
            ],
        );
    }

    #[test]
    fn vaan_myos() {
        assert_eq!(
            of(
                "FinnishVaanVain",
                "Se ei ole ainoastaan hidas vain myös kallis."
            ),
            ["vain"]
        );
        quiet(
            "FinnishVaanVain",
            &[
                "Se ei ole vain hidas vaan myös kallis.",
                "Emme käytä Javaa, vain Rustia.",
                "Vain myös-sana puuttuu.",
                "Hän tuli vain myös hakemaan avaimet.",
            ],
        );
    }

    #[test]
    fn relative_pronouns_agree() {
        assert_eq!(
            run(
                "Tiedostot, joka tallennettiin eilen, poistetaan. Palvelu, jotka \
                 käynnistyy aamulla, on uusi. Palvelin kaatui, joka aiheutti katkon. Listaa \
                 palvelut, mitkä ovat käytössä. Asiakas, jotka asensimme, toimii. Tiedostot, \
                 jonka nimet muuttuivat, siirrettiin."
            )
            .into_iter()
            .filter(|(r, _)| r == "FinnishRelative" || r == "FinnishAgreement")
            .collect::<Vec<_>>(),
            [
                ("FinnishRelative", "joka"),
                ("FinnishRelative", "jotka"),
                ("FinnishRelative", "joka"),
                ("FinnishRelative", "mitkä"),
                ("FinnishRelative", "jotka"),
                ("FinnishRelative", "jonka"),
            ]
            .map(|(r, t)| (r.to_string(), t.to_string()))
        );
        let fixes: Vec<String> = {
            let text = "Palvelin kaatui, joka aiheutti katkon. Asiakas, jotka asensimme, toimii. \
                        Tiedostot, jonka nimet muuttuivat, siirrettiin.";
            let v = crate::voikko::embedded();
            let chars: Vec<char> = text.chars().collect();
            let mut out = Vec::new();
            relative(v, &chars, &words(&chars), &mut out);
            out.iter()
                .flat_map(|l| &l.suggestions)
                .map(|s| match s {
                    Suggestion::ReplaceWith(c) => c.iter().collect(),
                    _ => String::new(),
                })
                .collect()
        };
        assert_eq!(fixes, ["mikä", "jonka", "joiden"]);
        quiet(
            "FinnishRelative",
            &[
                "Tiedostot, jotka tallennettiin eilen, poistetaan.",
                "Palvelu, joka käynnistyy aamulla, on uusi.",
                "Palvelin, tietokanta ja välimuisti, jotka päivitettiin, toimivat.",
                "Tiedot palvelusta, jotka tallennetaan, ovat salattuja.",
                "Kaikki, jotka osallistuivat, saivat palkinnon.",
                "Osa, jotka vastasivat, oli tyytyväisiä.",
                "Palvelin kaatui, mikä aiheutti katkon.",
                "Se tehtiin, jotta voimme jatkaa.",
                "Matti, joka soitti, on paikalla.",
                "Tämä on se, mitä haluan.",
                "Treenaan, joka päivä.",
                "Kysymys, mitkä tiedot tallennetaan, on auki.",
                "Tiimissä on seitsemän kehittäjää, joista kaksi on etänä.",
            ],
        );
    }

    #[test]
    fn comma_before_clauses() {
        assert_eq!(
            of(
                "FinnishComma",
                "Asennus tehdään ennen kuin järjestelmä otetaan käyttöön. Tiedosto joka \
                 tallennettiin eilen poistetaan. Lähdin kotiin sillä hän oli sairas. Kaikki jotka \
                 osallistuivat saivat palkinnon. Tee se mitä haluat. Soita heti kun olet valmis. \
                 Hän sanoi ettei tule. Palvelin jossa sovellus toimii on uusi. Soita sen \
                 jälkeen kun olet valmis."
            ),
            [
                "tehdään ennen kuin",
                "Tiedosto joka",
                "kotiin sillä",
                "Kaikki jotka",
                "se mitä",
                "Soita heti kun",
                "sanoi ettei",
                "Palvelin jossa",
                "jälkeen kun",
            ]
        );
        quiet(
            "FinnishComma",
            &[
                "Treenaan joka päivä.",
                "Joka tapauksessa ilmoita.",
                "Tarkistamme joka kerta lokit.",
                "Hän on sekä nopea että tarkka.",
                "Ja että se toimii.",
                "Tulen ja jos ehdin, autan.",
                "Sen jälkeen kun olet valmis, soita.",
                "Soita, sen jälkeen kun olet valmis.",
                "Soita sen jälkeen, kun olet valmis.",
                "Samaan aikaan kun palvelu päivittyy, varmuuskopio otetaan.",
                "Samalla kun asennus etenee, lokit tallentuvat.",
                "Voimme tavata vaikka huomenna.",
                "Maksa sillä kortilla.",
                "Ennen kuin aloitat, lue ohje.",
                "Aloita, ennen kuin on myöhäistä.",
                "Kerro, jos tarvitset apua.",
                "Se on parempi kuin joka toinen vaihtoehto.",
                "Tiedosto, jonka tallensit, on tässä.",
                "Tämä on sama kuin ennen.",
                "Tee se niin kuin haluat.",
                "Siksi että se toimii, käytämme sitä.",
                "Lähdin, siksi että olin väsynyt.",
                "Aina kun palvelu käynnistyy, loki kirjoitetaan.",
                "Voit valita mitä tahansa.",
                "Ne jotka.",
                "Kerro mitä tehdä.",
            ],
        );
    }
}
