//! French agreement and formal constructions supported by explicit lexical tables.
use std::collections::BTreeMap;

use super::lint::{Lint, LintKind, Span, Suggestion};
use super::spell_lang::{LangSpeller, quotations, tokens};
use unicode_normalization::{UnicodeNormalization, char::is_combining_mark};

pub const RULES: &[(&str, &str)] = &[
    (
        "FrenchPronounVerbAgreement",
        "French: personal subject pronouns agree with finite verbs",
    ),
    (
        "FrenchNounVerbAgreement",
        "French: clear known noun subjects agree with their finite verbs",
    ),
    (
        "FrenchArticleAgreement",
        "French: determiners agree with known nouns in gender and number",
    ),
    (
        "FrenchAdjectiveAgreement",
        "French: known attributive and predicative adjectives agree with their nouns",
    ),
    (
        "FrenchAuxiliaryParticiple",
        "French: avoir takes a past participle rather than an infinitive",
    ),
    (
        "FrenchFormalSubjunctive",
        "French: fixed necessity and purpose constructions require the subjunctive",
    ),
    (
        "FrenchPrepositionAccent",
        "French: aller takes à before a clear determiner-led destination",
    ),
    (
        "FrenchPrepositionContraction",
        "French: à and de contract with le and les before clear known noun phrases",
    ),
];

struct Word {
    start: usize,
    end: usize,
    lower: String,
    plain: bool,
}

fn elided(s: &str) -> bool {
    matches!(s, "j" | "n" | "l" | "d" | "qu" | "s" | "m" | "t" | "c")
}

fn words(chars: &[char]) -> Vec<Word> {
    let ts = tokens(chars);
    let mut quotes = quotations(chars, &ts);
    let mut open = None;
    for (i, &c) in chars.iter().enumerate() {
        if let Some((start, closing)) = open {
            if c == closing {
                quotes.push(start..i + 1);
                open = None;
            }
        } else if matches!(c, '"' | '«' | '“' | '`') {
            open = Some((
                i,
                match c {
                    '«' => '»',
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
    let mut ranges = Vec::with_capacity(ts.len());
    for (start, end) in ts {
        let mut s = start;
        // Split only grammatical proclitics, not `aujourd'hui`, `quelqu'un` or hyphenated inversions.
        while let Some(a) = (s..end).find(|&i| matches!(chars[i], '\'' | '’')) {
            let prefix: String = chars[s..a].iter().flat_map(|c| c.to_lowercase()).collect();
            if !elided(&prefix) {
                break;
            }
            ranges.push((s, a));
            s = a + 1;
        }
        if s < end {
            ranges.push((s, end));
        }
    }
    let mut q = 0;
    ranges
        .into_iter()
        .map(|(start, end)| {
            while q < quotes.len() && quotes[q].end <= start {
                q += 1;
            }
            let quoted = quotes
                .get(q)
                .is_some_and(|r| r.start < end && start < r.end);
            let slice = &chars[start..end];
            let plain = !quoted
                && slice
                    .iter()
                    .all(|c| c.is_alphabetic() || is_combining_mark(*c))
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
                lower: slice.iter().flat_map(|c| c.to_lowercase()).nfc().collect(),
                plain,
            }
        })
        .collect()
}

fn adjacent(chars: &[char], a: &Word, b: &Word) -> bool {
    if !a.plain || !b.plain || a.end >= b.start {
        return false;
    }
    let gap = &chars[a.end..b.start];
    (gap.iter().all(|c| c.is_whitespace()) && gap.iter().filter(|&&c| c == '\n').count() < 2)
        || (gap.len() == 1 && matches!(gap[0], '\'' | '’') && elided(&a.lower))
}

fn clause_start(chars: &[char], ws: &[Word], i: usize) -> bool {
    if i == 0 {
        return chars[..ws[i].start].iter().all(|c| c.is_whitespace());
    }
    let previous = &ws[i - 1];
    chars[previous.end..ws[i].start]
        .iter()
        .any(|c| matches!(c, '.' | '!' | '?' | ';' | ':' | ','))
        || (adjacent(chars, previous, &ws[i])
            && matches!(
                previous.lower.as_str(),
                "que" | "qu" | "si" | "quand" | "lorsque" | "puisque"
            ))
}

fn emit(
    out: &mut BTreeMap<String, Vec<Lint>>,
    enabled: &[&str],
    rule: &str,
    chars: &[char],
    span: Span,
    replacements: &[&str],
    message: &str,
) {
    if !enabled.contains(&rule) {
        return;
    }
    let capitalized = chars[span.start].is_uppercase();
    let suggestions = replacements
        .iter()
        .map(|replacement| {
            let mut fix: Vec<char> = replacement.chars().collect();
            if capitalized && !fix.is_empty() {
                let first = fix[0].to_uppercase();
                fix.splice(0..1, first);
            }
            Suggestion::ReplaceWith(fix)
        })
        .collect();
    out.entry(rule.to_string()).or_default().push(Lint {
        span,
        lint_kind: LintKind::Grammar,
        suggestions,
        message: message.to_string(),
        priority: 31,
    });
}

// Person order: je, tu, il/elle/on, nous, vous, ils/elles. The subjunctive
// accompanies each present-tense row; spelling syncretism is explicitly preserved.
struct Verb {
    indicative: [&'static str; 6],
    subjunctive: [&'static str; 6],
}
const VERBS: &[Verb] = &[
    Verb {
        indicative: ["suis", "es", "est", "sommes", "êtes", "sont"],
        subjunctive: ["sois", "sois", "soit", "soyons", "soyez", "soient"],
    },
    Verb {
        indicative: ["suis", "suis", "suit", "suivons", "suivez", "suivent"],
        subjunctive: ["suive", "suives", "suive", "suivions", "suiviez", "suivent"],
    },
    Verb {
        indicative: ["ai", "as", "a", "avons", "avez", "ont"],
        subjunctive: ["aie", "aies", "ait", "ayons", "ayez", "aient"],
    },
    Verb {
        indicative: ["vais", "vas", "va", "allons", "allez", "vont"],
        subjunctive: ["aille", "ailles", "aille", "allions", "alliez", "aillent"],
    },
    Verb {
        indicative: ["fais", "fais", "fait", "faisons", "faites", "font"],
        subjunctive: ["fasse", "fasses", "fasse", "fassions", "fassiez", "fassent"],
    },
    Verb {
        indicative: ["peux", "peux", "peut", "pouvons", "pouvez", "peuvent"],
        subjunctive: [
            "puisse",
            "puisses",
            "puisse",
            "puissions",
            "puissiez",
            "puissent",
        ],
    },
    Verb {
        indicative: ["veux", "veux", "veut", "voulons", "voulez", "veulent"],
        subjunctive: [
            "veuille",
            "veuilles",
            "veuille",
            "voulions",
            "vouliez",
            "veuillent",
        ],
    },
    Verb {
        indicative: ["dois", "dois", "doit", "devons", "devez", "doivent"],
        subjunctive: ["doive", "doives", "doive", "devions", "deviez", "doivent"],
    },
    Verb {
        indicative: ["sais", "sais", "sait", "savons", "savez", "savent"],
        subjunctive: ["sache", "saches", "sache", "sachions", "sachiez", "sachent"],
    },
    Verb {
        indicative: ["viens", "viens", "vient", "venons", "venez", "viennent"],
        subjunctive: [
            "vienne", "viennes", "vienne", "venions", "veniez", "viennent",
        ],
    },
    Verb {
        indicative: ["prends", "prends", "prend", "prenons", "prenez", "prennent"],
        subjunctive: [
            "prenne", "prennes", "prenne", "prenions", "preniez", "prennent",
        ],
    },
    Verb {
        indicative: ["lis", "lis", "lit", "lisons", "lisez", "lisent"],
        subjunctive: ["lise", "lises", "lise", "lisions", "lisiez", "lisent"],
    },
    Verb {
        indicative: ["écris", "écris", "écrit", "écrivons", "écrivez", "écrivent"],
        subjunctive: [
            "écrive",
            "écrives",
            "écrive",
            "écrivions",
            "écriviez",
            "écrivent",
        ],
    },
    Verb {
        indicative: ["dis", "dis", "dit", "disons", "dites", "disent"],
        subjunctive: ["dise", "dises", "dise", "disions", "disiez", "disent"],
    },
    Verb {
        indicative: ["mets", "mets", "met", "mettons", "mettez", "mettent"],
        subjunctive: ["mette", "mettes", "mette", "mettions", "mettiez", "mettent"],
    },
    Verb {
        indicative: ["vois", "vois", "voit", "voyons", "voyez", "voient"],
        subjunctive: ["voie", "voies", "voie", "voyions", "voyiez", "voient"],
    },
    Verb {
        indicative: ["parle", "parles", "parle", "parlons", "parlez", "parlent"],
        subjunctive: ["parle", "parles", "parle", "parlions", "parliez", "parlent"],
    },
    Verb {
        indicative: [
            "travaille",
            "travailles",
            "travaille",
            "travaillons",
            "travaillez",
            "travaillent",
        ],
        subjunctive: [
            "travaille",
            "travailles",
            "travaille",
            "travaillions",
            "travailliez",
            "travaillent",
        ],
    },
    Verb {
        indicative: [
            "utilise",
            "utilises",
            "utilise",
            "utilisons",
            "utilisez",
            "utilisent",
        ],
        subjunctive: [
            "utilise",
            "utilises",
            "utilise",
            "utilisions",
            "utilisiez",
            "utilisent",
        ],
    },
    Verb {
        indicative: [
            "vérifie",
            "vérifies",
            "vérifie",
            "vérifions",
            "vérifiez",
            "vérifient",
        ],
        subjunctive: [
            "vérifie",
            "vérifies",
            "vérifie",
            "vérifiions",
            "vérifiiez",
            "vérifient",
        ],
    },
    Verb {
        indicative: [
            "enregistre",
            "enregistres",
            "enregistre",
            "enregistrons",
            "enregistrez",
            "enregistrent",
        ],
        subjunctive: [
            "enregistre",
            "enregistres",
            "enregistre",
            "enregistrions",
            "enregistriez",
            "enregistrent",
        ],
    },
];
const PAST: &[[&str; 6]] = &[
    ["étais", "étais", "était", "étions", "étiez", "étaient"],
    ["avais", "avais", "avait", "avions", "aviez", "avaient"],
    [
        "allais", "allais", "allait", "allions", "alliez", "allaient",
    ],
    [
        "faisais",
        "faisais",
        "faisait",
        "faisions",
        "faisiez",
        "faisaient",
    ],
    ["serai", "seras", "sera", "serons", "serez", "seront"],
    ["aurai", "auras", "aura", "aurons", "aurez", "auront"],
    [
        "serais", "serais", "serait", "serions", "seriez", "seraient",
    ],
    [
        "aurais", "aurais", "aurait", "aurions", "auriez", "auraient",
    ],
];
fn person(s: &str) -> Option<usize> {
    match s {
        "je" | "j" => Some(0),
        "tu" => Some(1),
        "il" | "elle" | "on" => Some(2),
        "nous" => Some(3),
        "vous" => Some(4),
        "ils" | "elles" => Some(5),
        _ => None,
    }
}

fn mood(chars: &[char], ws: &[Word], i: usize) -> bool {
    if i < 2 || !matches!(ws[i - 1].lower.as_str(), "que" | "qu") {
        return false;
    }
    if !adjacent(chars, &ws[i - 1], &ws[i]) || !adjacent(chars, &ws[i - 2], &ws[i - 1]) {
        return false;
    }
    if ws[i - 2].lower == "bien" {
        if let Some(previous) = i
            .checked_sub(3)
            .and_then(|p| ws.get(p))
            .filter(|p| adjacent(chars, p, &ws[i - 2]))
        {
            let before = previous.lower.as_str();
            let reporting = VERBS.iter().any(|v| {
                matches!(v.indicative[0], "sais" | "dis" | "vois") && v.indicative.contains(&before)
            });
            // Here `bien` can be an adverb or noun instead of meaning “although”.
            if reporting
                || ARTICLES.iter().any(|row| row.contains(&before))
                || matches!(
                    before,
                    "si" | "aussi"
                        | "plus"
                        | "moins"
                        | "du"
                        | "au"
                        | "pense"
                        | "penses"
                        | "pensons"
                        | "pensez"
                        | "pensent"
                        | "crois"
                        | "croit"
                        | "croyons"
                        | "croyez"
                        | "croient"
                        | "savais"
                        | "savait"
                        | "savions"
                        | "saviez"
                        | "savaient"
                )
            {
                return false;
            }
        }
        return true;
    }
    if matches!(ws[i - 2].lower.as_str(), "pour" | "afin") {
        return true;
    }
    if i >= 3
        && ws[i - 2].lower == "faut"
        && ws[i - 3].lower == "il"
        && adjacent(chars, &ws[i - 3], &ws[i - 2])
    {
        return true;
    }
    i >= 4
        && matches!(
            ws[i - 2].lower.as_str(),
            "nécessaire" | "important" | "indispensable" | "essentiel"
        )
        && ws[i - 3].lower == "est"
        && ws[i - 4].lower == "il"
        && adjacent(chars, &ws[i - 4], &ws[i - 3])
        && adjacent(chars, &ws[i - 3], &ws[i - 2])
}

// Homographs and common-gender nouns (`livre`, `poste`, `mode`, `mémoire`, `enfant`) are omitted.
const NOUNS: &[(&str, &str, bool)] = &[
    ("page", "pages", true),
    ("phrase", "phrases", true),
    ("réponse", "réponses", true),
    ("question", "questions", true),
    ("erreur", "erreurs", true),
    ("règle", "règles", true),
    ("clé", "clés", true),
    ("valeur", "valeurs", true),
    ("variable", "variables", true),
    ("option", "options", true),
    ("fonction", "fonctions", true),
    ("connexion", "connexions", true),
    ("application", "applications", true),
    ("commande", "commandes", true),
    ("version", "versions", true),
    ("adresse", "adresses", true),
    ("méthode", "méthodes", true),
    ("configuration", "configurations", true),
    ("donnée", "données", true),
    ("documentation", "documentations", true),
    ("maison", "maisons", true),
    ("voiture", "voitures", true),
    ("personne", "personnes", true),
    ("femme", "femmes", true),
    ("document", "documents", false),
    ("système", "systèmes", false),
    ("fichier", "fichiers", false),
    ("message", "messages", false),
    ("programme", "programmes", false),
    ("résultat", "résultats", false),
    ("exemple", "exemples", false),
    ("problème", "problèmes", false),
    ("compte", "comptes", false),
    ("service", "services", false),
    ("serveur", "serveurs", false),
    ("paramètre", "paramètres", false),
    ("utilisateur", "utilisateurs", false),
    ("projet", "projets", false),
    ("produit", "produits", false),
    ("rapport", "rapports", false),
    ("nom", "noms", false),
    ("travail", "travaux", false),
    ("homme", "hommes", false),
    ("jardin", "jardins", false),
];
fn noun_in(
    s: &str,
    table: &'static [(&'static str, &'static str, bool)],
) -> Option<(&'static str, &'static str, bool, bool)> {
    table.iter().find_map(|&(sing, plural, fem)| {
        if s == sing {
            Some((sing, plural, fem, false))
        } else if s == plural {
            Some((sing, plural, fem, true))
        } else {
            None
        }
    })
}
fn noun(s: &str) -> Option<(&'static str, &'static str, bool, bool)> {
    noun_in(s, NOUNS)
}

// Also shared by the motion-accent rule; do not infer unknown destination nouns.
const DESTINATION_NOUNS: &[(&str, &str, bool)] = &[
    ("maison", "maisons", true),
    ("école", "écoles", true),
    ("gare", "gares", true),
    ("plage", "plages", true),
    ("bibliothèque", "bibliothèques", true),
    ("université", "universités", true),
    ("piscine", "piscines", true),
    ("banque", "banques", true),
    ("jardin", "jardins", false),
    ("hôpital", "hôpitaux", false),
    ("bureau", "bureaux", false),
    ("musée", "musées", false),
    ("église", "églises", true),
    ("page", "pages", true),
    ("section", "sections", true),
    ("étape", "étapes", true),
];

fn destination_noun(s: &str) -> Option<(&'static str, &'static str, bool, bool)> {
    noun_in(s, DESTINATION_NOUNS)
}
const ADJECTIVES: &[[&str; 4]] = &[
    ["nouveau", "nouvelle", "nouveaux", "nouvelles"],
    ["ancien", "ancienne", "anciens", "anciennes"],
    ["grand", "grande", "grands", "grandes"],
    ["petit", "petite", "petits", "petites"],
    ["bon", "bonne", "bons", "bonnes"],
    ["premier", "première", "premiers", "premières"],
    ["dernier", "dernière", "derniers", "dernières"],
    ["important", "importante", "importants", "importantes"],
    ["disponible", "disponible", "disponibles", "disponibles"],
    ["correct", "correcte", "corrects", "correctes"],
    ["complet", "complète", "complets", "complètes"],
    ["actif", "active", "actifs", "actives"],
    ["public", "publique", "publics", "publiques"],
    ["privé", "privée", "privés", "privées"],
    ["prêt", "prête", "prêts", "prêtes"],
    ["valide", "valide", "valides", "valides"],
    ["utile", "utile", "utiles", "utiles"],
    ["simple", "simple", "simples", "simples"],
    ["nécessaire", "nécessaire", "nécessaires", "nécessaires"],
    ["obligatoire", "obligatoire", "obligatoires", "obligatoires"],
    ["créé", "créée", "créés", "créées"],
    ["supprimé", "supprimée", "supprimés", "supprimées"],
    ["enregistré", "enregistrée", "enregistrés", "enregistrées"],
    ["configuré", "configurée", "configurés", "configurées"],
    ["installé", "installée", "installés", "installées"],
    ["ouvert", "ouverte", "ouverts", "ouvertes"],
    ["fermé", "fermée", "fermés", "fermées"],
    ["heureux", "heureuse", "heureux", "heureuses"],
    ["content", "contente", "contents", "contentes"],
    ["fatigué", "fatiguée", "fatigués", "fatiguées"],
    ["arrivé", "arrivée", "arrivés", "arrivées"],
    ["parti", "partie", "partis", "parties"],
    ["allé", "allée", "allés", "allées"],
    ["venu", "venue", "venus", "venues"],
];
fn adjective(s: &str) -> Option<&'static [&'static str; 4]> {
    ADJECTIVES.iter().find(|r| r.contains(&s))
}
fn vowel(s: &str) -> bool {
    s.chars()
        .next()
        .is_some_and(|c| "aeiouyàâéèêëîïôùûüœh".contains(c))
}

// Determiner families retain their meaning; number is established by the determiner.
const ARTICLES: &[[&str; 3]] = &[
    ["le", "la", "les"],
    ["un", "une", "des"],
    ["ce", "cette", "ces"],
    ["mon", "ma", "mes"],
    ["ton", "ta", "tes"],
    ["son", "sa", "ses"],
    ["notre", "notre", "nos"],
    ["votre", "votre", "vos"],
    ["leur", "leur", "leurs"],
    ["quel", "quelle", "quels"],
];
const PARTICIPLES: &[(&str, &str)] = &[
    ("installer", "installé"),
    ("configurer", "configuré"),
    ("utiliser", "utilisé"),
    ("enregistrer", "enregistré"),
    ("supprimer", "supprimé"),
    ("créer", "créé"),
    ("modifier", "modifié"),
    ("vérifier", "vérifié"),
    ("ouvrir", "ouvert"),
    ("fermer", "fermé"),
    ("écrire", "écrit"),
    ("lire", "lu"),
    ("faire", "fait"),
    ("prendre", "pris"),
    ("voir", "vu"),
    ("comprendre", "compris"),
    ("recevoir", "reçu"),
    ("envoyer", "envoyé"),
    ("acheter", "acheté"),
    ("manger", "mangé"),
    ("travailler", "travaillé"),
];

fn clitic(s: &str) -> bool {
    matches!(
        s,
        "ne" | "n"
            | "me"
            | "m"
            | "te"
            | "t"
            | "se"
            | "s"
            | "le"
            | "la"
            | "l"
            | "les"
            | "lui"
            | "leur"
            | "nous"
            | "vous"
            | "en"
            | "y"
    )
}

fn auxiliary_subject(chars: &[char], ws: &[Word], i: usize) -> bool {
    for j in (i.saturating_sub(4)..i).rev() {
        if !adjacent(chars, &ws[j], &ws[j + 1]) {
            return false;
        }
        if person(&ws[j].lower).is_some() && clause_start(chars, ws, j) {
            return true;
        }
        if !clitic(&ws[j].lower) {
            return false;
        }
    }
    false
}

fn noun_verb_fix(s: &str, p: usize) -> Option<&'static str> {
    let mut fix = None;
    for row in VERBS
        .iter()
        .flat_map(|v| [&v.indicative, &v.subjunctive])
        .chain(PAST.iter())
    {
        if !row.contains(&s) {
            continue;
        }
        // A valid reading, or differing tense/mood readings, forbids a correction.
        // For example, `allions` can be imperfect or present subjunctive.
        if row[p] == s || fix.is_some_and(|previous| previous != row[p]) {
            return None;
        }
        fix = Some(row[p]);
    }
    fix
}

fn noun_agreement(
    out: &mut BTreeMap<String, Vec<Lint>>,
    enabled: &[&str],
    chars: &[char],
    ws: &[Word],
    i: usize,
) {
    if !enabled.contains(&"FrenchNounVerbAgreement")
        || !clause_start(chars, ws, i)
        || (i > 0 && chars[ws[i - 1].end..ws[i].start].contains(&','))
    {
        return;
    }
    let w = &ws[i];
    if !matches!(
        w.lower.as_str(),
        "le" | "la" | "les" | "un" | "une" | "des" | "l"
    ) {
        return;
    }
    let Some(next) = ws.get(i + 1).filter(|n| adjacent(chars, w, n)) else {
        return;
    };
    let mut n = i + 1;
    if adjective(&next.lower).is_some()
        && ws
            .get(n + 1)
            .is_some_and(|head| adjacent(chars, next, head) && noun(&head.lower).is_some())
    {
        n += 1;
    }
    let head = &ws[n];
    let Some((_, _, fem, plural)) = noun(&head.lower) else {
        return;
    };
    if !head.plain || chars[head.start].is_uppercase() {
        return;
    }
    let determiner = match w.lower.as_str() {
        "le" => !fem && !plural && !vowel(&next.lower),
        "la" => fem && !plural && !vowel(&next.lower),
        "les" | "des" => plural,
        "un" => !fem && !plural,
        "une" => fem && !plural,
        "l" => {
            !plural
                && vowel(&next.lower)
                && chars[w.end..next.start].len() == 1
                && matches!(chars[w.end], '\'' | '’')
        }
        _ => false,
    };
    if !determiner {
        return;
    }
    let mut v = n + 1;
    while v < ws.len() && v <= n + 4 && adjacent(chars, &ws[v - 1], &ws[v]) && clitic(&ws[v].lower)
    {
        v += 1;
    }
    let Some(verb) = ws.get(v).filter(|verb| adjacent(chars, &ws[v - 1], verb)) else {
        return;
    };
    if chars[verb.start].is_uppercase() {
        return;
    }
    // Leave literary inversion and fronted objects alone when a later subject
    // is plausible, including an unknown name or an articulated noun phrase.
    for j in v + 1..ws.len() {
        if !adjacent(chars, &ws[j - 1], &ws[j]) {
            if chars[ws[j - 1].end..ws[j].start]
                .iter()
                .all(|c| c.is_whitespace())
            {
                // An unsplit apostrophe form may be an indefinite subject.
                return;
            }
            break;
        }
        if person(&ws[j].lower).is_some()
            || chars[ws[j].start].is_uppercase()
            || ARTICLES
                .iter()
                .any(|row| row.contains(&ws[j].lower.as_str()))
            || ws[j].lower == "l"
            || matches!(
                ws[j].lower.as_str(),
                "ce" | "cela"
                    | "ça"
                    | "celui"
                    | "celle"
                    | "ceux"
                    | "celles"
                    | "personne"
                    | "rien"
                    | "tout"
                    | "tous"
                    | "toutes"
                    | "aucun"
                    | "aucune"
            )
        {
            return;
        }
    }
    if let Some(fix) = noun_verb_fix(&verb.lower, if plural { 5 } else { 2 }) {
        emit(
            out,
            enabled,
            "FrenchNounVerbAgreement",
            chars,
            Span::new(verb.start, verb.end),
            &[fix],
            "Le verbe s’accorde en nombre avec le nom sujet.",
        );
    }
}

fn agree_adjective(
    out: &mut BTreeMap<String, Vec<Lint>>,
    enabled: &[&str],
    chars: &[char],
    ws: &[Word],
    i: usize,
    fem: bool,
    plural: bool,
) {
    let w = &ws[i];
    if ws.get(i + 1).is_some_and(|n| {
        adjacent(chars, w, n)
            && ((w.lower == "bon" && matches!(n.lower.as_str(), "marché" | "enfant"))
                || (w.lower == "grand"
                    && matches!(n.lower.as_str(), "public" | "format" | "standing")))
    }) {
        return;
    }
    if let Some(row) = adjective(&w.lower) {
        let fix = row[usize::from(fem) + 2 * usize::from(plural)];
        if w.lower != fix {
            emit(
                out,
                enabled,
                "FrenchAdjectiveAgreement",
                chars,
                Span::new(w.start, w.end),
                &[fix],
                "L’adjectif s’accorde en genre et en nombre avec le nom.",
            );
        }
    }
}

fn motion_form(chars: &[char], ws: &[Word], i: usize) -> bool {
    let w = &ws[i];
    let previous = i
        .checked_sub(1)
        .and_then(|j| ws.get(j))
        .filter(|p| adjacent(chars, p, w));
    match w.lower.as_str() {
        "aller" => {
            clause_start(chars, ws, i)
                || previous.is_some_and(|p| {
                    matches!(
                        p.lower.as_str(),
                        "pour" | "sans" | "de" | "d" | "faut" | "fallait"
                    ) || VERBS.iter().any(|row| {
                        matches!(row.indicative[0], "vais" | "peux" | "veux" | "dois")
                            && (row.indicative.contains(&p.lower.as_str())
                                || row.subjunctive.contains(&p.lower.as_str()))
                    })
                })
        }
        "allé" | "allée" | "allés" | "allées" => previous.is_some_and(|p| {
            matches!(
                p.lower.as_str(),
                "être"
                    | "été"
                    | "suis"
                    | "es"
                    | "est"
                    | "sommes"
                    | "êtes"
                    | "sont"
                    | "étais"
                    | "était"
                    | "étions"
                    | "étiez"
                    | "étaient"
                    | "serai"
                    | "seras"
                    | "sera"
                    | "serons"
                    | "serez"
                    | "seront"
                    | "serais"
                    | "serait"
                    | "serions"
                    | "seriez"
                    | "seraient"
                    | "sois"
                    | "soit"
                    | "soyons"
                    | "soyez"
                    | "soient"
                    | "fus"
                    | "fut"
                    | "fûmes"
                    | "fûtes"
                    | "furent"
            )
        }),
        "allant" => clause_start(chars, ws, i) || previous.is_some_and(|p| p.lower == "en"),
        _ => {
            VERBS.iter().any(|row| {
                row.indicative[0] == "vais"
                    && (row.indicative.contains(&w.lower.as_str())
                        || row.subjunctive.contains(&w.lower.as_str()))
            }) || matches!(
                w.lower.as_str(),
                "allais"
                    | "allait"
                    | "allions"
                    | "alliez"
                    | "allaient"
                    | "irai"
                    | "iras"
                    | "ira"
                    | "irons"
                    | "irez"
                    | "iront"
                    | "irais"
                    | "irait"
                    | "irions"
                    | "iriez"
                    | "iraient"
                    | "allai"
                    | "allas"
                    | "alla"
                    | "allâmes"
                    | "allâtes"
                    | "allèrent"
                    | "allasse"
                    | "allasses"
                    | "allât"
                    | "allassions"
                    | "allassiez"
                    | "allassent"
            )
        }
    }
}

fn preposition_accent(
    out: &mut BTreeMap<String, Vec<Lint>>,
    enabled: &[&str],
    chars: &[char],
    ws: &[Word],
    i: usize,
) {
    if !enabled.contains(&"FrenchPrepositionAccent")
        || !ws
            .get(i + 1)
            .is_some_and(|a| a.lower == "a" && adjacent(chars, &ws[i], a))
        || !motion_form(chars, ws, i)
    {
        return;
    }
    let a = &ws[i + 1];
    let Some(det) = ws.get(i + 2).filter(|d| adjacent(chars, a, d)) else {
        return;
    };
    // `le` and `les` require contractions, not merely an accent correction.
    if !matches!(
        det.lower.as_str(),
        "la" | "l"
            | "un"
            | "une"
            | "ce"
            | "cet"
            | "cette"
            | "ces"
            | "mon"
            | "ma"
            | "mes"
            | "ton"
            | "ta"
            | "tes"
            | "son"
            | "sa"
            | "ses"
            | "notre"
            | "nos"
            | "votre"
            | "vos"
            | "leur"
            | "leurs"
    ) {
        return;
    }
    let Some(mut head) = ws.get(i + 3).filter(|h| adjacent(chars, det, h)) else {
        return;
    };
    if det.lower == "l" && !matches!(&chars[det.end..head.start], ['\'' | '’']) {
        return;
    }
    if adjective(&head.lower).is_some() {
        let Some(noun) = ws.get(i + 4).filter(|n| adjacent(chars, head, n)) else {
            return;
        };
        head = noun;
    }
    // Places and navigation targets only: no broad `a`/`à` correction or inferred noun gender.
    if destination_noun(&head.lower).is_none() {
        return;
    }
    emit(
        out,
        enabled,
        "FrenchPrepositionAccent",
        chars,
        Span::new(a.start, a.end),
        &["à"],
        "La destination après aller est introduite ici par à.",
    );
}

fn preposition_contraction(
    out: &mut BTreeMap<String, Vec<Lint>>,
    enabled: &[&str],
    chars: &[char],
    ws: &[Word],
    i: usize,
) {
    if !enabled.contains(&"FrenchPrepositionContraction") {
        return;
    }
    let prep = &ws[i];
    let is_a = match prep.lower.as_str() {
        "à" => true,
        "de" => false,
        "a" if i
            .checked_sub(1)
            .is_some_and(|p| adjacent(chars, &ws[p], prep) && motion_form(chars, ws, p)) =>
        {
            true
        }
        _ => return,
    };
    let Some(det) = ws.get(i + 1).filter(|d| adjacent(chars, prep, d)) else {
        return;
    };
    if !matches!(det.lower.as_str(), "le" | "les") {
        return;
    }
    let Some(next) = ws.get(i + 2).filter(|n| adjacent(chars, det, n)) else {
        return;
    };
    let mut n = i + 2;
    let adj = adjective(&next.lower);
    if adj.is_some() {
        if !ws.get(n + 1).is_some_and(|h| adjacent(chars, next, h)) {
            return;
        }
        n += 1;
    }
    let head = &ws[n];
    let Some((_, _, fem, plural)) = noun(&head.lower).or_else(|| destination_noun(&head.lower))
    else {
        return;
    };
    // Capitalized articles/nouns may belong to titles. Apostrophe fragments are
    // neither ordinary prepositions nor definite noun phrases.
    if ws[i..=n].iter().any(|w| {
        !w.plain
            || (w.start != prep.start && chars[w.start].is_uppercase())
            || w.start
                .checked_sub(1)
                .is_some_and(|p| matches!(chars[p], '\'' | '’'))
            || chars.get(w.end).is_some_and(|c| matches!(c, '\'' | '’'))
    }) {
        return;
    }
    // Leave article/elision and adjective errors to their existing diagnostics:
    // contracting their spans would otherwise offer competing edits.
    if (det.lower == "les") != plural
        || (det.lower == "le" && (fem || vowel(&next.lower)))
        || adj.is_some_and(|row| next.lower != row[usize::from(fem) + 2 * usize::from(plural)])
    {
        return;
    }
    let replacement = match (is_a, plural) {
        (true, false) => "au",
        (true, true) => "aux",
        (false, false) => "du",
        (false, true) => "des",
    };
    emit(
        out,
        enabled,
        "FrenchPrepositionContraction",
        chars,
        Span::new(prep.start, det.end),
        &[replacement],
        "La préposition et l’article défini se contractent devant ce nom.",
    );
}

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
        preposition_accent(&mut out, enabled, chars, &ws, i);
        preposition_contraction(&mut out, enabled, chars, &ws, i);
        if let Some(p) = person(&w.lower).filter(|_| clause_start(chars, &ws, i)) {
            let mut v = i + 1;
            while v < ws.len()
                && v <= i + 4
                && adjacent(chars, &ws[v - 1], &ws[v])
                && clitic(&ws[v].lower)
            {
                v += 1;
            }
            if let Some(verb) = ws.get(v).filter(|verb| adjacent(chars, &ws[v - 1], verb)) {
                let subj = mood(chars, &ws, i);
                if let Some(row) = VERBS.iter().find(|r| {
                    r.indicative.contains(&verb.lower.as_str())
                        || r.subjunctive.contains(&verb.lower.as_str())
                }) {
                    let correction = |r: &Verb| {
                        if subj {
                            r.subjunctive[p]
                        } else if r.indicative.contains(&verb.lower.as_str()) {
                            r.indicative[p]
                        } else {
                            r.subjunctive[p]
                        }
                    };
                    let fix = correction(row);
                    // allions/alliez may be indicative imperfect or subjunctive present.
                    let ambiguous = if subj {
                        VERBS.iter().any(|r| r.subjunctive[p] == verb.lower)
                    } else {
                        PAST.iter().any(|r| r[p] == verb.lower)
                            || VERBS.iter().any(|r| {
                                r.indicative[p] == verb.lower || r.subjunctive[p] == verb.lower
                            })
                    };
                    if fix != verb.lower && !ambiguous {
                        // A certain agreement/mood error may still have several verb readings.
                        let mut choices = [""; VERBS.len()];
                        let mut len = 0;
                        for candidate in VERBS.iter().filter(|r| {
                            r.indicative.contains(&verb.lower.as_str())
                                || r.subjunctive.contains(&verb.lower.as_str())
                        }) {
                            let replacement = correction(candidate);
                            if !choices[..len].contains(&replacement) {
                                choices[len] = replacement;
                                len += 1;
                            }
                        }
                        emit(
                            &mut out,
                            enabled,
                            if subj {
                                "FrenchFormalSubjunctive"
                            } else {
                                "FrenchPronounVerbAgreement"
                            },
                            chars,
                            Span::new(verb.start, verb.end),
                            &choices[..len],
                            if len > 1 {
                                "Le sens du verbe détermine la correction parmi ces formes."
                            } else if subj {
                                "Cette construction demande le subjonctif."
                            } else {
                                "Le verbe s’accorde avec le pronom sujet."
                            },
                        );
                    }
                } else if !subj
                    && let Some(row) = PAST.iter().find(|r| r.contains(&verb.lower.as_str()))
                    && row[p] != verb.lower
                {
                    emit(
                        &mut out,
                        enabled,
                        "FrenchPronounVerbAgreement",
                        chars,
                        Span::new(verb.start, verb.end),
                        &[row[p]],
                        "Le verbe s’accorde avec le pronom sujet.",
                    );
                }
                let gender = match w.lower.as_str() {
                    "il" | "ils" => Some(false),
                    "elle" | "elles" => Some(true),
                    _ => None,
                };
                if let Some(fem) = gender {
                    let plural = p == 5;
                    let copula = if plural {
                        matches!(
                            verb.lower.as_str(),
                            "sont" | "étaient" | "seront" | "soient"
                        )
                    } else {
                        matches!(verb.lower.as_str(), "est" | "était" | "sera" | "soit")
                    };
                    if copula && ws.get(v + 1).is_some_and(|a| adjacent(chars, verb, a)) {
                        agree_adjective(&mut out, enabled, chars, &ws, v + 1, fem, plural);
                    }
                }
            }
        }
        let Some(next) = ws.get(i + 1).filter(|n| adjacent(chars, w, n)) else {
            continue;
        };
        if auxiliary_subject(chars, &ws, i)
            && matches!(
                w.lower.as_str(),
                "ai" | "as"
                    | "a"
                    | "avons"
                    | "avez"
                    | "ont"
                    | "avais"
                    | "avait"
                    | "avions"
                    | "aviez"
                    | "avaient"
            )
            && let Some(&(_, part)) = PARTICIPLES.iter().find(|&&(inf, _)| inf == next.lower)
        {
            emit(
                &mut out,
                enabled,
                "FrenchAuxiliaryParticiple",
                chars,
                Span::new(next.start, next.end),
                &[part],
                "Après avoir, le passé composé emploie un participe passé.",
            );
        }
        let article = ARTICLES.iter().find(|r| {
            r.contains(&w.lower.as_str())
                || (r[0] == "ce" && w.lower == "cet")
                || (r[0] == "quel" && w.lower == "quelles")
        });
        if article.is_none() && w.lower != "l" {
            continue;
        }
        let mut n = i + 1;
        if adjective(&next.lower).is_some()
            && ws
                .get(i + 2)
                .is_some_and(|a| adjacent(chars, next, a) && noun(&a.lower).is_some())
        {
            n += 1;
        }
        let Some(head) = ws.get(n) else {
            continue;
        };
        let Some((singular, plural_form, fem, noun_plural)) = noun(&head.lower) else {
            continue;
        };
        if !head.plain || chars[head.start].is_uppercase() {
            continue;
        }
        // le/la/les/l'/leur can be preverbal object clitics. These nouns also
        // have finite-verb readings, so a noun phrase needs positive context.
        if n == i + 1
            && matches!(w.lower.as_str(), "le" | "la" | "les" | "l" | "leur")
            && matches!(
                head.lower.as_str(),
                "commande"
                    | "commandes"
                    | "compte"
                    | "comptes"
                    | "adresse"
                    | "adresses"
                    | "règle"
                    | "règles"
                    | "programme"
                    | "programmes"
                    | "produit"
                    | "produits"
                    | "page"
                    | "pages"
            )
        {
            let licensed = (clause_start(chars, &ws, i)
                && ws.get(n + 1).is_some_and(|v| {
                    adjacent(chars, head, v)
                        && (VERBS
                            .iter()
                            .any(|r| r.indicative.contains(&v.lower.as_str()))
                            || PAST.iter().any(|r| r.contains(&v.lower.as_str())))
                }))
                || i.checked_sub(1).is_some_and(|p| {
                    adjacent(chars, &ws[p], w)
                        && matches!(
                            ws[p].lower.as_str(),
                            "de" | "d"
                                | "à"
                                | "dans"
                                | "sur"
                                | "sous"
                                | "avec"
                                | "sans"
                                | "pour"
                                | "par"
                                | "contre"
                                | "entre"
                                | "vers"
                                | "après"
                                | "avant"
                                | "chez"
                                | "selon"
                        )
                });
            if !licensed {
                continue;
            }
        }
        if w.lower == "l" && noun_plural && matches!(&chars[w.end..next.start], ['\'' | '’']) {
            emit(
                &mut out,
                enabled,
                "FrenchArticleAgreement",
                chars,
                Span::new(w.start, next.start),
                &["les "],
                "L’article élidé est singulier ; ce nom exige les.",
            );
        }
        let expected_plural = article.is_some_and(|r| w.lower == r[2] || w.lower == "quelles");
        let plural = if w.lower == "l" {
            noun_plural
        } else {
            expected_plural
        };
        if let Some(row) = article {
            if expected_plural != noun_plural {
                emit(
                    &mut out,
                    enabled,
                    "FrenchArticleAgreement",
                    chars,
                    Span::new(head.start, head.end),
                    &[if expected_plural {
                        plural_form
                    } else {
                        singular
                    }],
                    "Le nom s’accorde en nombre avec le déterminant.",
                );
            }
            if !expected_plural {
                let mut fix = row[usize::from(fem)];
                // Feminine possessives use mon/ton/son before a vowel; cet is the
                // masculine demonstrative before a vowel. Known nouns here have mute h.
                if vowel(&next.lower) {
                    if matches!(row[0], "mon" | "ton" | "son") {
                        fix = row[0];
                    }
                    if row[0] == "ce" && !fem {
                        fix = "cet";
                    }
                }
                if row[0] == "le" && vowel(&next.lower) {
                    emit(
                        &mut out,
                        enabled,
                        "FrenchArticleAgreement",
                        chars,
                        Span::new(w.start, next.start),
                        &["l’"],
                        "L’article s’élide devant une voyelle ou un h muet.",
                    );
                } else if fix != w.lower {
                    emit(
                        &mut out,
                        enabled,
                        "FrenchArticleAgreement",
                        chars,
                        Span::new(w.start, w.end),
                        &[fix],
                        "Le déterminant s’accorde avec le genre du nom.",
                    );
                }
            } else if row[0] == "quel" {
                let fix = if fem { "quelles" } else { "quels" };
                if fix != w.lower {
                    emit(
                        &mut out,
                        enabled,
                        "FrenchArticleAgreement",
                        chars,
                        Span::new(w.start, w.end),
                        &[fix],
                        "Le déterminant s’accorde avec le genre du nom.",
                    );
                }
            }
        }
        if n > i + 1 {
            agree_adjective(&mut out, enabled, chars, &ws, i + 1, fem, plural);
        }
        if let Some(after) = ws.get(n + 1).filter(|a| adjacent(chars, head, a)) {
            agree_adjective(&mut out, enabled, chars, &ws, n + 1, fem, plural);
            let copula = if plural {
                matches!(after.lower.as_str(), "sont" | "étaient" | "seront")
            } else {
                matches!(after.lower.as_str(), "est" | "était" | "sera")
            };
            if copula
                && clause_start(chars, &ws, i)
                && ws.get(n + 2).is_some_and(|p| adjacent(chars, after, p))
            {
                agree_adjective(&mut out, enabled, chars, &ws, n + 2, fem, plural);
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
    fn known_noun_subjects_preserve_tense_mood_and_character_spans() {
        for (text, start, end, replacement) in [
            ("La réponse sont correcte.", 11, 15, "est"),
            ("Un serveur ont répondu.", 11, 14, "a"),
            ("Une personne étaient disponible.", 13, 20, "était"),
            ("Des fichiers sera disponibles.", 13, 17, "seront"),
            ("Les systèmes serait utiles.", 13, 19, "seraient"),
            ("L’application n’ont pas répondu.", 16, 19, "a"),
            ("Les personnes me parle.", 17, 22, "parlent"),
            (
                "Je souhaite que les femmes puisse venir.",
                27,
                33,
                "puissent",
            ),
            ("Une nouvelle règle soient utile.", 19, 25, "soit"),
        ] {
            let chars: Vec<char> = text.chars().collect();
            let actual: Vec<_> = lints(&Speller, &chars, &["FrenchNounVerbAgreement"])
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
    fn noun_subjects_leave_clitics_mood_and_inversion_untouched() {
        for text in [
            "Le serveur est disponible.",
            "Des personnes sont arrivées.",
            "Une réponse serait utile.",
            "Les systèmes étaient prêts.",
            "L’application n’a pas répondu.",
            "Les femmes lui parlent.",
            "Il faut que les personnes soient prêtes.",
            "Je souhaite que la femme puisse venir.",
            "La page et le fichier sont prêts.",
            "La page, le fichier sont prêts.",
            "La réponse est-elle correcte ?",
            "Sont disponibles les documents demandés.",
            "Les données utilise Marie.",
            "Les données ne lit personne.",
            "Les données utilise quelqu’un.",
            "Dans le jardin sont arrivées des personnes.",
            "Pour les fichiers sont nécessaires des clés.",
            "Il les compte et je la commande.",
            "Il les trie, les compte et les range.",
            "Il l’adresse au service.",
            "Elle leur commande une voiture.",
            "Une enfant joue.",
            "« La réponse sont correcte » est une erreur.",
            "`Des fichiers est prêt`",
            "La_réponse sont disponibles.",
        ] {
            assert!(fixes("FrenchNounVerbAgreement", text).is_empty(), "{text}");
        }
    }

    #[test]
    fn ambiguous_verb_readings_and_conflicting_articles_do_not_guess() {
        // These errors have competing replacements or an inconsistent noun phrase.
        for text in [
            "Les personnes allions à la maison.",
            "Les personnes alliez à la maison.",
            "Une personne suis le chemin.",
            "Les fichier sont disponibles.",
        ] {
            assert!(fixes("FrenchNounVerbAgreement", text).is_empty(), "{text}");
        }
    }

    #[test]
    fn personal_agreement_with_clitics_and_inversion() {
        assert_eq!(
            fixes(
                "FrenchPronounVerbAgreement",
                "Nous sont prêts. Je ne le sais pas. Tu avez raison. J’as compris."
            ),
            vec![
                ("sont".into(), "sommes".into()),
                ("avez".into(), "as".into()),
                ("as".into(), "ai".into())
            ]
        );
        for text in [
            "Sommes-nous prêts ?",
            "Je vous ai envoyé une réponse.",
            "Nous allions à la maison.",
            "Tu suis les instructions.",
            "Tu et moi sommes prêts.",
            "Il faut que nous soyons prêts.",
            "« Nous sont » est une erreur.",
            "`tu avez`",
            "tu_avez = false",
        ] {
            assert!(
                fixes("FrenchPronounVerbAgreement", text).is_empty(),
                "{text}"
            );
        }
    }
    #[test]
    fn articles_number_gender_and_elision() {
        assert_eq!(
            fixes(
                "FrenchArticleAgreement",
                "Une système et un erreur. Les fichier sont prêts."
            ),
            vec![
                ("Une".into(), "Un".into()),
                ("un".into(), "une".into()),
                ("fichier".into(), "fichiers".into())
            ]
        );
        assert_eq!(
            fixes("FrenchArticleAgreement", "Le application est disponible."),
            vec![("Le ".into(), "L’".into())]
        );
        for text in [
            "Mon application et ma nouvelle application.",
            "Cet utilisateur reçoit une réponse.",
            "Le nouveau programme et les nouvelles pages.",
            "La livre et le livre sont différents.",
            "Une enfant joue.",
            "L’application est disponible.",
            "Il les compte et je la commande.",
            "Je les programme.",
            "Il ne les règle pas.",
            "Il l’adresse au service.",
            "Elle leur commande une voiture.",
            "Il les trie, les compte et les range.",
        ] {
            assert!(fixes("FrenchArticleAgreement", text).is_empty(), "{text}");
        }
    }
    #[test]
    fn adjectives_use_lexical_gender_and_number() {
        assert_eq!(
            fixes(
                "FrenchAdjectiveAgreement",
                "Une nouvelle page est prêt. Les fichiers sont disponible. Une règle important."
            ),
            vec![
                ("prêt".into(), "prête".into()),
                ("disponible".into(), "disponibles".into()),
                ("important".into(), "importante".into())
            ]
        );
        assert_eq!(
            fixes(
                "FrenchAdjectiveAgreement",
                "Elle est arrivé. Ils sont partie."
            ),
            vec![
                ("arrivé".into(), "arrivée".into()),
                ("partie".into(), "partis".into())
            ]
        );
        for text in [
            "Les nouvelles applications sont disponibles.",
            "La page et le fichier sont prêts.",
            "Une page, importante pour nous, est disponible.",
            "La réponse à ces messages est importante.",
            "Une règle simple.",
            "Des personnes heureuses.",
            "Une voiture bon marché.",
            "Une application grand public.",
            "Elle est bon enfant.",
            "Vous êtes prête.",
            "Nous sommes arrivées.",
            "On est prêts.",
        ] {
            assert!(fixes("FrenchAdjectiveAgreement", text).is_empty(), "{text}");
        }
    }
    #[test]
    fn auxiliary_and_fixed_subjunctive() {
        assert_eq!(
            fixes("FrenchAuxiliaryParticiple", "J’ai installer le programme."),
            vec![("installer".into(), "installé".into())]
        );
        assert_eq!(
            fixes(
                "FrenchFormalSubjunctive",
                "Il faut que nous sommes prêts. Pour qu’il peut lire."
            ),
            vec![
                ("sommes".into(), "soyons".into()),
                ("peut".into(), "puisse".into())
            ]
        );
        for text in [
            "Il faut que nous soyons prêts.",
            "Je pense que nous sommes prêts.",
            "Il est possible que nous soyons prêts.",
            "Pour que tu parles clairement.",
            "Il ne faut pas que nous soyons prêts.",
            "Il faut que nous allions à la maison.",
        ] {
            assert!(fixes("FrenchFormalSubjunctive", text).is_empty(), "{text}");
        }
        for text in [
            "J’ai dû installer le programme.",
            "J’ai fait installer le programme.",
            "Nous allons installer le programme.",
            "Elle a installé le programme.",
            "Une tâche à faire.",
            "Elle commence a travailler.",
        ] {
            assert!(
                fixes("FrenchAuxiliaryParticiple", text).is_empty(),
                "{text}"
            );
        }
    }

    #[test]
    fn motion_destinations_receive_accents_with_character_spans() {
        for (text, start, end, replacement) in [
            ("Il va a la maison.", 6, 7, "à"),
            ("Nous allons a cette école.", 12, 13, "à"),
            ("Elles sont allées a l’école.", 18, 19, "à"),
            ("Tu iras a une nouvelle bibliothèque.", 8, 9, "à"),
            ("Je veux aller a ma maison.", 14, 15, "à"),
            ("En allant a la gare, je téléphone.", 10, 11, "à"),
            ("Il va A la plage.", 6, 7, "À"),
            ("Va a l'hôpital.", 3, 4, "à"),
            ("Nous allons a la page suivante.", 12, 13, "à"),
        ] {
            let chars: Vec<char> = text.chars().collect();
            let actual: Vec<_> = lints(&Speller, &chars, &["FrenchPrepositionAccent"])
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
    fn accent_rule_preserves_avoir_nominal_motion_words_and_unclear_destinations() {
        for text in [
            "Il va à la maison.",
            "Elle est allée à l’école.",
            "Elle a la clé.",
            "Il a une maison.",
            "Cette allée a la largeur nécessaire.",
            "L’allée a la forme d’un cercle.",
            "Son aller a la priorité.",
            "Cet allant a la force de convaincre.",
            "Le laisser-aller a la vie dure.",
            "Il va à Paris.",
            "Elle est là où je travaille.",
            "Il va ou il reste.",
            "Elle pense a la maison.",
            "Il commence a travailler.",
            "Aller de Paris à Lyon.",
            "Il va, a-t-il dit, à la maison.",
            "Il va a la", // A fragment does not establish a destination phrase.
            "Il va a l école.",
            "« Il va a la maison » est un exemple.",
            "« Elles sont allées a l’école » est un exemple.",
            "Il va « a la maison ».",
            "`Il va a la gare`",
            "Il_va a la maison.",
            "Il va_a la maison.",
            "Il va a_la maison.",
            "Il va a la_maison.",
            "Il va a la maison_code.",
        ] {
            assert!(fixes("FrenchPrepositionAccent", text).is_empty(), "{text}");
        }
    }

    #[test]
    fn mandatory_contractions_replace_whole_character_spans() {
        for (text, start, end, replacement) in [
            ("Je vais à le bureau.", 8, 12, "au"),
            ("Je vais à les bureaux.", 8, 13, "aux"),
            ("Nous revenons de le bureau.", 14, 19, "du"),
            ("Nous revenons de les bureaux.", 14, 20, "des"),
            ("À le bureau, je travaille.", 0, 4, "Au"),
            ("De les bureaux, on voit le jardin.", 0, 6, "Des"),
            ("De le nouveau fichier.", 0, 5, "Du"),
            ("à les nouvelles applications.", 0, 5, "aux"),
            ("à le petit homme.", 0, 4, "au"),
            ("de les grands hôpitaux.", 0, 6, "des"),
            ("Il va a le bureau.", 6, 10, "au"),
            ("Il va a les bureaux.", 6, 11, "aux"),
            ("Nous allons a le jardin.", 12, 16, "au"),
            ("Elles sont allées a les bureaux.", 18, 23, "aux"),
            ("Je veux aller a le musée.", 14, 18, "au"),
            ("En allant a les jardins.", 10, 15, "aux"),
            ("Il va A le bureau.", 6, 10, "Au"),
            ("Je vais a\u{300} le bureau.", 8, 13, "au"),
            ("à les syste\u{300}mes.", 0, 5, "aux"),
            ("🙂 À le bureau.", 2, 6, "Au"),
            ("à\tles fichiers.", 0, 5, "aux"),
        ] {
            let chars: Vec<char> = text.chars().collect();
            let actual: Vec<_> = lints(&Speller, &chars, &["FrenchPrepositionContraction"])
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
    fn contractions_preserve_clitics_titles_fragments_and_motion_homographs() {
        for text in [
            "Je vais au bureau.",
            "Nous revenons du bureau.",
            "Je vais aux bureaux.",
            "Nous revenons des bureaux.",
            "Il essaie de le faire.",
            "Je commence à le lire.",
            "Il promet de les lire.",
            "Je pense à les faire.",
            "Il vient de Le Monde.",
            "Il vient de Le Bureau.",
            "Il pense à Les Fichiers.",
            "Il revient de le Bureau.",
            "Il va à le Nouveau bureau.",
            "« Je vais à le bureau » est un exemple.",
            "\"Nous revenons de le bureau.\"",
            "`Je vais a le bureau`",
            "Je vais « à le bureau ».",
            "Je vais à_le bureau.",
            "Je vais à le_bureau.",
            "Je vais à le bureau_code.",
            "Je vais à le bureau2.",
            "Je vais à le bureau-code.",
            "Je vais à le'bureau.",
            "Je vais à'le bureau.",
            "Je vais à le bureau'.",
            "Je vais 'à le bureau.",
            "Il vient d'e le bureau.",
            "Je vais à le.",
            "Je vais à les nouveaux.",
            "Je vais à le château.",
            "Nous revenons de les inconnus.",
            "Je vais à le, bureau.",
            "Je vais à les\n\nbureaux.",
            "Elle a le bureau.",
            "Elle a les fichiers.",
            "Elle pense a le bureau.",
            "Son aller a le mérite d’être court.",
            "Cette allée a le charme d’un jardin.",
            "Cet allant a les qualités requises.",
            "Le laisser-aller a le dernier mot.",
            "Il va, a-t-il dit, le long du jardin.",
            "Il_va a le bureau.",
            "Il va_a le bureau.",
            "Il va a_le bureau.",
            "Je vais à le maison.",
            "Je vais à le hôpital.",
            "Nous revenons de les fichier.",
            "Nous revenons de le fichiers.",
            "Je vais à le nouvelle bureau.",
            "Je vais à les grand bureaux.",
        ] {
            assert!(
                fixes("FrenchPrepositionContraction", text).is_empty(),
                "{text}"
            );
        }
    }

    #[test]
    fn contraction_candidates_leave_competing_agreement_edits_intact() {
        for (text, rule, source, replacement) in [
            (
                "Nous revenons de les fichier.",
                "FrenchArticleAgreement",
                "fichier",
                "fichiers",
            ),
            (
                "Il parle de le application.",
                "FrenchArticleAgreement",
                "le ",
                "l’",
            ),
            (
                "Il parle de le ancien fichier.",
                "FrenchArticleAgreement",
                "le ",
                "l’",
            ),
            (
                "Il parle de le nouvelle fichier.",
                "FrenchAdjectiveAgreement",
                "nouvelle",
                "nouveau",
            ),
        ] {
            let chars: Vec<char> = text.chars().collect();
            let source_chars = &chars;
            let actual: Vec<_> = lints(
                &Speller,
                &chars,
                &[
                    "FrenchPrepositionContraction",
                    "FrenchArticleAgreement",
                    "FrenchAdjectiveAgreement",
                ],
            )
            .into_iter()
            .flat_map(|(rule, lints)| {
                lints.into_iter().map(move |lint| {
                    let Suggestion::ReplaceWith(fix) = &lint.suggestions[0] else {
                        panic!("replacement required")
                    };
                    (
                        rule.clone(),
                        source_chars[lint.span.start..lint.span.end]
                            .iter()
                            .collect::<String>(),
                        fix.iter().collect::<String>(),
                    )
                })
            })
            .collect();
            assert_eq!(
                actual,
                vec![(
                    rule.to_string(),
                    source.to_string(),
                    replacement.to_string()
                )],
                "{text}"
            );
        }
    }

    #[test]
    fn bien_complements_comparisons_and_nouns_do_not_force_the_subjunctive() {
        for text in [
            "Je sais bien que nous sommes en retard.",
            "Vous savez bien que nous sommes en retard.",
            "Je savais bien que tu es présent.",
            "Je pense bien que vous avez raison.",
            "Il a plu, si bien que nous sommes restés chez nous.",
            "Il travaille aussi bien que nous travaillons.",
            "Le bien que nous faisons reste utile.",
            "Nous parlons du bien que nous faisons.",
        ] {
            assert!(fixes("FrenchFormalSubjunctive", text).is_empty(), "{text}");
        }
        assert_eq!(
            fixes(
                "FrenchFormalSubjunctive",
                "Bien que nous sommes en retard, nous partons."
            ),
            vec![("sommes".into(), "soyons".into())]
        );
        assert_eq!(
            fixes(
                "FrenchFormalSubjunctive",
                "Je sais, bien que nous sommes en retard."
            ),
            vec![("sommes".into(), "soyons".into())]
        );
    }

    #[test]
    fn homograph_errors_offer_each_verb_reading_without_flagging_valid_forms() {
        let text = "Il faut que tu suis les instructions.";
        let chars: Vec<_> = text.chars().collect();
        let found = lints(&Speller, &chars, &["FrenchFormalSubjunctive"]);
        let lint = &found["FrenchFormalSubjunctive"][0];
        let choices: Vec<String> = lint
            .suggestions
            .iter()
            .map(|s| {
                let Suggestion::ReplaceWith(chars) = s else {
                    panic!("replacement required")
                };
                chars.iter().collect()
            })
            .collect();
        assert_eq!(choices, ["sois", "suives"]);
        assert_eq!(lint.span, Span::new(15, 19));
        for text in [
            "Je suis les instructions.",
            "Tu suis les instructions.",
            "Il faut que tu suives les instructions.",
            "Il faut que tu sois le responsable.",
        ] {
            assert!(
                fixes("FrenchPronounVerbAgreement", text).is_empty(),
                "{text}"
            );
            assert!(fixes("FrenchFormalSubjunctive", text).is_empty(), "{text}");
        }
    }

    #[test]
    fn elided_definite_articles_are_singular_and_preserve_spacing() {
        for (text, original, replacement) in [
            ("L’applications sont disponibles.", "L’", "Les "),
            ("l'applications sont disponibles.", "l'", "les "),
            ("L’utilisateurs sont prêts.", "L’", "Les "),
            ("L’utiles applications sont disponibles.", "L’", "Les "),
        ] {
            assert_eq!(
                fixes("FrenchArticleAgreement", text),
                vec![(original.into(), replacement.into())],
                "{text}"
            );
            let chars: Vec<_> = text.chars().collect();
            let found = lints(&Speller, &chars, &["FrenchArticleAgreement"]);
            let lint = &found["FrenchArticleAgreement"][0];
            assert_eq!(lint.span, Span::new(0, 2));
        }
        for text in [
            "L’application est disponible.",
            "L'utilisateur est prêt.",
            "Ils l’ouvrent demain.",
            "L’Applications est le nom de la revue.",
            "« L’applications sont disponibles » est un exemple.",
        ] {
            assert!(fixes("FrenchArticleAgreement", text).is_empty(), "{text}");
        }
    }
}
