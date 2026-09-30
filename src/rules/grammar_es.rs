//! Conservative Spanish grammar from closed lexical paradigms, not suffix guesses.

use std::collections::BTreeMap;

use super::lint::{Lint, LintKind, Span, Suggestion};
use super::spell_lang::{LangSpeller, quotations, tokens};

pub const RULES: &[(&str, &str)] = &[
    (
        "SpanishArticleAgreement",
        "Spanish: determiners agree with known nouns in gender and number",
    ),
    (
        "SpanishAdjectiveAgreement",
        "Spanish: adjectives agree inside unambiguous noun phrases",
    ),
    (
        "SpanishPronounVerbAgreement",
        "Spanish: an explicit subject pronoun agrees with a finite verb",
    ),
    (
        "SpanishNounVerbAgreement",
        "Spanish: a clear known noun subject agrees with a finite verb",
    ),
    (
        "SpanishAuxiliaryParticiple",
        "Spanish: haber takes an invariant masculine singular participle",
    ),
    (
        "SpanishExistentialHaber",
        "Spanish: existential haber is singular",
    ),
    (
        "SpanishRequiredSubjunctive",
        "Spanish: explicit purpose and necessity clauses take the subjunctive",
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
    let mut quoted = quotations(chars, &ts);
    // The spelling helper deliberately ignores short quotations. Grammar must also leave
    // those, and inline code, verbatim.
    let mut open = None;
    for (i, &c) in chars.iter().enumerate() {
        if let Some((start, close)) = open {
            if c == close {
                quoted.push(start..i + 1);
                open = None;
            }
        } else {
            let close = match c {
                '"' | '`' => Some(c),
                '\'' if !chars
                    .get(i.wrapping_sub(1))
                    .is_some_and(|c| c.is_alphanumeric()) =>
                {
                    Some(c)
                }
                '“' => Some('”'),
                '«' => Some('»'),
                '‘' => Some('’'),
                _ => None,
            };
            if let Some(close) = close {
                open = Some((i, close));
            }
        }
    }
    if let Some((start, _)) = open {
        quoted.push(start..chars.len());
    }
    quoted.sort_unstable_by_key(|q| q.start);
    let mut quote = 0;
    ts.into_iter()
        .map(|(start, end)| {
            while quote < quoted.len() && quoted[quote].end <= start {
                quote += 1;
            }
            let raw = &chars[start..end];
            let plain = raw.iter().all(|c| c.is_alphabetic())
                && raw.iter().skip(1).all(|c| !c.is_uppercase())
                && !matches!(chars.get(start.wrapping_sub(1)), Some('_' | '@'))
                && !matches!(chars.get(end), Some('_' | '@'))
                && !quoted.get(quote).is_some_and(|q| q.contains(&start));
            Word {
                start,
                end,
                lower: raw.iter().flat_map(|c| c.to_lowercase()).collect(),
                plain,
            }
        })
        .collect()
}

fn adjacent(chars: &[char], a: &Word, b: &Word) -> bool {
    a.plain
        && b.plain
        && a.end < b.start
        && chars[a.end..b.start]
            .iter()
            .all(|&c| c == ' ' || c == '\t' || c == '\n')
        && chars[a.end..b.start].iter().filter(|&&c| c == '\n').count() < 2
}

fn add(
    out: &mut BTreeMap<String, Vec<Lint>>,
    enabled: &[&str],
    name: &str,
    chars: &[char],
    w: &Word,
    fix: &str,
    message: &str,
) {
    if !enabled.contains(&name) || w.lower == fix {
        return;
    }
    let replacement: Vec<char> = if chars[w.start].is_uppercase() {
        let mut cs = fix.chars();
        cs.next()
            .into_iter()
            .flat_map(char::to_uppercase)
            .chain(cs)
            .collect()
    } else {
        fix.chars().collect()
    };
    out.entry(name.to_owned()).or_default().push(Lint {
        span: Span::new(w.start, w.end),
        lint_kind: LintKind::Grammar,
        suggestions: vec![Suggestion::ReplaceWith(replacement)],
        message: message.to_owned(),
        priority: if name == "SpanishRequiredSubjunctive" {
            29
        } else {
            31
        },
    });
}

// Each pair has one fixed lexical gender. Common-gender nouns, stressed initial
// /a/ feminines (`el agua`), and ambiguous plural-only nouns are intentionally absent.
const NOUNS: &[(&str, &str, bool)] = &[
    ("archivo", "archivos", false),
    ("documento", "documentos", false),
    ("informe", "informes", false),
    ("mensaje", "mensajes", false),
    ("sistema", "sistemas", false),
    ("problema", "problemas", false),
    ("programa", "programas", false),
    ("mapa", "mapas", false),
    ("servidor", "servidores", false),
    ("proyecto", "proyectos", false),
    ("resultado", "resultados", false),
    ("recurso", "recursos", false),
    ("registro", "registros", false),
    ("permiso", "permisos", false),
    ("libro", "libros", false),
    ("contrato", "contratos", false),
    ("error", "errores", false),
    ("equipo", "equipos", false),
    ("proceso", "procesos", false),
    ("servicio", "servicios", false),
    ("cambio", "cambios", false),
    ("dato", "datos", false),
    ("página", "páginas", true),
    ("carpeta", "carpetas", true),
    ("cuenta", "cuentas", true),
    ("contraseña", "contraseñas", true),
    ("solicitud", "solicitudes", true),
    ("respuesta", "respuestas", true),
    ("versión", "versiones", true),
    ("configuración", "configuraciones", true),
    ("aplicación", "aplicaciones", true),
    ("herramienta", "herramientas", true),
    ("función", "funciones", true),
    ("opción", "opciones", true),
    ("regla", "reglas", true),
    ("tarea", "tareas", true),
    ("reunión", "reuniones", true),
    ("ciudad", "ciudades", true),
    ("casa", "casas", true),
    ("puerta", "puertas", true),
    ("mano", "manos", true),
    ("imagen", "imágenes", true),
];

// Order: masculine singular, feminine singular, masculine plural, feminine plural.
const DETERMINERS: &[[&str; 4]] = &[
    ["el", "la", "los", "las"],
    ["un", "una", "unos", "unas"],
    ["este", "esta", "estos", "estas"],
    ["ese", "esa", "esos", "esas"],
    ["aquel", "aquella", "aquellos", "aquellas"],
    ["nuestro", "nuestra", "nuestros", "nuestras"],
    ["vuestro", "vuestra", "vuestros", "vuestras"],
    ["todo", "toda", "todos", "todas"],
    ["otro", "otra", "otros", "otras"],
];
const ADJECTIVES: &[[&str; 4]] = &[
    ["nuevo", "nueva", "nuevos", "nuevas"],
    ["antiguo", "antigua", "antiguos", "antiguas"],
    ["pequeño", "pequeña", "pequeños", "pequeñas"],
    ["necesario", "necesaria", "necesarios", "necesarias"],
    ["correcto", "correcta", "correctos", "correctas"],
    ["incorrecto", "incorrecta", "incorrectos", "incorrectas"],
    ["completo", "completa", "completos", "completas"],
    ["vacío", "vacía", "vacíos", "vacías"],
    ["válido", "válida", "válidos", "válidas"],
    ["público", "pública", "públicos", "públicas"],
    ["privado", "privada", "privados", "privadas"],
    ["seguro", "segura", "seguros", "seguras"],
    ["abierto", "abierta", "abiertos", "abiertas"],
    ["cerrado", "cerrada", "cerrados", "cerradas"],
    ["activo", "activa", "activos", "activas"],
    ["inactivo", "inactiva", "inactivos", "inactivas"],
    ["obligatorio", "obligatoria", "obligatorios", "obligatorias"],
    ["distinto", "distinta", "distintos", "distintas"],
    ["disponible", "disponible", "disponibles", "disponibles"],
    ["importante", "importante", "importantes", "importantes"],
    ["grande", "grande", "grandes", "grandes"],
];

fn noun(word: &str) -> Option<usize> {
    NOUNS.iter().find_map(|&(sg, pl, feminine)| {
        if word == sg {
            Some(usize::from(feminine))
        } else if word == pl {
            Some(2 + usize::from(feminine))
        } else {
            None
        }
    })
}

fn noun_verb_homograph(word: &str) -> bool {
    matches!(
        word,
        "archivo"
            | "documento"
            | "informe"
            | "informes"
            | "registro"
            | "cambio"
            | "programa"
            | "programas"
            | "cuenta"
            | "cuentas"
            | "casa"
            | "casas"
            | "tarea"
            | "tareas"
            | "proceso"
            | "libro"
            | "contrato"
            | "equipo"
            | "dato"
    )
}

fn noun_phrases(
    chars: &[char],
    ws: &[Word],
    enabled: &[&str],
    out: &mut BTreeMap<String, Vec<Lint>>,
) {
    for i in 0..ws.len().saturating_sub(1) {
        let Some(det) = DETERMINERS
            .iter()
            .find(|forms| forms.contains(&ws[i].lower.as_str()))
        else {
            continue;
        };
        if !adjacent(chars, &ws[i], &ws[i + 1]) {
            continue;
        }
        let mut n = i + 1;
        let pre = ADJECTIVES
            .iter()
            .find(|forms| forms.contains(&ws[n].lower.as_str()));
        if pre.is_some() {
            n += 1;
            if n >= ws.len() || !adjacent(chars, &ws[n - 1], &ws[n]) {
                continue;
            }
        }
        let Some(kind) = noun(&ws[n].lower) else {
            continue;
        };
        // `la/los/las` can be object clitics before a finite verb.
        if pre.is_none()
            && matches!(ws[i].lower.as_str(), "la" | "los" | "las")
            && noun_verb_homograph(&ws[n].lower)
            && !(i > 0
                && adjacent(chars, &ws[i - 1], &ws[i])
                && matches!(
                    ws[i - 1].lower.as_str(),
                    "de" | "en" | "con" | "sin" | "sobre" | "para" | "por" | "a" | "tras" | "entre"
                ))
        {
            continue;
        }
        add(
            out,
            enabled,
            "SpanishArticleAgreement",
            chars,
            &ws[i],
            det[kind],
            "The determiner must agree with the noun's gender and number.",
        );
        if let Some(forms) = pre {
            add(
                out,
                enabled,
                "SpanishAdjectiveAgreement",
                chars,
                &ws[n - 1],
                forms[kind],
                "The adjective must agree with the noun's gender and number.",
            );
        }
        if n + 1 < ws.len() && adjacent(chars, &ws[n], &ws[n + 1]) {
            // A trailing adjective may describe a coordinated phrase instead.
            if i > 0 && matches!(ws[i - 1].lower.as_str(), "y" | "o" | "ni") {
                continue;
            }
            if i > 0
                && noun(&ws[i - 1].lower).is_some()
                && chars[ws[i - 1].end..ws[i].start].contains(&',')
            {
                continue;
            }
            if let Some(forms) = ADJECTIVES
                .iter()
                .find(|forms| forms.contains(&ws[n + 1].lower.as_str()))
            {
                add(
                    out,
                    enabled,
                    "SpanishAdjectiveAgreement",
                    chars,
                    &ws[n + 1],
                    forms[kind],
                    "The adjective must agree with the noun's gender and number.",
                );
            }
        }
    }
}

fn person(word: &str) -> Option<usize> {
    match word {
        "yo" => Some(0),
        "tú" => Some(1),
        "él" | "ella" | "usted" => Some(2),
        "nosotros" | "nosotras" => Some(3),
        "vosotros" | "vosotras" => Some(4),
        "ellos" | "ellas" | "ustedes" => Some(5),
        _ => None,
    }
}

// Present indicative and its corresponding present subjunctive. Forms shared by
// persons are matched without pretending that spelling provides morphology.
const VERBS: &[([&str; 6], [&str; 6])] = &[
    (
        ["soy", "eres", "es", "somos", "sois", "son"],
        ["sea", "seas", "sea", "seamos", "seáis", "sean"],
    ),
    (
        ["estoy", "estás", "está", "estamos", "estáis", "están"],
        ["esté", "estés", "esté", "estemos", "estéis", "estén"],
    ),
    (
        ["tengo", "tienes", "tiene", "tenemos", "tenéis", "tienen"],
        ["tenga", "tengas", "tenga", "tengamos", "tengáis", "tengan"],
    ),
    (
        ["he", "has", "ha", "hemos", "habéis", "han"],
        ["haya", "hayas", "haya", "hayamos", "hayáis", "hayan"],
    ),
    (
        ["puedo", "puedes", "puede", "podemos", "podéis", "pueden"],
        ["pueda", "puedas", "pueda", "podamos", "podáis", "puedan"],
    ),
    (
        ["debo", "debes", "debe", "debemos", "debéis", "deben"],
        ["deba", "debas", "deba", "debamos", "debáis", "deban"],
    ),
    (
        [
            "quiero", "quieres", "quiere", "queremos", "queréis", "quieren",
        ],
        [
            "quiera", "quieras", "quiera", "queramos", "queráis", "quieran",
        ],
    ),
    (
        ["hago", "haces", "hace", "hacemos", "hacéis", "hacen"],
        ["haga", "hagas", "haga", "hagamos", "hagáis", "hagan"],
    ),
    (
        ["voy", "vas", "va", "vamos", "vais", "van"],
        ["vaya", "vayas", "vaya", "vayamos", "vayáis", "vayan"],
    ),
    (
        ["vengo", "vienes", "viene", "venimos", "venís", "vienen"],
        ["venga", "vengas", "venga", "vengamos", "vengáis", "vengan"],
    ),
    (
        ["sé", "sabes", "sabe", "sabemos", "sabéis", "saben"],
        ["sepa", "sepas", "sepa", "sepamos", "sepáis", "sepan"],
    ),
    (
        ["digo", "dices", "dice", "decimos", "decís", "dicen"],
        ["diga", "digas", "diga", "digamos", "digáis", "digan"],
    ),
    (
        ["uso", "usas", "usa", "usamos", "usáis", "usan"],
        ["use", "uses", "use", "usemos", "uséis", "usen"],
    ),
    (
        [
            "guardo",
            "guardas",
            "guarda",
            "guardamos",
            "guardáis",
            "guardan",
        ],
        [
            "guarde",
            "guardes",
            "guarde",
            "guardemos",
            "guardéis",
            "guarden",
        ],
    ),
    (
        [
            "necesito",
            "necesitas",
            "necesita",
            "necesitamos",
            "necesitáis",
            "necesitan",
        ],
        [
            "necesite",
            "necesites",
            "necesite",
            "necesitemos",
            "necesitéis",
            "necesiten",
        ],
    ),
    (
        [
            "funciono",
            "funcionas",
            "funciona",
            "funcionamos",
            "funcionáis",
            "funcionan",
        ],
        [
            "funcione",
            "funciones",
            "funcione",
            "funcionemos",
            "funcionéis",
            "funcionen",
        ],
    ),
    (
        [
            "permito",
            "permites",
            "permite",
            "permitimos",
            "permitís",
            "permiten",
        ],
        [
            "permita",
            "permitas",
            "permita",
            "permitamos",
            "permitáis",
            "permitan",
        ],
    ),
    (
        [
            "existo",
            "existes",
            "existe",
            "existimos",
            "existís",
            "existen",
        ],
        [
            "exista",
            "existas",
            "exista",
            "existamos",
            "existáis",
            "existan",
        ],
    ),
    (
        [
            "contengo",
            "contienes",
            "contiene",
            "contenemos",
            "contenéis",
            "contienen",
        ],
        [
            "contenga",
            "contengas",
            "contenga",
            "contengamos",
            "contengáis",
            "contengan",
        ],
    ),
];
const PAST: &[[&str; 6]] = &[
    ["fui", "fuiste", "fue", "fuimos", "fuisteis", "fueron"],
    ["era", "eras", "era", "éramos", "erais", "eran"],
    [
        "estaba",
        "estabas",
        "estaba",
        "estábamos",
        "estabais",
        "estaban",
    ],
    ["tenía", "tenías", "tenía", "teníamos", "teníais", "tenían"],
    ["había", "habías", "había", "habíamos", "habíais", "habían"],
];

fn clause_start(chars: &[char], ws: &[Word], i: usize) -> bool {
    let preceding = chars[..ws[i].start]
        .iter()
        .rev()
        .find(|c| !c.is_whitespace());
    preceding.is_none()
        || preceding.is_some_and(|c| matches!(c, '.' | '!' | '?' | '¿' | '¡' | ';' | ':' | ','))
        || (i > 0
            && adjacent(chars, &ws[i - 1], &ws[i])
            && matches!(
                ws[i - 1].lower.as_str(),
                "que" | "porque" | "cuando" | "mientras" | "si"
            ))
}

fn pronoun_verbs(
    chars: &[char],
    ws: &[Word],
    enabled: &[&str],
    out: &mut BTreeMap<String, Vec<Lint>>,
) {
    for i in 0..ws.len().saturating_sub(1) {
        let Some(p) = person(&ws[i].lower) else {
            continue;
        };
        if !clause_start(chars, ws, i) || !adjacent(chars, &ws[i], &ws[i + 1]) {
            continue;
        }
        let mut v = i + 1;
        if matches!(ws[v].lower.as_str(), "no" | "ya" | "siempre") {
            v += 1;
            if v >= ws.len() || !adjacent(chars, &ws[v - 1], &ws[v]) {
                continue;
            }
        }
        for forms in VERBS
            .iter()
            .map(|(indicative, _)| indicative)
            .chain(PAST.iter())
        {
            if forms.contains(&ws[v].lower.as_str()) {
                // A contrastive tú + imperative need not have a comma.
                if p == 1
                    && ws[v].lower == forms[2]
                    && matches!(
                        forms[0],
                        "estoy"
                            | "uso"
                            | "guardo"
                            | "necesito"
                            | "funciono"
                            | "permito"
                            | "existo"
                            | "contengo"
                            | "sé"
                            | "debo"
                            | "quiero"
                    )
                {
                    break;
                }
                add(
                    out,
                    enabled,
                    "SpanishPronounVerbAgreement",
                    chars,
                    &ws[v],
                    forms[p],
                    "The finite verb must agree with the explicit subject pronoun.",
                );
                break;
            }
        }
    }
}

fn noun_verbs(
    chars: &[char],
    ws: &[Word],
    enabled: &[&str],
    out: &mut BTreeMap<String, Vec<Lint>>,
) {
    for i in 0..ws.len().saturating_sub(2) {
        if !clause_start(chars, ws, i)
            || chars[..ws[i].start]
                .iter()
                .rev()
                .find(|c| !c.is_whitespace())
                == Some(&',')
            || matches!(
                chars.get(ws[i].start.wrapping_sub(1)),
                Some('.' | '!' | '?' | ';' | ':')
            )
            || !adjacent(chars, &ws[i], &ws[i + 1])
        {
            continue;
        }
        let Some(det) = DETERMINERS
            .iter()
            .find(|forms| forms.contains(&ws[i].lower.as_str()))
        else {
            continue;
        };
        let mut n = i + 1;
        let pre = ADJECTIVES
            .iter()
            .find(|forms| forms.contains(&ws[n].lower.as_str()));
        if pre.is_some() {
            n += 1;
            if n >= ws.len() || !adjacent(chars, &ws[n - 1], &ws[n]) {
                continue;
            }
        }
        let Some(kind) = noun(&ws[n].lower) else {
            continue;
        };
        if ws[i].lower != det[kind]
            || pre.is_some_and(|forms| ws[n - 1].lower != forms[kind])
            || matches!(ws[n].lower.as_str(), "equipo" | "equipos")
            || (pre.is_none()
                && matches!(ws[i].lower.as_str(), "la" | "los" | "las")
                && noun_verb_homograph(&ws[n].lower))
        {
            continue;
        }
        let mut v = n + 1;
        if v < ws.len()
            && adjacent(chars, &ws[n], &ws[v])
            && ADJECTIVES.iter().any(|forms| ws[v].lower == forms[kind])
        {
            v += 1;
        }
        if v < ws.len() && matches!(ws[v].lower.as_str(), "no" | "ya" | "siempre") {
            if !adjacent(chars, &ws[v - 1], &ws[v]) {
                continue;
            }
            v += 1;
        }
        if v >= ws.len()
            || !adjacent(chars, &ws[v - 1], &ws[v])
            || matches!(chars.get(ws[v].end), Some('-' | '\'' | '’'))
        {
            continue;
        }
        // Object fronting permits a following subject, including a bare noun
        // or a name. Leave the whole ambiguous clause untouched.
        let copular = VERBS.iter().take(2).any(|(indicative, subjunctive)| {
            indicative.contains(&ws[v].lower.as_str())
                || subjunctive.contains(&ws[v].lower.as_str())
        }) || PAST
            .iter()
            .take(3)
            .any(|forms| forms.contains(&ws[v].lower.as_str()));
        let mut following_subject = false;
        for t in v + 1..ws.len() {
            if !ws[t].plain
                && chars[ws[t - 1].end..ws[t].start]
                    .iter()
                    .all(|c| c.is_whitespace())
            {
                following_subject = true;
                break;
            }
            if !adjacent(chars, &ws[t - 1], &ws[t]) {
                break;
            }
            let word = ws[t].lower.as_str();
            if noun(word).is_some()
                || person(word).is_some()
                || word == "vos"
                || DETERMINERS.iter().any(|forms| forms.contains(&word))
                || chars[ws[t].start].is_uppercase()
                || (!copular
                    && !ADJECTIVES.iter().any(|forms| forms.contains(&word))
                    && !matches!(
                        word,
                        "no" | "ya"
                            | "siempre"
                            | "aquí"
                            | "allí"
                            | "hoy"
                            | "ayer"
                            | "mañana"
                            | "todavía"
                            | "bien"
                            | "mal"
                            | "correctamente"
                    ))
            {
                following_subject = true;
                break;
            }
        }
        if following_subject {
            continue;
        }
        let p = if kind < 2 { 2 } else { 5 };
        let mut fix = None;
        let mut ambiguous = false;
        for forms in VERBS
            .iter()
            .flat_map(|(indicative, subjunctive)| [indicative, subjunctive])
            .chain(PAST.iter())
        {
            // First/second-person verbs can instead have an omitted subject
            // after a fronted object. Only compare the two third-person forms.
            if ws[v].lower == forms[2] || ws[v].lower == forms[5] {
                if fix.is_some_and(|previous| previous != forms[p]) {
                    ambiguous = true;
                    break;
                }
                fix = Some(forms[p]);
            }
        }
        if !ambiguous && let Some(fix) = fix {
            add(
                out,
                enabled,
                "SpanishNounVerbAgreement",
                chars,
                &ws[v],
                fix,
                "The finite verb must agree with the explicit noun subject.",
            );
        }
    }
}

const PARTICIPLES: &[[&str; 4]] = &[
    ["enviado", "enviada", "enviados", "enviadas"],
    ["guardado", "guardada", "guardados", "guardadas"],
    ["escrito", "escrita", "escritos", "escritas"],
    ["hecho", "hecha", "hechos", "hechas"],
    ["abierto", "abierta", "abiertos", "abiertas"],
    ["cerrado", "cerrada", "cerrados", "cerradas"],
    ["terminado", "terminada", "terminados", "terminadas"],
    ["aprobado", "aprobada", "aprobados", "aprobadas"],
    ["visto", "vista", "vistos", "vistas"],
    ["leído", "leída", "leídos", "leídas"],
    ["recibido", "recibida", "recibidos", "recibidas"],
];

fn auxiliaries(
    chars: &[char],
    ws: &[Word],
    enabled: &[&str],
    out: &mut BTreeMap<String, Vec<Lint>>,
) {
    for i in 0..ws.len().saturating_sub(1) {
        if !adjacent(chars, &ws[i], &ws[i + 1]) {
            continue;
        }
        let w = ws[i].lower.as_str();
        if (VERBS[3].0.contains(&w)
            || VERBS[3].1.contains(&w)
            || PAST[4].contains(&w)
            || matches!(
                w,
                "haber" | "habiendo" | "habrá" | "habrán" | "habría" | "habrían"
            ))
            && let Some(forms) = PARTICIPLES
                .iter()
                .find(|forms| forms.contains(&ws[i + 1].lower.as_str()))
        {
            add(
                out,
                enabled,
                "SpanishAuxiliaryParticiple",
                chars,
                &ws[i + 1],
                forms[0],
                "With haber, the compound-tense participle is invariant masculine singular.",
            );
        }
        let fix = match w {
            "habían" => "había",
            "hubieron" => "hubo",
            "habrán" => "habrá",
            "habrían" => "habría",
            "hayan" => "haya",
            "hubieran" => "hubiera",
            "hubiesen" => "hubiese",
            _ => continue,
        };
        let mut n = i + 1;
        if DETERMINERS
            .iter()
            .any(|forms| forms.contains(&ws[n].lower.as_str()))
            || matches!(
                ws[n].lower.as_str(),
                "dos" | "tres" | "cuatro" | "muchos" | "muchas" | "varios" | "varias"
            )
        {
            n += 1;
            if n >= ws.len() || !adjacent(chars, &ws[n - 1], &ws[n]) {
                continue;
            }
        }
        if noun(&ws[n].lower).is_some_and(|kind| kind >= 2) {
            add(
                out,
                enabled,
                "SpanishExistentialHaber",
                chars,
                &ws[i],
                fix,
                "Existential haber is impersonal and remains singular, even before a plural noun.",
            );
        }
    }
}

fn subjunctive(
    chars: &[char],
    ws: &[Word],
    enabled: &[&str],
    out: &mut BTreeMap<String, Vec<Lint>>,
) {
    for q in 1..ws.len().saturating_sub(1) {
        if ws[q].lower != "que" || !adjacent(chars, &ws[q - 1], &ws[q]) {
            continue;
        }
        let fixed = matches!(ws[q - 1].lower.as_str(), "para" | "sin")
            || (q >= 2
                && ws[q - 2].lower == "a"
                && ws[q - 1].lower == "menos"
                && adjacent(chars, &ws[q - 2], &ws[q - 1]))
            || (q >= 2
                && ws[q - 2].lower == "antes"
                && ws[q - 1].lower == "de"
                && adjacent(chars, &ws[q - 2], &ws[q - 1]))
            || (q >= 2
                && ws[q - 2].lower == "es"
                && matches!(
                    ws[q - 1].lower.as_str(),
                    "necesario" | "imprescindible" | "importante" | "obligatorio"
                )
                && adjacent(chars, &ws[q - 2], &ws[q - 1]));
        if !fixed || !adjacent(chars, &ws[q], &ws[q + 1]) {
            continue;
        }
        let mut v = q + 1;
        let p = person(&ws[v].lower);
        if p.is_some() {
            v += 1;
            if v >= ws.len() || !adjacent(chars, &ws[v - 1], &ws[v]) {
                continue;
            }
        }
        if ws[v].lower == "no" {
            v += 1;
            if v >= ws.len() || !adjacent(chars, &ws[v - 1], &ws[v]) {
                continue;
            }
        }
        for (indicative, subj) in VERBS {
            if let Some(observed) = indicative.iter().position(|&form| form == ws[v].lower) {
                add(
                    out,
                    enabled,
                    "SpanishRequiredSubjunctive",
                    chars,
                    &ws[v],
                    subj[p.unwrap_or(observed)],
                    "This purpose, exclusion or necessity clause requires the subjunctive.",
                );
                break;
            }
        }
    }
}

pub fn lints(
    _sp: &dyn LangSpeller,
    chars: &[char],
    enabled: &[&str],
) -> BTreeMap<String, Vec<Lint>> {
    if !RULES.iter().any(|(name, _)| enabled.contains(name)) {
        return BTreeMap::new();
    }
    let ws = words(chars);
    let mut out = BTreeMap::new();
    if enabled.contains(&"SpanishArticleAgreement")
        || enabled.contains(&"SpanishAdjectiveAgreement")
    {
        noun_phrases(chars, &ws, enabled, &mut out);
    }
    if enabled.contains(&"SpanishPronounVerbAgreement") {
        pronoun_verbs(chars, &ws, enabled, &mut out);
    }
    if enabled.contains(&"SpanishNounVerbAgreement") {
        noun_verbs(chars, &ws, enabled, &mut out);
    }
    if enabled.contains(&"SpanishAuxiliaryParticiple")
        || enabled.contains(&"SpanishExistentialHaber")
    {
        auxiliaries(chars, &ws, enabled, &mut out);
    }
    if enabled.contains(&"SpanishRequiredSubjunctive") {
        subjunctive(chars, &ws, enabled, &mut out);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixes(rule: &str, text: &str) -> Vec<(String, String)> {
        let sp = crate::rules::spell_lang::speller("es", &crate::config::Config::default())
            .expect("bundled Spanish");
        let chars: Vec<char> = text.chars().collect();
        lints(&*sp, &chars, &[rule])
            .into_values()
            .flatten()
            .map(|lint| {
                let original = chars[lint.span.start..lint.span.end].iter().collect();
                let Suggestion::ReplaceWith(fix) = &lint.suggestions[0] else {
                    panic!("replacement")
                };
                (original, fix.iter().collect())
            })
            .collect()
    }

    fn quiet(rule: &str, examples: &[&str]) {
        for text in examples {
            assert!(fixes(rule, text).is_empty(), "{rule}: {text}");
        }
    }

    #[test]
    fn known_noun_subject_number_and_character_spans() {
        let rule = "SpanishNounVerbAgreement";
        let sp = crate::rules::spell_lang::speller("es", &crate::config::Config::default())
            .expect("bundled Spanish");
        for (text, start, end, replacement) in [
            ("Los sistemas está disponibles.", 13, 17, "están"),
            ("El sistema están disponible.", 11, 16, "está"),
            ("Un documento son suficiente.", 13, 16, "es"),
            ("Unos documentos es suficientes.", 16, 18, "son"),
            ("Espero que las páginas sea accesibles.", 23, 26, "sean"),
            ("La versión estaban disponible.", 11, 18, "estaba"),
            ("Las nuevas imágenes no está disponibles.", 23, 27, "están"),
            ("Una página importante eran suficiente.", 22, 26, "era"),
            ("Los sistemas funciona correctamente.", 13, 21, "funcionan"),
            (
                "Es posible que el sistema existan todavía.",
                26,
                33,
                "exista",
            ),
        ] {
            let chars: Vec<char> = text.chars().collect();
            let found: Vec<_> = lints(&*sp, &chars, &[rule])
                .into_values()
                .flatten()
                .map(|lint| {
                    let Suggestion::ReplaceWith(fix) = &lint.suggestions[0] else {
                        panic!("replacement")
                    };
                    (
                        lint.span.start,
                        lint.span.end,
                        fix.iter().collect::<String>(),
                    )
                })
                .collect();
            assert_eq!(found, [(start, end, replacement.to_owned())], "{text}");
        }
    }

    #[test]
    fn noun_subject_ambiguities_remain_untouched() {
        quiet(
            "SpanishNounVerbAgreement",
            &[
                "Es posible que los sistemas estén disponibles.",
                "Quizá una página sea suficiente.",
                "Están disponibles las páginas?",
                "El archivo y la carpeta están disponibles.",
                "El archivo, la carpeta están disponibles.",
                "La mayoría de los sistemas están disponibles.",
                "El equipo están muy unidos.",
                "En los sistemas está disponible.",
                "Para las páginas es suficiente.",
                "El documento guardan los usuarios.",
                "Un documento guardan ellos.",
                "Un documento guardan María y Juan.",
                "Un documento guardan usuarios.",
                "Un documento guardan McDonald y Juan.",
                "Un documento contiene imágenes.",
                "Los documentos guardamos aquí.",
                "Las páginas tienes aquí.",
                "Los archivo siempre.",
                "La cuenta siempre.",
                "No las informes todavía.",
                "Es necesario que las funciones estén documentadas.",
                "Una artista está disponible.",
                "Vos tenés permiso.",
                "«Los sistemas está disponibles» es una cita.",
                "`La versión estaban disponible` es código.",
                "Los sistemas_está disponibles.",
                "foo.el sistema están disponible.",
            ],
        );
    }

    #[test]
    fn lexical_agreement_and_unicode_spans() {
        assert_eq!(
            fixes("SpanishArticleAgreement", "Aquí está la problema."),
            [("la".into(), "el".into())]
        );
        assert_eq!(
            fixes("SpanishArticleAgreement", "Un imagen aparece."),
            [("Un".into(), "Una".into())]
        );
        assert_eq!(
            fixes("SpanishAdjectiveAgreement", "Las páginas nuevo funcionan."),
            [("nuevo".into(), "nuevas".into())]
        );
        assert_eq!(
            fixes("SpanishAdjectiveAgreement", "La nuevos versión funciona."),
            [("nuevos".into(), "nueva".into())]
        );
        quiet(
            "SpanishArticleAgreement",
            &[
                "El agua está limpia.",
                "La mano está fría.",
                "Los sistemas funcionan.",
                "Una artista trabaja.",
                "Las grandes ciudades crecen.",
                "La registro cada mañana.",
                "Los archivo cada noche.",
                "Ella la programa con cuidado.",
            ],
        );
        quiet(
            "SpanishArticleAgreement",
            &[
                "Los cuentas cada día.",
                "No las informes todavía.",
                "Los contrato mañana.",
                "La libro de cualquier obligación.",
            ],
        );
        quiet(
            "SpanishAdjectiveAgreement",
            &[
                "Las páginas importantes funcionan.",
                "El archivo y la carpeta nuevos están disponibles.",
                "La página del documento nuevo está disponible.",
                "Los grandes archivos funcionan.",
            ],
        );
        quiet(
            "SpanishAdjectiveAgreement",
            &["El archivo, la carpeta nuevos están disponibles."],
        );
    }

    #[test]
    fn explicit_subjects_not_inversion_or_collectives() {
        assert_eq!(
            fixes("SpanishPronounVerbAgreement", "Ellos tiene acceso."),
            [("tiene".into(), "tienen".into())]
        );
        assert_eq!(
            fixes(
                "SpanishPronounVerbAgreement",
                "Sé que nosotros es responsables."
            ),
            [("es".into(), "somos".into())]
        );
        quiet(
            "SpanishPronounVerbAgreement",
            &[
                "Tienen ellos acceso?",
                "Ella y él tienen acceso.",
                "La mayoría de ellos tiene acceso.",
                "Para ellos es importante.",
                "Yo tenía acceso.",
                "Tú guarda los documentos.",
                "Vos tenés acceso.",
                "Nosotros, los responsables, tenemos acceso.",
            ],
        );
    }

    #[test]
    fn auxiliary_and_impersonal_constructions() {
        assert_eq!(
            fixes(
                "SpanishAuxiliaryParticiple",
                "Ellas han escritas las respuestas."
            ),
            [("escritas".into(), "escrito".into())]
        );
        assert_eq!(
            fixes("SpanishExistentialHaber", "Habían varios errores."),
            [("Habían".into(), "Había".into())]
        );
        assert_eq!(
            fixes("SpanishExistentialHaber", "Hubieron problemas."),
            [("Hubieron".into(), "Hubo".into())]
        );
        quiet(
            "SpanishAuxiliaryParticiple",
            &[
                "Ellas están preparadas.",
                "Las respuestas han sido escritas.",
                "Hemos escrito las respuestas.",
            ],
        );
        quiet(
            "SpanishExistentialHaber",
            &[
                "Ellos habían recibido respuestas.",
                "Hubieron de salir temprano.",
                "Había muchos errores.",
                "Hayan terminado o no, saldremos.",
            ],
        );
    }

    #[test]
    fn ordinary_and_formal_prose_remains_clean() {
        let examples = [
            "Si ustedes tienen dudas, la respuesta está en las páginas nuevas.",
            "Nosotros hemos recibido las solicitudes y ellas han enviado los documentos.",
            "Es necesario que ustedes tengan acceso para que puedan usar el servicio.",
            "La mayoría de los usuarios tiene una cuenta y hay varios problemas pendientes.",
            "El archivo y la nueva carpeta importantes están disponibles.",
            "Antes de que estén listos los informes, ella quiere revisar la configuración.",
            "La imagen está en el informe y las manos están limpias.",
            "'ellos tiene' se reproduce literalmente.",
        ];
        for (rule, _) in RULES {
            quiet(rule, &examples);
        }
    }

    #[test]
    fn required_subjunctive_and_verbatim_boundaries() {
        assert_eq!(
            fixes(
                "SpanishRequiredSubjunctive",
                "Es necesario que ella tiene acceso."
            ),
            [("tiene".into(), "tenga".into())]
        );
        assert_eq!(
            fixes(
                "SpanishRequiredSubjunctive",
                "Lo explico para que ustedes saben usarlo."
            ),
            [("saben".into(), "sepan".into())]
        );
        quiet(
            "SpanishRequiredSubjunctive",
            &[
                "Sé que ella tiene acceso.",
                "Es necesario que tengan acceso.",
                "Lo explico para que sepan usarlo.",
                "Antes de que llegue ella, saldremos.",
                "Es necesario, que él tiene dudas es evidente.",
            ],
        );
        for rule in RULES.iter().map(|(name, _)| *name) {
            quiet(
                rule,
                &[
                    "«Ellos tiene acceso» es una cita.",
                    "\"la problema\" es una cita.",
                    "`ellos tiene` es código.",
                    "ellos_tiene es un identificador.",
                    "La, problema.",
                ],
            );
        }
    }
}
