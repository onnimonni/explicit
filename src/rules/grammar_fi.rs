//! Finnish grammar rules on top of the Voikko morphology ([`crate::voikko`]): split and
//! wrongly joined compounds, capitalization, `kun` for `kuin`, and the comma before a
//! subordinate clause. Each rule is conservative: it fires only where Finnish orthography
//! leaves no choice.

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
        "Finnish: `kuin`, not `kun`, after a comparative (`parempi kuin`)",
    ),
    (
        "FinnishComma",
        "Finnish: a comma before a subordinate clause (`jos`, `että`, `koska`, `kun`, ...)",
    ),
    (
        "FinnishVaanVain",
        "Finnish: `vain` (only) and `vaan` (but, after a negation) confused",
    ),
    (
        "FinnishAgreement",
        "Finnish: a plural subject with a singular verb or the reverse (`tiedot siirtyy`)",
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
];
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

fn kuin(v: &Voikko, chars: &[char], ws: &[Word], out: &mut Vec<Lint>) {
    for pair in ws.windows(2) {
        let (a, b) = (&pair[0], &pair[1]);
        if b.lower == "kun" && plain_gap(chars, a, b) && is_comparative(v, a) {
            let fix = if b.text.starts_with('K') {
                "Kuin"
            } else {
                "kuin"
            };
            out.push(lint(
                b.start,
                b.end,
                LintKind::WordChoice,
                "In a comparison, write `kuin`.".to_string(),
                fix,
            ));
        }
    }
}

fn comma(v: &Voikko, chars: &[char], ws: &[Word], out: &mut Vec<Lint>) {
    for k in 1..ws.len() {
        let c = &ws[k];
        let question = QUESTION_WORDS.contains(&c.lower.as_str());
        if !CLAUSE_WORDS.contains(&c.lower.as_str()) && !question {
            continue;
        }
        let prev = &ws[k - 1];
        if !plain_gap(chars, prev, c) || !c.text.starts_with(char::is_lowercase) {
            continue;
        }
        // `mitä tahansa`, `mitä enemmän`: no clause.
        if question
            && ws.get(k + 1).is_some_and(|n| {
                matches!(
                    n.lower.as_str(),
                    "tahansa" | "hyvänsä" | "enemmän" | "pikemmin"
                )
            })
        {
            continue;
        }
        // Question words open a clause after a verb (`kertoo, mitä`).
        if question
            && !readings(v, &prev.lower)
                .iter()
                .all(|r| r.class == Some("teonsana"))
        {
            continue;
        }
        if c.lower == "kun" && is_comparative(v, prev) {
            continue;
        }
        // `sekä ... että`: correlative, no comma.
        let sentence_start = ws[..k]
            .iter()
            .rposition(|w| {
                let g: String = chars[w.end..].iter().take(3).collect();
                g.trim_start().starts_with(['.', '!', '?'])
            })
            .map_or(0, |i| i + 1);
        if c.lower == "että" && ws[sentence_start..k].iter().any(|w| w.lower == "sekä") {
            continue;
        }
        let (first, before) = if UNIT_BEFORE.contains(&prev.lower.as_str()) {
            match k.checked_sub(2).map(|i| &ws[i]) {
                Some(b) if plain_gap(chars, b, prev) => (prev, b),
                _ => continue,
            }
        } else {
            (c, prev)
        };
        if NO_COMMA_AFTER.contains(&before.lower.as_str()) || k - 1 < sentence_start {
            continue;
        }
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
}
