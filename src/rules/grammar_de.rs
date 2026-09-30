//! Conservative German grammar with explicit lexical evidence, not suffix-based guesses.
use std::collections::BTreeMap;

use super::lint::{Lint, LintKind, Span, Suggestion};
use super::spell_lang::{LangSpeller, quotations, tokens};

pub const RULES: &[(&str, &str)] = &[
    (
        "GermanPronounVerbAgreement",
        "German: a personal subject pronoun agrees with its finite verb",
    ),
    (
        "GermanNounVerbAgreement",
        "German: clear known noun subjects agree with their finite verbs",
    ),
    (
        "GermanArticleAgreement",
        "German: articles agree with known nouns and unambiguous prepositional case",
    ),
    (
        "GermanAuxiliaryParticiple",
        "German: haben takes a past participle, except in replacement-infinitive constructions",
    ),
    (
        "GermanSubordinateWordOrder",
        "German: the finite copula follows a short subordinate predicate",
    ),
];

struct Word {
    start: usize,
    end: usize,
    lower: String,
    plain: bool,
}

fn words(chars: &[char]) -> Vec<Word> {
    let ts = tokens(chars);
    let mut quotes = quotations(chars, &ts);
    // The shared quotation detector deliberately ignores short quotations. Grammar must
    // also leave short examples, German quotation marks and inline code untouched.
    let mut open = None;
    for (i, &c) in chars.iter().enumerate() {
        if let Some((start, closing)) = open {
            if c == closing {
                quotes.push(start..i + 1);
                open = None;
            }
        } else if matches!(c, '"' | '«' | '„' | '“' | '`') {
            open = Some((
                i,
                match c {
                    '«' => '»',
                    '„' => '“',
                    '“' => '”',
                    _ => c,
                },
            ));
        }
    }
    if let Some((start, _)) = open {
        quotes.push(start..chars.len());
    }
    quotes.sort_unstable_by_key(|q| q.start);
    let mut q = 0;
    ts.into_iter()
        .map(|(start, end)| {
            while q < quotes.len() && quotes[q].end <= start {
                q += 1;
            }
            let quoted = quotes
                .get(q)
                .is_some_and(|r| r.start < end && start < r.end);
            let slice = &chars[start..end];
            let plain = !quoted
                && slice.iter().all(|c| c.is_alphabetic())
                && !slice.iter().skip(1).any(|c| c.is_uppercase())
                && !start
                    .checked_sub(1)
                    .and_then(|i| chars.get(i))
                    .is_some_and(|c| *c == '_' || c.is_alphanumeric())
                && !chars
                    .get(end)
                    .is_some_and(|c| *c == '_' || c.is_alphanumeric());
            Word {
                start,
                end,
                lower: slice.iter().flat_map(|c| c.to_lowercase()).collect(),
                plain,
            }
        })
        .collect()
}

fn adjacent(chars: &[char], a: &Word, b: &Word) -> bool {
    a.plain
        && b.plain
        && a.end < b.start
        && chars[a.end..b.start].iter().all(|c| c.is_whitespace())
        && chars[a.end..b.start].iter().filter(|&&c| c == '\n').count() < 2
}

fn clause_start(chars: &[char], ws: &[Word], i: usize) -> bool {
    if i == 0 {
        return chars[..ws[i].start].iter().all(|c| c.is_whitespace());
    }
    let previous = &ws[i - 1];
    let gap = &chars[previous.end..ws[i].start];
    gap.iter()
        .any(|c| matches!(c, '.' | '!' | '?' | ';' | ':' | ','))
        || (adjacent(chars, previous, &ws[i])
            && matches!(
                previous.lower.as_str(),
                "dass"
                    | "weil"
                    | "ob"
                    | "wenn"
                    | "obwohl"
                    | "damit"
                    | "bevor"
                    | "nachdem"
                    | "während"
            ))
}

fn emit(
    out: &mut BTreeMap<String, Vec<Lint>>,
    enabled: &[&str],
    rule: &str,
    chars: &[char],
    span: Span,
    replacement: &str,
    message: &str,
) {
    if !enabled.contains(&rule) {
        return;
    }
    let mut fix: Vec<char> = replacement.chars().collect();
    if chars[span.start].is_uppercase() && !fix.is_empty() {
        let first: Vec<char> = fix[0].to_uppercase().collect();
        fix.splice(0..1, first);
    }
    out.entry(rule.to_string()).or_default().push(Lint {
        span,
        lint_kind: LintKind::Grammar,
        suggestions: vec![Suggestion::ReplaceWith(fix)],
        message: message.to_string(),
        priority: 31,
    });
}

// Person order: ich, du, er/sie/es, wir, ihr, sie. Each row is one tense.
const VERBS: &[[&str; 6]] = &[
    ["bin", "bist", "ist", "sind", "seid", "sind"],
    ["war", "warst", "war", "waren", "wart", "waren"],
    ["habe", "hast", "hat", "haben", "habt", "haben"],
    ["hatte", "hattest", "hatte", "hatten", "hattet", "hatten"],
    ["werde", "wirst", "wird", "werden", "werdet", "werden"],
    ["wurde", "wurdest", "wurde", "wurden", "wurdet", "wurden"],
    ["kann", "kannst", "kann", "können", "könnt", "können"],
    ["muss", "musst", "muss", "müssen", "müsst", "müssen"],
    ["darf", "darfst", "darf", "dürfen", "dürft", "dürfen"],
    ["soll", "sollst", "soll", "sollen", "sollt", "sollen"],
    ["will", "willst", "will", "wollen", "wollt", "wollen"],
    ["weiß", "weißt", "weiß", "wissen", "wisst", "wissen"],
    ["gehe", "gehst", "geht", "gehen", "geht", "gehen"],
    ["komme", "kommst", "kommt", "kommen", "kommt", "kommen"],
    ["mache", "machst", "macht", "machen", "macht", "machen"],
    [
        "arbeite",
        "arbeitest",
        "arbeitet",
        "arbeiten",
        "arbeitet",
        "arbeiten",
    ],
    ["lese", "liest", "liest", "lesen", "lest", "lesen"],
    [
        "schreibe",
        "schreibst",
        "schreibt",
        "schreiben",
        "schreibt",
        "schreiben",
    ],
    [
        "spreche", "sprichst", "spricht", "sprechen", "sprecht", "sprechen",
    ],
    ["sehe", "siehst", "sieht", "sehen", "seht", "sehen"],
    ["nehme", "nimmst", "nimmt", "nehmen", "nehmt", "nehmen"],
    ["gebe", "gibst", "gibt", "geben", "gebt", "geben"],
    ["finde", "findest", "findet", "finden", "findet", "finden"],
    [
        "brauche", "brauchst", "braucht", "brauchen", "braucht", "brauchen",
    ],
    ["nutze", "nutzt", "nutzt", "nutzen", "nutzt", "nutzen"],
    [
        "verwende",
        "verwendest",
        "verwendet",
        "verwenden",
        "verwendet",
        "verwenden",
    ],
    [
        "speichere",
        "speicherst",
        "speichert",
        "speichern",
        "speichert",
        "speichern",
    ],
    ["prüfe", "prüfst", "prüft", "prüfen", "prüft", "prüfen"],
    ["öffne", "öffnest", "öffnet", "öffnen", "öffnet", "öffnen"],
    [
        "erhalte",
        "erhältst",
        "erhält",
        "erhalten",
        "erhaltet",
        "erhalten",
    ],
];

fn person(s: &str) -> Option<usize> {
    match s {
        "ich" => Some(0),
        "du" => Some(1),
        "er" | "es" | "man" | "sie" => Some(2),
        "wir" => Some(3),
        "ihr" => Some(4),
        _ => None,
    }
}

// Singular/plural pairs are explicit: no grammatical guesses from word endings.
// Syncretic pairs such as `Fenster/Fenster` and `Leiter/Leiter` are deliberately absent.
const NOUNS: &[(&str, &str, usize)] = &[
    ("datei", "dateien", 1),
    ("nachricht", "nachrichten", 1),
    ("seite", "seiten", 1),
    ("aufgabe", "aufgaben", 1),
    ("frage", "fragen", 1),
    ("antwort", "antworten", 1),
    ("regel", "regeln", 1),
    ("änderung", "änderungen", 1),
    ("verbindung", "verbindungen", 1),
    ("anwendung", "anwendungen", 1),
    ("einstellung", "einstellungen", 1),
    ("version", "versionen", 1),
    ("beschreibung", "beschreibungen", 1),
    ("möglichkeit", "möglichkeiten", 1),
    ("adresse", "adressen", 1),
    ("bericht", "berichte", 0),
    ("text", "texte", 0),
    ("befehl", "befehle", 0),
    ("dienst", "dienste", 0),
    ("prozess", "prozesse", 0),
    ("zugang", "zugänge", 0),
    ("baum", "bäume", 0),
    ("tag", "tage", 0),
    ("system", "systeme", 2),
    ("dokument", "dokumente", 2),
    ("programm", "programme", 2),
    ("problem", "probleme", 2),
    ("ergebnis", "ergebnisse", 2),
    ("beispiel", "beispiele", 2),
    ("gerät", "geräte", 2),
    ("konto", "konten", 2),
    ("haus", "häuser", 2),
    ("kind", "kinder", 2),
    ("auto", "autos", 2),
    ("buch", "bücher", 2),
];

fn noun(s: &str) -> Option<usize> {
    NOUNS.iter().find_map(|&(singular, plural, gender)| {
        if s == singular {
            Some(gender)
        } else if s == plural
            || (!plural.ends_with('n')
                && !plural.ends_with('s')
                && s.strip_suffix('n') == Some(plural))
        {
            Some(3)
        } else {
            None
        }
    })
}

fn noun_clause_start(chars: &[char], ws: &[Word], i: usize) -> bool {
    clause_start(chars, ws, i)
        && (i == 0
            || !chars[ws[i - 1].end..ws[i].start].contains(&',')
            || matches!(
                ws[i - 1].lower.as_str(),
                "dass"
                    | "weil"
                    | "ob"
                    | "wenn"
                    | "obwohl"
                    | "damit"
                    | "bevor"
                    | "nachdem"
                    | "während"
            ))
}

fn later_subject(chars: &[char], ws: &[Word], verb: usize) -> bool {
    for j in verb + 1..ws.len() {
        if !adjacent(chars, &ws[j - 1], &ws[j]) {
            break;
        }
        // Feminine, neuter and plural nominatives can also be fronted objects.
        // Unknown capitalized nouns and names can be the inverted subject too.
        if person(&ws[j].lower).is_some()
            || chars[ws[j].start].is_uppercase()
            || matches!(
                ws[j].lower.as_str(),
                "jemand"
                    | "niemand"
                    | "jeder"
                    | "jede"
                    | "jedes"
                    | "alle"
                    | "beide"
                    | "einige"
                    | "mehrere"
                    | "viele"
                    | "wenige"
                    | "etwas"
                    | "nichts"
                    | "dieser"
                    | "diese"
                    | "dieses"
            )
        {
            return true;
        }
    }
    false
}

fn noun_agreement(
    out: &mut BTreeMap<String, Vec<Lint>>,
    enabled: &[&str],
    chars: &[char],
    ws: &[Word],
    i: usize,
) {
    if !enabled.contains(&"GermanNounVerbAgreement") || !noun_clause_start(chars, ws, i) {
        return;
    }
    let w = &ws[i];
    let (head, plural) = if let Some(n) = ws.get(i + 1).filter(|n| adjacent(chars, w, n)) {
        let explicit = NOUNS.iter().find_map(|&(singular, plural, gender)| {
            if n.lower == singular
                && (w.lower == ["der", "die", "das"][gender]
                    || w.lower == ["ein", "eine", "ein"][gender])
            {
                Some(false)
            } else if n.lower == plural && w.lower == "die" {
                Some(true)
            } else {
                None
            }
        });
        if let Some(plural) = explicit {
            (i + 1, plural)
        } else if NOUNS.iter().any(|&(_, plural, _)| w.lower == plural) {
            // German indefinite plural subjects do not need an article.
            (i, true)
        } else {
            return;
        }
    } else {
        return;
    };
    if !chars[ws[head].start].is_uppercase() {
        return;
    }
    let Some(verb) = ws.get(head + 1).filter(|v| adjacent(chars, &ws[head], v)) else {
        return;
    };
    if chars[verb.start].is_uppercase() || later_subject(chars, ws, head + 1) {
        return;
    }
    let Some(row) = VERBS.iter().find(|r| r.contains(&verb.lower.as_str())) else {
        return;
    };
    let p = if plural { 5 } else { 2 };
    // Present third-person subjunctive I shares the first-person indicative form.
    let subjunctive = !plural
        && row[0] == verb.lower
        && !matches!(
            row[0],
            "bin" | "war" | "hatte" | "wurde" | "kann" | "muss" | "darf" | "soll" | "will" | "weiß"
        );
    if row[p] != verb.lower && !subjunctive {
        emit(
            out,
            enabled,
            "GermanNounVerbAgreement",
            chars,
            Span::new(verb.start, verb.end),
            row[p],
            "Das finite Verb muss mit dem Subjekt übereinstimmen.",
        );
    }
}

const PARTICIPLES: &[(&str, &str)] = &[
    ("machen", "gemacht"),
    ("arbeiten", "gearbeitet"),
    ("schreiben", "geschrieben"),
    ("lesen", "gelesen"),
    ("sehen", "gesehen"),
    ("finden", "gefunden"),
    ("nehmen", "genommen"),
    ("geben", "gegeben"),
    ("sprechen", "gesprochen"),
    ("prüfen", "geprüft"),
    ("öffnen", "geöffnet"),
    ("speichern", "gespeichert"),
    ("installieren", "installiert"),
    ("konfigurieren", "konfiguriert"),
    ("verwenden", "verwendet"),
    ("nutzen", "genutzt"),
    ("ändern", "geändert"),
    ("löschen", "gelöscht"),
    ("erstellen", "erstellt"),
    ("starten", "gestartet"),
    ("beenden", "beendet"),
    ("warten", "gewartet"),
    ("lernen", "gelernt"),
    ("kaufen", "gekauft"),
    ("bezahlen", "bezahlt"),
    ("erhalten", "erhalten"),
];

pub fn lints(
    _sp: &dyn LangSpeller,
    chars: &[char],
    enabled: &[&str],
) -> BTreeMap<String, Vec<Lint>> {
    let ws = words(chars);
    let mut out = BTreeMap::new();
    for (i, w) in ws.iter().enumerate() {
        if !w.plain {
            continue;
        }
        noun_agreement(&mut out, enabled, chars, &ws, i);
        let Some(next) = ws.get(i + 1).filter(|n| adjacent(chars, w, n)) else {
            continue;
        };
        if let Some(p) = person(&w.lower).filter(|_| clause_start(chars, &ws, i))
            && let Some(row) = VERBS.iter().find(|row| row.contains(&next.lower.as_str()))
        {
            // Third-person subjunctive I (er habe/komme/werde) is not an error.
            let subjunctive = p == 2
                && row[0] == next.lower
                && !matches!(
                    row[0],
                    "bin"
                        | "war"
                        | "hatte"
                        | "wurde"
                        | "kann"
                        | "muss"
                        | "darf"
                        | "soll"
                        | "will"
                        | "weiß"
                );
            // Initial `es` can introduce a following plural subject; `sie` can be plural or formal.
            let ambiguous_third = matches!(w.lower.as_str(), "sie" | "es") && next.lower == row[5];
            if row[p] != next.lower
                && !ambiguous_third
                && !subjunctive
                && !chars[next.start].is_uppercase()
            {
                emit(
                    &mut out,
                    enabled,
                    "GermanPronounVerbAgreement",
                    chars,
                    Span::new(next.start, next.end),
                    row[p],
                    "Das finite Verb muss mit dem Subjekt übereinstimmen.",
                );
            }
        }
        if let Some(gender) = noun(&next.lower).filter(|_| chars[next.start].is_uppercase()) {
            let definite = matches!(
                w.lower.as_str(),
                "der" | "die" | "das" | "den" | "dem" | "des"
            );
            let indefinite = matches!(
                w.lower.as_str(),
                "ein" | "eine" | "einen" | "einem" | "einer" | "eines"
            );
            if definite || indefinite {
                let prep = i
                    .checked_sub(1)
                    .and_then(|p| ws.get(p))
                    .filter(|p| adjacent(chars, p, w));
                let case = prep
                    .and_then(|p| match p.lower.as_str() {
                        "aus" | "bei" | "mit" | "nach" | "seit" | "von" | "zu" | "gegenüber" => {
                            Some(1)
                        }
                        "durch" | "für" | "gegen" | "ohne" | "um" => Some(2),
                        _ => None,
                    })
                    .or_else(|| {
                        // Initial objects can invert with the verb: `Der Datei ist ein
                        // Name zugeordnet`, `Den Bericht habe ich gelesen`. Do not
                        // reinterpret an article that is valid in any case for this noun.
                        let compatible = match gender {
                            0 => matches!(
                                w.lower.as_str(),
                                "der" | "den" | "dem" | "des" | "ein" | "einen" | "einem" | "eines"
                            ),
                            1 => matches!(w.lower.as_str(), "die" | "der" | "eine" | "einer"),
                            2 => matches!(
                                w.lower.as_str(),
                                "das" | "dem" | "des" | "ein" | "einem" | "eines"
                            ),
                            _ => matches!(w.lower.as_str(), "die" | "der" | "den"),
                        };
                        if !compatible
                            && clause_start(chars, &ws, i)
                            && ws.get(i + 2).is_some_and(|v| {
                                adjacent(chars, next, v)
                                    && VERBS.iter().any(|r| r.contains(&v.lower.as_str()))
                            })
                        {
                            Some(0)
                        } else {
                            None
                        }
                    });
                if let Some(case) = case {
                    let forms = if definite {
                        [
                            ["der", "die", "das", "die"],
                            ["dem", "der", "dem", "den"],
                            ["den", "die", "das", "die"],
                        ]
                    } else {
                        [
                            ["ein", "eine", "ein", ""],
                            ["einem", "einer", "einem", ""],
                            ["einen", "eine", "ein", ""],
                        ]
                    };
                    let fix = forms[case][gender];
                    if !fix.is_empty() && fix != w.lower {
                        emit(
                            &mut out,
                            enabled,
                            "GermanArticleAgreement",
                            chars,
                            Span::new(w.start, w.end),
                            fix,
                            "Der Artikel muss zu Genus, Numerus und Kasus des Nomens passen.",
                        );
                    }
                }
            }
        }
        if matches!(
            w.lower.as_str(),
            "habe" | "hast" | "hat" | "haben" | "habt" | "hatte" | "hattest" | "hatten" | "hattet"
        ) && let Some(&(_, participle)) = PARTICIPLES
            .iter()
            .find(|&&(inf, part)| inf == next.lower && inf != part)
        {
            let mut replacement_infinitive = false;
            for j in i + 2..ws.len().min(i + 8) {
                if !adjacent(chars, &ws[j - 1], &ws[j]) {
                    break;
                }
                if matches!(
                    ws[j].lower.as_str(),
                    "müssen"
                        | "können"
                        | "dürfen"
                        | "sollen"
                        | "wollen"
                        | "lassen"
                        | "sehen"
                        | "hören"
                        | "helfen"
                        | "gelernt"
                        | "gelehrt"
                        | "geübt"
                        | "gekonnt"
                        | "gemusst"
                        | "gedurft"
                ) {
                    replacement_infinitive = true;
                    break;
                }
            }
            if !replacement_infinitive && !chars[next.start].is_uppercase() {
                emit(
                    &mut out,
                    enabled,
                    "GermanAuxiliaryParticiple",
                    chars,
                    Span::new(next.start, next.end),
                    participle,
                    "Nach haben steht hier das Partizip II.",
                );
            }
        }
        if matches!(w.lower.as_str(), "dass" | "weil" | "ob" | "obwohl" | "wenn")
            && person(&next.lower).is_some()
            && let (Some(verb), Some(pred)) = (ws.get(i + 2), ws.get(i + 3))
        {
            let terminal = chars[pred.end..]
                .iter()
                .find(|c| !c.is_whitespace())
                .is_none_or(|c| matches!(c, '.' | '!' | '?' | ',' | ';'));
            if adjacent(chars, next, verb)
                && adjacent(chars, verb, pred)
                && terminal
                && matches!(
                    verb.lower.as_str(),
                    "bin" | "bist" | "ist" | "sind" | "seid" | "war" | "warst" | "waren" | "wart"
                )
                && matches!(
                    pred.lower.as_str(),
                    "bereit"
                        | "gültig"
                        | "wichtig"
                        | "möglich"
                        | "notwendig"
                        | "verfügbar"
                        | "sicher"
                        | "richtig"
                        | "falsch"
                        | "fertig"
                        | "zufrieden"
                        | "krank"
                        | "müde"
                )
            {
                let replacement = format!("{} {}", pred.lower, verb.lower);
                emit(
                    &mut out,
                    enabled,
                    "GermanSubordinateWordOrder",
                    chars,
                    Span::new(verb.start, pred.end),
                    &replacement,
                    "Im Nebensatz steht das finite Verb nach dem Prädikat.",
                );
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Speller;
    impl LangSpeller for Speller {
        fn check(&self, _: &str) -> bool {
            true
        }
        fn suggest(&self, _: &str) -> Vec<String> {
            Vec::new()
        }
    }
    fn fixes(rule: &str, text: &str) -> Vec<(String, String)> {
        let chars: Vec<char> = text.chars().collect();
        lints(&Speller, &chars, &[rule])
            .into_values()
            .flatten()
            .map(|l| {
                let Suggestion::ReplaceWith(fix) = &l.suggestions[0] else {
                    panic!("replacement required")
                };
                (
                    chars[l.span.start..l.span.end].iter().collect(),
                    fix.iter().collect(),
                )
            })
            .collect()
    }
    #[test]
    fn known_noun_subjects_preserve_tense_and_character_spans() {
        for (text, start, end, replacement) in [
            ("Die Änderung sind gültig.", 13, 17, "ist"),
            ("Ein Gerät haben funktioniert.", 10, 15, "hat"),
            ("Eine Verbindung waren aktiv.", 16, 21, "war"),
            ("Die Kinder war müde.", 11, 14, "waren"),
            ("Berichte wird geprüft.", 9, 13, "werden"),
            ("Ich weiß, dass die Systeme ist bereit.", 27, 30, "sind"),
        ] {
            let chars: Vec<char> = text.chars().collect();
            let actual: Vec<_> = lints(&Speller, &chars, &["GermanNounVerbAgreement"])
                .into_values()
                .flatten()
                .map(|lint| {
                    let Suggestion::ReplaceWith(fix) = &lint.suggestions[0] else {
                        panic!("replacement required")
                    };
                    (lint.span, fix.iter().collect::<String>())
                })
                .collect();
            assert_eq!(
                actual,
                vec![(Span::new(start, end), replacement.to_string())],
                "{text}"
            );
        }
    }

    #[test]
    fn noun_subjects_leave_case_inversion_and_coordination_untouched() {
        for text in [
            "Der Dienst ist verfügbar.",
            "Eine Anwendung war bereit.",
            "Die Geräte werden geprüft.",
            "Dateien sind vorhanden.",
            "Die Datei haben wir geöffnet.",
            "Das Buch liest die Frau.",
            "Die Seiten liest der Benutzer.",
            "Die Fragen hat der Lehrer beantwortet.",
            "Den Bericht haben die Kinder gelesen.",
            "Der Datei sind Namen zugeordnet.",
            "Mit der Datei ist alles in Ordnung.",
            "Für die Datei sind mehrere Schritte nötig.",
            "Die Datei und das Dokument sind bereit.",
            "Der Bericht, die Nachricht sind fertig.",
            "Ist die Datei bereit?",
            "Heute sind die Geräte verfügbar.",
            "Der Dienst komme morgen, sagte er.",
            "Eine Anwendung habe Zugriff, heißt es.",
            "Es sind Dateien vorhanden. Sie sind bereit.",
            "Die Fenster sind offen.",
            "„Die Datei sind bereit“ ist ein Beispiel.",
            "`Die Geräte ist bereit`",
            "Die_Datei sind verfügbar.",
        ] {
            assert!(fixes("GermanNounVerbAgreement", text).is_empty(), "{text}");
        }
    }

    #[test]
    fn pronouns_preserve_tense_and_subjunctive() {
        assert_eq!(
            fixes(
                "GermanPronounVerbAgreement",
                "Ich bist bereit. Wir hat Zeit. Du waren müde."
            ),
            vec![
                ("bist".into(), "bin".into()),
                ("hat".into(), "haben".into()),
                ("waren".into(), "warst".into())
            ]
        );
        for text in [
            "Sie ist bereit. Sie sind bereit.",
            "Es sind drei Lösungen vorhanden.",
            "Es werden viele Aufgaben bearbeitet.",
            "Es können mehrere Personen teilnehmen.",
            "Er sagt, dass er habe arbeiten müssen.",
            "Heute bist du müde.",
            "Du und ich sind bereit.",
            "Er komme morgen, sagte sie.",
            "„Du bist“ ist korrekt.",
            "`ich bist`",
            "ich_bist = true",
        ] {
            assert!(
                fixes("GermanPronounVerbAgreement", text).is_empty(),
                "{text}"
            );
        }
    }
    #[test]
    fn articles_use_lexical_gender_and_governed_case() {
        assert_eq!(
            fixes(
                "GermanArticleAgreement",
                "Mit die Datei und für der Bericht."
            ),
            vec![("die".into(), "der".into()), ("der".into(), "den".into())]
        );
        assert_eq!(
            fixes(
                "GermanArticleAgreement",
                "Das Datei ist verfügbar. Eine System ist bereit."
            ),
            vec![("Das".into(), "Die".into()), ("Eine".into(), "Ein".into())]
        );
        for text in [
            "Mit der Datei und für den Bericht.",
            "Die neuen Systeme sind verfügbar.",
            "Die Fenster sind offen.",
            "In der Anwendung steht der Bericht.",
            "Das Mädchen liest ein Buch.",
            "Der Datei fehlt ein Name.",
            "Der Datei ist ein Name zugeordnet.",
            "Den Bericht habe ich gelesen.",
        ] {
            assert!(fixes("GermanArticleAgreement", text).is_empty(), "{text}");
        }
    }
    #[test]
    fn auxiliary_respects_replacement_infinitives() {
        assert_eq!(
            fixes("GermanAuxiliaryParticiple", "Er hat installieren."),
            vec![("installieren".into(), "installiert".into())]
        );
        for text in [
            "Er hat arbeiten müssen.",
            "Sie hat ihn arbeiten lassen.",
            "Wir haben Lesen und Schreiben geübt.",
            "Er hat lesen gelernt.",
            "Er ist arbeiten.",
            "Sie hat geschrieben.",
        ] {
            assert!(
                fixes("GermanAuxiliaryParticiple", text).is_empty(),
                "{text}"
            );
        }
    }
    #[test]
    fn short_subordinate_copula_is_final() {
        assert_eq!(
            fixes("GermanSubordinateWordOrder", "Ich weiß, dass er ist müde."),
            vec![("ist müde".into(), "müde ist".into())]
        );
        for text in [
            "Ich weiß, dass er müde ist.",
            "Wenn er ist, was er behauptet, hilft er.",
            "Er sagt, dass sie bereit ist und wartet.",
            "Weil er ist wirklich müde.",
            "„weil er ist müde“ ist ein Beispiel.",
        ] {
            assert!(
                fixes("GermanSubordinateWordOrder", text).is_empty(),
                "{text}"
            );
        }
    }
}
