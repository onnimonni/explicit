//! Conservative Portuguese grammar, with both Brazilian and European lexical forms.

use std::collections::BTreeMap;

use super::lint::{Lint, LintKind, Span, Suggestion};
use super::spell_lang::{LangSpeller, quotations, tokens};

pub const RULES: &[(&str, &str)] = &[
    (
        "PortugueseArticleAgreement",
        "Portuguese: determiners agree with known nouns in gender and number",
    ),
    (
        "PortugueseAdjectiveAgreement",
        "Portuguese: adjectives agree inside unambiguous noun phrases",
    ),
    (
        "PortuguesePronounVerbAgreement",
        "Portuguese: an explicit subject pronoun agrees with a finite verb",
    ),
    (
        "PortugueseNounVerbAgreement",
        "Portuguese: a clear known noun subject agrees with a finite verb",
    ),
    (
        "PortugueseModalInfinitive",
        "Portuguese: poder and dever take an infinitive, not a finite verb",
    ),
    (
        "PortugueseExistentialHaver",
        "Portuguese: existential haver is singular",
    ),
    (
        "PortugueseRequiredSubjunctive",
        "Portuguese: explicit purpose and necessity clauses take the subjunctive",
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
        priority: if name == "PortugueseRequiredSubjunctive" {
            29
        } else {
            31
        },
    });
}

// Only fixed-gender nouns: not `estudante`, `colega`, or nouns with dialect-dependent
// gender. Both `ficheiro/arquivo` and pre-/post-agreement orthographies are accepted.
const NOUNS: &[(&str, &str, bool)] = &[
    ("arquivo", "arquivos", false),
    ("ficheiro", "ficheiros", false),
    ("documento", "documentos", false),
    ("relatório", "relatórios", false),
    ("sistema", "sistemas", false),
    ("problema", "problemas", false),
    ("programa", "programas", false),
    ("mapa", "mapas", false),
    ("servidor", "servidores", false),
    ("projeto", "projetos", false),
    ("projecto", "projectos", false),
    ("resultado", "resultados", false),
    ("recurso", "recursos", false),
    ("registo", "registos", false),
    ("registro", "registros", false),
    ("livro", "livros", false),
    ("contrato", "contratos", false),
    ("erro", "erros", false),
    ("processo", "processos", false),
    ("serviço", "serviços", false),
    ("dado", "dados", false),
    ("pedido", "pedidos", false),
    ("acesso", "acessos", false),
    ("texto", "textos", false),
    ("utilizador", "utilizadores", false),
    ("usuário", "usuários", false),
    ("página", "páginas", true),
    ("pasta", "pastas", true),
    ("conta", "contas", true),
    ("senha", "senhas", true),
    ("mensagem", "mensagens", true),
    ("imagem", "imagens", true),
    ("resposta", "respostas", true),
    ("versão", "versões", true),
    ("configuração", "configurações", true),
    ("aplicação", "aplicações", true),
    ("ferramenta", "ferramentas", true),
    ("função", "funções", true),
    ("opção", "opções", true),
    ("regra", "regras", true),
    ("tarefa", "tarefas", true),
    ("reunião", "reuniões", true),
    ("cidade", "cidades", true),
    ("casa", "casas", true),
    ("porta", "portas", true),
    ("mão", "mãos", true),
    ("ação", "ações", true),
    ("acção", "acções", true),
    ("equipa", "equipas", true),
    ("equipe", "equipes", true),
];

// Order: masculine singular, feminine singular, masculine plural, feminine plural.
const DETERMINERS: &[[&str; 4]] = &[
    ["o", "a", "os", "as"],
    ["um", "uma", "uns", "umas"],
    ["este", "esta", "estes", "estas"],
    ["esse", "essa", "esses", "essas"],
    ["aquele", "aquela", "aqueles", "aquelas"],
    ["nosso", "nossa", "nossos", "nossas"],
    ["vosso", "vossa", "vossos", "vossas"],
    ["meu", "minha", "meus", "minhas"],
    ["teu", "tua", "teus", "tuas"],
    ["seu", "sua", "seus", "suas"],
    ["outro", "outra", "outros", "outras"],
    ["do", "da", "dos", "das"],
    ["no", "na", "nos", "nas"],
    ["ao", "à", "aos", "às"],
];
const ADJECTIVES: &[[&str; 4]] = &[
    ["novo", "nova", "novos", "novas"],
    ["antigo", "antiga", "antigos", "antigas"],
    ["pequeno", "pequena", "pequenos", "pequenas"],
    ["necessário", "necessária", "necessários", "necessárias"],
    ["correto", "correta", "corretos", "corretas"],
    ["correcto", "correcta", "correctos", "correctas"],
    ["incorreto", "incorreta", "incorretos", "incorretas"],
    ["incorrecto", "incorrecta", "incorrectos", "incorrectas"],
    ["completo", "completa", "completos", "completas"],
    ["vazio", "vazia", "vazios", "vazias"],
    ["válido", "válida", "válidos", "válidas"],
    ["público", "pública", "públicos", "públicas"],
    ["privado", "privada", "privados", "privadas"],
    ["seguro", "segura", "seguros", "seguras"],
    ["aberto", "aberta", "abertos", "abertas"],
    ["fechado", "fechada", "fechados", "fechadas"],
    ["ativo", "ativa", "ativos", "ativas"],
    ["activo", "activa", "activos", "activas"],
    ["inativo", "inativa", "inativos", "inativas"],
    ["inactivo", "inactiva", "inactivos", "inactivas"],
    ["obrigatório", "obrigatória", "obrigatórios", "obrigatórias"],
    ["distinto", "distinta", "distintos", "distintas"],
    ["disponível", "disponível", "disponíveis", "disponíveis"],
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
        "arquivo"
            | "documento"
            | "programa"
            | "programas"
            | "conta"
            | "contas"
            | "casa"
            | "casas"
            | "registo"
            | "registro"
            | "pedido"
            | "dado"
            | "projeto"
            | "projecto"
            | "livro"
            | "contrato"
            | "processo"
            | "pasta"
            | "pastas"
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
        // `a` is also an ordinary preposition: `a pedido`, `referente a problema`,
        // `a dado momento`. Do not invent an article without licensing evidence.
        if ws[i].lower == "a"
            && det[kind] != "a"
            && !(i > 0
                && adjacent(chars, &ws[i - 1], &ws[i])
                && matches!(
                    ws[i - 1].lower.as_str(),
                    "com"
                        | "de"
                        | "em"
                        | "sem"
                        | "sobre"
                        | "por"
                        | "vejo"
                        | "leio"
                        | "abro"
                        | "fecho"
                        | "guardo"
                        | "envio"
                        | "recebo"
                        | "escrevo"
                        | "crio"
                        | "edito"
                        | "publico"
                ))
        {
            continue;
        }
        // Definite articles are also object clitics before these common finite
        // verbs; a noun reading needs more than dictionary validity.
        if pre.is_none()
            && matches!(ws[i].lower.as_str(), "o" | "a" | "os" | "as")
            && noun_verb_homograph(&ws[n].lower)
            && !(i > 0
                && adjacent(chars, &ws[i - 1], &ws[i])
                && matches!(
                    ws[i - 1].lower.as_str(),
                    "com" | "de" | "em" | "sem" | "sobre" | "por"
                ))
        {
            continue;
        }
        add(
            out,
            enabled,
            "PortugueseArticleAgreement",
            chars,
            &ws[i],
            det[kind],
            "The determiner must agree with the noun's gender and number.",
        );
        if let Some(forms) = pre {
            add(
                out,
                enabled,
                "PortugueseAdjectiveAgreement",
                chars,
                &ws[n - 1],
                forms[kind],
                "The adjective must agree with the noun's gender and number.",
            );
        }
        if n + 1 < ws.len() && adjacent(chars, &ws[n], &ws[n + 1]) {
            if i > 0 && matches!(ws[i - 1].lower.as_str(), "e" | "ou" | "nem") {
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
                    "PortugueseAdjectiveAgreement",
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
        "eu" => Some(0),
        "ele" | "ela" | "você" => Some(2),
        "nós" => Some(3),
        "vós" => Some(4),
        "eles" | "elas" | "vocês" => Some(5),
        _ => None,
    }
    // tu is deliberately unclassified: its second-/third-person use varies in Brazil.
}

const VERBS: &[([&str; 6], [&str; 6])] = &[
    (
        ["sou", "és", "é", "somos", "sois", "são"],
        ["seja", "sejas", "seja", "sejamos", "sejais", "sejam"],
    ),
    (
        ["estou", "estás", "está", "estamos", "estais", "estão"],
        [
            "esteja",
            "estejas",
            "esteja",
            "estejamos",
            "estejais",
            "estejam",
        ],
    ),
    (
        ["tenho", "tens", "tem", "temos", "tendes", "têm"],
        ["tenha", "tenhas", "tenha", "tenhamos", "tenhais", "tenham"],
    ),
    (
        ["hei", "hás", "há", "havemos", "haveis", "hão"],
        ["haja", "hajas", "haja", "hajamos", "hajais", "hajam"],
    ),
    (
        ["posso", "podes", "pode", "podemos", "podeis", "podem"],
        ["possa", "possas", "possa", "possamos", "possais", "possam"],
    ),
    (
        ["devo", "deves", "deve", "devemos", "deveis", "devem"],
        ["deva", "devas", "deva", "devamos", "devais", "devam"],
    ),
    (
        ["quero", "queres", "quer", "queremos", "quereis", "querem"],
        [
            "queira",
            "queiras",
            "queira",
            "queiramos",
            "queirais",
            "queiram",
        ],
    ),
    (
        ["faço", "fazes", "faz", "fazemos", "fazeis", "fazem"],
        ["faça", "faças", "faça", "façamos", "façais", "façam"],
    ),
    (
        ["vou", "vais", "vai", "vamos", "ides", "vão"],
        ["vá", "vás", "vá", "vamos", "vades", "vão"],
    ),
    (
        ["venho", "vens", "vem", "vimos", "vindes", "vêm"],
        ["venha", "venhas", "venha", "venhamos", "venhais", "venham"],
    ),
    (
        ["sei", "sabes", "sabe", "sabemos", "sabeis", "sabem"],
        ["saiba", "saibas", "saiba", "saibamos", "saibais", "saibam"],
    ),
    (
        ["digo", "dizes", "diz", "dizemos", "dizeis", "dizem"],
        ["diga", "digas", "diga", "digamos", "digais", "digam"],
    ),
    (
        ["uso", "usas", "usa", "usamos", "usais", "usam"],
        ["use", "uses", "use", "usemos", "useis", "usem"],
    ),
    (
        [
            "guardo",
            "guardas",
            "guarda",
            "guardamos",
            "guardais",
            "guardam",
        ],
        [
            "guarde",
            "guardes",
            "guarde",
            "guardemos",
            "guardeis",
            "guardem",
        ],
    ),
    (
        [
            "preciso",
            "precisas",
            "precisa",
            "precisamos",
            "precisais",
            "precisam",
        ],
        [
            "precise",
            "precises",
            "precise",
            "precisemos",
            "preciseis",
            "precisem",
        ],
    ),
    (
        [
            "funciono",
            "funcionas",
            "funciona",
            "funcionamos",
            "funcionais",
            "funcionam",
        ],
        [
            "funcione",
            "funciones",
            "funcione",
            "funcionemos",
            "funcioneis",
            "funcionem",
        ],
    ),
    (
        [
            "permito",
            "permites",
            "permite",
            "permitimos",
            "permitis",
            "permitem",
        ],
        [
            "permita",
            "permitas",
            "permita",
            "permitamos",
            "permitais",
            "permitam",
        ],
    ),
    (
        [
            "existo",
            "existes",
            "existe",
            "existimos",
            "existis",
            "existem",
        ],
        [
            "exista",
            "existas",
            "exista",
            "existamos",
            "existais",
            "existam",
        ],
    ),
    (
        [
            "escrevo",
            "escreves",
            "escreve",
            "escrevemos",
            "escreveis",
            "escrevem",
        ],
        [
            "escreva",
            "escrevas",
            "escreva",
            "escrevamos",
            "escrevais",
            "escrevam",
        ],
    ),
];
const PAST: &[[&str; 6]] = &[
    ["fui", "foste", "foi", "fomos", "fostes", "foram"],
    ["era", "eras", "era", "éramos", "éreis", "eram"],
    [
        "estava",
        "estavas",
        "estava",
        "estávamos",
        "estáveis",
        "estavam",
    ],
    ["tinha", "tinhas", "tinha", "tínhamos", "tínheis", "tinham"],
    ["havia", "havias", "havia", "havíamos", "havíeis", "haviam"],
];

fn clause_start(chars: &[char], ws: &[Word], i: usize) -> bool {
    let preceding = chars[..ws[i].start]
        .iter()
        .rev()
        .find(|c| !c.is_whitespace());
    preceding.is_none()
        || preceding.is_some_and(|c| matches!(c, '.' | '!' | '?' | ';' | ':' | ','))
        || (i > 0
            && adjacent(chars, &ws[i - 1], &ws[i])
            && matches!(
                ws[i - 1].lower.as_str(),
                "que" | "porque" | "quando" | "enquanto" | "se"
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
        if matches!(ws[v].lower.as_str(), "não" | "já" | "sempre") {
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
                // `vimos` is both present `vir` and past `ver`. A correction would
                // choose a tense without evidence, so leave that form alone.
                if ws[v].lower == "vimos" {
                    break;
                }
                add(
                    out,
                    enabled,
                    "PortuguesePronounVerbAgreement",
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
        // Contracted prepositions (`dos`, `nas`, `aos`) are never subjects.
        let Some(det) = DETERMINERS
            .iter()
            .take(11)
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
            || matches!(
                ws[n].lower.as_str(),
                "equipa" | "equipas" | "equipe" | "equipes"
            )
            || (pre.is_none()
                && matches!(ws[i].lower.as_str(), "o" | "a" | "os" | "as")
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
        if v < ws.len() && matches!(ws[v].lower.as_str(), "não" | "já" | "sempre") {
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
        // A following subject can license object fronting. Bare nouns and
        // names are possible subjects too, not just determiner-led phrases.
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
                || word == "tu"
                || DETERMINERS.iter().any(|forms| forms.contains(&word))
                || chars[ws[t].start].is_uppercase()
                || (!copular
                    && !ADJECTIVES.iter().any(|forms| forms.contains(&word))
                    && !matches!(
                        word,
                        "não"
                            | "já"
                            | "sempre"
                            | "aqui"
                            | "ali"
                            | "hoje"
                            | "ontem"
                            | "amanhã"
                            | "ainda"
                            | "bem"
                            | "mal"
                            | "corretamente"
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
            // First/second-person forms can have a pro-dropped subject after
            // a fronted object. Compare only third-person singular/plural.
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
                "PortugueseNounVerbAgreement",
                chars,
                &ws[v],
                fix,
                "The finite verb must agree with the explicit noun subject.",
            );
        }
    }
}

const FINITE_TO_INFINITIVE: &[(&str, &str)] = &[
    ("é", "ser"),
    ("são", "ser"),
    ("está", "estar"),
    ("estão", "estar"),
    ("tem", "ter"),
    ("têm", "ter"),
    ("faz", "fazer"),
    ("fazem", "fazer"),
    ("diz", "dizer"),
    ("dizem", "dizer"),
    ("escreve", "escrever"),
    ("escrevem", "escrever"),
    ("sabem", "saber"),
    ("usam", "usar"),
    ("guardam", "guardar"),
    ("permitem", "permitir"),
    ("funcionam", "funcionar"),
];

fn modals(chars: &[char], ws: &[Word], enabled: &[&str], out: &mut BTreeMap<String, Vec<Lint>>) {
    for i in 0..ws.len().saturating_sub(2) {
        if person(&ws[i].lower).is_none()
            || !clause_start(chars, ws, i)
            || !adjacent(chars, &ws[i], &ws[i + 1])
        {
            continue;
        }
        let modal = ws[i + 1].lower.as_str();
        if !VERBS[4].0.contains(&modal) && !VERBS[5].0.contains(&modal) {
            continue;
        }
        let mut v = i + 2;
        if !adjacent(chars, &ws[v - 1], &ws[v]) {
            continue;
        }
        if ws[v].lower == "não" {
            v += 1;
            if v >= ws.len() || !adjacent(chars, &ws[v - 1], &ws[v]) {
                continue;
            }
        }
        if let Some(&(_, infinitive)) = FINITE_TO_INFINITIVE
            .iter()
            .find(|&&(finite, _)| finite == ws[v].lower)
        {
            add(
                out,
                enabled,
                "PortugueseModalInfinitive",
                chars,
                &ws[v],
                infinitive,
                "After poder or dever, use the infinitive rather than a finite verb.",
            );
        }
    }
}

fn existential(
    chars: &[char],
    ws: &[Word],
    enabled: &[&str],
    out: &mut BTreeMap<String, Vec<Lint>>,
) {
    for i in 0..ws.len().saturating_sub(1) {
        if !adjacent(chars, &ws[i], &ws[i + 1]) {
            continue;
        }
        let fix = match ws[i].lower.as_str() {
            "haviam" => "havia",
            "houveram" => "houve",
            "haverão" => "haverá",
            "haveriam" => "haveria",
            "hajam" => "haja",
            "houvessem" => "houvesse",
            "houverem" => "houver",
            _ => continue,
        };
        let mut n = i + 1;
        if DETERMINERS
            .iter()
            .take(11)
            .any(|forms| forms.contains(&ws[n].lower.as_str()))
            || matches!(
                ws[n].lower.as_str(),
                "dois" | "duas" | "três" | "quatro" | "muitos" | "muitas" | "vários" | "várias"
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
                "PortugueseExistentialHaver",
                chars,
                &ws[i],
                fix,
                "Existential haver is impersonal and remains singular before a plural noun.",
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
        let fixed = matches!(ws[q - 1].lower.as_str(), "para" | "sem" | "antes")
            || (q >= 2
                && ws[q - 2].lower == "a"
                && ws[q - 1].lower == "menos"
                && adjacent(chars, &ws[q - 2], &ws[q - 1]))
            || (q >= 2
                && ws[q - 2].lower == "é"
                && matches!(
                    ws[q - 1].lower.as_str(),
                    "necessário" | "preciso" | "importante" | "essencial" | "fundamental"
                )
                && adjacent(chars, &ws[q - 2], &ws[q - 1]));
        if !fixed || !adjacent(chars, &ws[q], &ws[q + 1]) {
            continue;
        }
        let mut v = q + 1;
        let p = person(&ws[v].lower);
        // tu can still be an explicit subject here without choosing a regional
        // agreement: infer the person from the observed indicative instead.
        if p.is_some() || ws[v].lower == "tu" {
            v += 1;
            if v >= ws.len() || !adjacent(chars, &ws[v - 1], &ws[v]) {
                continue;
            }
        }
        if ws[v].lower == "não" {
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
                    "PortugueseRequiredSubjunctive",
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
    if enabled.contains(&"PortugueseArticleAgreement")
        || enabled.contains(&"PortugueseAdjectiveAgreement")
    {
        noun_phrases(chars, &ws, enabled, &mut out);
    }
    if enabled.contains(&"PortuguesePronounVerbAgreement") {
        pronoun_verbs(chars, &ws, enabled, &mut out);
    }
    if enabled.contains(&"PortugueseNounVerbAgreement") {
        noun_verbs(chars, &ws, enabled, &mut out);
    }
    if enabled.contains(&"PortugueseModalInfinitive") {
        modals(chars, &ws, enabled, &mut out);
    }
    if enabled.contains(&"PortugueseExistentialHaver") {
        existential(chars, &ws, enabled, &mut out);
    }
    if enabled.contains(&"PortugueseRequiredSubjunctive") {
        subjunctive(chars, &ws, enabled, &mut out);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixes(rule: &str, text: &str) -> Vec<(String, String)> {
        let sp = crate::rules::spell_lang::speller("pt", &crate::config::Config::default())
            .expect("bundled Portuguese");
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
        let rule = "PortugueseNounVerbAgreement";
        let sp = crate::rules::spell_lang::speller("pt", &crate::config::Config::default())
            .expect("bundled Portuguese");
        for (text, start, end, replacement) in [
            ("Os sistemas está disponíveis.", 12, 16, "estão"),
            ("O sistema estão disponível.", 10, 15, "está"),
            ("Um ficheiro são suficiente.", 12, 15, "é"),
            ("Uns arquivos é suficientes.", 13, 14, "são"),
            ("Espero que as páginas seja acessíveis.", 22, 26, "sejam"),
            ("A versão estavam disponível.", 9, 16, "estava"),
            ("As novas imagens não está disponíveis.", 21, 25, "estão"),
            ("Uma página importante eram suficiente.", 22, 26, "era"),
            ("Os sistemas funciona corretamente.", 12, 20, "funcionam"),
            ("É possível que o sistema existam ainda.", 25, 32, "exista"),
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
    fn noun_subject_ambiguities_and_regional_forms_remain_untouched() {
        quiet(
            "PortugueseNounVerbAgreement",
            &[
                "É possível que os sistemas estejam disponíveis.",
                "Talvez uma página seja suficiente.",
                "Estão disponíveis as páginas?",
                "O arquivo e a pasta estão disponíveis.",
                "O arquivo, a pasta estão disponíveis.",
                "A maioria dos sistemas estão disponíveis.",
                "A equipe estão muito unidos.",
                "A equipa estão muito unidos.",
                "Nos sistemas está disponível.",
                "Dos arquivos vem a resposta.",
                "Às páginas é dedicada a atenção.",
                "Para as páginas é suficiente.",
                "Um ficheiro guardam os utilizadores.",
                "Um ficheiro guardam eles.",
                "Um ficheiro guardam Maria e João.",
                "Um ficheiro guardam utilizadores.",
                "Um ficheiro guardam McDonald e João.",
                "Um ficheiro guardam imagens.",
                "Os ficheiros guardamos aqui.",
                "As páginas tens aqui.",
                "Tu tem acesso.",
                "Tu tens acesso.",
                "Você tem acesso.",
                "A gente tem acesso.",
                "Os sistemas vão funcionar.",
                "Os sistemas vimos ontem.",
                "Eu as documento com cuidado.",
                "Eu a arquivo sempre.",
                "Uns ficheiros guarda-se aqui.",
                "«Os sistemas está disponíveis» é uma citação.",
                "`A versão estavam disponível` é código.",
                "Os sistemas_está disponíveis.",
                "foo.o sistema estão disponível.",
            ],
        );
    }

    #[test]
    fn lexical_gender_number_and_unicode_spans() {
        assert_eq!(
            fixes("PortugueseArticleAgreement", "Também vejo a problema."),
            [("a".into(), "o".into())]
        );
        assert_eq!(
            fixes("PortugueseArticleAgreement", "Um imagem aparece."),
            [("Um".into(), "Uma".into())]
        );
        assert_eq!(
            fixes("PortugueseAdjectiveAgreement", "As páginas novo funcionam."),
            [("novo".into(), "novas".into())]
        );
        assert_eq!(
            fixes("PortugueseAdjectiveAgreement", "A novos versão funciona."),
            [("novos".into(), "nova".into())]
        );
        quiet(
            "PortugueseArticleAgreement",
            &[
                "O sistema funciona.",
                "A mão está fria.",
                "Uma estudante trabalha.",
                "Os grandes ficheiros estão disponíveis.",
                "A equipe trabalha.",
                "A equipa trabalha.",
                "A pedido do cliente, enviamos o relatório.",
                "A dado momento, o sistema parou.",
                "O erro refere-se a problema conhecido.",
                "Eu as documento com cuidado.",
            ],
        );
        quiet(
            "PortugueseArticleAgreement",
            &[
                "Tu os contas todos os dias.",
                "Eu as contrato amanhã.",
                "Eu a livro de qualquer obrigação.",
                "Eu as projeto com cuidado.",
            ],
        );
        quiet(
            "PortugueseAdjectiveAgreement",
            &[
                "As páginas importantes funcionam.",
                "O arquivo e a pasta novos estão disponíveis.",
                "Os ficheiros activos estão disponíveis.",
                "Os arquivos ativos estão disponíveis.",
                "A página do documento novo funciona.",
            ],
        );
        quiet(
            "PortugueseAdjectiveAgreement",
            &["O arquivo, a pasta novos estão disponíveis."],
        );
    }

    #[test]
    fn subjects_and_regional_variants() {
        assert_eq!(
            fixes("PortuguesePronounVerbAgreement", "Eles tem acesso."),
            [("tem".into(), "têm".into())]
        );
        assert_eq!(
            fixes(
                "PortuguesePronounVerbAgreement",
                "Sei que nós é responsáveis."
            ),
            [("é".into(), "somos".into())]
        );
        quiet(
            "PortuguesePronounVerbAgreement",
            &[
                "Têm eles acesso?",
                "Ela e ele têm acesso.",
                "A maioria deles tem acesso.",
                "A gente tem acesso.",
                "Para eles é importante.",
                "Eu tinha acesso.",
                "Tu tens acesso.",
                "Tu tem acesso.",
                "Você tem acesso.",
                "Vós tendes acesso.",
                "Nós vimos os resultados.",
                "Nós, os responsáveis, temos acesso.",
            ],
        );
    }

    #[test]
    fn modals_and_existential_haver() {
        assert_eq!(
            fixes("PortugueseModalInfinitive", "Nós podemos fazem isso."),
            [("fazem".into(), "fazer".into())]
        );
        assert_eq!(
            fixes("PortugueseModalInfinitive", "Ele deve não escreve aqui."),
            [("escreve".into(), "escrever".into())]
        );
        assert_eq!(
            fixes("PortugueseExistentialHaver", "Haviam vários erros."),
            [("Haviam".into(), "Havia".into())]
        );
        assert_eq!(
            fixes("PortugueseExistentialHaver", "Houveram problemas."),
            [("Houveram".into(), "Houve".into())]
        );
        quiet(
            "PortugueseModalInfinitive",
            &[
                "Nós podemos fazer isso.",
                "Ele pode, escreve a autora, sair.",
                "Pode ele escrever aqui?",
                "Nós temos guardadas as imagens.",
            ],
        );
        quiet(
            "PortugueseExistentialHaver",
            &[
                "Eles haviam recebido respostas.",
                "Eles houveram por bem sair.",
                "Havia muitos erros.",
                "Tem muitos arquivos aqui.",
                "Existem problemas.",
                "Se eles houverem terminado, sairemos.",
            ],
        );
    }

    #[test]
    fn ordinary_and_formal_prose_remains_clean() {
        let examples = [
            "Se vocês têm dúvidas, a resposta está nas páginas novas.",
            "Nós temos recebido os pedidos e elas têm enviado os documentos.",
            "É necessário que vocês tenham acesso para que possam usar o serviço.",
            "A maioria dos utilizadores tem uma conta e há vários problemas pendentes.",
            "O arquivo e a nova pasta importantes estão disponíveis.",
            "Antes que estejam prontos os relatórios, ela quer rever a configuração.",
            "A equipe pode usar os arquivos ativos e a equipa pode usar os ficheiros activos.",
            "'eles tem' é uma transcrição literal.",
        ];
        for (rule, _) in RULES {
            quiet(rule, &examples);
        }
    }

    #[test]
    fn subjunctive_triggers_and_verbatim_boundaries() {
        assert_eq!(
            fixes(
                "PortugueseRequiredSubjunctive",
                "É necessário que ela tem acesso."
            ),
            [("tem".into(), "tenha".into())]
        );
        assert_eq!(
            fixes(
                "PortugueseRequiredSubjunctive",
                "Explico para que vocês sabem usar."
            ),
            [("sabem".into(), "saibam".into())]
        );
        quiet(
            "PortugueseRequiredSubjunctive",
            &[
                "Sei que ela tem acesso.",
                "É necessário que tenham acesso.",
                "Explico para que saibam usar.",
                "Antes que chegue ela, sairemos.",
                "É necessário, que ele tem dúvidas é evidente.",
                "Para que nós vamos juntos, basta combinar.",
            ],
        );
        for rule in RULES.iter().map(|(name, _)| *name) {
            quiet(
                rule,
                &[
                    "«Eles tem acesso» é uma citação.",
                    "\"a problema\" é uma citação.",
                    "`eles tem` é código.",
                    "eles_tem é um identificador.",
                    "A, problema.",
                ],
            );
        }
    }
}
