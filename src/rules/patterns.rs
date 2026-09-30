// explicit-disable-file grammar/* slop/* prose/* -- comments quote the mistakes these rules detect
//! Our own tokenizer and pattern-only grammar rules, for engines that do not run Harper.
//!
//! Spans are char indices into the segment text (as Harper's are), so findings flow through the
//! same filters as Harper's. Rule names match Harper's, so `grammar/<Name>` config keeps working.

use std::collections::BTreeMap;

use super::lint::{Lint, LintKind, Span, Suggestion};
use super::words::words;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Letters, with inner apostrophes, dots, underscores or `@` (`don't`, `e.g`, `foo.rs`).
    Word,
    /// Digits (with inner `.`/`,`) and a letter suffix starting at `suffix` (`2nd`, `10.5s`).
    Number {
        suffix: usize,
    },
    /// Spaces and tabs.
    Space,
    Newline,
    /// URLs and other chunks nothing should look at.
    Unlintable,
    Punct(char),
}

#[derive(Debug, Clone, Copy)]
pub struct Token {
    pub start: usize,
    pub end: usize,
    pub kind: Kind,
}

impl Token {
    pub fn text(&self, chars: &[char]) -> String {
        chars[self.start..self.end].iter().collect()
    }
}

fn joiner(c: char) -> bool {
    matches!(c, '\'' | '’' | '.' | '_' | '@')
}

/// Split `chars` into words, numbers, whitespace and punctuation.
pub fn tokenize(chars: &[char]) -> Vec<Token> {
    let mut out = Vec::new();
    let mut i = 0;
    let n = chars.len();
    while i < n {
        let c = chars[i];
        let start = i;
        let chunk_start = i == 0 || chars[i - 1].is_whitespace();
        if chunk_start && !c.is_whitespace() {
            let end = (i..n).find(|&j| chars[j].is_whitespace()).unwrap_or(n);
            let chunk: String = chars[i..end].iter().collect();
            if chunk.contains("://") || chunk.starts_with("www.") {
                out.push(Token {
                    start,
                    end,
                    kind: Kind::Unlintable,
                });
                i = end;
                continue;
            }
        }
        let kind = if c == '\n' {
            i += 1;
            Kind::Newline
        } else if c.is_whitespace() {
            while i < n && chars[i].is_whitespace() && chars[i] != '\n' {
                i += 1;
            }
            Kind::Space
        } else if c.is_ascii_digit() {
            while i < n
                && (chars[i].is_ascii_digit()
                    || (matches!(chars[i], '.' | ',')
                        && chars.get(i + 1).is_some_and(char::is_ascii_digit)))
            {
                i += 1;
            }
            let suffix = i;
            while i < n && chars[i].is_alphabetic() {
                i += 1;
            }
            Kind::Number { suffix }
        } else if c.is_alphabetic() {
            while i < n
                && (chars[i].is_alphanumeric()
                    || (joiner(chars[i]) && chars.get(i + 1).is_some_and(|c| c.is_alphanumeric())))
            {
                i += 1;
            }
            Kind::Word
        } else {
            i += 1;
            Kind::Punct(c)
        };
        out.push(Token {
            start,
            end: i,
            kind,
        });
    }
    out
}

/// Pattern rules this module implements (Harper's names).
pub const RULES: &[&str] = &[
    "AnA",
    "RepeatedWords",
    "ThenThan",
    "Didnt",
    "TheMy",
    "CorrectNumberSuffix",
    "NumberSuffixCapitalization",
    "PronounVerbAgreement",
];

/// Only whitespace (at most one line break) between tokens `a` and `b`.
fn spaced(tokens: &[Token], a: usize, b: usize) -> bool {
    b > a + 1
        && tokens[a + 1..b]
            .iter()
            .all(|t| matches!(t.kind, Kind::Space | Kind::Newline))
        && tokens[a + 1..b]
            .iter()
            .filter(|t| t.kind == Kind::Newline)
            .count()
            <= 1
}

/// Lints of the rules in `on` (names from [`RULES`]) for one segment text.
pub fn lint(
    chars: &[char],
    tokens: &[Token],
    on: &[&str],
    american: bool,
) -> BTreeMap<String, Vec<Lint>> {
    let mut out = BTreeMap::new();
    let words: Vec<usize> = (0..tokens.len())
        .filter(|&i| tokens[i].kind == Kind::Word)
        .collect();
    let lower: Vec<String> = tokens
        .iter()
        .map(|t| t.text(chars).to_lowercase())
        .collect();
    let spaced = |a: usize, b: usize| spaced(tokens, a, b);
    let pairs: Vec<(usize, usize)> = words
        .windows(2)
        .map(|w| (w[0], w[1]))
        .filter(|&(a, b)| spaced(a, b))
        .collect();
    let mut push = |name: &str, lint: Lint| {
        out.entry(name.to_string())
            .or_insert_with(Vec::new)
            .push(lint)
    };
    let cx = on
        .iter()
        .any(|n| CONFUSABLES.contains(n) || *n == "PronounVerbAgreement")
        .then(|| Cx::new(chars, tokens, &words, &lower, &spaced));
    let span = |a: usize, b: usize| Span::new(tokens[a].start, tokens[b].end);
    let replace = |s: &str| Suggestion::ReplaceWith(s.chars().collect());

    for name in on {
        match *name {
            "AnA" => {
                for &(a, b) in &pairs {
                    let first = tokens[a].text(chars);
                    let is_a = match first.as_str() {
                        "a" | "A" => true,
                        "an" | "An" => false,
                        _ => continue,
                    };
                    let second = tokens[b].text(chars);
                    // Acronyms (`a/an SQL`, `a/an HTTP`, `SQLite`) are read either way, and
                    // identifiers (`u16`, `RST_STREAM`, `foo.rs`) have no settled reading.
                    if second.chars().filter(|c| c.is_uppercase()).count() >= 2
                        || second.contains(['_', '.', '@'])
                        || second.chars().any(|c| c.is_ascii_digit())
                    {
                        continue;
                    }
                    if first == "A" {
                        // `an A`, `a A`: a letter. `A-B` / `A-F`: grades.
                        let prev = words.iter().rev().find(|&&w| w < a);
                        if prev.is_some_and(|&p| matches!(lower[p].as_str(), "a" | "an")) {
                            continue;
                        }
                    }
                    let word: String = second
                        .split(|c: char| !c.is_alphanumeric())
                        .next()
                        .unwrap_or("")
                        .to_string();
                    let want_a = match initial_sound(&word, american) {
                        Sound::Vowel => false,
                        Sound::Consonant => true,
                        Sound::Either => continue,
                    };
                    if want_a != is_a {
                        let fix = match (is_a, first.starts_with('A')) {
                            (true, true) => "An",
                            (true, false) => "an",
                            (false, true) => "A",
                            (false, false) => "a",
                        };
                        push(
                            "AnA",
                            Lint {
                                span: span(a, a),
                                lint_kind: LintKind::Miscellaneous,
                                suggestions: vec![replace(fix)],
                                message: "Incorrect indefinite article.".to_owned(),
                                priority: 31,
                            },
                        );
                    }
                }
            }
            "RepeatedWords" => {
                for &(a, b) in &pairs {
                    if lower[a] != lower[b] || REPEAT_OK.contains(&lower[a].as_str()) {
                        continue;
                    }
                    let prev = words
                        .iter()
                        .rev()
                        .find(|&&w| w < a)
                        .filter(|&&p| spaced(p, a))
                        .map(|&p| lower[p].as_str());
                    // `what it is is`; but `the library is is`.
                    if lower[a] == "is"
                        && matches!(prev, Some("it" | "this" | "that" | "what" | "which"))
                    {
                        continue;
                    }
                    // `No no, ...` / `Now now, ...` open a sentence; mid-sentence they are typos.
                    if matches!(lower[a].as_str(), "no" | "now" | "yes")
                        && (prev.is_none() || chars[tokens[a].start].is_uppercase())
                    {
                        continue;
                    }
                    let hyphen = |i: Option<&Token>| i.is_some_and(|t| t.kind == Kind::Punct('-'));
                    if hyphen(a.checked_sub(1).and_then(|i| tokens.get(i)))
                        || hyphen(tokens.get(b + 1))
                        || lower[a].contains(['.', '_', '@'])
                    {
                        continue;
                    }
                    push(
                        "RepeatedWords",
                        Lint {
                            span: span(a, b),
                            lint_kind: LintKind::Repetition,
                            suggestions: vec![Suggestion::ReplaceWith(
                                chars[tokens[a].start..tokens[a].end].to_vec(),
                            )],
                            message: "Did you mean to repeat this word?".to_owned(),
                            priority: 128,
                        },
                    );
                }
            }
            "ThenThan" => {
                for (k, &(a, b)) in pairs.iter().enumerate() {
                    if lower[b] != "then" {
                        continue;
                    }
                    // `then` must be followed by whitespace and something other than `that`
                    // (a word, or code: `longer then ``timeout```).
                    let blank = |t: &Token| matches!(t.kind, Kind::Space | Kind::Newline);
                    if !tokens.get(b + 1).is_some_and(blank) {
                        continue;
                    }
                    let Some(c) = (b + 2..tokens.len()).find(|&i| !blank(&tokens[i])) else {
                        continue;
                    };
                    let w = lower[a].as_str();
                    let ok_next = match tokens[c].kind {
                        Kind::Word => lower[c] != "that" || w == "other",
                        Kind::Punct(p) => matches!(p, '`' | '\'' | '"' | '(' | '['),
                        Kind::Number { .. } => true,
                        _ => false,
                    };
                    if !ok_next {
                        continue;
                    }
                    let before = pairs[..k]
                        .iter()
                        .rev()
                        .find(|(_, y)| *y == a)
                        .map(|&(x, _)| lower[x].as_str());
                    let comparative = COMPARATIVES.contains(&w)
                        || (matches!(before, Some("more" | "less"))
                            && (adjective_like(w) || MORE_ADJECTIVES.contains(&w)))
                        || w == "rather"
                        || (w == "other"
                            && before.is_some_and(|b| {
                                matches!(
                                    b,
                                    "no" | "any"
                                        | "anything"
                                        | "something"
                                        | "nothing"
                                        | "everything"
                                        | "anyone"
                                        | "someone"
                                        | "everyone"
                                        | "nobody"
                                        | "none"
                                )
                            }));
                    // `more memory then the old one`, `fewer lines of code then before`: a
                    // comparative up to six words back, over nouns and adjectives only, and no
                    // verb after `then` (`add more tests then run them`, `more tests then the CI
                    // will pass`: a sequence).
                    let window = !comparative && {
                        let mut j = a;
                        let mut found = false;
                        for _ in 0..6 {
                            let wj = lower[j].as_str();
                            let l = lex(wj);
                            let filler = wj == "of"
                                || ((l.noun || l.adj)
                                    && (!l.finite || l.plural)
                                    && !l.function
                                    && !l.ing
                                    && !l.part
                                    && !AUX.contains(&wj)
                                    && !wj.contains('\''));
                            if !filler {
                                break;
                            }
                            let Some(&(x, _)) = pairs[..k].iter().rev().find(|(_, y)| *y == j)
                            else {
                                break;
                            };
                            let wx = lower[x].as_str();
                            if matches!(wx, "more" | "less" | "fewer") || COMPARATIVES.contains(&wx)
                            {
                                // `one or more letters then whitespace`: a quantity.
                                let before = pairs[..k]
                                    .iter()
                                    .rev()
                                    .find(|(_, y)| *y == x)
                                    .map(|&(v, _)| lower[v].as_str());
                                found = !matches!(before, Some("or" | "and" | "one" | "two"));
                                break;
                            }
                            j = x;
                        }
                        let mut after = Vec::new();
                        let mut y = b;
                        while after.len() < 5 {
                            let Some(&(_, z)) = pairs.iter().find(|(x, _)| *x == y) else {
                                break;
                            };
                            after.push(lower[z].as_str());
                            y = z;
                        }
                        let verb_after = after.first().is_some_and(|f| {
                            let l = lex(f);
                            l.verb
                                && !l.function
                                && !l.adj
                                && !matches!(
                                    *f,
                                    "expected"
                                        | "needed"
                                        | "required"
                                        | "necessary"
                                        | "planned"
                                        | "anticipated"
                                        | "intended"
                                        | "advertised"
                                        | "allowed"
                                        | "average"
                                        | "before"
                                )
                        }) || after
                            .iter()
                            .any(|f| AUX.contains(f) || f.contains('\'') || lex(f).finite);
                        // `slice it`, `drop the rest`: a verb and its object.
                        let verb_object = after.first().is_some_and(|f| lex(f).verb)
                            && after.get(1).is_some_and(|o| {
                                matches!(
                                    *o,
                                    "it" | "them"
                                        | "the"
                                        | "a"
                                        | "an"
                                        | "this"
                                        | "that"
                                        | "these"
                                        | "those"
                                        | "its"
                                        | "their"
                                        | "our"
                                        | "your"
                                        | "my"
                                )
                            });
                        // After a condition (`if ... lower limit then the earlier test has
                        // caught it`) a short phrase must close the comparison; `first ... then`
                        // is a sequence.
                        let back = |set: &[&str]| {
                            (a.saturating_sub(40)..a).any(|t| set.contains(&lower[t].as_str()))
                        };
                        let conditional = back(&["if", "when", "whenever", "unless"]);
                        found
                            && !verb_after
                            && !verb_object
                            && !back(&["first", "firstly"])
                            && (!conditional || after.len() <= 3)
                            && lower[a] != "one"
                    };
                    if !comparative && !window {
                        continue;
                    }
                    let fix = if tokens[b].text(chars).starts_with('T') {
                        "Than"
                    } else {
                        "than"
                    };
                    push(
                        "ThenThan",
                        Lint {
                            span: span(b, b),
                            lint_kind: LintKind::Miscellaneous,
                            suggestions: vec![replace(fix)],
                            message: "Did you mean `than`?".to_owned(),
                            priority: 31,
                        },
                    );
                }
                if let Some(cx) = &cx {
                    confusables("ThenThan", cx, &mut push);
                }
            }
            "Didnt" => {
                for &(a, b) in &pairs {
                    if lower[b] == "dint"
                        && matches!(
                            lower[a].as_str(),
                            "i" | "you" | "he" | "she" | "it" | "we" | "they"
                        )
                    {
                        push(
                            "Didnt",
                            Lint {
                                span: span(b, b),
                                lint_kind: LintKind::Typo,
                                suggestions: vec![replace("didn't")],
                                message: "Consider using `didn't` here.".to_owned(),
                                priority: 63,
                            },
                        );
                    }
                }
            }
            "TheMy" => {
                const POSS: &[&str] = &["my", "your", "his", "her", "its", "our", "their"];
                for &(a, b) in &pairs {
                    let (x, y) = (lower[a].as_str(), lower[b].as_str());
                    let the_poss = x == "the" && POSS.contains(&y);
                    // `your the best`, `their the kind`, `its the`: contractions, flagged by
                    // `YourYoure`, `TheirToTheyre` and `ItsContraction`.
                    let poss_the = POSS.contains(&x)
                        && y == "the"
                        && !matches!(x, "her" | "your" | "their" | "its");
                    let second_upper = chars[tokens[b].start].is_uppercase();
                    if (the_poss || poss_the) && !second_upper {
                        let poss = if the_poss { &tokens[b] } else { &tokens[a] };
                        push(
                            "TheMy",
                            Lint {
                                span: span(a, b),
                                lint_kind: LintKind::Repetition,
                                suggestions: vec![
                                    Suggestion::ReplaceWith(chars[poss.start..poss.end].to_vec()),
                                    replace("the"),
                                ],
                                message: "Use either the definite article 'the' or the possessive. Using both together is ungrammatical in English.".to_owned(),
                                priority: 127,
                            },
                        );
                    }
                }
            }
            "CorrectNumberSuffix" | "NumberSuffixCapitalization" => {
                for t in tokens {
                    let Kind::Number { suffix } = t.kind else {
                        continue;
                    };
                    if t.end - suffix != 2 {
                        continue;
                    }
                    let suf: String = chars[suffix..t.end].iter().collect();
                    let suf_lower = suf.to_lowercase();
                    if !matches!(suf_lower.as_str(), "st" | "nd" | "rd" | "th") {
                        continue;
                    }
                    let digits: String = chars[t.start..suffix]
                        .iter()
                        .filter(|c| c.is_ascii_digit())
                        .collect();
                    let sp = Span::new(suffix, t.end);
                    if *name == "CorrectNumberSuffix" {
                        if chars[t.start..suffix].contains(&'.') {
                            continue;
                        }
                        let Ok(v) = digits.parse::<u64>() else {
                            continue;
                        };
                        let correct = match (v % 100, v % 10) {
                            (11..=13, _) => "th",
                            (_, 1) => "st",
                            (_, 2) => "nd",
                            (_, 3) => "rd",
                            _ => "th",
                        };
                        if suf_lower != correct {
                            push(
                                name,
                                Lint {
                                    span: sp,
                                    lint_kind: LintKind::Miscellaneous,
                                    suggestions: vec![replace(correct)],
                                    message: "This number needs a different suffix to sound right."
                                        .to_owned(),
                                    ..Default::default()
                                },
                            );
                        }
                    } else if suf.chars().any(char::is_uppercase) {
                        push(
                            name,
                            Lint {
                                span: sp,
                                lint_kind: LintKind::Capitalization,
                                suggestions: vec![replace(&suf_lower)],
                                message: "This suffix should be lowercase".to_owned(),
                                ..Default::default()
                            },
                        );
                    }
                }
            }
            "PronounVerbAgreement" => {
                let Some(cx) = &cx else { continue };
                for k in 0..cx.wl.len() {
                    let Some(fix) = pronoun_verb(cx, k) else {
                        continue;
                    };
                    let t = tokens[cx.words[k + 1]];
                    push(
                        "PronounVerbAgreement",
                        Lint {
                            span: Span::new(t.start, t.end),
                            lint_kind: LintKind::Grammar,
                            suggestions: vec![replace(&match_case(&t.text(chars), &fix))],
                            message: "The form of the verb must agree in grammatical number with the pronoun.".to_owned(),
                            priority: 31,
                        },
                    );
                }
            }
            "SubjectVerbAgreement" => {
                let Some(cx) = &cx else { continue };
                for v in 0..cx.wl.len() {
                    let Some(fix) = subject_verb(cx, v) else {
                        continue;
                    };
                    let t = tokens[cx.words[v]];
                    let orig = t.text(chars);
                    let mut fix = match_case(&orig, &fix);
                    if orig.contains('’') {
                        fix = fix.replace('\'', "’");
                    }
                    push(
                        "SubjectVerbAgreement",
                        Lint {
                            span: Span::new(t.start, t.end),
                            lint_kind: LintKind::Grammar,
                            suggestions: vec![replace(&fix)],
                            message: "The form of the verb must agree in grammatical number with its subject.".to_owned(),
                            priority: 31,
                        },
                    );
                }
            }
            n if CONFUSABLES.contains(&n) => {
                if let Some(cx) = &cx {
                    confusables(n, cx, &mut push);
                }
            }
            _ => {}
        }
    }
    out
}

/// Word-confusion rules that run in every engine, Harper's included (where both flag a word,
/// overlap removal keeps one). Harper's names where Harper has the rule, our own otherwise.
pub const CONFUSABLES: &[&str] = &[
    "ThenThan",
    "YourYoure",
    "ThereOwn",
    "ThereToTheir",
    "TheirToThere",
    "TheirToTheyre",
    "TheyreToTheir",
    "LoseLoose",
    "AffectEffect",
    "WhoseWhos",
    "WeatherWhether",
    "PrincipalPrinciple",
    "ComplimentComplement",
    "ItsContraction",
    "ItsPossessive",
    "WitchWhich",
    "ExceptAccept",
    "WriteRight",
    "SubjectVerbAgreement",
];

/// Our [`RULES`] and [`CONFUSABLES`] that carry a Harper rule's name, described for
/// `explicit rules --all` in builds without Harper (which lists Harper's own descriptions).
pub const HARPER_NAMED: &[(&str, &str)] = &[
    (
        "AnA",
        "Flags `a` before a vowel sound and `an` before a consonant sound.",
    ),
    (
        "RepeatedWords",
        "Flags a word written twice in a row (`the the`).",
    ),
    (
        "ThenThan",
        "Flags `then` in comparisons (`better then`) and `than` for time (`since than`).",
    ),
    (
        "Didnt",
        "Flags `dint` and similar misspellings of `didn't`.",
    ),
    (
        "TheMy",
        "Flags two determiners in a row (`the my`, `my the`).",
    ),
    (
        "CorrectNumberSuffix",
        "Flags a wrong ordinal suffix (`1th`, `2rd`).",
    ),
    (
        "NumberSuffixCapitalization",
        "Flags an uppercase ordinal suffix (`2ND`).",
    ),
    (
        "PronounVerbAgreement",
        "Flags a verb that does not agree with the pronoun before it (`they runs`, `she go`).",
    ),
    ("ThereOwn", "Flags `there own` where `their own` is meant."),
    (
        "ThereToTheir",
        "Flags `there` before a noun where the possessive `their` is meant.",
    ),
    (
        "TheirToThere",
        "Flags `their` where the place or existential `there` is meant.",
    ),
    (
        "TheirToTheyre",
        "Flags `their` where `they're` (they are) is meant.",
    ),
    (
        "TheyreToTheir",
        "Flags `they're` before a noun where `their` is meant.",
    ),
    (
        "ItsContraction",
        "Flags `its` where `it's` (it is / it has) is meant.",
    ),
    (
        "ItsPossessive",
        "Flags `it's` before a noun where the possessive `its` is meant.",
    ),
];

/// Descriptions of the [`CONFUSABLES`] Harper does not have, for `explicit rules --all`.
pub const OWN_DESCRIPTIONS: &[(&str, &str)] = &[
    (
        "YourYoure",
        "Flags `you're` where the possessive `your` is meant (`you're own`, `you're code`) and `your` where `you're` is meant (`your not`, `your going to`).",
    ),
    (
        "LoseLoose",
        "Flags `loose` used as a verb (`will loose data`, `you loose it`); the verb is `lose`.",
    ),
    (
        "AffectEffect",
        "Flags `effect` used as a verb (`doesn't effect`, `effected by`) and `affect` used as a noun (`no affect`, `side affects`).",
    ),
    (
        "WhoseWhos",
        "Flags `who's` before a noun (`who's timestamp is`) and `whose` before a verb (`whose going`).",
    ),
    (
        "WeatherWhether",
        "Flags `weather` introducing a clause (`check weather it`, `weather or not`); the conjunction is `whether`.",
    ),
    (
        "PrincipalPrinciple",
        "Flags `principle` as an adjective (`principle risk`) and `principal` as a rule (`principal of least privilege`, `in principal`).",
    ),
    (
        "ComplimentComplement",
        "Flags `compliment` meaning `go well with` (`compliment each other`, `two's compliment`); that is `complement`.",
    ),
    (
        "WitchWhich",
        "Flags `witch` used as the relative pronoun (`a list witch is`, `in witch case`); that is `which`.",
    ),
    (
        "ExceptAccept",
        "Flags `except` used as the verb `accept` (`we will except the terms`) and `accept for` meaning `except for`.",
    ),
    (
        "WriteRight",
        "Flags `write` where `right` is meant (`that's write`, `the write way`, `turn write`).",
    ),
    (
        "SubjectVerbAgreement",
        "Flags a verb whose number does not match the noun subject right before it (`the server run`, `the files contains`, `the results is`, `each of the tests are`).",
    ),
];

/// Words and links between them, for the confusable rules.
struct Cx<'a> {
    chars: &'a [char],
    tokens: &'a [Token],
    /// Token index of each word.
    words: &'a [usize],
    /// Lowercased word text, curly apostrophes straightened.
    wl: Vec<String>,
    /// Word `k` and word `k + 1` are separated by whitespace only.
    link: Vec<bool>,
}

impl<'a> Cx<'a> {
    fn new(
        chars: &'a [char],
        tokens: &'a [Token],
        words: &'a [usize],
        lower: &[String],
        spaced: &dyn Fn(usize, usize) -> bool,
    ) -> Self {
        let wl = words.iter().map(|&i| lower[i].replace('’', "'")).collect();
        let link = (0..words.len())
            .map(|k| k + 1 < words.len() && spaced(words[k], words[k + 1]))
            .collect();
        Cx {
            chars,
            tokens,
            words,
            wl,
            link,
        }
    }

    /// The word `n` places after word `k`, through whitespace only.
    fn next(&self, k: usize, n: usize) -> Option<&str> {
        (0..n)
            .all(|i| self.link.get(k + i).copied().unwrap_or(false))
            .then(|| self.wl[k + n].as_str())
    }

    /// The word `n` places before word `k`, through whitespace only.
    fn prev(&self, k: usize, n: usize) -> Option<&str> {
        (k >= n && (k - n..k).all(|i| self.link[i])).then(|| self.wl[k - n].as_str())
    }

    /// First non-space token after word `k`.
    fn after(&self, k: usize) -> Option<Kind> {
        self.tokens[self.words[k] + 1..]
            .iter()
            .find(|t| t.kind != Kind::Space)
            .map(|t| t.kind)
    }

    /// First non-space token before word `k`.
    fn before(&self, k: usize) -> Option<Kind> {
        self.tokens[..self.words[k]]
            .iter()
            .rev()
            .find(|t| t.kind != Kind::Space)
            .map(|t| t.kind)
    }

    /// Word `k` is directly followed by a hyphen (`not-null`, `always-on`).
    fn hyphenated(&self, k: usize) -> bool {
        if k >= self.words.len() {
            return false;
        }
        self.tokens
            .get(self.words[k] + 1)
            .is_some_and(|t| t.kind == Kind::Punct('-'))
            || self.words[k]
                .checked_sub(1)
                .and_then(|i| self.tokens.get(i))
                .is_some_and(|t| t.kind == Kind::Punct('-'))
    }

    /// Word `k` ends its clause: sentence punctuation or the end of the text follows.
    fn clause_end(&self, k: usize) -> bool {
        match self.after(k) {
            None => true,
            Some(Kind::Punct('.' | '!' | '?' | ';')) => !self.punct_starts_word(k),
            _ => false,
        }
    }

    /// The punctuation after word `k` is glued to a word after it: `their .proto file`.
    fn punct_starts_word(&self, k: usize) -> bool {
        let Some(i) =
            (self.words[k] + 1..self.tokens.len()).find(|&i| self.tokens[i].kind != Kind::Space)
        else {
            return false;
        };
        self.tokens
            .get(i + 1)
            .is_some_and(|t| matches!(t.kind, Kind::Word | Kind::Number { .. }))
    }

    fn capitalized(&self, k: usize) -> bool {
        self.chars[self.tokens[self.words[k]].start].is_uppercase()
    }
}

fn is_in(w: Option<&str>, set: &[&str]) -> bool {
    w.is_some_and(|w| set.contains(&w))
}

/// Linking verbs, modals and `do` forms: `is there`, `will there`.
const BE_MODAL: &[&str] = &[
    "is",
    "are",
    "was",
    "were",
    "be",
    "been",
    "being",
    "isn't",
    "aren't",
    "wasn't",
    "weren't",
    "will",
    "would",
    "can",
    "could",
    "may",
    "might",
    "must",
    "should",
    "shall",
    "won't",
    "wouldn't",
    "can't",
    "couldn't",
    "shouldn't",
    "do",
    "does",
    "did",
];

/// Verbs that follow a plural subject: `you're upstreams are`, `who's pages get`.
const PLURAL_VERBS: &[&str] = &[
    "are", "were", "have", "had", "don't", "didn't", "aren't", "weren't", "haven't", "can",
    "can't", "will", "won't", "would", "should", "must", "may", "might", "need", "get", "go",
    "look", "run", "fail", "answer", "return", "work",
];

/// Singular verbs: `who's timestamp is`.
const SINGULAR_VERBS: &[&str] = &[
    "is", "was", "has", "isn't", "wasn't", "hasn't", "doesn't", "can", "can't", "will", "won't",
    "would", "should", "must", "may", "might", "accepts", "matches", "returns", "fails", "gets",
    "runs", "works", "looks", "needs",
];

/// Nouns that are never predicates or verbs: after `you're` / `they're` / `who's` a possessive
/// is meant, and after `there` (not in a question) `their` is.
const NOUNS: &[&str] = &[
    "account",
    "accounts",
    "api",
    "apis",
    "app",
    "apps",
    "application",
    "applications",
    "architecture",
    "backend",
    "backends",
    "behavior",
    "behaviour",
    "behaviors",
    "behaviours",
    "blog",
    "browser",
    "browsers",
    "bug",
    "bugs",
    "business",
    "certificate",
    "certificates",
    "children",
    "cluster",
    "clusters",
    "colleagues",
    "company",
    "computer",
    "computers",
    "config",
    "configs",
    "configuration",
    "configurations",
    "container",
    "containers",
    "contents",
    "credentials",
    "customer",
    "customers",
    "dashboard",
    "dashboards",
    "data",
    "database",
    "databases",
    "dependencies",
    "deployment",
    "deployments",
    "device",
    "devices",
    "documentation",
    "domain",
    "domains",
    "email",
    "emails",
    "endpoint",
    "endpoints",
    "environment",
    "environments",
    "family",
    "files",
    "folder",
    "folders",
    "framework",
    "friends",
    "handler",
    "handlers",
    "hardware",
    "inbox",
    "infrastructure",
    "keys",
    "laptop",
    "laptops",
    "library",
    "libraries",
    "machine",
    "machines",
    "manager",
    "money",
    "names",
    "network",
    "opinion",
    "organization",
    "organisation",
    "owners",
    "package",
    "packages",
    "parents",
    "password",
    "passwords",
    "permissions",
    "phone",
    "platform",
    "platforms",
    "plugin",
    "plugins",
    "preferences",
    "privacy",
    "profile",
    "program",
    "project",
    "projects",
    "repo",
    "repos",
    "repository",
    "repositories",
    "responsibility",
    "responsibilities",
    "schema",
    "secrets",
    "server",
    "servers",
    "services",
    "session",
    "sessions",
    "settings",
    "setup",
    "software",
    "system",
    "systems",
    "team",
    "teams",
    "timestamp",
    "token",
    "tokens",
    "tooling",
    "upstream",
    "upstreams",
    "username",
    "users",
    "website",
    "websites",
    "workflow",
    "workload",
    "workloads",
    "workspace",
];

/// Nouns that double as verbs (`people there code in Rust`): not after `there`.
const VERB_NOUNS: &[&str] = &[
    "code", "change", "changes", "value", "values", "work", "state", "input", "output", "job",
    "name", "feedback", "build", "cache", "needs", "life", "request", "requests", "service",
    "file", "test", "tests", "product", "products", "key",
];

/// Prenominal modifiers between a possessive and its noun: `your existing config`.
const MODIFIERS: &[&str] = &[
    "own",
    "existing",
    "current",
    "previous",
    "default",
    "primary",
    "main",
    "local",
    "first",
    "secret",
    "private",
    "public",
    "entire",
    "whole",
    "favorite",
    "favourite",
    "preferred",
    "custom",
    "personal",
    "internal",
    "new",
    "old",
    "respective",
];

fn is_noun(w: &str, verbs_ok: bool) -> bool {
    NOUNS.contains(&w) || (verbs_ok && VERB_NOUNS.contains(&w))
}

/// Word `k` (`you're`, `they're`, `who's`, `there`) starts a noun phrase that wants a
/// possessive: `you're own`, `you're code`, `you're secret key`, `you're upstreams are`.
fn possessive_phrase(cx: &Cx, k: usize, there: bool) -> bool {
    const FUNCTION: &[&str] = &[
        "in", "on", "to", "at", "for", "and", "or", "but", "then", "so", "because", "if", "when",
        "of", "by", "with", "after", "before", "time", "place", "class", "rate", "choice",
    ];
    if cx.next(k, 1) == Some("own") {
        return true;
    }
    // `you're first shift`, but `you're first in line`, `you're first to arrive`.
    if !there
        && cx.next(k, 1) == Some("first")
        && cx.next(k, 2).is_some_and(|w| {
            !FUNCTION.contains(&w)
                && !w.ends_with("ly")
                && !w.ends_with("ing")
                && !w.ends_with("ed")
        })
        && !cx.hyphenated(k + 1)
    {
        return true;
    }
    let mut i = 1;
    while i <= 2 && is_in(cx.next(k, i), MODIFIERS) {
        i += 1;
    }
    let Some(noun) = cx.next(k, i) else {
        return false;
    };
    if cx.hyphenated(k + i) {
        return false;
    }
    let after = cx.next(k, i + 1);
    if there {
        // `there` + a noun that can also be a verb needs a verb after it: `there value is`,
        // but `the people there value quiet`.
        return NOUNS.contains(&noun)
            || (VERB_NOUNS.contains(&noun) && is_in(after, &["is", "was", "has"]));
    }
    if !is_noun(noun, true) {
        return false;
    }
    // `they're output in order`: a noun that is also a verb needs a verb after it.
    if i == 1 && !NOUNS.contains(&noun) && !finite(after) {
        return false;
    }
    // `they're build tools for Rust`: a plural compound can be a predicate.
    if i == 1
        && !NOUNS.contains(&noun)
        && after.is_some_and(|a| lex(a).plural && lex(a).noun)
        && !is_in(cx.next(k, i + 2), PLURAL_AUX)
        && !is_in(cx.next(k, i + 2), PLURAL_VERBS)
    {
        return false;
    }
    // A plural or mass noun can be a predicate (`they're users`, `they're data`) unless a verb
    // follows it; a bare singular count noun cannot.
    let plural = noun.ends_with('s') && !noun.ends_with("ss") && noun != "business";
    let mass = matches!(
        noun,
        "data" | "software" | "hardware" | "money" | "feedback"
    );
    if plural || mass {
        let verbs = if plural { PLURAL_VERBS } else { SINGULAR_VERBS };
        return is_in(after, verbs);
    }
    true
}

fn match_case(orig: &str, fix: &str) -> String {
    if orig.chars().next().is_some_and(char::is_uppercase) {
        let mut c = fix.chars();
        c.next()
            .map(|f| f.to_uppercase().chain(c).collect())
            .unwrap_or_default()
    } else {
        fix.to_string()
    }
}

fn confusables(name: &str, cx: &Cx, push: &mut dyn FnMut(&str, Lint)) {
    let mut flag = |k: usize, fix: &str, message: &str| {
        if cx.quoted(k) {
            return;
        }
        let t = cx.tokens[cx.words[k]];
        let orig = t.text(cx.chars);
        push(
            name,
            Lint {
                span: Span::new(t.start, t.end),
                lint_kind: LintKind::WordChoice,
                suggestions: vec![Suggestion::ReplaceWith(
                    match_case(&orig, fix).chars().collect(),
                )],
                message: message.to_owned(),
                priority: 31,
            },
        );
    };
    let ing = |w: Option<&str>| w.is_some_and(|w| w.len() > 4 && w.ends_with("ing"));
    for k in 0..cx.wl.len() {
        let w = cx.wl[k].as_str();
        let next = cx.next(k, 1);
        let prev = cx.prev(k, 1);
        match name {
            // `then` after a comparative is ours in `lint`; this is the other direction.
            "ThenThan" => {
                let not_comparative = || {
                    let p2 = cx.prev(k, 2);
                    !is_in(
                        p2,
                        &["rather", "other", "much", "far", "way", "even", "any", "no"],
                    ) && !is_in(p2, COMPARATIVES)
                };
                // `Than we restart`: a sentence never starts with `than`.
                let initial = cx.capitalized(k)
                    && cx.sentence_start(k)
                    && next.is_some_and(|n| {
                        is_in(
                            Some(n),
                            &[
                                "we", "you", "i", "it", "they", "he", "she", "the", "this", "run",
                                "restart", "try",
                            ],
                        )
                    });
                // `If it fails, than we retry`: a comma, then a clause, and no comparative in
                // the clause before.
                let after_comma = matches!(cx.before_nl(k).map(|t| t.kind), Some(Kind::Punct(',')))
                    && next.is_some_and(|n| {
                        is_in(
                            Some(n),
                            &[
                                "we", "you", "i", "it", "they", "he", "she", "the", "this",
                                "there", "it's", "we'll", "you'll", "i'll", "we're", "you're",
                                "they're", "just", "simply", "call", "run", "use", "try",
                                "restart", "retry", "check", "go", "add", "set",
                            ],
                        )
                    })
                    && !cx.capitalized(k)
                    && !(k.saturating_sub(20)..k).any(|i| {
                        let p = cx.wl[i].as_str();
                        COMPARATIVES.contains(&p)
                            || matches!(
                                p,
                                "rather"
                                    | "other"
                                    | "else"
                                    | "different"
                                    | "differently"
                                    | "prefer"
                                    | "preferred"
                                    | "preferable"
                                    | "less"
                                    | "more"
                                    | "fewer"
                            )
                            || (p.len() > 4 && p.ends_with("er") && lex(p).adj)
                    });
                if w == "than"
                    && !cx.hyphenated(k)
                    && (after_comma
                        || (matches!(prev, Some("and") if not_comparative())
                            || matches!(prev, Some("until" | "till" | "since")))
                        || (prev == Some("from") && next == Some("on"))
                        || (prev == Some("back") && not_comparative() && !cx.hyphenated(k - 1))
                        || initial)
                {
                    flag(k, "then", "Did you mean `then`?");
                }
            }
            "YourYoure" => {
                if w == "you're"
                    && (possessive_phrase(cx, k, false)
                        || possessed_noun(cx, k)
                        || possessive_slot(cx, k))
                {
                    flag(
                        k,
                        "your",
                        "Use the possessive `your` before a noun; `you're` means `you are`.",
                    );
                } else if w == "your" && !cx.hyphenated(k) {
                    let n2 = cx.next(k, 2);
                    let contraction = match next {
                        Some("not") => !ing(n2) && !cx.hyphenated(k + 1),
                        Some("sure" | "gonna") => true,
                        // `your probably using`; but `your already generated token`.
                        Some("probably" | "already" | "definitely") => {
                            ing(n2)
                                || is_in(
                                    n2,
                                    &[
                                        "a", "an", "the", "not", "in", "on", "at", "there", "here",
                                        "aware", "familiar", "done", "right", "wrong", "able",
                                    ],
                                )
                        }
                        Some("trying" | "able" | "supposed" | "allowed" | "about" | "free") => {
                            n2 == Some("to")
                        }
                        Some("ready") => {
                            n2 == Some("to") || n2 == Some("for") || cx.punct_end(k + 1)
                        }
                        Some("welcome") => n2.is_none() && cx.clause_end(k + 1) || n2 == Some("to"),
                        // `your a`, `your the best`; `your A record` is DNS.
                        Some("a" | "an" | "the") => !cx.capitalized(k + 1) && !object_slot(cx, k),
                        Some("done") => cx.punct_end(k + 1) || n2 == Some("with"),
                        Some("right") => is_in(n2, &["about", "that"]),
                        Some("all") => n2 == Some("set"),
                        Some("going") => is_in(n2, &["to", "back", "home"]),
                        _ => predicate_follows(cx, k),
                    };
                    if contraction {
                        flag(
                            k,
                            "you're",
                            "Use the contraction `you're` (you are) here; `your` is possessive.",
                        );
                    }
                }
            }
            "ThereOwn" => {
                if matches!(w, "there" | "they're") && next == Some("own") {
                    flag(k, "their", "Did you mean `their own`?");
                }
            }
            "ThereToTheir" => {
                // `out there data is scarce`, `in there code runs`: `there` is a place, and a
                // noun after it starts the next clause unless the sentence ends there (`sat in
                // there car.`).
                const PLACE: &[&str] = &[
                    "out",
                    "over",
                    "up",
                    "down",
                    "in",
                    "back",
                    "from",
                    "here",
                    "near",
                    "under",
                    "around",
                    "right",
                    "get",
                    "go",
                    "went",
                    "going",
                    "gone",
                    "stay",
                    "stayed",
                    "sit",
                    "sat",
                    "live",
                    "lived",
                    "living",
                    "work",
                    "worked",
                    "working",
                    "been",
                    "inside",
                    "outside",
                    "somewhere",
                    "anywhere",
                    "everywhere",
                ];
                // `autonomy over there tooling`: a preposition after a noun, not a place.
                let after_noun = cx.prev(k, 2).is_some_and(|w| lex(w).nounish())
                    && is_in(
                        prev,
                        &[
                            "over", "in", "from", "under", "around", "near", "inside", "outside",
                        ],
                    );
                let place = (is_in(prev, PLACE) && !cx.punct_end(k + 1) && !after_noun)
                    || (is_in(prev, &["and", "or"])
                        && is_in(
                            cx.prev(k, 2),
                            &["here", "back", "forth", "up", "over", "down", "now"],
                        ));
                if w == "there"
                    && next != Some("own")
                    && !is_in(prev, BE_MODAL)
                    && !place
                    && (possessive_phrase(cx, k, true)
                        || there_object(cx, k)
                        || is_in(next, &["existing", "respective"])
                        || there_modified_noun(cx, k)
                        || there_their(cx, k))
                {
                    flag(k, "their", "Did you mean `their`?");
                } else if w == "there's" && !cx.hyphenated(k) {
                    // `the choice is there's.`, `ours and there's`: the
                    // possessive pronoun `theirs`.
                    let pronoun_end = (cx.punct_end(k) || cx.after(k).is_none())
                        && (is_in(prev, &["is", "was", "are", "were", "be", "than"])
                            || (is_in(prev, &["and", "or"])
                                && is_in(
                                    cx.prev(k, 2),
                                    &["ours", "yours", "mine", "his", "hers"],
                                )));
                    if pronoun_end {
                        flag(k, "theirs", "Did you mean the possessive `theirs`?");
                    }
                }
            }
            "TheirToThere" => {
                if w != "their" || cx.hyphenated(k) {
                    continue;
                }
                let be = is_in(
                    next,
                    &[
                        "is", "are", "was", "were", "isn't", "aren't", "wasn't", "weren't", "has",
                        "hasn't", "have", "haven't",
                    ],
                ) && !cx.hyphenated(k + 1);
                // `their will be`, `their seems to be`, `their exist many`.
                let n2 = cx.next(k, 2);
                let modal_be = is_in(
                    next,
                    &[
                        "will",
                        "would",
                        "could",
                        "should",
                        "must",
                        "might",
                        "may",
                        "can",
                        "won't",
                        "wouldn't",
                        "shouldn't",
                        "couldn't",
                    ],
                ) && (n2 == Some("be")
                    || (n2 == Some("not") && cx.next(k, 3) == Some("be")));
                let seems = is_in(
                    next,
                    &[
                        "seems", "seem", "appears", "appear", "happens", "happen", "used", "tends",
                        "tend",
                    ],
                ) && n2 == Some("to")
                    && cx.next(k, 3) == Some("be");
                let exist = is_in(next, &["exist", "exists", "existed"])
                    && is_in(
                        n2,
                        &[
                            "a", "an", "no", "many", "some", "several", "multiple", "only",
                        ],
                    );
                // `stay their.`, but not a pronoun list (`his, her or their.`) or a phrase cut
                // off by the end of a comment (`for their`).
                let end = cx.clause_end(k)
                    && !is_in(prev, &["his", "her", "or", "and", "my", "your", "our"])
                    && cx.before(k) != Some(Kind::Punct('/'))
                    // The end of the text may be a comment line broken mid-sentence (`give
                    // users time to do their`).
                    && cx.after(k).is_some();
                if be || end || modal_be || seems || exist {
                    flag(k, "there", "Did you mean `there`?");
                }
            }
            "TheirToTheyre" => {
                if w != "their" || cx.hyphenated(k) {
                    continue;
                }
                let n2 = cx.next(k, 2);
                let contraction = match next {
                    Some("not") => !ing(n2) && !cx.hyphenated(k + 1),
                    Some("gonna") => true,
                    // `their probably using`; but `their already generated token`, `their
                    // almost always correct output`.
                    Some("almost" | "already" | "probably" | "definitely") => {
                        !cx.hyphenated(k + 1)
                            && (ing(n2)
                                || is_in(
                                    n2,
                                    &[
                                        "a", "an", "the", "not", "in", "on", "at", "there", "here",
                                        "aware", "done", "right", "wrong", "able",
                                    ],
                                ))
                    }
                    Some("going" | "trying" | "able" | "supposed" | "allowed" | "about") => {
                        n2 == Some("to")
                    }
                    Some("a" | "an" | "the") => !cx.capitalized(k + 1) && !object_slot(cx, k),
                    _ => predicate_follows(cx, k),
                };
                if contraction {
                    flag(k, "they're", "Did you mean `they're` (they are)?");
                }
            }
            "TheyreToTheir" => {
                if w == "they're"
                    && next != Some("own")
                    && (possessive_phrase(cx, k, false)
                        || possessed_noun(cx, k)
                        || possessive_slot(cx, k))
                {
                    flag(k, "their", "Did you mean the possessive `their`?");
                }
            }
            "LoseLoose" => lose_loose(cx, k, &mut flag),
            "AffectEffect" => affect_effect(cx, k, &mut flag),
            "WhoseWhos" => {
                if w == "who's" {
                    let n1 = next.unwrap_or("");
                    let n2 = cx.next(k, 2);
                    const NOT_NOUN: &[&str] = &[
                        "here",
                        "there",
                        "next",
                        "first",
                        "last",
                        "left",
                        "right",
                        "in",
                        "on",
                        "out",
                        "up",
                        "down",
                        "not",
                        "who",
                        "that",
                        "what",
                        "it",
                        "this",
                        "online",
                        "offline",
                        "around",
                        "home",
                        "sure",
                        "ready",
                        "done",
                        "responsible",
                        "available",
                        "interested",
                        "affected",
                        "a",
                        "an",
                        "the",
                        "still",
                        "also",
                        "just",
                        "really",
                        "never",
                        "always",
                        "been",
                        "got",
                        "to",
                        "free",
                        "able",
                        "willing",
                        "awake",
                        "new",
                        "late",
                        "behind",
                        "ahead",
                    ];
                    let noun_subject = !n1.is_empty()
                        && !NOT_NOUN.contains(&n1)
                        && !ing(Some(n1))
                        && !n1.ends_with("ed")
                        && !n1.ends_with("ly")
                        && (is_in(n2, &["is", "was", "has", "isn't", "wasn't", "hasn't"])
                            || (is_in(n2, SINGULAR_VERBS) && n2.is_some_and(|v| v.ends_with('s'))));
                    if possessive_phrase(cx, k, false)
                        || noun_subject
                        || possessed_noun(cx, k)
                        || possessive_slot(cx, k)
                    {
                        flag(k, "whose", "Did you mean the possessive `whose`?");
                    }
                } else if w == "whose" && !cx.hyphenated(k) {
                    // `ciphers whose the block size`: a stray article after a relative `whose`.
                    let relative = prev.is_some_and(|p| {
                        let l = lex(p);
                        l.noun && !l.function && !l.part && (!l.verb || l.plural)
                    });
                    let contraction = match next {
                        Some("going" | "been" | "gonna" | "got") => true,
                        Some("not") => !ing(cx.next(k, 2)) && !cx.hyphenated(k + 1),
                        Some("a" | "an" | "the") => !cx.capitalized(k + 1) && !relative,
                        Some("there" | "here") => cx.punct_end(k + 1) || cx.after(k + 1).is_none(),
                        Some(
                            "responsible" | "coming" | "calling" | "next" | "online" | "available",
                        ) => {
                            !relative
                                && (cx.punct_end(k + 1)
                                    || is_in(
                                        cx.next(k, 2),
                                        &["for", "to", "in", "on", "with", "now", "today"],
                                    ))
                        }
                        _ => !relative && predicate_follows(cx, k),
                    };
                    if contraction {
                        flag(k, "who's", "Did you mean `who's` (who is)?");
                    }
                }
            }
            "WeatherWhether" => {
                if w != "weather" {
                    continue;
                }
                const ASK: &[&str] = &[
                    "know",
                    "knows",
                    "knew",
                    "known",
                    "check",
                    "checks",
                    "checked",
                    "checking",
                    "decide",
                    "decides",
                    "decided",
                    "deciding",
                    "determine",
                    "determines",
                    "determined",
                    "determining",
                    "see",
                    "sees",
                    "ask",
                    "asks",
                    "asked",
                    "asking",
                    "wonder",
                    "wondering",
                    "unsure",
                    "sure",
                    "doubt",
                    "indicate",
                    "indicates",
                    "indicating",
                    "tell",
                    "tells",
                    "telling",
                    "show",
                    "shows",
                    "choose",
                    "chooses",
                    "control",
                    "controls",
                    "matter",
                    "matters",
                    "on",
                    "of",
                    "about",
                    "regardless",
                    "irrespective",
                    "leak",
                    "leaks",
                    "leaking",
                    "reveal",
                    "reveals",
                    "verify",
                    "verifies",
                    "confirm",
                    "confirms",
                    "detect",
                    "detects",
                    "test",
                    "tests",
                    "learn",
                    "find",
                    "report",
                    "reports",
                    "specify",
                    "specifies",
                    "flag",
                    "flags",
                    "toggle",
                    "toggles",
                    "clear",
                    "unclear",
                    "and",
                    "consider",
                    "considering",
                    "evaluate",
                    "evaluates",
                    "track",
                    "tracks",
                    "record",
                    "records",
                    "signal",
                    "signals",
                ];
                const CLAUSE: &[&str] = &[
                    "a",
                    "an",
                    "the",
                    "it",
                    "its",
                    "you",
                    "your",
                    "we",
                    "our",
                    "they",
                    "their",
                    "he",
                    "she",
                    "his",
                    "her",
                    "i",
                    "my",
                    "this",
                    "that",
                    "these",
                    "those",
                    "there",
                    "any",
                    "anyone",
                    "someone",
                    "something",
                    "anything",
                    "each",
                    "every",
                    "all",
                    "some",
                    "to",
                    "or",
                ];
                const PRONOUN: &[&str] = &[
                    "it", "they", "we", "you", "i", "he", "she", "there", "it's", "they're",
                    "we're", "you're", "i'm", "there's", "that's", "he's", "she's",
                ];
                // `of` / `on` introduce `whether` only after some words: `the question of
                // whether`, `depends on whether`; not `because of weather we`.
                let of_ok = match prev {
                    Some("of") => is_in(
                        cx.prev(k, 2),
                        &[
                            "question",
                            "matter",
                            "issue",
                            "regardless",
                            "irrespective",
                            "independent",
                            "independently",
                            "choice",
                            "decision",
                            "indication",
                            "sign",
                            "test",
                            "check",
                            "knowledge",
                            "idea",
                            "uncertainty",
                            "determination",
                        ],
                    ),
                    Some("on") => is_in(
                        cx.prev(k, 2),
                        &[
                            "depends",
                            "depend",
                            "depending",
                            "depended",
                            "based",
                            "focus",
                            "focused",
                            "decide",
                            "decided",
                            "agree",
                            "agreed",
                            "hinge",
                            "hinges",
                            "hinged",
                            "rely",
                            "relies",
                            "conditional",
                            "dependent",
                            "contingent",
                            "vote",
                        ],
                    ),
                    _ => true,
                };
                let or_not = next == Some("or") && cx.next(k, 2) == Some("not");
                let clause = is_in(prev, ASK)
                    && is_in(next, CLAUSE)
                    && of_ok
                    && !(prev == Some("and") && next == Some("the"));
                // `Weather it works or not`, `decide weather we`: a clause after a verb or at
                // the start; not `bad weather we had`, `the weather they`, `will weather it`.
                let verb_before = prev.is_some_and(|p| {
                    let l = lex(p);
                    l.verb && !l.noun && !l.adj && !l.function && !BE_MODAL.contains(&p)
                });
                let comma = matches!(cx.before_nl(k).map(|t| t.kind), Some(Kind::Punct(',')));
                let initial = cx.sentence_start(k) && cx.capitalized(k);
                let asked = is_in(prev, ASK) && of_ok && prev != Some("and");
                let pronoun_clause =
                    is_in(next, PRONOUN) && !cx.hyphenated(k) && (verb_before || initial || comma);
                // `Weather the build passes or not`, `, weather your tests pass`: a subject and
                // a verb follow, or `X or Y` does; not `Weather the storm` or `weather data`.
                let det = is_in(
                    next,
                    &[
                        "the", "a", "an", "this", "that", "these", "those", "your", "our", "their",
                        "my", "its", "his", "her", "any", "all", "each", "every",
                    ],
                );
                let subject_verb = det
                    && !cx.hyphenated(k)
                    && (initial || comma || verb_before || asked)
                    && (2..=4).any(|i| {
                        cx.next(k, i).is_some_and(|w| {
                            AUX.contains(&w) || {
                                let l = lex(w);
                                l.finite && !l.noun && !l.function
                            }
                        }) && (2..i).all(|j| {
                            cx.next(k, j).is_some_and(|w| {
                                let l = lex(w);
                                (l.noun || l.adj) && !l.function
                            })
                        })
                    });
                let either_or = (initial || comma || verb_before || asked)
                    && (is_in(next, PRONOUN) || is_in(next, CLAUSE) || det)
                    && next != Some("or")
                    && !cx.hyphenated(k)
                    && (3..=10).any(|i| {
                        cx.next(k, i) == Some("or") && (2..i).all(|j| cx.next(k, j) != Some("and"))
                    })
                    && (next != Some("the") && next != Some("a") || subject_verb);
                if or_not || clause || pronoun_clause || subject_verb || either_or {
                    flag(k, "whether", "Did you mean the conjunction `whether`?");
                }
            }
            "PrincipalPrinciple" => {
                const ROLE: &[&str] = &[
                    "risk",
                    "risks",
                    "engineer",
                    "engineers",
                    "investigator",
                    "investigators",
                    "component",
                    "components",
                    "architect",
                    "architects",
                    "author",
                    "authors",
                    "maintainer",
                    "maintainers",
                    "contributor",
                    "contributors",
                    "reason",
                    "reasons",
                    "cause",
                    "causes",
                    "concern",
                    "concerns",
                    "source",
                    "sources",
                    "advantage",
                    "benefit",
                    "goal",
                    "aim",
                    "focus",
                    "role",
                    "amount",
                    "balance",
                    "owner",
                    "developer",
                    "developers",
                    "scientist",
                    "officer",
                    "character",
                    "characters",
                    "actor",
                    "actors",
                    "difference",
                    "purpose",
                    "objective",
                    "task",
                    "duty",
                    "job",
                    "target",
                    "users",
                    "investment",
                    "payment",
                    "challenge",
                    "driver",
                    "drivers",
                ];
                const NOUN_BEFORE: &[&str] = &[
                    "design",
                    "core",
                    "guiding",
                    "first",
                    "basic",
                    "fundamental",
                    "general",
                    "key",
                    "main",
                    "underlying",
                    "same",
                    "this",
                    "that",
                    "each",
                    "every",
                    "uncertainty",
                    "exclusion",
                    "pigeonhole",
                    "least",
                ];
                // `design principals`, `guiding principal`: rules.
                const RULE_BEFORE: &[&str] = &[
                    "design",
                    "guiding",
                    "fundamental",
                    "basic",
                    "moral",
                    "ethical",
                    "architectural",
                    "solid",
                    "dry",
                    "kiss",
                ];
                // `the principle idea`, `our principle problem`: after a determiner, a noun that
                // is not a verb (`the principle applies`) makes `principle` a modifier.
                let modifier = prev.is_some_and(|p| {
                    matches!(
                        p,
                        "the" | "a" | "our" | "my" | "your" | "their" | "his" | "her" | "its"
                    )
                }) && cx.plain(k, 1).is_some_and(|n| {
                    let l = lex(n);
                    l.known
                        && l.noun
                        && !l.adv
                        && !l.ing
                        && !l.part
                        && !l.finite
                        && !l.plural
                        && !l.function
                        && !matches!(
                            n,
                            "violation" | "violations" | "statement" | "name" | "names"
                        )
                }) && !cx.hyphenated(k)
                    && !cx.hyphenated(k + 1);
                if w == "principle" && (is_in(next, ROLE) || modifier) && !is_in(prev, NOUN_BEFORE)
                {
                    flag(k, "principal", "Did you mean `principal` (main)?");
                } else if matches!(w, "principal" | "principals")
                    && (is_in(prev, RULE_BEFORE)
                        || (w == "principals" && is_in(prev, &["first", "core"])))
                    && !cx.hyphenated(k)
                    && !is_in(next, ROLE)
                    && !is_in(
                        next,
                        &[
                            "investigator",
                            "investigators",
                            "name",
                            "names",
                            "id",
                            "ids",
                        ],
                    )
                {
                    let fix = if w == "principal" {
                        "principle"
                    } else {
                        "principles"
                    };
                    flag(k, fix, "Did you mean `principle` (a rule or belief)?");
                } else if w == "principal" {
                    let rule = next == Some("of")
                        && is_in(
                            cx.next(k, 2),
                            &[
                                "least",
                                "charity",
                                "locality",
                                "separation",
                                "superposition",
                                "relativity",
                                "equivalence",
                                "uncertainty",
                                "parsimony",
                                "sufficient",
                                "explosion",
                                "inertia",
                            ],
                        );
                    let in_principle = is_in(prev, &["in", "on"])
                        && (cx.clause_end(k)
                            || cx.after(k) == Some(Kind::Punct(','))
                            || is_in(next, &["it", "this", "that", "we", "you", "they", "yes"]));
                    // `a matter of principal`, `the principal behind the design`.
                    let matter = prev == Some("of")
                        && is_in(cx.prev(k, 2), &["matter", "question", "point"])
                        && (cx.clause_end(k) || cx.after(k) == Some(Kind::Punct(',')));
                    let behind = next == Some("behind")
                        && is_in(prev, &["the", "a", "same", "core", "key", "main"])
                        && is_in(
                            cx.next(k, 2),
                            &["the", "this", "that", "it", "these", "our", "its", "their"],
                        );
                    if rule || in_principle || matter || behind {
                        flag(
                            k,
                            "principle",
                            "Did you mean `principle` (a rule or belief)?",
                        );
                    }
                }
            }
            "ComplimentComplement" => {
                if let Some(stem) = w.strip_prefix("compliment")
                    && matches!(stem, "" | "s" | "ed" | "ing")
                {
                    let each_other = matches!(
                        (next, cx.next(k, 2)),
                        (Some("each"), Some("other")) | (Some("one"), Some("another"))
                    );
                    let well = is_in(prev, &["nicely", "perfectly"])
                        || is_in(next, &["nicely", "perfectly"]);
                    // `two's compliment`, `bitwise compliment`: the arithmetic complement.
                    let arithmetic = is_in(
                        prev,
                        &[
                            "two's", "ones'", "one's", "twos", "ones", "bitwise", "binary", "radix",
                        ],
                    );
                    if each_other || well || arithmetic {
                        flag(
                            k,
                            &format!("complement{stem}"),
                            "Did you mean `complement` (go well with)?",
                        );
                    }
                } else if w == "complimentary"
                    && (is_in(
                        next,
                        &["colors", "colours", "angles", "strands", "slackness"],
                    ) || (next == Some("to")
                        && matches!(
                            (cx.next(k, 2), cx.next(k, 3)),
                            (Some("each"), Some("other")) | (Some("one"), Some("another"))
                        )))
                {
                    flag(
                        k,
                        "complementary",
                        "Did you mean `complementary` (completing each other)?",
                    );
                }
            }
            "ItsContraction" if w == "its" && its_contraction(cx, k) => {
                flag(k, "it's", "Did you mean `it's` (it is / it has)?");
            }
            "ItsPossessive" if w == "it's" && its_possessive(cx, k) => {
                flag(k, "its", "Did you mean the possessive `its`?");
            }
            "WitchWhich" if w == "witch" && witch_which(cx, k) => {
                flag(k, "which", "Did you mean `which`?");
            }
            "ExceptAccept" => except_accept(cx, k, &mut flag),
            "WriteRight" if w == "write" && write_right(cx, k) => {
                flag(k, "right", "Did you mean `right`?");
            }
            _ => {}
        }
    }
}

/// Determiners and pronouns that start an object: `lose the data`, `lose it`.
const OBJECTS: &[&str] = &[
    "the",
    "a",
    "an",
    "it",
    "them",
    "your",
    "their",
    "our",
    "my",
    "his",
    "her",
    "its",
    "this",
    "that",
    "these",
    "those",
    "all",
    "any",
    "some",
    "everything",
    "anything",
    "nothing",
    "data",
    "money",
    "time",
    "track",
    "access",
    "information",
    "state",
    "progress",
    "work",
    "changes",
    "precision",
    "context",
    "control",
    "focus",
    "sight",
    "sleep",
    "weight",
    "users",
    "customers",
    "connection",
    "writes",
    "events",
    "messages",
    "cents",
    "packets",
    "history",
    "files",
    "interest",
    "hope",
    "faith",
    "face",
    "touch",
    "bits",
];

/// `loose` as an adjective before these: `to loose coupling`.
const LOOSE_NOUNS: &[&str] = &[
    "ends",
    "end",
    "coupling",
    "coupled",
    "fit",
    "fitting",
    "typing",
    "grip",
    "threads",
    "thread",
    "translation",
    "interpretation",
    "match",
    "matching",
    "bounds",
    "bound",
    "comparison",
    "equality",
    "mode",
    "rules",
    "schema",
    "ordering",
    "validation",
    "parsing",
    "sense",
    "definition",
    "collection",
    "federation",
    "leaf",
    "leaves",
    "change",
    "connection",
];

fn lose_loose(cx: &Cx, k: usize, flag: &mut dyn FnMut(usize, &str, &str)) {
    const AUX: &[&str] = &[
        "will",
        "would",
        "could",
        "can",
        "cannot",
        "can't",
        "won't",
        "wouldn't",
        "couldn't",
        "may",
        "might",
        "must",
        "should",
        "shall",
        "shouldn't",
        "don't",
        "doesn't",
        "didn't",
        "do",
        "does",
        "did",
    ];
    let w = cx.wl[k].as_str();
    let (prev, next) = (cx.prev(k, 1), cx.next(k, 1));
    let object = is_in(next, OBJECTS) || cx.clause_end(k);
    match w {
        "loose" => {
            if is_in(next, LOOSE_NOUNS) || cx.hyphenated(k) {
                return;
            }
            let verb = match prev {
                Some("to") => object,
                // `will not loose`, `never loose data`; but `is not loose`.
                Some("not" | "never") => {
                    is_in(cx.prev(k, 2), AUX)
                        || (is_in(next, OBJECTS) && !is_in(cx.prev(k, 2), BE_MODAL))
                }
                Some("i" | "you" | "we" | "they") => true,
                Some(p) => AUX.contains(&p),
                None => false,
            };
            if verb {
                flag(
                    k,
                    "lose",
                    "Did you mean `lose` (the verb)? `loose` means not tight.",
                );
            }
        }
        "loosing" => {
            const BEFORE: &[&str] = &[
                "is", "are", "am", "was", "were", "been", "be", "being", "you're", "we're",
                "they're", "i'm", "it's", "not", "keep", "keeps", "stop", "avoid", "avoids",
                "without", "of", "from", "risk", "prevent", "prevents", "start", "started",
            ];
            if is_in(prev, BEFORE) || is_in(next, OBJECTS) {
                flag(k, "losing", "Did you mean `losing`?");
            }
        }
        "looses" if is_in(next, OBJECTS) => flag(k, "loses", "Did you mean `loses`?"),
        _ => {}
    }
}

fn affect_effect(cx: &Cx, k: usize, flag: &mut dyn FnMut(usize, &str, &str)) {
    const AUX: &[&str] = &[
        "will",
        "would",
        "could",
        "can",
        "cannot",
        "can't",
        "won't",
        "wouldn't",
        "couldn't",
        "may",
        "might",
        "must",
        "should",
        "shouldn't",
        "does",
        "do",
        "did",
        "doesn't",
        "don't",
        "didn't",
    ];
    const ADVERBS: &[&str] = &[
        "negatively",
        "adversely",
        "positively",
        "directly",
        "significantly",
        "greatly",
        "severely",
        "heavily",
        "badly",
        "seriously",
        "drastically",
        "mostly",
        "primarily",
        "only",
        "also",
        "rarely",
        "never",
        "not",
    ];
    const TARGETS: &[&str] = &[
        "the",
        "your",
        "their",
        "our",
        "its",
        "my",
        "his",
        "her",
        "how",
        "whether",
        "performance",
        "users",
        "us",
        "you",
        "them",
        "me",
        "him",
        "only",
        "all",
        "both",
        "every",
        "each",
        "any",
        "anyone",
        "everyone",
        "other",
        "existing",
        "this",
        "these",
        "those",
        "it",
    ];
    const NOUN_ADJ: &[&str] = &[
        "no",
        "any",
        "side",
        "positive",
        "negative",
        "adverse",
        "desired",
        "intended",
        "opposite",
        "same",
        "significant",
        "huge",
        "noticeable",
        "measurable",
        "net",
        "overall",
        "cascading",
        "lasting",
        "immediate",
        "visible",
        "butterfly",
        "placebo",
        "network",
        "domino",
        "ripple",
        "chilling",
        "knock-on",
    ];
    let w = cx.wl[k].as_str();
    let (prev, next) = (cx.prev(k, 1), cx.next(k, 1));
    let aux_before = |p: Option<&str>| match p {
        Some("not" | "never") => is_in(cx.prev(k, 2), AUX),
        Some(p) => AUX.contains(&p),
        None => false,
    };
    /// Objects of `effect` meaning `bring about`: `effect a change`, `effect the transfer`.
    const BROUGHT: &[&str] = &[
        "change",
        "changes",
        "transfer",
        "transfers",
        "transition",
        "reform",
        "reforms",
        "repair",
        "repairs",
        "cure",
        "merger",
        "payment",
        "payments",
        "release",
        "escape",
        "rescue",
        "entry",
        "compromise",
        "sale",
        "settlement",
        "shift",
        "switch",
        "handover",
        "migration",
        "recovery",
        "transformation",
        "improvement",
        "improvements",
    ];
    /// Nouns that form compounds with `effects`: `sound effects`, `network effects`.
    const COMPOUND: &[&str] = &[
        "sound",
        "side",
        "network",
        "special",
        "visual",
        "audio",
        "lighting",
        "particle",
        "weather",
        "health",
        "halo",
        "placebo",
        "domino",
        "ripple",
        "butterfly",
        "cascade",
        "greenhouse",
        "doppler",
        "bandwagon",
        "snowball",
        "wealth",
        "memory",
        "cache",
        "caching",
        "observer",
        "interaction",
        "treatment",
        "order",
        "size",
        "time",
        "screen",
        "post",
        "text",
        "shadow",
        "hover",
        "blur",
        "glow",
        "transition",
        "animation",
        "motion",
        "scroll",
        "fade",
        "state",
        "layout",
        "render",
        "rendering",
        "performance",
        "ui",
        "css",
    ];
    // `the change effects the result`: a verb with an object; not `the change effects the
    // team had`, where a clause describes the effects.
    let object_ok = |from: usize| {
        let head = if is_in(
            cx.next(k, from),
            &["the", "a", "an", "this", "these", "those"],
        ) {
            cx.next(k, from + 1)
        } else {
            cx.next(k, from)
        };
        !is_in(head, BROUGHT)
            && !(from..from + 4).any(|i| {
                is_in(
                    cx.next(k, i),
                    &[
                        "had", "have", "has", "was", "were", "caused", "produced", "created",
                        "made", "brought",
                    ],
                )
            })
    };
    match w {
        "effect" => {
            // `these changes effect the cache`, `they effect performance`.
            let subject = prev.is_some_and(|p| {
                let l = lex(p);
                matches!(p, "we" | "they" | "you" | "i")
                    || (l.plural
                        && !matches!(
                            p,
                            "takes"
                                | "has"
                                | "does"
                                | "goes"
                                | "comes"
                                | "puts"
                                | "gets"
                                | "brings"
                                | "yields"
                        )
                        && l.noun
                        && !l.function
                        && !l.adj
                        && !l.part
                        && !l.ing
                        && !COMPOUND.contains(&p))
            });
            let verb = if prev == Some("to") {
                is_in(next, TARGETS) && next != Some("the")
            } else {
                (aux_before(prev)
                    || (is_in(prev, ADVERBS) && aux_before(cx.prev(k, 2)))
                    || (subject && object_ok(1)))
                    && is_in(next, TARGETS)
            };
            if verb {
                flag(
                    k,
                    "affect",
                    "Did you mean the verb `affect`? `effect` is the noun.",
                );
            }
        }
        "effecting" => {
            const WHOM: &[&str] = &[
                "performance",
                "users",
                "us",
                "you",
                "them",
                "me",
                "him",
                "your",
                "their",
                "our",
                "my",
                "its",
                "everyone",
                "anyone",
                "customers",
            ];
            if is_in(
                prev,
                &[
                    "is",
                    "are",
                    "was",
                    "were",
                    "be",
                    "been",
                    "not",
                    "also",
                    "negatively",
                    "adversely",
                ],
            ) && is_in(next, WHOM)
            {
                flag(k, "affecting", "Did you mean `affecting`?");
            }
        }
        "effects" => {
            const SUBJECT: &[&str] = &["it", "this", "which", "that", "also", "directly"];
            let subject = is_in(prev, SUBJECT) || is_in(prev, ADVERBS);
            let noun_phrase = is_in(
                cx.prev(k, 2),
                &[
                    "the", "its", "any", "no", "side", "their", "these", "those", "of",
                ],
            );
            // `this effects only X`; but `these effects only appear`.
            let only_ok = next != Some("only") || is_in(prev, &["it", "this"]);
            // `the change effects the result`, `this setting effects performance`: a singular
            // noun subject after a determiner.
            let noun_subject = prev.is_some_and(|p| {
                let l = lex(p);
                l.noun
                    && !l.plural
                    && !l.adj
                    && !l.function
                    && !l.ing
                    && !l.part
                    && !COMPOUND.contains(&p)
                    && !NOUN_ADJ.contains(&p)
            }) && is_in(
                cx.prev(k, 2),
                &[
                    "the", "a", "an", "this", "that", "each", "every", "your", "our", "their",
                    "its", "any",
                ],
            ) && next != Some("only")
                && object_ok(1);
            if (subject && !noun_phrase || noun_subject) && is_in(next, TARGETS) && only_ok {
                flag(k, "affects", "Did you mean the verb `affects`?");
            }
        }
        "effected" => {
            const WHO: &[&str] = &[
                "users",
                "customers",
                "services",
                "systems",
                "crates",
                "files",
                "packages",
                "people",
                "everyone",
                "anyone",
                "those",
                "nodes",
                "hosts",
                "regions",
                "tenants",
                "accounts",
                "requests",
                "components",
                "modules",
                "badly",
                "negatively",
                "adversely",
                "directly",
                "severely",
                "not",
                "most",
                "least",
            ];
            // `can be effected by a simple combination`: brought about, correctly.
            let brought_about = prev == Some("be")
                && (is_in(cx.prev(k, 2), AUX)
                    || (is_in(cx.prev(k, 2), &["then", "also", "only", "easily", "still"])
                        && is_in(cx.prev(k, 3), AUX)))
                && is_in(
                    cx.next(k, 2),
                    &["a", "an", "means", "using", "simply", "calling"],
                );
            if (next == Some("by") || is_in(prev, WHO)) && !brought_about {
                flag(k, "affected", "Did you mean `affected`?");
            }
        }
        "affect" | "affects" => {
            let plural = w == "affects";
            let fix = if plural { "effects" } else { "effect" };
            // `padding side affects the results`: a verb with an object.
            let verb_object = plural && is_in(next, TARGETS);
            let in_effect = !plural
                && ((prev == Some("into") && !cx.hyphenated(k))
                    || (prev == Some("in")
                        && is_in(
                            cx.prev(k, 2),
                            &[
                                "is",
                                "are",
                                "was",
                                "were",
                                "be",
                                "been",
                                "remain",
                                "remains",
                                "remained",
                                "stay",
                                "stays",
                                "still",
                                "now",
                                "currently",
                                "go",
                                "goes",
                                "went",
                                "put",
                                "puts",
                            ],
                        ))
                    || (prev == Some("and") && cx.prev(k, 2) == Some("cause")));
            // `a big affect on`, `a lasting affect.`, `unintended affects on`: an adjective,
            // then what follows a noun.
            let noun_next = is_in(
                next,
                &[
                    "on", "of", "upon", "in", "is", "was", "are", "were", "and", "or", "that",
                ],
            ) || cx.sentence_end(k);
            let adjective = prev.is_some_and(|p| {
                let l = lex(p);
                (l.adj && !l.verb && !l.function && !l.ing && !l.part && !l.adv
                    || matches!(
                        p,
                        "little"
                            | "much"
                            | "unintended"
                            | "unexpected"
                            | "unwanted"
                            | "undesired"
                            | "undesirable"
                            | "dramatic"
                            | "profound"
                            | "direct"
                            | "indirect"
                    ))
                    && !p.ends_with("ly")
                    && !matches!(
                        p,
                        "flat" | "blunted" | "restricted" | "labile" | "inappropriate"
                    )
            }) && !cx.hyphenated(k - 1);
            // `its affects on`, `the affects are`: a determiner makes it the noun.
            let determined = plural
                && is_in(
                    prev,
                    &[
                        "its", "their", "his", "her", "our", "your", "these", "those", "the",
                        "some", "many", "various", "several", "such", "similar", "any",
                    ],
                );
            let noun = in_effect
                || (adjective && noun_next)
                || (determined && noun_next && !is_in(next, &["and", "or", "that"]))
                || (!plural && prev == Some("an") && !cx.hyphenated(k))
                || (is_in(prev, NOUN_ADJ) && !(plural && prev == Some("any")) && !verb_object)
                || (is_in(prev, &["the", "an"]) && is_in(next, &["of", "on"]) && !plural)
                || (plural && prev == Some("the") && is_in(next, &["of", "on"]))
                || (!plural && is_in(prev, &["take", "takes", "took", "taken", "taking", "into"]));
            // `any` / `no` + verb: `does any affect it?` is rare; `no affect` is the noun.
            if noun && !(prev == Some("any") && is_in(next, TARGETS)) {
                flag(
                    k,
                    fix,
                    "Did you mean the noun `effect`? `affect` is the verb.",
                );
            }
        }
        _ => {}
    }
}

/// `witch` at word `k` is the relative pronoun `which`: `a list witch is`, `, witch means`,
/// `in witch case`. Not a witch: after an article or adjective (`the witch is`, `a good witch
/// can`), or anywhere near Halloween words.
fn witch_which(cx: &Cx, k: usize) -> bool {
    const NEXT: &[&str] = &[
        "is", "was", "are", "were", "one", "ones", "means", "meant", "you", "we", "they", "i",
        "it", "usually", "makes", "made", "can", "will", "should", "has", "have", "had", "would",
        "could", "may", "might", "must", "also", "includes", "contains", "allows", "lets",
        "provides", "returns", "requires", "gives", "causes", "leads", "is", "isn't", "doesn't",
        "does", "did", "then", "in", "of", "way", "version", "file", "files", "case", "order",
        "type", "kind",
    ];
    const MAGIC: &[&str] = &[
        "witches",
        "witchcraft",
        "witchy",
        "magic",
        "magical",
        "spell",
        "spells",
        "broom",
        "broomstick",
        "halloween",
        "wizard",
        "wizards",
        "coven",
        "cauldron",
        "potion",
        "potions",
        "sorcerer",
        "sorceress",
        "sorcery",
        "hex",
        "hexes",
        "curse",
        "cursed",
        "wicked",
        "hunt",
        "hunts",
        "hunting",
        "hunter",
        "hunters",
        "salem",
        "costume",
        "costumes",
        "fairy",
        "tale",
        "tales",
        "ghost",
        "ghosts",
        "vampire",
        "warlock",
        "occult",
        "pagan",
        "wiccan",
        "wicca",
        "sabbath",
        "witch's",
        "witches'",
        "trials",
        "burned",
        "burnt",
        "stake",
        "elf",
        "elves",
        "goblin",
        "goblins",
        "troll",
        "trolls",
        "dragon",
        "dragons",
        "oz",
        "narnia",
        "hazel",
        "doctor",
        "doctors",
        "queen",
        "king",
        "princess",
        "prince",
        "castle",
        "forest",
        "enchanted",
        "enchantress",
        "mage",
        "mages",
        "druid",
        "necromancer",
        "monster",
        "monsters",
        "zombie",
        "pumpkin",
        "cat",
        "hat",
    ];
    if cx.hyphenated(k) || cx.wl.iter().any(|w| MAGIC.contains(&w.as_str())) {
        return false;
    }
    let Some(next) = cx.next(k, 1) else {
        return false;
    };
    let before = cx.before_nl(k).map(|t| t.kind);
    let prev = cx.prev(k, 1);
    match prev {
        // `in witch case`, `of witch the`: a preposition before `which`.
        Some(
            "in" | "of" | "on" | "to" | "by" | "for" | "from" | "at" | "through" | "during"
            | "with",
        ) => {
            let l = lex(next);
            (l.nounish()
                || is_in(Some(next), DETS)
                || is_in(Some(next), &["we", "you", "they", "i", "it"]))
                && !cx.capitalized(k)
                && !is_in(cx.prev(k, 2), &["a", "an", "the"])
        }
        Some(p) => {
            if !NEXT.contains(&next) || cx.capitalized(k) {
                return false;
            }
            let l = lex(p);
            // A noun then `witch is`: `a list witch is sorted`; but `a sea witch is` may be a
            // witch, so after an article only a verb that a witch rarely does counts.
            const VERBS: &[&str] = &[
                "returns", "contains", "allows", "provides", "requires", "includes", "means",
                "lets", "gives", "causes", "leads", "makes",
            ];
            let article = is_in(cx.prev(k, 2), &["a", "an", "the"]);
            (l.nounish() || l.plural && !l.function) && (!article || VERBS.contains(&next))
        }
        None => {
            NEXT.contains(&next)
                && (matches!(before, Some(Kind::Punct(',')))
                    || (cx.sentence_start(k)
                        && is_in(
                            Some(next),
                            &["one", "is", "means", "way", "version", "file", "of"],
                        )))
        }
    }
}

/// `except` used as `accept` (`we will except the terms`, `please except my apology`) and
/// `accept for` used as `except for` (`everything accept for the key`).
fn except_accept(cx: &Cx, k: usize, flag: &mut dyn FnMut(usize, &str, &str)) {
    const MODAL: &[&str] = &[
        "will",
        "would",
        "can",
        "could",
        "must",
        "should",
        "may",
        "might",
        "cannot",
        "can't",
        "won't",
        "wouldn't",
        "couldn't",
        "shouldn't",
        "please",
        "we",
        "they",
        "i",
    ];
    const WANT: &[&str] = &[
        "want",
        "wants",
        "wanted",
        "need",
        "needs",
        "have",
        "has",
        "had",
        "able",
        "refuse",
        "refused",
        "refuses",
        "going",
        "willing",
        "agree",
        "agreed",
        "decide",
        "decided",
        "happy",
        "glad",
        "ready",
        "choose",
        "chose",
        "unwilling",
        "unable",
    ];
    const OBJECT: &[&str] = &[
        "the",
        "a",
        "an",
        "this",
        "these",
        "those",
        "your",
        "our",
        "their",
        "my",
        "his",
        "her",
        "its",
        "it",
        "them",
        "any",
        "every",
        "no",
        "some",
        "contributions",
        "patches",
        "cookies",
        "payment",
        "payments",
        "cash",
        "donations",
        "responsibility",
        "liability",
        "input",
        "connections",
        "requests",
        "terms",
        "changes",
        "invitations",
        "invites",
        "defeat",
    ];
    let w = cx.wl[k].as_str();
    let (prev, next) = (cx.prev(k, 1), cx.next(k, 1));
    if cx.hyphenated(k) {
        return;
    }
    // `don't except it to work`: that is `expect`, not `accept`.
    let expect = (1..=4).any(|i| cx.next(k, i) == Some("to"));
    let object = is_in(next, OBJECT)
        || cx.plain(k, 1).is_some_and(|n| {
            let l = lex(n);
            !cx.hyphenated(k + 1)
                && (l.nounish() && l.plural
                    || (l.nounish() || l.adj && !l.function)
                        && cx.plain(k, 2).is_some_and(|m| {
                            let l2 = lex(m);
                            l2.nounish() && l2.plural
                        }))
        });
    const BE: &[&str] = &[
        "is", "are", "was", "were", "be", "been", "being", "get", "gets", "got", "isn't", "aren't",
        "wasn't", "weren't", "not",
    ];
    if w == "except" {
        // `will not except`, `doesn't except the`, `to except the`.
        let negated = is_in(prev, &["not", "never"])
            && (is_in(cx.prev(k, 2), MODAL) || is_in(cx.prev(k, 2), &["does", "do", "did"]));
        let modal = is_in(prev, MODAL)
            || is_in(prev, &["doesn't", "don't", "didn't"])
            || negated
            || (prev == Some("to") && is_in(cx.prev(k, 2), WANT));
        if modal && object && !expect {
            flag(k, "accept", "Did you mean `accept` (to receive or agree)?");
        }
    } else if w == "excepts" {
        // `the function excepts a string`: the verb `accepts` (`excepts` is legal jargon).
        let subject = prev.is_some_and(|p| {
            let l = lex(p);
            matches!(
                p,
                "it" | "which" | "that" | "who" | "he" | "she" | "also" | "only" | "now"
            ) || (l.noun && !l.plural && !l.function && !l.adj)
        });
        let obj = object || is_in(next, &["one", "two", "any", "only", "either", "both"]);
        if subject && obj && !expect {
            flag(k, "accepts", "Did you mean `accepts`?");
        }
    } else if w == "excepted" {
        // `is widely excepted`, `was excepted by the maintainers`; but `excepted from`.
        let by_adverb = is_in(
            prev,
            &[
                "widely",
                "generally",
                "commonly",
                "universally",
                "broadly",
                "readily",
                "well",
                "internationally",
            ],
        );
        let passive = is_in(prev, BE)
            && is_in(
                next,
                &["by", "as", "into", "if", "once", "without", "unless"],
            );
        if (by_adverb || passive) && next != Some("from") {
            flag(k, "accepted", "Did you mean `accepted`?");
        }
    } else if w == "excepting" {
        // `we are excepting pull requests`; `excepting the first` is a preposition.
        let noun_next = cx.plain(k, 1).is_some_and(|n| lex(n).nounish());
        if is_in(prev, BE) && prev != Some("not") && (object || noun_next) {
            flag(k, "accepting", "Did you mean `accepting`?");
        }
    } else if w == "accept"
        && next != Some("for")
        && is_in(
            prev,
            &[
                "everything",
                "anything",
                "nothing",
                "everyone",
                "anyone",
                "everybody",
                "anybody",
                "nobody",
            ],
        )
        && !is_in(cx.prev(k, 2), MODAL)
        && !is_in(
            cx.prev(k, 2),
            &["let", "lets", "make", "makes", "help", "helps", "to"],
        )
        && (is_in(next, OBJECT) || is_in(next, &["when", "if", "where", "in", "on", "that"]))
    {
        // `everything accept the key`: `everything` takes `accepts`, so `except` is meant.
        flag(k, "except", "Did you mean `except`?");
    } else if w == "accept" && next == Some("for") {
        const ALL: &[&str] = &[
            "everything",
            "all",
            "nothing",
            "anything",
            "everyone",
            "anyone",
            "nobody",
            "none",
            "identical",
            "same",
            "fine",
            "empty",
            "else",
            "everybody",
            "anybody",
        ];
        let n2 = cx.next(k, 2);
        let exception = is_in(prev, ALL)
            || (matches!(cx.before_nl(k).map(|t| t.kind), Some(Kind::Punct(',')))
                && is_in(
                    n2,
                    &["the", "a", "an", "this", "that", "some", "one", "two"],
                ))
            || (cx.sentence_start(k)
                && cx.capitalized(k)
                && is_in(n2, &["the", "this", "that", "some", "one", "two"]));
        if exception {
            flag(k, "except", "Did you mean `except for`?");
        }
    }
}

/// `write` for `right`, in a few unambiguous phrases: `that's write.`, `the write way`, `turn
/// write at`, `you're write about`.
fn write_right(cx: &Cx, k: usize) -> bool {
    if cx.hyphenated(k) {
        return false;
    }
    let mut p = 1;
    if is_in(
        cx.prev(k, 1),
        &[
            "exactly",
            "absolutely",
            "totally",
            "quite",
            "so",
            "completely",
            "just",
        ],
    ) {
        p = 2;
    }
    // `that's not write`.
    if cx.prev(k, p) == Some("not") {
        p += 1;
    }
    let prev = cx.prev(k, p);
    let end = cx.punct_end(k) || matches!(cx.after(k), Some(Kind::Punct(',')));
    let next = cx.next(k, 1);
    if p == 1 && !cx.hyphenated(k + 1) {
        // `do it write now`, `is write here`, `happening write now`: `right` + a place or time;
        // not `let them write here`, `files that write now`.
        let n = next.unwrap_or("");
        if matches!(n, "away" | "now" | "here" | "there")
            && (is_in(
                prev,
                &["it", "them", "me", "us", "him", "is", "are", "was", "were"],
            ) || prev.is_some_and(|w| {
                let l = lex(w);
                l.ing && !l.noun
            }))
            && !is_in(
                cx.prev(k, 2),
                &[
                    "let", "lets", "make", "makes", "made", "have", "has", "had", "help", "helps",
                    "watch", "see", "saw", "hear", "letting", "making", "helping",
                ],
            )
        {
            return true;
        }
        // `write click the icon`, `your write hand`.
        if matches!(
            n,
            "click" | "clicks" | "clicked" | "clicking" | "hand" | "handed" | "angle" | "angles"
        ) {
            return true;
        }
        // `It's all write.`, `All write, let's go`; not `we all write.`
        if prev == Some("all")
            && end
            && !is_in(cx.prev(k, 2), &["we", "you", "they", "us", "them", "y'all"])
        {
            return true;
        }
        // `the button to the write of the field`.
        if prev == Some("the") && cx.prev(k, 2) == Some("to") && n == "of" {
            return true;
        }
    }
    // `It works, write?`: a tag question.
    if matches!(cx.before_nl(k).map(|t| t.kind), Some(Kind::Punct(',')))
        && matches!(cx.after(k), Some(Kind::Punct('?')))
    {
        return true;
    }
    match prev {
        Some("that's" | "thats" | "sounds" | "looks" | "feels" | "seems") => end,
        Some("is") if cx.prev(k, p + 1) == Some("that") => end,
        // `Is that write?`, `Is it write?`
        Some("it" | "that") if is_in(cx.prev(k, p + 1), &["is", "was", "isn't"]) => end,
        Some("you're" | "youre") => end || is_in(next, &["about"]),
        Some("are") if cx.prev(k, p + 1) == Some("you") => end || is_in(next, &["about"]),
        Some("the") if p == 1 => {
            is_in(
                next,
                &[
                    "way", "thing", "answer", "choice", "decision", "person", "track", "idea",
                    "move", "fit",
                ],
            ) && !cx.hyphenated(k + 1)
        }
        Some("turn") if p == 1 => {
            end || is_in(next, &["at", "onto", "into", "here", "after", "again"])
        }
        _ => false,
    }
}

/// Word classes of one word, from Harper's curated dictionary ([`super::words`], vendored; no
/// tagging, so it is cheap and
/// works in every engine). Classes overlap: `code` is a noun and a verb, `ready` an adjective,
/// noun and verb; the rules ask for the combinations that leave one reading.
#[derive(Debug, Default, Clone, Copy)]
struct Lex {
    known: bool,
    noun: bool,
    verb: bool,
    adj: bool,
    adv: bool,
    /// Pronoun, determiner, preposition or conjunction.
    function: bool,
    prep: bool,
    /// `-ing` form of a verb.
    ing: bool,
    /// Past tense or participle of a verb.
    part: bool,
    /// Simple past or third person singular present: a finite verb.
    finite: bool,
    plural: bool,
}

fn lex(w: &str) -> Lex {
    let Some(m) = words().meta(w) else {
        return Lex::default();
    };
    let verb = m.is_verb();
    Lex {
        known: true,
        noun: m.is_noun(),
        verb,
        adj: m.is_adjective(),
        adv: m.is_adverb(),
        function: m.is_pronoun() || m.is_determiner() || m.is_preposition() || m.is_conjunction(),
        prep: m.is_preposition(),
        ing: m.is_verb_progressive_form() || (verb && w.len() > 4 && w.ends_with("ing")),
        part: m.is_verb_past_form()
            || m.is_verb_past_participle_form()
            || (verb && w.len() > 3 && w.ends_with("ed")),
        finite: verb
            && (m.is_verb_third_person_singular_present_form() || m.is_verb_simple_past_form()),
        plural: m.is_plural_noun_only() || m.is_non_singular_noun() && w.ends_with('s'),
    }
}

impl Lex {
    /// Only a noun (or a noun and a verb): no adjective, adverb, function-word or participle
    /// reading. `car`, `config`, `idea`; not `home`, `ready`, `set`, `done`.
    fn nounish(self) -> bool {
        self.known
            && self.noun
            && !self.adj
            && !self.adv
            && !self.function
            && !self.ing
            && !self.part
    }

    /// Only an adjective: `sure`, `important`, `obvious`, `available`.
    fn pure_adj(self) -> bool {
        self.adj && !self.noun && !self.verb && !self.function && !self.adv
    }

    /// In the dictionary but without a word class (`pointers`): as good as unknown.
    fn tagless(self) -> bool {
        !(self.noun || self.verb || self.adj || self.adv || self.function)
    }

    /// Only an adverb: `probably`, `really`, `always`.
    fn adv_only(self) -> bool {
        self.adv && !self.adj && !self.noun && !self.verb && !self.function
    }
}

/// Auxiliaries and modals that make a finite verb phrase.
const AUX: &[&str] = &[
    "is",
    "are",
    "was",
    "were",
    "has",
    "have",
    "had",
    "will",
    "would",
    "can",
    "could",
    "should",
    "must",
    "may",
    "might",
    "shall",
    "does",
    "did",
    "isn't",
    "aren't",
    "wasn't",
    "weren't",
    "hasn't",
    "haven't",
    "hadn't",
    "won't",
    "wouldn't",
    "can't",
    "cannot",
    "couldn't",
    "shouldn't",
    "doesn't",
    "didn't",
    "don't",
];

/// Auxiliaries after a singular subject: `you're config is`.
const SINGULAR_AUX: &[&str] = &[
    "is",
    "was",
    "has",
    "isn't",
    "wasn't",
    "hasn't",
    "will",
    "would",
    "can",
    "could",
    "should",
    "must",
    "may",
    "might",
    "won't",
    "wouldn't",
    "can't",
    "cannot",
    "couldn't",
    "shouldn't",
    "doesn't",
    "didn't",
    "does",
    "did",
];

/// Auxiliaries after a plural subject: `they're tests are`.
const PLURAL_AUX: &[&str] = &[
    "are",
    "were",
    "have",
    "had",
    "aren't",
    "weren't",
    "haven't",
    "will",
    "would",
    "can",
    "could",
    "should",
    "must",
    "may",
    "might",
    "won't",
    "wouldn't",
    "can't",
    "cannot",
    "couldn't",
    "shouldn't",
    "don't",
    "didn't",
    "do",
    "did",
];

/// Prepositions that take a possessive, never a clause: `in you're code`, `of it's size`.
const POSSESSIVE_PREV: &[&str] = &[
    "of",
    "in",
    "on",
    "for",
    "with",
    "from",
    "into",
    "onto",
    "by",
    "at",
    "to",
    "under",
    "over",
    "through",
    "throughout",
    "without",
    "within",
    "via",
    "per",
    "about",
    "against",
    "among",
    "during",
    "toward",
    "towards",
    "upon",
    "across",
    "along",
    "behind",
    "beyond",
    "below",
    "above",
    "beneath",
    "despite",
    "inside",
    "outside",
    "around",
    "between",
    "plus",
    "off",
];

/// Words that open a clause, so a contraction (`you're`, `it's`) may follow.
const OPENERS: &[&str] = &[
    "and", "but", "so", "or", "because", "if", "when", "while", "since", "though", "although",
    "that", "unless", "until", "once", "whether", "as", "where", "why", "how", "what", "then",
    "now", "think", "thought", "guess", "hope", "know", "knew", "sure", "maybe", "perhaps", "yes",
    "no", "well", "bet", "believe", "say", "said", "says", "feel", "mean", "means", "assume",
    "assuming", "suppose", "realize", "notice", "see", "seems", "whenever", "wherever", "ok",
    "okay", "oh", "hey", "anyway",
];

/// Openers after which an `-ing` word reads as a verb: `if your using`, not `and your building`.
const ING_OPENERS: &[&str] = &[
    "if", "when", "while", "since", "because", "think", "guess", "hope", "know", "sure", "unless",
    "until", "whether", "bet", "assume", "assuming", "whenever",
];

/// Verbs that take a clause without `that`: `assert it's a power of two`, `know it's time`.
const CLAUSE_VERBS: &[&str] = &[
    "think",
    "thinks",
    "thought",
    "know",
    "knows",
    "knew",
    "guess",
    "say",
    "says",
    "said",
    "hope",
    "mean",
    "means",
    "meant",
    "assume",
    "assumes",
    "assuming",
    "believe",
    "suppose",
    "figure",
    "bet",
    "realize",
    "realise",
    "notice",
    "noticed",
    "see",
    "saw",
    "feel",
    "felt",
    "imagine",
    "hear",
    "heard",
    "understand",
    "assert",
    "asserts",
    "ensure",
    "ensures",
    "check",
    "checks",
    "confirm",
    "confirms",
    "indicate",
    "indicates",
    "indicating",
    "show",
    "shows",
    "note",
    "notes",
    "claim",
    "claims",
    "pretend",
    "seems",
    "seem",
    "wish",
    "sure",
    "given",
    "remember",
    "forget",
    "learn",
    "learned",
    "decide",
    "decided",
    "prove",
    "proves",
    "argue",
    "agree",
    "admit",
    "explain",
    "reckon",
    "recall",
    "suspect",
    "doubt",
    "expect",
    "expects",
    "fear",
    "worry",
    "trust",
    "promise",
    "warn",
    "tell",
    "told",
    "report",
    "reports",
    "discover",
    "discovered",
    "guarantee",
    "guarantees",
    "suggest",
    "suggests",
    "mention",
    "read",
    "pray",
    "swear",
    "bet",
    "insist",
    "demand",
    "ask",
    "asks",
    "asked",
    "wonder",
    "wondering",
];

/// Determiners and pronouns that start a noun phrase or an object.
const DETS: &[&str] = &[
    "a", "an", "the", "this", "that", "these", "those", "my", "your", "our", "their", "his", "her",
    "its", "no", "some", "any", "all", "both", "each", "every", "many", "several", "most", "one",
    "another", "only",
];

/// Adjectives that the dictionary also lists as nouns or verbs but that, at the start of a
/// clause and before `to` / `that`, make a predicate: `so its best to`, `if your happy with`.
const PREDICATIVE: &[&str] = &[
    "best",
    "better",
    "easier",
    "harder",
    "fine",
    "good",
    "great",
    "bad",
    "happy",
    "ok",
    "okay",
    "safe",
    "clear",
    "easy",
    "hard",
    "true",
    "false",
    "likely",
    "possible",
    "impossible",
    "worth",
    "nice",
    "cool",
    "glad",
    "sorry",
    "free",
    "right",
    "wrong",
    "correct",
    "necessary",
    "important",
    "essential",
    "unlikely",
    "common",
    "rare",
    "normal",
    "natural",
    "simpler",
    "cheaper",
    "faster",
    "useful",
    "helpful",
    "difficult",
    "tricky",
    "able",
    "unable",
    "ready",
    "welcome",
    "responsible",
    "familiar",
    "aware",
    "sure",
    "certain",
    "clearer",
    "safer",
    "due",
    "awesome",
    "amazing",
    "brilliant",
    "busy",
    "late",
];

/// Nouns with an adjective ending that the dictionary also lists as adjectives: `its logic.`
const NOUN_LIKE_ADJ: &[&str] = &[
    "logic",
    "music",
    "topic",
    "magic",
    "traffic",
    "graphic",
    "graphics",
    "public",
    "critic",
    "clinic",
    "fabric",
    "panic",
    "mechanic",
    "epic",
    "rhetoric",
    "arithmetic",
    "metric",
    "capital",
    "total",
    "original",
    "final",
    "principal",
    "signal",
    "rival",
    "local",
    "general",
    "normal",
    "potential",
    "content",
    "intent",
    "parent",
    "client",
    "agent",
    "component",
    "constant",
    "variant",
    "tenant",
    "servant",
    "elephant",
    "event",
    "moment",
    "student",
    "patient",
    "resident",
    "president",
    "equivalent",
    "dependent",
    "descendant",
    "relative",
    "native",
    "objective",
    "executive",
    "alternative",
    "representative",
    "initiative",
    "detective",
    "narrative",
    "primitive",
    "directive",
    "perspective",
    "collective",
    "creative",
    "individual",
    "manual",
    "material",
    "official",
    "rational",
    "terminal",
    "binary",
    "primary",
    "secondary",
    "summary",
    "library",
    "boundary",
    "dictionary",
    "temporary",
    "ordinary",
    "arbitrary",
    "auxiliary",
    "ancillary",
    "stationary",
    "military",
    "sanitary",
    "valuable",
    "variable",
    "cable",
    "table",
    "label",
    "able",
    "mobile",
    "profile",
    "file",
    "missile",
    "textile",
    "personal",
    "professional",
    "commercial",
    "chemical",
    "digital",
    "physical",
    "mental",
    "medical",
    "classic",
    "plastic",
    "electric",
    "hospital",
    "animal",
    "proposal",
    "rental",
    "arrival",
    "approval",
    "removal",
    "renewal",
    "survival",
    "tutorial",
    "editorial",
    "special",
    "national",
    "regional",
    "periodical",
    "radical",
    "vertical",
    "horizontal",
    "diagonal",
    "interval",
    "portal",
    "crystal",
    "metal",
    "pedal",
    "journal",
    "channel",
    "opponent",
    "accent",
    "percent",
    "ancient",
    "applicant",
    "assistant",
    "consultant",
    "participant",
    "merchant",
    "occupant",
    "defendant",
    "attendant",
    "executable",
    "deliverable",
    "observable",
    "callable",
    "iterable",
    "renderable",
    "runnable",
    "consumable",
    "wearable",
    "collectible",
    "disposable",
    "receivable",
    "payable",
    "comparable",
    "printable",
    "portable",
    "vegetable",
    "syllable",
    "constable",
    "timetable",
    "notable",
    "valuables",
];

/// Number words: `it's two words`.
const NUMBERS: &[&str] = &[
    "zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten", "twice",
    "half", "both",
];

/// Adjectives that are attributive (`your own`, `its only`), never a predicate after a
/// contraction.
const NOT_PREDICATE: &[&str] = &[
    "own",
    "only",
    "very",
    "same",
    "other",
    "whole",
    "entire",
    "main",
    "first",
    "last",
    "next",
    "time",
    "way",
    "part",
    "place",
    "turn",
    "share",
    "fill",
    "lot",
    "new",
    "old",
    "full",
    "real",
    "fair",
    "kind",
    "usual",
    "previous",
    "current",
    "left",
    "top",
    "bottom",
    "total",
    "average",
    "overall",
    "default",
    "favorite",
    "favourite",
    "various",
    "respective",
    "former",
    "latter",
    "inner",
    "outer",
    "upper",
    "lower",
    "chief",
    "key",
    "sole",
    "mere",
    "utmost",
    "little",
    "big",
    "high",
    "low",
    "dead",
    "wounded",
    "injured",
    "elderly",
    "young",
    "poor",
    "rich",
    "faithful",
    "beloved",
    "departed",
    "accused",
    "chosen",
    "betrothed",
    "deceased",
    "damned",
    "unborn",
    "fallen",
    "married",
];

/// Nouns that are also predicates without an article: `you're history`, `it's lunch time`.
const NOT_POSSESSED: &[&str] = &[
    "history",
    "toast",
    "company",
    "family",
    "trouble",
    "royalty",
    "dinner",
    "lunch",
    "breakfast",
    "chaos",
    "magic",
    "fun",
    "noon",
    "midnight",
    "home",
    "nobody",
    "somebody",
    "everybody",
    "anybody",
    "someone",
    "everyone",
    "anyone",
    "something",
    "everything",
    "anything",
    "nothing",
    "who",
    "whom",
    "what",
    "which",
    "that",
    "this",
    "it",
    "there",
    "here",
    "time",
    "way",
    "part",
    "kind",
    "sort",
    "type",
    "bit",
    "lot",
    "thing",
    "shame",
    "pity",
    "matter",
    "means",
];

impl Cx<'_> {
    /// First token before word `k` that is not whitespace (line breaks included).
    fn before_nl(&self, k: usize) -> Option<Token> {
        self.tokens[..self.words[k]]
            .iter()
            .rev()
            .find(|t| !matches!(t.kind, Kind::Space | Kind::Newline))
            .copied()
    }

    /// Word `k` starts a sentence: nothing or sentence punctuation before it.
    fn sentence_start(&self, k: usize) -> bool {
        match self.before_nl(k) {
            None => true,
            Some(t) => matches!(
                t.kind,
                Kind::Punct('.' | '!' | '?' | ':' | ';' | '(' | '"' | '“' | '|' | '*' | '>')
            ),
        }
    }

    /// Word `k` starts a clause: a sentence start, a comma or an [`OPENERS`] word before it.
    fn clause_start(&self, k: usize) -> bool {
        self.sentence_start(k)
            || matches!(self.before_nl(k).map(|t| t.kind), Some(Kind::Punct(',')))
            || is_in(self.prev(k, 1), OPENERS)
    }

    /// Word `k` starts a clause where a contraction is likely: a sentence start, a comma, or a
    /// subordinator or verb of thinking (`if`, `when`, `think`), not `and` / `that` / `as`.
    fn strong_open(&self, k: usize) -> bool {
        self.sentence_start(k)
            || matches!(self.before_nl(k).map(|t| t.kind), Some(Kind::Punct(',')))
            || is_in(self.prev(k, 1), ING_OPENERS)
            || is_in(
                self.prev(k, 1),
                &[
                    "though", "although", "once", "maybe", "perhaps", "believe", "say", "said",
                    "says", "mean", "suppose", "realize", "seems", "thought", "knew", "now",
                ],
            )
    }

    /// Nothing that could extend a noun phrase follows word `k`: sentence punctuation or a comma
    /// and a new clause (`, so`, `, we`).
    fn sentence_end(&self, k: usize) -> bool {
        const CLAUSE: &[&str] = &[
            "i",
            "we",
            "you",
            "they",
            "he",
            "she",
            "it",
            "so",
            "but",
            "and",
            "then",
            "which",
            "because",
            "since",
            "though",
            "although",
            "or",
            "if",
            "when",
            "just",
            "please",
            "thanks",
            "otherwise",
            "that's",
            "it's",
            "there's",
            "we're",
            "you're",
            "they're",
            "i'm",
            "right",
        ];
        if k >= self.words.len() {
            return false;
        }
        // The end of the text is not enough: a code comment line may break mid-sentence.
        match self.after(k) {
            Some(Kind::Punct('.' | '!' | '?' | ';')) => !self.punct_starts_word(k),
            Some(Kind::Punct(',')) => {
                k + 1 < self.words.len() && CLAUSE.contains(&self.wl[k + 1].as_str())
            }
            _ => false,
        }
    }

    /// A run of spaces follows word `k` on its line: blanked code (`your encrypted `.env``).
    fn gap_after(&self, k: usize) -> bool {
        k < self.words.len()
            && self
                .tokens
                .get(self.words[k] + 1)
                .is_some_and(|t| t.kind == Kind::Space && t.end - t.start >= 3)
    }

    /// Word `k` ends its sentence: sentence punctuation (not the end of text) follows.
    fn punct_end(&self, k: usize) -> bool {
        k < self.words.len()
            && matches!(self.after(k), Some(Kind::Punct('.' | '!' | '?' | ';')))
            && !self.punct_starts_word(k)
    }

    /// Word `k` opens a quotation (`"there own"`): a word mentioned, not used.
    fn quoted(&self, k: usize) -> bool {
        self.words[k]
            .checked_sub(1)
            .and_then(|i| self.tokens.get(i))
            .is_some_and(|t| matches!(t.kind, Kind::Punct('"' | '“' | '\'' | '‘' | '`')))
    }

    /// Word `k` is written in capitals throughout (`OTHER`, `SET`).
    fn all_caps(&self, k: usize) -> bool {
        let t = self.tokens[self.words[k]];
        t.end - t.start > 1 && self.chars[t.start..t.end].iter().all(|c| !c.is_lowercase())
    }

    /// Next word `n` places after `k`, lowercase, not hyphenated: something a rule may read.
    fn plain(&self, k: usize, n: usize) -> Option<&str> {
        let w = self.next(k, n)?;
        (!self.hyphenated(k + n) && !self.capitalized(k + n)).then_some(w)
    }
}

/// Word `k` sits where a possessive or object goes (`can have their a new pod`, `with your
/// the`): a stray article after it is a typo, not a contraction.
fn object_slot(cx: &Cx, k: usize) -> bool {
    is_in(
        cx.prev(k, 1),
        &[
            "have", "has", "had", "having", "get", "got", "gets", "keep", "keeps", "use", "uses",
        ],
    ) || is_in(cx.prev(k, 1), POSSESSIVE_PREV)
}

/// Word `k` follows a transitive verb, where an object (so a possessive) goes: `verifies it's
/// checksum`, `so put you're routes first`, `catches there cousins`. Verbs that take a clause
/// (`know you're`, `assert it's`) do not count.
fn transitive_before(cx: &Cx, k: usize) -> bool {
    let Some(p) = cx.prev(k, 1) else {
        return false;
    };
    if CLAUSE_VERBS.contains(&p)
        || AUX.contains(&p)
        || OPENERS.contains(&p)
        || BE_MODAL.contains(&p)
        || matches!(
            p,
            "be" | "been" | "being" | "am" | "get" | "got" | "go" | "went" | "come"
        )
    {
        return false;
    }
    let l = lex(p);
    if !l.verb || l.function || l.ing || (l.adj && !l.finite) {
        return false;
    }
    let p2 = cx.prev(k, 2);
    if is_in(p2, DETS) {
        return false;
    }
    (l.finite && !l.part && !cx.sentence_start(k - 1))
        || cx.clause_start(k - 1)
        || is_in(
            p2,
            &[
                "to", "will", "would", "can", "could", "should", "must", "may", "might", "don't",
                "didn't", "never", "always", "please", "cannot", "won't", "i", "you", "we", "they",
            ],
        )
}

fn finite(w: Option<&str>) -> bool {
    w.is_some_and(|w| {
        AUX.contains(&w) || {
            let l = lex(w);
            l.finite && !l.function
        }
    })
}

/// Word `k` (`you're`, `they're`, `who's`, `it's`) is followed by a bare noun that only a
/// possessive fits: `in you're code`, `they're config is`, `who's car broke`, `it's contents
/// are`. A bare singular noun needs a verb after it; a noun that is also a verb or a plural noun
/// needs an auxiliary (`they're tests are`, but `they're users of`).
fn possessed_noun(cx: &Cx, k: usize) -> bool {
    let Some(x) = cx.plain(k, 1) else {
        return false;
    };
    // A preposition asks for a possessive (`of it's buffer`), and so does `than` before a noun
    // that ends the clause (`took longer than it's interval;`).
    let prep = is_in(cx.prev(k, 1), POSSESSIVE_PREV)
        || (cx.prev(k, 1) == Some("than")
            && lex(x).nounish()
            && !PREDICATIVE.contains(&x)
            && (cx.punct_end(k + 1) || cx.sentence_end(k + 1)));
    // `it's caller's responsibility`: a possessive noun means an article is missing, unless
    // a preposition asks for a possessive (`relative to it's parent's box`).
    if NOT_POSSESSED.contains(&x) || cx.hyphenated(k) || (x.ends_with("'s") && !prep) {
        return false;
    }
    // `put you're most specific routes`: a verb's object, with modifiers before the noun.
    if transitive_before(cx, k) {
        let mut i = 1;
        if is_in(cx.next(k, i), &["most", "more", "least", "very"]) {
            i += 1;
        }
        if cx.plain(k, i).is_some_and(|w| {
            let l = lex(w);
            // `need it's current state`: attributive-only adjectives count too.
            l.adj
                && !l.function
                && !matches!(
                    w,
                    "own" | "only" | "very" | "time" | "way" | "part" | "lot" | "kind"
                )
        }) {
            i += 1;
        }
        if i > 1
            && cx
                .plain(k, i)
                .is_some_and(|w| lex(w).nounish() && !NOT_POSSESSED.contains(&w))
        {
            return true;
        }
    }
    let l = lex(x);
    // After a preposition any noun will do (`of it's buffer`), even one that is also an
    // adjective.
    if prep {
        let l = lex(x.trim_end_matches("'s"));
        return l.known && l.noun && !l.function && !l.ing && !l.part && !l.adv;
    }
    if !l.nounish() {
        return false;
    }
    if cx.next(k, 2).is_some() && cx.hyphenated(k + 2) {
        return false;
    }
    if transitive_before(cx, k) {
        return true;
    }
    let after = cx.next(k, 2);
    // `it's failure modes are`: a compound noun, then an auxiliary.
    if !l.plural
        && after.is_some_and(|w| {
            let l2 = lex(w);
            l2.nounish() && !l2.verb && !cx.capitalized(k + 2)
        })
        && is_in(cx.next(k, 3), AUX)
        && !cx.hyphenated(k + 3)
    {
        return true;
    }
    if l.plural {
        is_in(after, PLURAL_AUX)
    } else if l.verb {
        is_in(after, SINGULAR_AUX)
    } else {
        is_in(after, SINGULAR_AUX) || finite(after)
    }
}

/// Nouns that make a predicate without an article: `you're team lead`, `you're root`.
const ROLE_NOUNS: &[&str] = &[
    "lead",
    "owner",
    "admin",
    "administrator",
    "root",
    "member",
    "guest",
    "host",
    "user",
    "maintainer",
    "author",
    "reviewer",
    "captain",
    "chair",
    "head",
    "boss",
    "king",
    "queen",
    "friend",
    "president",
    "manager",
    "expert",
    "champion",
    "winner",
    "loser",
    "human",
    "alone",
    "next",
    "sudo",
    "superuser",
    "maintainers",
    "fans",
    "friends",
    "family",
    "halfway",
    "overkill",
    "nonsense",
    "rubbish",
    "garbage",
    "gold",
    "shit",
    "crap",
    "bullshit",
    "fire",
    "live",
    "online",
    "offline",
    "public",
    "private",
    "dead",
    "done",
    "proof",
    "evidence",
    "boilerplate",
    "legacy",
];

/// Word `k` (`you're`, `they're`, `who's`) starts a noun phrase that cannot be a predicate, so
/// the possessive (`your`, `their`, `whose`) is meant: attributive modifiers (`existing`,
/// `broken`), then a singular count noun with no article (`you're current branch`, `you're pull
/// request`), or a noun phrase after a verb that takes an object (`rotate you're existing
/// credentials`) or before an auxiliary (`they're build tools are`). A plural noun alone can be
/// a predicate: `they're great tools`, `you're new users`.
fn possessive_slot(cx: &Cx, k: usize) -> bool {
    const ATTRIBUTIVE: &[&str] = &[
        "existing",
        "own",
        "previous",
        "entire",
        "whole",
        "favorite",
        "favourite",
        "preferred",
        "respective",
        "default",
        "current",
        "primary",
        "broken",
    ];
    let mut i = 1;
    let mut modded = false;
    while i <= 2 {
        let Some(m) = cx.plain(k, i) else {
            return false;
        };
        let l = lex(m);
        let attributive = ATTRIBUTIVE.contains(&m)
            || (l.adj
                && !l.noun
                && !l.verb
                && !l.adv
                && !l.function
                && !PREDICATIVE.contains(&m)
                && !NOT_PREDICATE.contains(&m));
        if !attributive {
            break;
        }
        i += 1;
        modded = true;
    }
    // After a modifier or another noun, `credentials` / `requests` are nouns, not verbs.
    let noun = |j: usize, after_mod: bool| {
        cx.plain(k, j).filter(|w| {
            let l = lex(w);
            l.nounish()
                && (!l.finite || (after_mod && l.plural))
                && !NOT_POSSESSED.contains(w)
                && !ROLE_NOUNS.contains(w)
                && !PREDICATIVE.contains(w)
                && !cx.hyphenated(k + j)
        })
    };
    let Some(mut head) = noun(i, modded) else {
        return false;
    };
    let compound = noun(i + 1, true);
    if let Some(h) = compound {
        head = h;
        i += 1;
    }
    let after = cx.next(k, i + 1);
    if cx.next(k, i).is_some() && cx.hyphenated(k + i + 1) {
        return false;
    }
    // `Rotate you're keys`: an imperative (the dictionary also lists `rotate` as an adjective).
    let imperative = k > 0
        && cx.prev(k, 1).is_some_and(|p| {
            let l = lex(p);
            l.verb
                && !l.function
                && !l.noun
                && !CLAUSE_VERBS.contains(&p)
                && !BE_MODAL.contains(&p)
                && !AUX.contains(&p)
        })
        && cx.sentence_start(k - 1)
        && cx.capitalized(k - 1);
    let object = imperative || transitive_before(cx, k) || is_in(cx.prev(k, 1), POSSESSIVE_PREV);
    let Some(m) = words().meta(head) else {
        return false;
    };
    let plural = lex(head).plural || m.is_plural_noun_only();
    if plural {
        return (object && (modded || compound.is_some())) || is_in(after, PLURAL_AUX);
    }
    // A singular count noun needs an article to be a predicate.
    let count = m.is_countable_noun() && !m.is_mass_noun_only();
    (count || object || is_in(after, SINGULAR_AUX))
        && after != Some("of")
        && (modded || compound.is_some() || object || is_in(after, SINGULAR_AUX))
}

/// Word `k` (`your`, `their`, `whose`, `its`) is followed by a predicate, so the contraction
/// (`you're`, `they're`, `who's`, `it's`) is meant: `your aware of`, `its important to`, `their
/// probably the`, `if your using the`.
fn predicate_follows(cx: &Cx, k: usize) -> bool {
    const FOLLOW: &[&str] = &[
        "to", "that", "about", "with", "of", "for", "whether", "how", "if", "when", "because",
        "enough",
    ];
    // After an adjective that is also a noun (`best`, `likely`): `your best to` is possessive
    // unless the clause starts here (`so its best to`), and `of` stays possessive (`your kind of`).
    const FOLLOW_OPEN: &[&str] = &[
        "to", "that", "whether", "how", "if", "because", "with", "about",
    ];
    const STRONG: &[&str] = &["a", "an", "the", "not", "been", "gonna"];
    const OBJECT: &[&str] = &[
        "a", "an", "the", "it", "this", "that", "these", "those", "them", "me", "us", "him", "my",
        "our", "his", "some", "any", "all",
    ];
    let Some(n1) = cx.plain(k, 1) else {
        return false;
    };
    // `your specified `return_url`.`: blanked code after the word is the noun.
    if cx.hyphenated(k) || cx.gap_after(k + 1) {
        return false;
    }
    let tail = |j: usize| cx.punct_end(j) || is_in(cx.next(j, 1), FOLLOW);
    // `once its finished, restart`: in a clause opened by a subordinator, a comma ends it.
    let subordinate =
        is_in(cx.prev(k, 1), ING_OPENERS) || is_in(cx.prev(k, 1), &["once", "though", "although"]);
    // The end of the text counts only there too: `// will succeed if its undefined`.
    let ends = |j: usize| {
        cx.sentence_end(j) || (subordinate && matches!(cx.after(j), Some(Kind::Punct(',')) | None))
    };
    let l1 = lex(n1);
    // `its very slow.`, `your so right about`, `its too late to`: an intensifier and a
    // predicate adjective that no noun follows.
    if matches!(
        n1,
        "very" | "so" | "too" | "pretty" | "quite" | "extremely" | "fairly" | "super"
    ) && !cx.hyphenated(k + 1)
    {
        if let Some(n2) = cx.plain(k, 2)
            && !NOT_PREDICATE.contains(&n2)
            && !cx.hyphenated(k + 2)
        {
            let l2 = lex(n2);
            let comma = cx.after(k + 2) == Some(Kind::Punct(','))
                && cx
                    .wl
                    .get(k + 3)
                    .is_some_and(|w| !lex(w).adj && !lex(w).noun || lex(w).function);
            if l2.adj && !l2.part && !l2.function && (tail(k + 2) || ends(k + 2) || comma) {
                return true;
            }
        }
        return false;
    }
    // `its only because`, `its only a test`: `only` then an article or a clause.
    if n1 == "only"
        && !cx.hyphenated(k + 1)
        && is_in(
            cx.plain(k, 2),
            &[
                "a", "an", "the", "because", "when", "if", "me", "us", "them", "you",
            ],
        )
    {
        return true;
    }
    if NOT_PREDICATE.contains(&n1) {
        return false;
    }
    if matches!(n1, "done" | "sure" | "unsure" | "aware" | "unaware") && tail(k + 1) {
        return true;
    }
    // `retries their exhausted.`: a participle cannot end a noun phrase.
    if l1.part && !l1.noun && (cx.punct_end(k + 1) || ends(k + 1)) {
        return true;
    }
    // `now its smallest.`: a superlative after a possessive is a noun.
    if l1.pure_adj() && !n1.ends_with("est") && (tail(k + 1) || ends(k + 1)) {
        return true;
    }
    if PREDICATIVE.contains(&n1)
        && cx.clause_start(k)
        && (is_in(cx.next(k, 2), FOLLOW_OPEN)
            || (n1 == "worth" && lex_ing(cx.next(k, 2)))
            || (!n1.ends_with("est") && ends(k + 1)))
    {
        return true;
    }
    // `if your correct,`, `when its active.`, `Its automatic.`: an adjective ends a clause
    // that opens with the possessive. Many nouns are also adjectives (`its header,`, `its
    // capital.`, `and their potential.`), so those need a subordinator or a bare sentence.
    if l1.adj && !l1.function && !l1.ing && !n1.ends_with("est") {
        let bare_end = cx.punct_end(k + 1);
        let fires = if l1.noun {
            (subordinate && bare_end)
                || (cx.sentence_start(k)
                    && cx.punct_end(k + 1)
                    && adjective_like(n1)
                    && !NOUN_LIKE_ADJ.contains(&n1))
        } else {
            cx.strong_open(k) && ends(k + 1)
        };
        if fires {
            return true;
        }
    }
    // `their interested in`, `its based on`, `your similar to`, `its compatible with`: an
    // adjective or participle, then a preposition; a noun phrase would come first.
    if (l1.adj || l1.part && !l1.noun)
        && !l1.function
        && !l1.ing
        && !n1.ends_with("est")
        && !cx.hyphenated(k + 1)
    {
        let n2 = cx.plain(k, 2);
        let prep = is_in(
            n2,
            &[
                "in", "on", "at", "by", "with", "from", "to", "into", "onto", "about", "than",
                "within", "without", "under", "after", "before", "during", "through", "via",
                "against", "upon", "behind", "beyond", "among", "between", "toward", "towards",
            ],
        ) && !cx.gap_after(k + 1);
        // `your logged in user`, `its built in support`: a particle and a noun.
        let particle = is_in(
            n2,
            &["in", "out", "up", "on", "off", "down", "over", "into"],
        ) && cx.next(k, 3).is_some_and(|w| {
            let l3 = lex(w);
            (l3.noun || l3.adj) && !l3.function && !l3.ing
        });
        // `their potential to grow`, `your terminal with`, `its core in`: a noun is possible,
        // so only an adjective-shaped word before `with` / `to` / `from` settles it.
        let fits = !l1.noun
            || (n2 == Some("than") && (COMPARATIVES.contains(&n1) || n1.ends_with("er") && l1.adj))
            || (adjective_like(n1)
                && !NOUN_LIKE_ADJ.contains(&n1)
                && is_in(n2, &["with", "to", "from"]));
        // `its meant for`, `their responsible for`; `for` after a noun (`your best for`) is
        // common, so only a word that cannot be a noun.
        let for_ok = n2 == Some("for") && !l1.noun && !cx.gap_after(k + 1);
        if (prep && fits || for_ok) && !particle {
            return true;
        }
    }
    // `their probably the`, `your really sure`, `its always been`.
    if l1.adv_only() || matches!(n1, "just" | "still" | "only") {
        let n2 = cx.plain(k, 2);
        if is_in(n2, STRONG) {
            return true;
        }
        // `its just me`, `its only because`, `its just like`.
        if is_in(
            n2,
            &["me", "us", "them", "you", "him", "because", "like", "about"],
        ) && !cx.hyphenated(k + 2)
        {
            return true;
        }
        if let Some(n2) = n2
            && n2 != "own"
            && !NOT_PREDICATE.contains(&n2)
        {
            let l2 = lex(n2);
            if l2.pure_adj() && tail(k + 2) {
                return true;
            }
            if l2.ing && !l2.noun && is_in(cx.next(k, 3), OBJECT) {
                return true;
            }
            // `its still locked after that`, `its probably missing from`: a participle or
            // adjective that no noun follows.
            if (l2.part || l2.ing || l2.adj) && !cx.hyphenated(k + 2) {
                let n3 = cx.next(k, 3);
                let bare = cx.punct_end(k + 2)
                    || ends(k + 2)
                    || n3.is_some_and(|w| {
                        let l3 = lex(w);
                        l3.prep && !l3.noun && !matches!(w, "of" | "as" | "like")
                    });
                if bare {
                    return true;
                }
            }
        }
    }
    // `if your using the`; but `and your building in`, `your running shoes`.
    l1.ing && is_in(cx.prev(k, 1), ING_OPENERS) && is_in(cx.next(k, 2), OBJECT)
}

fn lex_ing(w: Option<&str>) -> bool {
    w.is_some_and(|w| lex(w).ing)
}

/// `there` at word `k` stands for `their`: `because of there code`, `There config is wrong`,
/// `and there tests are`.
fn there_their(cx: &Cx, k: usize) -> bool {
    /// Prepositions `there` does not follow as a place: `of there`, `with there`.
    const NONLOC: &[&str] = &[
        "of", "for", "with", "at", "without", "against", "among", "despite", "during", "via",
        "per", "about",
    ];
    const CONJ: &[&str] = &[
        "and", "but", "because", "so", "if", "when", "since", "while", "although", "though",
        "unless", "whether", "until", "once",
    ];
    const TIME: &[&str] = &[
        "years",
        "months",
        "weeks",
        "days",
        "hours",
        "minutes",
        "seconds",
        "decades",
        "centuries",
        "times",
        "ages",
        "nights",
        "mornings",
        "evenings",
        "year",
        "month",
        "week",
        "day",
        "hour",
        "minute",
        "night",
    ];
    let Some(x) = cx.plain(k, 1) else {
        return false;
    };
    let l = lex(x);
    if !l.nounish()
        || (l.finite && !l.plural)
        || TIME.contains(&x)
        || NOT_POSSESSED.contains(&x)
        || cx.hyphenated(k)
    {
        return false;
    }
    let prev = cx.prev(k, 1);
    let after = cx.next(k, 2);
    if is_in(prev, NONLOC) {
        // `north of there lies a town`: a verb after the noun makes `there` a place.
        return !(l.finite && (is_in(after, DETS) || after.is_none()));
    }
    // `catches there cousins.`: the object of a verb, ending the sentence.
    if transitive_before(cx, k)
        && !is_in(
            prev,
            &[
                "put", "left", "leave", "keep", "kept", "stay", "live", "lived", "sit", "stand",
                "work", "worked", "meet", "met", "saw", "see",
            ],
        )
        && cx.punct_end(k + 1)
    {
        return true;
    }
    let open = (cx.sentence_start(k) || is_in(prev, CONJ))
        && prev != Some("here")
        && cx.prev(k, 2) != Some("here");
    if open && !(l.finite && !l.plural) {
        return if l.plural {
            is_in(after, PLURAL_AUX)
        } else {
            is_in(after, SINGULAR_AUX)
        };
    }
    false
}

/// `there` at word `k` starts a possessed noun phrase: a possessive modifier and a noun (`there
/// new office`, `there current setup`) or a possessive noun (`there team's goals`).
fn there_modified_noun(cx: &Cx, k: usize) -> bool {
    const MOD: &[&str] = &[
        "current",
        "previous",
        "new",
        "old",
        "default",
        "primary",
        "main",
        "personal",
        "entire",
        "whole",
        "favorite",
        "favourite",
        "preferred",
        "original",
        "latest",
        "usual",
        "own",
    ];
    let Some(x) = cx.plain(k, 1) else {
        return false;
    };
    if cx.hyphenated(k) {
        return false;
    }
    let noun = |w: &str| {
        let l = lex(w);
        l.nounish() && !NOT_POSSESSED.contains(&w) && !l.function
    };
    if let Some(stem) = x.strip_suffix("'s") {
        return noun(stem) && !is_in(cx.prev(k, 1), &["is", "was", "and", "or"]);
    }
    if !MOD.contains(&x) || cx.hyphenated(k + 1) {
        return false;
    }
    // `out there new ideas emerge` is a place; `is there new data?` a question (both ruled out
    // by the caller). The noun must not be a verb that makes `there` the subject's place.
    cx.plain(k, 2)
        .is_some_and(|n| noun(n) && !cx.hyphenated(k + 2) && !(lex(n).finite && !lex(n).plural))
}

/// `there` at word `k` is a verb's object: `load there importers first`, `we call there
/// handlers`. Verbs of place and motion (`put`, `store`, `went`, `live`) take the adverb
/// (`we moved there years ago`, `store there files`), so they do not count.
fn there_object(cx: &Cx, k: usize) -> bool {
    const LOCATIVE: &[&str] = &[
        "put",
        "puts",
        "putting",
        "move",
        "moves",
        "moved",
        "moving",
        "store",
        "stores",
        "stored",
        "keep",
        "keeps",
        "kept",
        "leave",
        "leaves",
        "left",
        "copy",
        "copies",
        "copied",
        "send",
        "sends",
        "sent",
        "save",
        "saves",
        "saved",
        "place",
        "places",
        "placed",
        "install",
        "installs",
        "installed",
        "deploy",
        "deploys",
        "deployed",
        "run",
        "runs",
        "ran",
        "stay",
        "stays",
        "stayed",
        "live",
        "lives",
        "lived",
        "work",
        "works",
        "worked",
        "go",
        "goes",
        "went",
        "get",
        "gets",
        "got",
        "come",
        "comes",
        "came",
        "arrive",
        "arrived",
        "travel",
        "sit",
        "sat",
        "stand",
        "stood",
        "remain",
        "remains",
        "remained",
        "wait",
        "waited",
        "stop",
        "stopped",
        "start",
        "started",
        "find",
        "found",
        "add",
        "adds",
        "added",
        "write",
        "writes",
        "wrote",
        "mount",
        "mounted",
        "drop",
        "dropped",
        "land",
        "landed",
        "hang",
        "meet",
        "met",
        "visit",
        "visited",
        "reach",
        "reached",
        "exist",
        "exists",
        "lie",
        "lies",
        "lay",
        "belong",
        "belongs",
        "happen",
        "happens",
        "happened",
        "appear",
        "appears",
        "appeared",
        "post",
        "posted",
        "pass",
        "passed",
        "push",
        "pushed",
        "upload",
        "uploaded",
        "navigate",
        "point",
        "points",
        "link",
        "links",
        "linked",
        "redirect",
        "list",
        "listed",
        "define",
        "defined",
        "declare",
        "declared",
        "use",
        "used",
        "uses",
        "set",
        "sets",
        "do",
        "does",
        "did",
        "make",
        "made",
        "have",
        "has",
        "had",
        "live",
        "die",
        "died",
        "sleep",
        "slept",
        "eat",
        "ate",
        "grow",
        "grew",
        "stuck",
        "hid",
        "hide",
        "hidden",
        "created",
        "create",
        "creates",
        "log",
        "logged",
        "print",
        "printed",
        "show",
        "shows",
        "showed",
        "shown",
        "say",
        "says",
        "said",
        "type",
        "enter",
        "paste",
        "insert",
        "inserted",
        "included",
        "include",
        "includes",
    ];
    const TIME: &[&str] = &[
        "years",
        "months",
        "weeks",
        "days",
        "hours",
        "minutes",
        "seconds",
        "times",
        "year",
        "month",
        "week",
        "day",
        "hour",
        "minute",
        "night",
        "morning",
        "evening",
        "ago",
        "today",
        "yesterday",
        "tomorrow",
        "tonight",
        "now",
        "once",
        "twice",
        "often",
        "again",
        "already",
        "yet",
        "instead",
        "too",
        "either",
        "first",
        "last",
        "next",
        "before",
        "after",
        "later",
        "daily",
        "weekly",
        "anyway",
        "forever",
        "overnight",
        "together",
        "alone",
        "long",
        "while",
    ];
    let Some(p) = cx.prev(k, 1) else {
        return false;
    };
    if LOCATIVE.contains(&p) || cx.hyphenated(k) || cx.hyphenated(k - 1) {
        return false;
    }
    let imperative = cx.sentence_start(k - 1) && cx.capitalized(k - 1) && {
        let l = lex(p);
        l.verb && !l.function && !l.noun && !CLAUSE_VERBS.contains(&p)
    };
    if !imperative && !transitive_before(cx, k) {
        return false;
    }
    let Some(n) = cx.plain(k, 1) else {
        return false;
    };
    let l = lex(n);
    if !l.nounish()
        || AUX.contains(&n)
        || BE_MODAL.contains(&n)
        || TIME.contains(&n)
        || NOT_POSSESSED.contains(&n)
        || cx.hyphenated(k + 1)
        || (l.finite && !l.plural)
    {
        return false;
    }
    // `we call there handlers run`: not a noun phrase that ends here.
    !finite(cx.next(k, 2))
}

/// `its` at word `k` stands for `it's`: `its a bug`, `its been fixed`, `its not`, `if its
/// using the`, `its important to`, `its going to`.
fn its_contraction(cx: &Cx, k: usize) -> bool {
    let Some(n1) = cx.plain(k, 1) else {
        return false;
    };
    if cx.hyphenated(k) || its_possessive_context(cx, k) {
        return false;
    }
    let n2 = cx.next(k, 2);
    match n1 {
        "a" | "an" | "the" | "been" | "gonna" | "how" | "what" | "why" | "where" => true,
        "not" => !cx.hyphenated(k + 1),
        "got" => is_in(n2, &["to", "a", "an", "the", "no", "nothing", "some"]),
        "going" => is_in(n2, &["to", "on"]),
        "time" => cx.clause_start(k) && n2 == Some("to") && cx.next(k, 3) != Some("live"),
        _ => predicate_follows(cx, k),
    }
}

/// `its` at word `k` sits where only a possessive fits, whatever follows: after a preposition
/// or `has` (`with its associated`, `has its this`), after a transitive verb (`lends its
/// buffer`), or before a noun phrase (`its limit on`, `its set of`, `its named mask values`).
fn its_possessive_context(cx: &Cx, k: usize) -> bool {
    const HAVE: &[&str] = &["has", "have", "had", "having", "keeps", "keep", "kept"];
    const LOCATIVE: &[&str] = &[
        "left", "right", "top", "bottom", "own", "only", "whole", "entire", "current", "previous",
    ];
    let prev = cx.prev(k, 1);
    if is_in(prev, POSSESSIVE_PREV) || is_in(prev, HAVE) {
        return true;
    }
    let Some(n1) = cx.next(k, 1) else {
        return false;
    };
    // `its contains`, `its defer ... ran`: a bare verb, where `it's` would not fit either.
    let l1 = lex(n1);
    if l1.verb
        && !l1.noun
        && !l1.part
        && !l1.adj
        && !l1.ing
        && !AUX.contains(&n1)
        && (!l1.finite || n1.ends_with('s'))
        && !matches!(n1, "been" | "got" | "gotten" | "gonna")
    {
        return true;
    }
    let strong = matches!(
        n1,
        "a" | "an" | "the" | "not" | "been" | "our" | "my" | "your"
    ) && !cx.capitalized(k + 1);
    if let Some(p) = prev
        && !strong
        && !CLAUSE_VERBS.contains(&p)
        && !AUX.contains(&p)
        && !matches!(p, "be" | "been" | "being" | "am")
        && !is_in(cx.prev(k, 2), DETS)
    {
        let lp = lex(p);
        if lp.verb && !lp.function && !lp.pure_adj() {
            return true;
        }
    }
    let only_clause = n1 == "only"
        && is_in(
            cx.next(k, 2),
            &[
                "a", "an", "the", "because", "when", "if", "me", "us", "them", "you",
            ],
        );
    if !strong
        && (cx.capitalized(k + 1)
            || (LOCATIVE.contains(&n1) && !only_clause)
            || n1.chars().count() == 1)
    {
        return true;
    }
    // `a datetime and its offset to`: after `and` a noun is possessed; the contraction would
    // need a comma or a clearer predicate.
    if is_in(prev, &["and", "or", "nor"])
        && !strong
        && ((l1.noun && !l1.function) || n1 == "being")
        && !PREDICATIVE.contains(&n1)
    {
        return true;
    }
    if l1.tagless() && !strong {
        return true;
    }
    let n2 = cx.next(k, 2);
    if l1.noun && !l1.function {
        // `its limit`, `its code`: a plain noun; `its set of`, `its uses are`.
        if !l1.adj && !l1.part && !l1.ing {
            return true;
        }
        if n2 == Some("of") || (finite(n2) && n1 != "been") {
            return true;
        }
        // `its limit on`: a noun that is also a verb, then a preposition.
        if l1.verb && !l1.part && !l1.ing && n2.is_some_and(|w| w != "to" && lex(w).prep) {
            return true;
        }
    }
    if is_in(prev, &["is", "are", "was", "were"]) && l1.noun && !l1.function {
        return true;
    }
    // `its just fine.`, `its really bad.`: an adverb and a predicate adjective ending the
    // sentence; not `its only child.` (`only` never reads as `it's only`).
    let adverb_predicate = matches!(
        n1,
        "just" | "still" | "really" | "always" | "never" | "probably" | "definitely" | "actually"
    ) && n2.is_some_and(|w| lex(w).adj && !lex(w).function)
        && cx.punct_end(k + 2);
    // `its named mask`, `its associated [`Schema`]`, `its closing partner`: a modifier.
    if (l1.part || l1.ing || l1.adj) && !strong && n1 != "being" && !adverb_predicate {
        // `its associated `Schema``: blanked code (a run of spaces) is the noun.
        let gap = cx
            .tokens
            .get(cx.words[k + 1] + 1)
            .is_some_and(|t| t.kind == Kind::Space && t.end - t.start >= 3)
            && cx
                .tokens
                .get(cx.words[k + 1] + 2)
                .is_some_and(|t| t.kind != Kind::Newline);
        if gap {
            return true;
        }
        let next_tok = cx.tokens[cx.words[k + 1] + 1..]
            .iter()
            .find(|t| !matches!(t.kind, Kind::Space | Kind::Newline))
            .map(|t| t.kind);
        match next_tok {
            Some(Kind::Punct(c)) if !matches!(c, '.' | ',' | ';' | ':' | '!' | '?' | ')') => {
                return true;
            }
            Some(Kind::Word) => {
                if let Some(n2) = n2 {
                    let l2 = lex(n2);
                    let nounlike = l2.tagless()
                        || cx.capitalized(k + 2)
                        || (l2.noun
                            && !l2.function
                            && !l2.adv
                            && !l2.part
                            && !AUX.contains(&n2)
                            && !(l2.ing && !l2.noun));
                    if nounlike {
                        return true;
                    }
                } else {
                    // A word across more than one line break or after code: unknown.
                    return true;
                }
            }
            Some(Kind::Number { .. } | Kind::Unlintable) => return true,
            _ => {}
        }
    }
    false
}

/// `it's` at word `k` stands for `its`: `it's own`, `of it's size`, `It's value is`.
fn its_possessive(cx: &Cx, k: usize) -> bool {
    if cx.hyphenated(k) {
        return false;
    }
    if cx.next(k, 1) == Some("own") && !cx.hyphenated(k + 1) {
        return true;
    }
    possessed_noun(cx, k) && !its_contraction_context(cx, k)
}

/// `it's` at word `k` reads as `it is` whatever the dictionary says of the next word: `it's
/// likely we`, `it's reserved`, `it's true`, `it's encryption */`, `it's null terminated`.
fn its_contraction_context(cx: &Cx, k: usize) -> bool {
    let Some(n1) = cx.next(k, 1) else {
        return true;
    };
    if n1 == "own" {
        return false;
    }
    // `pretend it's Jan 1`, `I think it's time`: a verb that takes a clause, never an object.
    if is_in(
        cx.prev(k, 1),
        &[
            "think", "thinks", "thought", "know", "knows", "knew", "guess", "say", "says", "said",
            "hope", "assume", "assumes", "believe", "believes", "suppose", "pretend", "pretends",
            "realize", "realise", "bet", "imagine", "feel", "felt", "seems", "seem", "wish",
            "doubt", "suspect", "reckon", "sure", "hear", "heard",
        ],
    ) {
        return true;
    }
    if cx.hyphenated(k + 1)
        || n1.chars().count() == 1
        || (n1.ends_with("'s") && !is_in(cx.prev(k, 1), POSSESSIVE_PREV))
    {
        return true;
    }
    let l1 = lex(n1);
    let n2 = cx.next(k, 2);
    let l2 = n2.map(lex).unwrap_or_default();
    let n2_noun = n2.is_some_and(|n2| {
        (l2.tagless() || cx.capitalized(k + 2) || (l2.noun && !l2.function && !l2.adv))
            && !AUX.contains(&n2)
    });
    let prep_before = is_in(cx.prev(k, 1), POSSESSIVE_PREV) || cx.prev(k, 1) == Some("than");
    // `it's likely slower`, `it's way faster`, `it's lower priority`, `it's two words`.
    if n2.is_some_and(|w| (w.ends_with("er") && l2.adj) || matches!(w, "more" | "less"))
        || COMPARATIVES.contains(&n1)
        || NUMBERS.contains(&n1)
    {
        return true;
    }
    // `it's possible (although`, `it's likely we`, `with it's impossible to`; but `at it's
    // best`.
    if PREDICATIVE.contains(&n1)
        && (!prep_before
            || is_in(
                n2,
                &["to", "that", "than", "we", "you", "they", "i", "it", "if"],
            ))
    {
        return true;
    }
    // `some of it's stuck after`: an adjective before a preposition is a predicate.
    if prep_before
        && l1.adj
        && (l1.verb || l1.part)
        && n2.is_some_and(|w| !matches!(w, "of" | "as" | "like") && lex(w).prep && !lex(w).noun)
    {
        return true;
    }
    // `show it's part of`, `it's kind of`: a predicate noun with `of`.
    if matches!(n1, "part" | "kind" | "sort" | "one") && n2 == Some("of") {
        return true;
    }
    // `it's caller(*)`: an identifier.
    if cx
        .tokens
        .get(cx.words[k + 1] + 1)
        .is_some_and(|t| t.kind == Kind::Punct('('))
    {
        return true;
    }
    // `returning it's span`, `match it's length`: a transitive verb wants an object.
    let verb_before = cx.prev(k, 1).is_some_and(|p| {
        let l = lex(p);
        l.verb
            && !l.part
            && !l.function
            && !l.noun
            && !CLAUSE_VERBS.contains(&p)
            && !AUX.contains(&p)
            && !matches!(p, "be" | "been" | "being" | "am")
    }) || cx
        .prev(k, 1)
        .is_some_and(|p| lex(p).ing && !CLAUSE_VERBS.contains(&p));
    // An adjective, adverb, participle or function word: `it's reserved`, `it's within`,
    // `it's must`; but `on it's binary form`, `It's main purpose is`. At the start of a
    // clause an adjective and a noun are a predicate: `It's valid CSS`.
    if (l1.adj && !l1.noun)
        || l1.adv_only()
        || l1.function
        || l1.part
        || l1.ing
        || AUX.contains(&n1)
    {
        // `find it's bounding box`, `delete it's corresponding entry`.
        let verb_ctx = cx.prev(k, 1).is_some_and(|p| {
            let l = lex(p);
            l.verb
                && !l.function
                && !CLAUSE_VERBS.contains(&p)
                && !AUX.contains(&p)
                && !OPENERS.contains(&p)
                && !matches!(p, "be" | "been" | "being" | "am")
                && !is_in(cx.prev(k, 2), DETS)
        });
        let object = prep_before || verb_before || verb_ctx || finite(cx.next(k, 3));
        return !(l1.adj && n2_noun && object);
    }
    if prep_before || verb_before || finite(n2) {
        return false;
    }
    // A noun that ends the clause or is followed by a predicate reads as `it is <noun>`:
    // `it's RSA.`, `it's time to`, `it's position dependent`, `it's likely we`.
    let after = cx.tokens[cx.words[k + 1] + 1..]
        .iter()
        .find(|t| !matches!(t.kind, Kind::Space | Kind::Newline))
        .map(|t| t.kind);
    match after {
        None => true,
        Some(Kind::Punct(c)) => !matches!(c, '(' | '[' | '`' | '"' | '\''),
        Some(Kind::Word) => n2.is_some_and(|n2| {
            matches!(
                n2,
                "to" | "that" | "than" | "if" | "and" | "or" | "so" | "but"
            ) || l2.adv_only()
                || (l2.adj && (!l2.noun || adjective_like(n2)))
                || l2.part
                || (l2.ing && !l2.noun)
                || (l2.function && !l2.prep)
        }),
        _ => false,
    }
}

/// Modals and other verbs that do not inflect: `he can`, `it must`, `she need not`.
const UNINFLECTED: &[&str] = &[
    "can", "could", "will", "would", "shall", "should", "may", "might", "must", "ought", "need",
    "dare", "used", "be", "am", "are", "were", "let's", "please", "cannot",
];

/// Words before a third-person pronoun that take a bare verb after it: auxiliaries and modals
/// (`does it work`, `can he run`), and verbs taking the subjunctive (`suggested she go`).
const BARE_AFTER: &[&str] = &[
    "that",
    "lest",
    "demand",
    "demanded",
    "demanding",
    "demands",
    "insist",
    "insisted",
    "insisting",
    "insists",
    "recommend",
    "recommended",
    "recommending",
    "recommends",
    "request",
    "requested",
    "requesting",
    "requests",
    "require",
    "required",
    "requiring",
    "requires",
    "suggest",
    "suggested",
    "suggesting",
    "suggests",
    "propose",
    "proposed",
    "ask",
    "asked",
    "let",
    "lets",
    "letting",
    "make",
    "makes",
    "made",
    "making",
    "help",
    "helps",
    "helped",
    "helping",
    "have",
    "has",
    "had",
    "having",
    "see",
    "saw",
    "seen",
    "watch",
    "watched",
    "hear",
    "heard",
    "to",
    "and",
    "or",
    "nor",
    "not",
    "than",
    "as",
    "whom",
    "am",
    "be",
    "being",
    "been",
];

/// Words after which `it` is a subject, not an object: `if it work`, `when it fail`.
const SUBJECT_OPENERS: &[&str] = &[
    "if", "when", "because", "unless", "while", "although", "though", "once", "whenever",
    "wherever", "whether", "where", "how", "why", "so", "but", "then", "now",
];

/// Verbs after which `you` is an object and a plural noun its second object: `give you results`,
/// `show you errors`.
fn object_you(prev: &str) -> bool {
    let l = lex(prev);
    (l.verb && !l.function && !AUX.contains(&prev) && !BE_MODAL.contains(&prev)) || l.prep
}

/// The 3rd person singular present of the verb `base` (`run` -> `runs`, `go` -> `goes`,
/// `study` -> `studies`), if the dictionary knows `base` as an uninflected verb.
fn third_person(base: &str) -> Option<String> {
    let irregular = match base {
        "have" => Some("has"),
        "do" => Some("does"),
        "don't" => Some("doesn't"),
        "haven't" => Some("hasn't"),
        "be" | "am" | "are" => None,
        _ => None,
    };
    if let Some(i) = irregular {
        return Some(i.to_string());
    }
    if UNINFLECTED.contains(&base) || base.len() < 2 || !base.chars().all(char::is_alphabetic) {
        return None;
    }
    let m = words().meta(base)?;
    // `he set`, `it cost`, `she married`: the simple past reads fine.
    if m.is_verb_simple_past_form() || m.is_verb_past_form() || base.ends_with("ed") {
        return None;
    }
    inflect(base)
}

/// [`third_person`] without the simple-past check: `set` -> `sets`.
fn inflect(base: &str) -> Option<String> {
    let m = words().meta(base)?;
    if !m.is_verb()
        || m.is_verb_third_person_singular_present_form()
        || m.is_verb_progressive_form()
        || m.is_adverb()
        || m.is_conjunction()
        || m.is_preposition()
        || m.is_pronoun()
        || m.is_determiner()
    {
        return None;
    }
    let mut cands = vec![format!("{base}s"), format!("{base}es")];
    if let Some(stem) = base.strip_suffix('y') {
        cands.insert(0, format!("{stem}ies"));
    }
    cands.into_iter().find(|c| {
        words()
            .meta(c)
            .is_some_and(|m| m.is_verb_third_person_singular_present_form())
    })
}

/// The uninflected verb of the 3rd person singular present `form` (`runs` -> `run`, `goes` ->
/// `go`, `passes` -> `pass`), for a plural or first/second person subject (`is` -> `are`).
fn base_form(form: &str, subject: &str) -> Option<String> {
    let irregular = match form {
        "is" => Some(if subject == "i" { "am" } else { "are" }),
        "has" => Some("have"),
        "does" => Some("do"),
        "doesn't" => Some("don't"),
        "hasn't" => Some("haven't"),
        "isn't" => (subject != "i").then_some("aren't"),
        _ => None,
    };
    if let Some(i) = irregular {
        return Some(i.to_string());
    }
    if form.contains('\'') {
        return None;
    }
    let m = words().meta(form)?;
    if !m.is_verb_third_person_singular_present_form() {
        return None;
    }
    let mut cands: Vec<String> = Vec::new();
    if let Some(stem) = form.strip_suffix("ies") {
        cands.push(format!("{stem}y"));
    }
    if let Some(stem) = form.strip_suffix("es")
        && ["ss", "sh", "ch", "x", "z", "o"]
            .iter()
            .any(|e| stem.ends_with(e))
    {
        cands.push(stem.to_string());
    }
    if let Some(stem) = form.strip_suffix('s') {
        cands.push(stem.to_string());
    }
    cands
        .into_iter()
        .find(|c| inflect(c).as_deref() == Some(form))
}

/// Verbs that read as adverbs or adjectives right after `it`: `it further reduces`, `it sort
/// of works`, `if it clear` (a missing `is`).
const NOT_VERB_AFTER_IT: &[&str] = &[
    "clear", "open", "free", "empty", "clean", "close", "dry", "idle", "quiet", "warm", "cool",
    "slow", "secure", "right", "wrong", "complete", "correct", "blank", "better", "worse", "busy",
    "total", "equal", "even", "live", "full", "safe", "fine", "stable", "further", "also", "still",
    "just", "only", "never", "always", "last", "first", "back", "then", "now", "once", "again",
    "rather", "well", "hardly", "really", "finally", "sort", "kind", "up", "down", "out", "off",
    "over", "away", "around", "home", "later", "together", "alone", "twice",
];

/// `PronounVerbAgreement`: the verb right after a subject pronoun has the wrong number (`they
/// runs`, `you presses`, `she go`, `it work`). Returns the corrected verb. `I` is left alone: in
/// technical text it is mostly a variable (`I is the index`).
fn pronoun_verb(cx: &Cx, k: usize) -> Option<String> {
    let w = cx.wl[k].as_str();
    let third = match w {
        "he" | "she" | "it" => true,
        "you" | "we" | "they" => false,
        _ => return None,
    };
    let verb = cx.next(k, 1)?;
    // `he-man`, `it-run`, `you're`, `They Run` (a title), `THEY`.
    if cx.hyphenated(k)
        || cx.hyphenated(k + 1)
        || cx.quoted(k)
        || cx.quoted(k + 1)
        || cx.capitalized(k + 1)
        || cx.all_caps(k)
        || cx.tokens[cx.words[k]].end - cx.tokens[cx.words[k]].start != w.len()
    {
        return None;
    }
    // `he/she`, `[t]he value`, `fork/execs`.
    if matches!(
        cx.before(k),
        Some(Kind::Punct('/' | '(' | '[' | ']' | '-' | '&'))
    ) || cx.after(k + 1) == Some(Kind::Punct('/'))
    {
        return None;
    }
    let prev = cx.prev(k, 1);
    let next = cx.next(k, 2);
    // `does it work`, `can you uses`: a question or an auxiliary before the pronoun. `they
    // types of groups`, `it sort of works`: a noun. `it heap allocates`: a modifier.
    if is_in(prev, AUX) || is_in(prev, BE_MODAL) || next == Some("of") || finite(next) {
        return None;
    }
    if third {
        let fix = third_person(verb)?;
        if is_in(prev, BARE_AFTER) {
            return None;
        }
        // `it need not`, `he dare not`.
        if is_in(next, &["not", "only"]) {
            return None;
        }
        if w == "it" {
            // `make it work`, `let it run`, `watch it fail`: `it` is an object unless it opens
            // a clause. `the characters before it represent`: a preposition's object.
            let subject = cx.sentence_start(k)
                || matches!(cx.before_nl(k).map(|t| t.kind), Some(Kind::Punct(',')))
                || is_in(prev, SUBJECT_OPENERS);
            if !subject || NOT_VERB_AFTER_IT.contains(&verb) {
                return None;
            }
            // `It main use case is`, `It single type`: `its` before a noun.
            if lex(verb).adj
                && cx
                    .plain(k, 2)
                    .is_some_and(|n| lex(n).nounish() && !lex(n).function)
            {
                return None;
            }
        } else {
            // `if he run` (subjunctive), `before he enter`; `of he base`, `fromt he file`,
            // `he type tree is` (lowercase at a sentence start): typos of `the`.
            let the_typo = prev.is_some_and(|p| {
                let l = lex(p);
                l.prep
                    || !l.known
                    || DETS.contains(&p)
                    // `return he result`: a verb's object, where `he` cannot go.
                    || (l.verb && !l.function && !l.part && !CLAUSE_VERBS.contains(&p))
            }) || (cx.sentence_start(k) && !cx.capitalized(k));
            if the_typo || is_in(prev, &["if", "before", "until", "till", "lest"]) {
                return None;
            }
        }
        return Some(fix);
    }
    let fix = base_form(verb, w)?;
    if is_in(prev, &["or", "nor"]) {
        return None;
    }
    if w == "you" {
        // `lets you sets`: after `let` / `make` / `help` the verb is bare too.
        let causative = is_in(
            prev,
            &[
                "let", "lets", "letting", "make", "makes", "making", "made", "help", "helps",
                "helping", "helped",
            ],
        );
        // `give you results`, `tell you is`, `in you is`: `you` is an object. `you guys`.
        if !causative && prev.is_some_and(object_you) || matches!(verb, "guys" | "folks") {
            return None;
        }
    }
    Some(fix)
}

/// Subordinators and conjunctions after which a noun phrase is its clause's subject: `if the
/// server run`, `because the files contains`. Left out: `as` (`such as the file size`), `and` /
/// `or` (compound subjects), verbs (`make the server run`) and words that are prepositions too
/// (`the entries since the last flush are`, `the options after the flag are`).
const SV_OPENERS: &[&str] = &[
    "if", "when", "whenever", "because", "while", "whilst", "although", "though", "unless", "once",
    "but", "so", "where", "wherever", "then", "now",
];

/// Openers after which a singular subject takes the subjunctive `were`: `if the server were
/// down`.
const SUBJUNCTIVE_OPENERS: &[&str] = &[
    "if", "unless", "though", "although", "whether", "where", "wherever",
];

/// Determiners of a singular noun: `the server`, `each file`. `that` (a conjunction or relative
/// pronoun just as often), `any` and `no` are left out.
const SG_DETS: &[&str] = &[
    "the", "a", "an", "this", "each", "every", "another", "one", "my", "your", "our", "their",
    "his", "her", "its",
];

/// Number words that determine a plural noun: `two tests`.
const NUMBER_DETS: &[&str] = &[
    "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten", "eleven", "twelve",
];

/// Determiners of a plural noun that are never singular pronouns, so a noun follows them: `the
/// logs shows` (`logs` a noun), unlike `all runs fine`. Plus [`NUMBER_DETS`].
const PL_DETS_STRICT: &[&str] = &[
    "the", "these", "those", "many", "several", "both", "various", "multiple", "my", "your", "our",
    "their", "his", "her", "its",
];

/// Units: a number of them is an amount, which takes a singular verb (`four spaces is enough`).
const SV_UNITS: &[&str] = &[
    "spaces",
    "bytes",
    "bits",
    "seconds",
    "minutes",
    "hours",
    "days",
    "weeks",
    "months",
    "years",
    "times",
    "dollars",
    "euros",
    "cents",
    "miles",
    "meters",
    "metres",
    "inches",
    "feet",
    "percent",
    "lines",
    "characters",
    "chars",
    "columns",
    "rows",
    "points",
    "pixels",
    "kilobytes",
    "megabytes",
    "gigabytes",
    "milliseconds",
];

/// Heads a subject-verb number check leaves alone: collective and invariant nouns that take
/// either number (`the team are`, `the data is`, `the fish swim`), quantity nouns whose `of`
/// complement decides (`the rest are`), nouns that are not subjects in adverbial phrases
/// (`each time`, `every day`) and elliptic heads (`the first`, `the following`, `the maximum`).
const SV_HEAD_SKIP: &[&str] = &[
    "data",
    "metadata",
    "staff",
    "police",
    "media",
    "series",
    "species",
    "news",
    "team",
    "group",
    "committee",
    "council",
    "panel",
    "family",
    "crew",
    "board",
    "government",
    "company",
    "audience",
    "public",
    "jury",
    "band",
    "party",
    "crowd",
    "army",
    "community",
    "population",
    "management",
    "faculty",
    "club",
    "union",
    "couple",
    "pair",
    "majority",
    "minority",
    "number",
    "lot",
    "bunch",
    "handful",
    "rest",
    "remainder",
    "total",
    "kind",
    "sort",
    "type",
    "variety",
    "range",
    "set",
    "means",
    "headquarters",
    "crossroads",
    "whereabouts",
    "fish",
    "sheep",
    "deer",
    "aircraft",
    "offspring",
    "personnel",
    "none",
    "half",
    "percent",
    "percentage",
    "proportion",
    "fraction",
    "part",
    "portion",
    "other",
    "same",
    "former",
    "latter",
    "following",
    "above",
    "below",
    "first",
    "second",
    "third",
    "last",
    "next",
    "final",
    "best",
    "worst",
    "top",
    "bottom",
    "maximum",
    "minimum",
    "max",
    "min",
    "average",
    "time",
    "day",
    "week",
    "month",
    "year",
    "night",
    "morning",
    "evening",
    "afternoon",
    "hour",
    "minute",
    "moment",
    "weekend",
    "way",
    "today",
    "tomorrow",
    "yesterday",
    "while",
    "one",
    "ones",
    // `its contents is`: often treated as a mass noun.
    "contents",
    "criteria",
    "phenomena",
];

/// Plurals without `-s`.
const IRREGULAR_PLURALS: &[&str] = &[
    "people", "children", "men", "women", "indices", "vertices", "matrices", "mice", "feet",
    "teeth", "geese",
];

/// Verbs whose simple past is the base form: `Gatsby's foot beat a tattoo`.
const PAST_AS_BASE: &[&str] = &[
    "beat",
    "bet",
    "bid",
    "broadcast",
    "burst",
    "cast",
    "cost",
    "cut",
    "fit",
    "forecast",
    "hit",
    "hurt",
    "let",
    "put",
    "quit",
    "read",
    "rid",
    "set",
    "shed",
    "shut",
    "slit",
    "spit",
    "split",
    "spread",
    "thrust",
    "upset",
    "wet",
];

/// Words that read as verbs but head noun phrases after a noun: `a big thank you`, `a shout out`,
/// `many optimisations thanks to`, `the container init process`, `returns true`.
const SV_NOT_VERBS: &[&str] = &[
    "thank",
    "thanks",
    "shout",
    "shouts",
    "init",
    "true",
    "false",
    "zero",
    "login",
    "logout",
    "setup",
    "backup",
    "lookup",
    "cleanup",
    "shutdown",
    "startup",
    "rollback",
    "checkout",
    "signup",
    "teardown",
    "handshake",
];

/// Adverbs that may stand between a subject and its verb: `the server also run`.
fn sv_adverb(w: &str) -> bool {
    matches!(
        w,
        "also"
            | "always"
            | "never"
            | "often"
            | "sometimes"
            | "still"
            | "only"
            | "just"
            | "already"
            | "usually"
            | "typically"
            | "automatically"
            | "currently"
            | "rarely"
            | "generally"
            | "normally"
            | "actually"
            | "simply"
            | "really"
            | "frequently"
            | "occasionally"
            | "periodically"
            | "silently"
    )
}

/// Word `h` may head a subject noun phrase at all: letters only, not an auxiliary, a number or
/// one of [`SV_HEAD_SKIP`].
fn sv_head_ok(cx: &Cx, h: usize) -> bool {
    let w = cx.wl[h].as_str();
    w.chars().all(char::is_alphabetic)
        && !SV_HEAD_SKIP.contains(&w)
        && !w.ends_with("ics")
        && !AUX.contains(&w)
        && !UNINFLECTED.contains(&w)
        && !NUMBERS.contains(&w)
        && !NUMBER_DETS.contains(&w)
}

/// Word `h` heads a singular subject: `server`, `config`, `status`, `API`.
fn sv_singular_head(cx: &Cx, h: usize) -> bool {
    let w = cx.wl[h].as_str();
    if !sv_head_ok(cx, h) {
        return false;
    }
    let l = lex(w);
    if cx.all_caps(h) {
        return w.len() >= 2 && !w.ends_with('s') && !l.function && !l.adv;
    }
    if cx.capitalized(h) {
        return false;
    }
    if w.ends_with('s') && !["ss", "us", "is", "as"].iter().any(|e| w.ends_with(e)) {
        return false;
    }
    // `the middleware are`: a mass noun used for several things.
    l.known
        && l.noun
        && !l.plural
        && !l.function
        && !l.ing
        && !l.part
        && !l.adv
        && !words().meta(w).is_some_and(|m| m.is_mass_noun_only())
}

/// Word `h` heads a plural subject: `files`, `people`, `APIs`. The singular (`file`) must be a
/// noun too, so `status` or `news` never count.
fn sv_plural_head(cx: &Cx, h: usize) -> bool {
    let t = cx.tokens[cx.words[h]];
    let orig: String = cx.chars[t.start..t.end].iter().collect();
    let w = cx.wl[h].as_str();
    if !sv_head_ok(cx, h) {
        return false;
    }
    // `APIs`, `URLs`.
    if orig.len() >= 3
        && orig.ends_with('s')
        && orig[..orig.len() - 1]
            .chars()
            .all(|c| c.is_ascii_uppercase())
    {
        return true;
    }
    if cx.capitalized(h) {
        return false;
    }
    if IRREGULAR_PLURALS.contains(&w) {
        return true;
    }
    if !w.ends_with('s') || ["ss", "us", "is"].iter().any(|e| w.ends_with(e)) {
        return false;
    }
    let l = lex(w);
    if !(l.noun && l.plural && !l.function) {
        return false;
    }
    let mut stems = vec![w[..w.len() - 1].to_string()];
    if let Some(s) = w.strip_suffix("es") {
        stems.push(s.to_string());
    }
    if let Some(s) = w.strip_suffix("ies") {
        stems.push(format!("{s}y"));
    }
    stems.iter().any(|s| {
        let l = lex(s);
        l.noun && !l.function
    })
}

/// A word that may modify the head of a subject: an adjective, a noun, a participle or a
/// proper name (`the new`, `the user`, `the cached`, `the Redis`).
fn sv_modifier(cx: &Cx, i: usize) -> bool {
    let w = cx.wl[i].as_str();
    if !w.chars().all(char::is_alphabetic)
        || AUX.contains(&w)
        || UNINFLECTED.contains(&w)
        || NUMBERS.contains(&w)
    {
        return false;
    }
    let l = lex(w);
    if cx.capitalized(i) && !l.known {
        return true;
    }
    l.known
        && (l.adj || l.noun || l.part || l.ing)
        && !l.function
        && !l.adv_only()
        && !DETS.contains(&w)
        && !(l.finite && !l.noun && !l.adj)
}

/// The subject noun phrase that ends right before verb `v`: `(start, determiner, head)`. At
/// most one adverb between head and verb, at most two modifiers between determiner and head,
/// all separated by single spaces. A possessive (`the user's`) counts as a determiner.
fn sv_subject(cx: &Cx, v: usize) -> Option<(usize, usize, usize)> {
    let linked = |i: usize| cx.link[i] && !cx.gap_after(i);
    let mut h = v.checked_sub(1)?;
    if !linked(h) {
        return None;
    }
    if sv_adverb(&cx.wl[h]) && !cx.capitalized(h) {
        h = h.checked_sub(1)?;
        if !linked(h) {
            return None;
        }
    }
    let mut i = h;
    for _ in 0..3 {
        i = i.checked_sub(1)?;
        if !linked(i) {
            return None;
        }
        let w = cx.wl[i].as_str();
        let possessive = w.len() > 2
            && w.ends_with("'s")
            && !matches!(
                w,
                "it's"
                    | "that's"
                    | "there's"
                    | "what's"
                    | "here's"
                    | "who's"
                    | "let's"
                    | "he's"
                    | "she's"
                    | "where's"
                    | "how's"
            );
        if DETS.contains(&w) || NUMBER_DETS.contains(&w) || possessive {
            let mut start = i;
            if possessive && i > 0 && linked(i - 1) && DETS.contains(&cx.wl[i - 1].as_str()) {
                start = i - 1;
            }
            // `all the files`, `both the tests`.
            if start > 0 && linked(start - 1) && matches!(cx.wl[start - 1].as_str(), "all" | "both")
            {
                start -= 1;
            }
            return Some((start, i, h));
        }
        if !sv_modifier(cx, i) {
            return None;
        }
    }
    None
}

/// Word `d` starts a clause's subject: a sentence start, a comma or a subordinator before it.
fn sv_open(cx: &Cx, d: usize, singular: bool) -> bool {
    // Right after blanked inline code (`` `foo` the files ``).
    let code_before = cx.words[d]
        .checked_sub(1)
        .map(|i| cx.tokens[i])
        .is_some_and(|t| t.kind == Kind::Space && t.end - t.start >= 3);
    if cx.hyphenated(d) || cx.quoted(d) || code_before {
        return false;
    }
    if cx.sentence_start(d) {
        // `the storage vs. the debt are`: lowercase after a period is an abbreviation.
        let period = matches!(cx.before_nl(d).map(|t| t.kind), Some(Kind::Punct('.')));
        return !period || cx.capitalized(d);
    }
    if cx.capitalized(d) {
        return false;
    }
    if matches!(cx.before_nl(d).map(|t| t.kind), Some(Kind::Punct(','))) {
        return true;
    }
    // `to scale the model to that many replicas is guaranteed`.
    if cx.prev(d, 1) == Some("that") && matches!(cx.wl[d].as_str(), "many" | "much" | "few") {
        return false;
    }
    // `every time the autoscaler add`: a conjunction.
    if cx.prev(d, 1) == Some("time")
        && is_in(
            cx.prev(d, 2),
            &["every", "each", "any", "next", "first", "last"],
        )
    {
        return true;
    }
    match cx.prev(d, 1) {
        // `ensure that the user have access`: the subjunctive.
        Some("that") => !singular,
        p => is_in(p, SV_OPENERS),
    }
}

/// Objects a verb can take: `the server run the job`.
const OBJECT_START: &[&str] = &[
    "the", "a", "an", "this", "these", "those", "it", "them", "its", "their", "our", "your", "his",
    "her", "all", "any", "each", "every", "some", "several", "multiple", "both", "us", "me", "him",
    "you",
];

/// Lexical verb `v` reads as a verb after its subject, not as the last noun of the subject (`the
/// config change broke it`, `the user request body`, `a big thank you`). `plural`: the subject is
/// plural, where a noun compound is unlikely (`the tests passes.`). `loose`: after `each of the
/// files`, where no compound goes either.
fn sv_verb_slot(cx: &Cx, v: usize, det: &str, plural: bool, loose: bool) -> bool {
    let w = cx.wl[v].as_str();
    let l = lex(w);
    if l.adv || l.function || NOT_VERB_AFTER_IT.contains(&w) || SV_NOT_VERBS.contains(&w) {
        return false;
    }
    let next = cx.next(v, 1);
    let verb_next = next.is_some_and(|n| {
        let l = lex(n);
        AUX.contains(&n) || (l.finite && !l.noun && !l.adj)
    });
    if verb_next || next == Some("of") || cx.after(v) == Some(Kind::Punct('(')) {
        return false;
    }
    // `the channel notify here is safe`, `if an attribute correspond with a name is found`:
    // another verb follows before any object, so `v` belongs to the subject.
    for n in 1..=6 {
        let Some(x) = cx.next(v, n) else { break };
        if OBJECT_START.contains(&x) || matches!(x, "that" | "which" | "who" | "my") {
            break;
        }
        if AUX.contains(&x) {
            return false;
        }
    }
    if loose {
        return true;
    }
    // `the usual enqueue and dequeue operations`, `read/write`.
    if is_in(next, &["and", "or", "nor"]) || cx.after(v) == Some(Kind::Punct('/')) {
        return false;
    }
    if !(l.noun || l.adj) {
        return true;
    }
    // `a big thank you`, `a full stack web framework`.
    if !plural && matches!(det, "a" | "an") {
        return false;
    }
    // `the bounds checks to avoid`, `the quartiles values for`, `the tests results show`: a
    // plural noun after a plural noun is a (misspelled) compound unless an object follows.
    let plural_noun = l.plural && w.ends_with('s') && !(w == "needs" && next == Some("to"));
    // `the server run out of memory`; after a plural head `the file sizes differs.`, `the
    // tests pass on Windows`, `the APIs returns JSON`.
    if is_in(next, &["out", "down", "away", "back", "off"])
        || (next == Some("up") && cx.next(v, 2) != Some("of"))
        || (plural && next.is_some() && cx.capitalized(v + 1))
        || (plural
            && !plural_noun
            && (cx.punct_end(v) || next.is_some_and(|n| lex(n).prep || lex(n).adv_only())))
    {
        return true;
    }
    // `the main type you want`: `you` / `it` may start a relative clause.
    if !is_in(next, OBJECT_START) || is_in(next, &["you", "it"]) {
        return false;
    }
    // `the config change the team made`, `the default port the exporter uses`: a relative
    // clause after a noun phrase.
    !(2..=5).any(|n| {
        cx.next(v, n).is_some_and(|x| {
            let lx = lex(x);
            let after_noun = cx.prev(v + n, 1).is_some_and(|p| {
                let lp = lex(p);
                lp.noun && !lp.function
            });
            AUX.contains(&x) || (lx.finite && !lx.adj && (!lx.noun || after_noun))
        })
    })
}

/// `SubjectVerbAgreement`: a verb right after a noun subject has the wrong number (`the server
/// run`, `the files contains`, `the results is`, `each of the tests are`). Returns the fix.
fn subject_verb(cx: &Cx, v: usize) -> Option<String> {
    let w = cx.wl[v].as_str();
    if cx.capitalized(v) || cx.quoted(v) || cx.hyphenated(v) || cx.gap_after(v) {
        return None;
    }
    let (sg_fix, pl_fix): (Option<String>, Option<String>) = match w {
        "are" => (Some("is".into()), None),
        "were" => (Some("was".into()), None),
        "aren't" => (Some("isn't".into()), None),
        "weren't" => (Some("wasn't".into()), None),
        "have" => (Some("has".into()), None),
        "haven't" => (Some("hasn't".into()), None),
        "do" => (Some("does".into()), None),
        "don't" => (Some("doesn't".into()), None),
        "is" => (None, Some("are".into())),
        "was" => (None, Some("were".into())),
        "isn't" => (None, Some("aren't".into())),
        "wasn't" => (None, Some("weren't".into())),
        "has" => (None, Some("have".into())),
        "hasn't" => (None, Some("haven't".into())),
        "does" => (None, Some("do".into())),
        "doesn't" => (None, Some("don't".into())),
        _ if w.contains('\'') || !w.chars().all(char::is_alphabetic) => return None,
        _ if PAST_AS_BASE.contains(&w) || SV_NOT_VERBS.contains(&w) => return None,
        _ => {
            // `hid` -> `hides`: `-es` only after a sibilant or `o`.
            let sg = third_person(w).filter(|f| {
                f.strip_prefix(w) != Some("es")
                    || ["s", "sh", "ch", "x", "z", "o"]
                        .iter()
                        .any(|e| w.ends_with(e))
            });
            (sg, base_form(w, "they"))
        }
    };
    if sg_fix.is_none() && pl_fix.is_none() {
        return None;
    }
    let lexical = !w.contains('\'')
        && !matches!(
            w,
            "are" | "were" | "have" | "do" | "is" | "was" | "has" | "does"
        );
    let subjunctive = |d: usize| {
        matches!(w, "were" | "weren't")
            && (is_in(cx.prev(d, 1), SUBJUNCTIVE_OPENERS)
                || is_in(cx.prev(d, 2), &["as", "even", "only", "wish"]))
    };
    // `each of the files are`, `one of them fail`, `the number of retries are`.
    if let Some(fix) = &sg_fix
        && let Some(of) = sv_of_phrase(cx, v)
    {
        let q = of.checked_sub(1)?;
        // `one of these access specifiers`: `these` a determiner, not a pronoun.
        if !cx.link[q] || (lexical && matches!(cx.wl[v - 1].as_str(), "these" | "those")) {
            return None;
        }
        let start = match cx.wl[q].as_str() {
            "each" | "one" => q,
            "number" => {
                let mut s = q.checked_sub(1)?;
                if !cx.link[s] {
                    return None;
                }
                if matches!(
                    cx.wl[s].as_str(),
                    "total" | "maximum" | "minimum" | "max" | "min" | "average" | "overall"
                ) {
                    s = s.checked_sub(1)?;
                    if !cx.link[s] {
                        return None;
                    }
                }
                if cx.wl[s] != "the" {
                    return None;
                }
                s
            }
            _ => return None,
        };
        let open = sv_open(cx, start, true)
            || is_in(cx.prev(start, 1), &["only", "least", "exactly", "just"]);
        // `all but one of the characters are removed`: several things.
        let all_but = cx.prev(start, 1) == Some("but") && cx.prev(start, 2) == Some("all");
        if !open
            || all_but
            || subjunctive(start)
            || (lexical && !sv_verb_slot(cx, v, "", true, true))
        {
            return None;
        }
        return Some(fix.clone());
    }
    // `Email addresses is unique`: a bare plural with a modifier opening the sentence.
    if let Some(fix) = &pl_fix
        && !lexical
        && sv_bare_plural(cx, v)
    {
        return Some(fix.clone());
    }
    let (start, det, h) = sv_subject(cx, v)?;
    let d = cx.wl[det].as_str();
    let possessive = d.ends_with("'s");
    let slot = |plural: bool| !lexical || sv_verb_slot(cx, v, d, plural, false);
    if let Some(fix) = sg_fix
        && (SG_DETS.contains(&d) || possessive)
        // `all the middleware are`: a mass noun used for several things.
        && !matches!(cx.wl[start].as_str(), "all" | "both")
        && sv_singular_head(cx, h)
        // `the sales team`, `the encoded values use are`.
        && !(det + 1..h).any(|i| lex(&cx.wl[i]).plural)
        && sv_open(cx, start, true)
        && !subjunctive(start)
        && slot(false)
    {
        let next = cx.next(v, 1);
        // `the SSL verify, PSK client`: an acronym is often a modifier.
        if lexical && cx.all_caps(h) && !is_in(next, OBJECT_START) {
            return None;
        }
        // `the key take away`, `the main type you want`, `the stale acquire work`: after an
        // adjective the verb may be a noun.
        let content_next = next.is_some_and(|n| {
            let l = lex(n);
            !OBJECT_START.contains(&n) && !l.function && !l.adv_only() && (l.noun || !l.known)
        });
        if lexical && lex(&cx.wl[h]).adj && (lex(w).noun || content_next) {
            return None;
        }
        // `all but one receiver are`, `all but the last field are`: several things.
        if cx.prev(start, 1) == Some("but") && cx.prev(start, 2) == Some("all") {
            return None;
        }
        return Some(fix);
    }
    if let Some(fix) = pl_fix
        && (PL_DETS_STRICT.contains(&d)
            || NUMBER_DETS.contains(&d)
            || matches!(d, "all" | "some")
            || possessive)
        && sv_plural_head(cx, h)
        && sv_open(cx, start, false)
        && slot(true)
    {
        // `four spaces is enough`, `two hours gives time`: an amount.
        if NUMBER_DETS.contains(&d) && (!lexical || SV_UNITS.contains(&cx.wl[h].as_str())) {
            return None;
        }
        // `the server logs requests`: `logs` may be the verb and `requests` a noun; `the
        // syntax SQLite supports is`: a relative clause. Only right after a determiner that no
        // verb can follow is such a head surely a noun.
        let ambiguous = words()
            .meta(&cx.wl[h])
            .is_some_and(|m| m.is_verb_third_person_singular_present_form());
        if ambiguous
            && (!lexical || lex(w).noun)
            && (h != det + 1
                || !(PL_DETS_STRICT.contains(&d) || NUMBER_DETS.contains(&d) || possessive))
        {
            return None;
        }
        return Some(fix);
    }
    None
}

/// A plural subject without a determiner opens a sentence right before verb `v`: `Email
/// addresses is`. It needs a modifier or two, all lowercase but a
/// sentence's first word, and no gerund (`Sending emails is slow`) or quantifier (`More tests
/// is better`).
fn sv_bare_plural(cx: &Cx, v: usize) -> bool {
    let Some(h) = v.checked_sub(1) else {
        return false;
    };
    if !cx.link[h]
        || cx.gap_after(h)
        || !sv_plural_head(cx, h)
        || cx.all_caps(h)
        || SV_UNITS.contains(&cx.wl[h].as_str())
        // `Go protocol buffers is an open source project`: the name of one thing.
        || is_in(cx.next(v, 1), &["a", "an", "one"])
    {
        return false;
    }
    let bare_modifier = |i: usize| {
        let w = cx.wl[i].as_str();
        let l = lex(w);
        cx.link[i]
            && !cx.gap_after(i)
            && sv_modifier(cx, i)
            && l.known
            && !l.ing
            && !l.part
            && !l.plural
            && !matches!(
                w,
                "more" | "fewer" | "less" | "most" | "many" | "few" | "enough" | "extra" | "too"
            )
    };
    for f in [h.checked_sub(1), h.checked_sub(2)].into_iter().flatten() {
        if !(f..h).all(bare_modifier) || (f + 1..h).any(|i| cx.capitalized(i)) {
            return false;
        }
        let code_before = cx.words[f].checked_sub(1).is_some_and(|i| {
            cx.tokens[i].kind == Kind::Space && cx.tokens[i].end - cx.tokens[i].start >= 3
        });
        if code_before || cx.quoted(f) || cx.hyphenated(f) {
            return false;
        }
        // Not after `if` / `when`: `if happy eyeballs is enabled` names a feature.
        if cx.sentence_start(f) && cx.capitalized(f) {
            return true;
        }
    }
    false
}

/// The `of` before a plural noun phrase that ends right before verb `v`: `of the files`, `of
/// them`, `of retries`.
fn sv_of_phrase(cx: &Cx, v: usize) -> Option<usize> {
    let h = v.checked_sub(1)?;
    if !cx.link[h] || cx.gap_after(h) {
        return None;
    }
    if matches!(cx.wl[h].as_str(), "them" | "these" | "those" | "us") {
        let of = h.checked_sub(1)?;
        return (cx.link[of] && cx.wl[of] == "of").then_some(of);
    }
    if !sv_plural_head(cx, h) {
        return None;
    }
    // `the number of references reaches zero`: `reaches` may be the verb.
    let ambiguous = words()
        .meta(&cx.wl[h])
        .is_some_and(|m| m.is_verb_third_person_singular_present_form());
    let mut i = h;
    for _ in 0..4 {
        i = i.checked_sub(1)?;
        if !cx.link[i] || cx.gap_after(i) {
            return None;
        }
        let w = cx.wl[i].as_str();
        if w == "of" {
            return Some(i);
        }
        if PL_DETS_STRICT.contains(&w) {
            continue;
        }
        let l = lex(w);
        if (ambiguous && l.noun && !l.adj) || !sv_modifier(cx, i) || l.plural {
            return None;
        }
    }
    None
}

/// Harper rules whose findings [`veto`] re-checks with the same word-class tests as ours.
pub const VETOED: &[&str] = &["ThenThan", "ItsContraction", "ItsPossessive", "ThereOwn"];

/// Drop findings of the [`VETOED`] rules (Harper's or ours) whose context rules them out.
pub fn veto(chars: &[char], tokens: &[Token], lints: &mut BTreeMap<String, Vec<Lint>>) {
    if !VETOED
        .iter()
        .any(|n| lints.get(*n).is_some_and(|v| !v.is_empty()))
    {
        return;
    }
    let words: Vec<usize> = (0..tokens.len())
        .filter(|&i| tokens[i].kind == Kind::Word)
        .collect();
    let lower: Vec<String> = tokens
        .iter()
        .map(|t| t.text(chars).to_lowercase())
        .collect();
    let spaced = |a: usize, b: usize| spaced(tokens, a, b);
    let cx = Cx::new(chars, tokens, &words, &lower, &spaced);
    for name in VETOED {
        if let Some(v) = lints.get_mut(*name) {
            v.retain(|l| {
                let Some(k) = words.iter().position(|&t| tokens[t].start == l.span.start) else {
                    return true;
                };
                !cx.quoted(k) && !vetoed(name, &cx, k)
            });
        }
    }
}

fn vetoed(name: &str, cx: &Cx, k: usize) -> bool {
    let w = cx.wl[k].as_str();
    match (name, w) {
        ("ThenThan", "then") => {
            let Some(p) = cx.prev(k, 1) else {
                return false;
            };
            // `even then`; `OTHER then`; `to lower then`; `the number then`, `a header then`:
            // not comparatives. `if lower then buf is valid`: a clause follows.
            p == "even"
                || cx.all_caps(k - 1)
                || cx.prev(k, 2) == Some("to")
                || ((p.ends_with("er") || p.ends_with("or"))
                    && !matches!(p, "other" | "rather" | "further" | "farther" | "sooner")
                    && !COMPARATIVES.contains(&p)
                    && !MORE_ADJECTIVES.contains(&p)
                    && lex(p).noun)
                || is_in(cx.next(k, 2), AUX)
                // `if it is larger then any remaining registers are ignored`, `if targeting P
                // or later then passing these values will fail`: after a condition, a clause
                // with its own verb; but `bigger then the other end can accept`.
                || (k.saturating_sub(15)..k)
                    .any(|i| matches!(cx.wl[i].as_str(), "if" | "when" | "whenever" | "unless"))
                    && (3..=5).any(|i| {
                    is_in(
                        cx.next(k, i),
                        &[
                            "will", "would", "should", "can", "could", "must", "may", "might",
                            "shall", "are", "were", "won't", "cannot", "can't",
                        ],
                    )
                })
        }
        ("ItsContraction", "its") => its_possessive_context(cx, k),
        ("ItsPossessive", "it's") => its_contraction_context(cx, k),
        ("ThereOwn", "there") => there_own_ok(cx, k),
        _ => false,
    }
}

/// `people there own nice cars`, `companies there own the property`: `own` is a verb.
fn there_own_ok(cx: &Cx, k: usize) -> bool {
    is_in(cx.next(k, 2), DETS)
        || cx.prev(k, 1).is_some_and(|p| {
            let l = lex(p);
            l.plural && !l.function && !l.part
        })
}

/// Repetitions that are grammatical or idiomatic.
const REPEAT_OK: &[&str] = &[
    "had", "that", "bye", "so", "ha", "hear", "chop", "tut", "knock", "boo", "choo", "go", "night",
    "well", "there", "very", "really", "blah", "la", "ho", "hip", "mahi", "aye", "yada", "dum",
    "tick", "ding", "bla", "wait", "never", "far", "round", "again", "over", "more", "many", "you",
    "do", "can",
];

/// Common comparative adjectives and adverbs (followed by `then`, they want `than`).
const COMPARATIVES: &[&str] = &[
    "more",
    "less",
    "better",
    "worse",
    "fewer",
    "greater",
    "larger",
    "smaller",
    "bigger",
    "higher",
    "lower",
    "faster",
    "slower",
    "quicker",
    "easier",
    "harder",
    "simpler",
    "cheaper",
    "longer",
    "shorter",
    "safer",
    "cleaner",
    "clearer",
    "stronger",
    "weaker",
    "lighter",
    "heavier",
    "earlier",
    "wider",
    "narrower",
    "deeper",
    "shallower",
    "louder",
    "quieter",
    "richer",
    "poorer",
    "smarter",
    "nicer",
    "further",
    "farther",
    "likelier",
    "busier",
    "happier",
    "tighter",
    "looser",
    "stricter",
    "broader",
    "thinner",
    "thicker",
    "denser",
    "closer",
    "sooner",
    "rarer",
    "fresher",
    "cooler",
    "warmer",
    "hotter",
    "colder",
    "younger",
    "older",
    "newer",
    "riskier",
    "costlier",
    "leaner",
    "smoother",
    "tinier",
    "steeper",
    "softer",
    "cheaper",
    "sharper",
    "stabler",
    "sturdier",
    "saner",
    "wiser",
    "worthier",
];

/// Common adjectives after `more` / `less` that [`adjective_like`] misses.
const MORE_ADJECTIVES: &[&str] = &[
    "common",
    "often",
    "complex",
    "secure",
    "strict",
    "stable",
    "robust",
    "verbose",
    "difficult",
    "accurate",
    "precise",
    "concise",
    "mature",
    "compact",
    "recent",
    "complicated",
    "sophisticated",
    "detailed",
    "advanced",
    "popular",
    "similar",
    "familiar",
    "flexible",
    "readable",
    "likely",
];

/// `more X then`: X looks like an adjective or adverb by its ending.
fn adjective_like(w: &str) -> bool {
    [
        "able", "ible", "ful", "ous", "ive", "ic", "al", "ent", "ant", "ly", "less", "ary", "ile",
    ]
    .iter()
    .any(|s| w.len() > s.len() + 2 && w.ends_with(s))
}

/// How the number whose integer digits are `digits` starts when read aloud: `8`, `80`, `800`,
/// `8,000` (eight...), `11` (eleven) and `18` (eighteen) with a vowel; `1` / `100` (one, a
/// hundred), `429` (four...) and `30` (thirty) with a consonant. `1100` / `1800` (eleven
/// hundred or one thousand...) and `08` are read either way.
pub fn number_sound(digits: &str) -> Sound {
    let d: Vec<u8> = digits
        .bytes()
        .filter(u8::is_ascii_digit)
        .map(|b| b - b'0')
        .collect();
    let n = d.len();
    if n == 0 || (n > 1 && d[0] == 0) || (n == 4 && d[0] == 1 && matches!(d[1], 1 | 8)) {
        return Sound::Either;
    }
    let vowel = match n % 3 {
        2 => {
            let v = d[0] * 10 + d[1];
            v == 11 || v == 18 || (80..=89).contains(&v)
        }
        _ => d[0] == 8,
    };
    if vowel {
        Sound::Vowel
    } else {
        Sound::Consonant
    }
}

/// `a` / `an` right before a number (`a 8-byte key`, `an 100 ms delay`, `a 80% drop`, `a 8GB
/// disk`): the byte range of each wrong article in the segment text `text` and its fix. `orig`
/// is the source under the segment (same length), so numbers blanked as noise (`8GB`, `10ms`,
/// `8th`) still count; a number in code (`` a `8` ``) never follows the article directly there.
pub fn ana_numbers(text: &str, orig: &str) -> Vec<(std::ops::Range<usize>, &'static str)> {
    let (t, o) = (text.as_bytes(), orig.as_bytes());
    let mut out = Vec::new();
    if t.len() != o.len() {
        return out;
    }
    let mut i = 0;
    while i < t.len() {
        let start = i;
        if !t[i].is_ascii_alphabetic() {
            i += 1;
            continue;
        }
        while i < t.len() && (t[i].is_ascii_alphanumeric() || matches!(t[i], b'_' | b'\'' | b'-')) {
            i += 1;
        }
        let word = &text[start..i];
        let is_a = match word {
            "a" | "A" => true,
            "an" | "An" => false,
            _ => continue,
        };
        // The article starts a word in the source too (not `x.a`, `` `b`a ``).
        if start > 0 && !(o[start - 1].is_ascii_whitespace() || matches!(o[start - 1], b'(' | b'"'))
        {
            continue;
        }
        // `Row A 8`, `option A 12`: a capital `A` mid-sentence is a label.
        let before = text[..start].trim_end();
        if word == "A" && !(before.is_empty() || before.ends_with(['.', '!', '?', ':'])) {
            continue;
        }
        // Whitespace (at most one line break), the same in the source, then a digit.
        let mut m = i;
        while m < t.len() && matches!(t[m], b' ' | b'\t' | b'\n') && t[m] == o[m] {
            m += 1;
        }
        if m == i
            || m >= t.len()
            || !o[m].is_ascii_digit()
            || t[i..m].iter().filter(|&&b| b == b'\n').count() > 1
        {
            continue;
        }
        // Integer digits, `,` only as a thousands separator.
        let mut e = m;
        let mut digits = String::new();
        while e < o.len() {
            if o[e].is_ascii_digit() {
                digits.push(o[e] as char);
                e += 1;
            } else if o[e] == b','
                && o.len() > e + 3
                && o[e + 1..e + 4].iter().all(u8::is_ascii_digit)
                && !o.get(e + 4).is_some_and(u8::is_ascii_digit)
            {
                e += 1;
            } else {
                break;
            }
        }
        // `a 1/2 inch` (a half), `a 3_000`, `a 0x10`: other readings.
        if matches!(o.get(e), Some(b'/' | b'_' | b'x' | b'X')) && digits == "0"
            || matches!(o.get(e), Some(b'/' | b'_'))
        {
            continue;
        }
        // `a 89ab`, `a 8f3c` (hex: `a` is a variable or the reading unclear); not `8b`, `30d`.
        let suffix: Vec<u8> = o[e..]
            .iter()
            .take_while(|b| b.is_ascii_alphanumeric())
            .copied()
            .collect();
        if suffix.len() >= 2 && suffix.iter().all(u8::is_ascii_hexdigit) {
            continue;
        }
        // `(8kb, an 16kb and 32kb)`, `between 0 an 1`: `an` is a typo of `and`.
        let prev_token = orig[..start]
            .trim_end()
            .trim_end_matches(',')
            .trim_end()
            .rsplit(char::is_whitespace)
            .next()
            .unwrap_or("")
            .trim_start_matches(|c: char| !c.is_alphanumeric());
        if prev_token.starts_with(|c: char| c.is_ascii_digit()) {
            continue;
        }
        let want_a = match number_sound(&digits) {
            Sound::Vowel => false,
            Sound::Consonant => true,
            Sound::Either => continue,
        };
        if want_a != is_a {
            let fix = match (is_a, word.starts_with('A')) {
                (true, true) => "An",
                (true, false) => "an",
                (false, true) => "A",
                (false, false) => "a",
            };
            out.push((start..start + word.len(), fix));
        }
    }
    out
}

#[derive(Debug, PartialEq, Eq)]
pub enum Sound {
    Vowel,
    Consonant,
    /// Readers differ (`a/an SQL`, `a/an URL`): no finding either way.
    Either,
}

/// Whether `word` starts with a vowel sound, for `a` / `an`.
pub fn initial_sound(word: &str, american: bool) -> Sound {
    let Some(first) = word.chars().next() else {
        return Sound::Either;
    };
    if first.is_ascii_digit() {
        return Sound::Either;
    }
    // A partial initialism (`mDNS`, `RFLink`): the leading run of one case is read as letters.
    let chars: Vec<char> = word.chars().collect();
    let initialism: &[char] = if chars.len() >= 2 && chars[1].is_uppercase() {
        let up = chars[0].is_uppercase();
        let end = chars
            .iter()
            .position(|c| c.is_alphabetic() && c.is_uppercase() != up)
            .unwrap_or(chars.len());
        // `RFLink`: the last capital starts the word part.
        let end = if end < chars.len() && up && end > 1 {
            end - 1
        } else {
            end
        };
        &chars[..end]
    } else {
        &[]
    };
    let upper: String = word.to_uppercase();
    if matches!(
        upper.as_str(),
        "SQL" | "URL" | "URLS" | "LED" | "FAQ" | "FAQS" | "SAT"
    ) {
        return Sound::Either;
    }
    let letters = |c: char| {
        if "AEFHILMNORSX".contains(c.to_ascii_uppercase()) {
            Sound::Vowel
        } else {
            Sound::Consonant
        }
    };
    if chars.len() == 1 {
        return letters(first);
    }
    if !initialism.is_empty() {
        let all_upper = initialism
            .iter()
            .all(|c| !c.is_alphabetic() || c.is_uppercase());
        if !all_upper || !pronounceable(initialism) {
            return letters(first);
        }
    }
    let w = word.to_lowercase();
    let starts = |p: &[&str]| p.iter().any(|p| w.starts_with(p));
    if w == "npm" || (w.starts_with("mp") && w[2..].starts_with(|c: char| c.is_ascii_digit())) {
        return Sound::Vowel;
    }
    if starts(&["ubi"]) {
        return Sound::Either;
    }
    // Silent `h`, `un` + consonant, `herb` (American).
    if starts(&[
        "hour", "honest", "honor", "honour", "heir", "unin", "unim", "x-",
    ]) || (american && w.starts_with("herb"))
    {
        return Sound::Vowel;
    }
    // Vowel letters read as `y`/`w`.
    if starts(&[
        "uni", "usu", "usa", "use", "uti", "ure", "uri", "ura", "uro", "ubiq", "uk", "ufo",
        "ukulele", "eu", "ewe", "once", "unanim", "unary",
    ]) || w == "one"
        || w.starts_with("one-")
        || w.starts_with("u-")
    {
        return Sound::Consonant;
    }
    if w.starts_with(['a', 'e', 'i', 'o', 'u']) {
        Sound::Vowel
    } else {
        Sound::Consonant
    }
}

/// An all-caps word read as a word (`NASA`, `JPEG`) rather than letter by letter: its first
/// three letters go consonant-vowel-consonant or consonant-vowel-vowel.
fn pronounceable(w: &[char]) -> bool {
    let v = |c: &char| "AEIOU".contains(c.to_ascii_uppercase());
    match w {
        [a, b, c, ..] if a.is_alphabetic() && b.is_alphabetic() && c.is_alphabetic() => {
            !v(a) && v(b)
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every rule of ours is described for `rules --all`: by Harper (named like one of Harper's)
    /// or in [`OWN_DESCRIPTIONS`].
    #[test]
    fn rules_are_described() {
        use std::collections::BTreeSet;
        let own: BTreeSet<&str> = OWN_DESCRIPTIONS.iter().map(|d| d.0).collect();
        let named: BTreeSet<&str> = HARPER_NAMED.iter().map(|d| d.0).collect();
        let all: BTreeSet<&str> = RULES
            .iter()
            .chain(CONFUSABLES)
            .copied()
            .filter(|r| !own.contains(r))
            .collect();
        assert_eq!(named, all);
        #[cfg(feature = "harper")]
        {
            use harper_core::linting::LintGroup;
            use harper_core::spell::FstDictionary;
            let g =
                LintGroup::new_curated(FstDictionary::curated(), harper_core::Dialect::American);
            let harper: BTreeSet<&str> = g.iter_keys().collect();
            assert!(named.is_subset(&harper), "{:?}", named.difference(&harper));
            assert!(own.is_disjoint(&harper), "{:?}", own.intersection(&harper));
        }
    }

    #[test]
    fn initial_sounds() {
        use Sound::*;
        for (w, want) in [
            ("hour", Vowel),
            ("honest", Vowel),
            ("heir", Vowel),
            ("unicorn", Consonant),
            ("user", Consonant),
            ("university", Consonant),
            ("one", Consonant),
            ("once", Consonant),
            ("European", Consonant),
            ("FAQ", Either),
            ("SQL", Either),
            ("URL", Either),
            ("HTML", Vowel),
            ("API", Vowel),
            ("JSON", Consonant),
            ("NASA", Consonant),
            ("LLM", Vowel),
            ("USB", Consonant),
            ("mDNS", Vowel),
            ("npm", Vowel),
            ("apple", Vowel),
            ("uninstall", Vowel),
            ("unusual", Vowel),
            ("x-ray", Vowel),
            ("house", Consonant),
            ("unary", Consonant),
        ] {
            assert_eq!(initial_sound(w, true), want, "{w}");
        }
        assert_eq!(initial_sound("herb", true), Vowel);
        assert_eq!(initial_sound("herb", false), Consonant);
    }

    #[test]
    fn tokens() {
        let chars: Vec<char> = "Don't see e.g. foo.rs at https://x.y/z or 2nd."
            .chars()
            .collect();
        let words: Vec<String> = tokenize(&chars)
            .iter()
            .filter(|t| matches!(t.kind, Kind::Word | Kind::Number { .. }))
            .map(|t| t.text(&chars))
            .collect();
        assert_eq!(words, ["Don't", "see", "e.g", "foo.rs", "at", "or", "2nd"]);
    }

    /// Words flagged by `rule` in `text`, with the first suggestion.
    fn flagged(rule: &str, text: &str) -> Vec<String> {
        let chars: Vec<char> = text.chars().collect();
        let t = tokenize(&chars);
        lint(&chars, &t, &[rule], true)
            .get(rule)
            .into_iter()
            .flatten()
            .map(|l| {
                let w: String = chars[l.span.start..l.span.end].iter().collect();
                match l.suggestions.first() {
                    Some(Suggestion::ReplaceWith(r)) => {
                        format!("{w}>{}", r.iter().collect::<String>())
                    }
                    _ => w,
                }
            })
            .collect()
    }

    /// [`check`], reporting every failure at once.
    fn check_all(rule: &str, bad: &[(&str, &str)], good: &[&str]) {
        let mut fails = Vec::new();
        for (text, want) in bad {
            let got = flagged(rule, text);
            if got != [*want] {
                fails.push(format!("missed: {text} -> {got:?}"));
            }
        }
        for text in good {
            let got = flagged(rule, text);
            if !got.is_empty() {
                fails.push(format!("false positive: {text} -> {got:?}"));
            }
        }
        assert!(fails.is_empty(), "{rule}:\n{}", fails.join("\n"));
    }

    fn check(rule: &str, bad: &[(&str, &str)], good: &[&str]) {
        for (text, want) in bad {
            assert_eq!(flagged(rule, text), [*want], "{rule} should flag: {text}");
        }
        for text in good {
            assert!(
                flagged(rule, text).is_empty(),
                "{rule} false positive: {text} -> {:?}",
                flagged(rule, text)
            );
        }
    }

    #[test]
    fn predicates_after_possessives() {
        check(
            "YourYoure",
            &[
                ("I think your wrong.", "your>you're"),
                ("If your correct, we ship.", "your>you're"),
                ("Your absolutely right about that.", "Your>You're"),
                ("If your interested in helping, ask.", "your>you're"),
                ("Your very close.", "Your>You're"),
                ("I know your so busy, but ask.", "your>you're"),
                ("When your located in the EU, read this.", "your>you're"),
                ("Your just like me.", "Your>You're"),
                ("Thanks, your awesome.", "your>you're"),
            ],
            &[
                "Do your best.",
                "Your logged in user is shown.",
                "Your potential for growth is huge.",
                "Share your personal, private notes.",
                "Your very own copy.",
                "Your so called fix broke it.",
                "Your pretty printer is nice.",
                "Your default for this is fine.",
                "Pick your favorite.",
            ],
        );
        check(
            "ItsContraction",
            &[
                ("Its ready.", "Its>It's"),
                ("I think its just me.", "its>it's"),
                ("Its automatic.", "Its>It's"),
                ("Once its finished, restart.", "its>it's"),
                ("If its finished by noon, merge.", "its>it's"),
                ("Its based on the old code.", "Its>It's"),
                ("Its compatible with Python 3.", "Its>It's"),
                ("Its too late to change.", "Its>It's"),
                ("Its just fine.", "Its>It's"),
                ("Its only because of caching.", "Its>It's"),
                ("Wait before its used for login.", "its>it's"),
                ("It runs once; its meant for housekeeping.", "its>it's"),
                ("It will succeed if its undefined", "its>it's"),
                ("Then if its smaller than a byte, pad it.", "its>it's"),
                ("Hmm, its broken, so revert.", "its>it's"),
            ],
            &[
                "The crate and its potential.",
                "The tool and its built in support.",
                "Each node keeps its own copy.",
                "Its very existence is a bug.",
                "Its main job is caching.",
                "At its core, it is simple.",
                "The widget and its automatic, configurable layout.",
                "Its default is fine.",
                "The cache and its current size.",
            ],
        );
        check(
            "TheirToTheyre",
            &[
                ("I think their wrong.", "their>they're"),
                ("Their based on the old API.", "Their>They're"),
                ("Once their finished, restart.", "their>they're"),
            ],
            &[
                "They did their best.",
                "Their potential to grow is large.",
                "Their logged in users see it.",
            ],
        );
    }

    #[test]
    fn then_after_comma() {
        check(
            "ThenThan",
            &[
                ("If it fails, than we retry.", "than>then"),
                ("Build it first, than run the tests.", "than>then"),
                ("Once it starts, than it's fine.", "than>then"),
            ],
            &[
                "It is faster, than we thought.",
                "Rather this, than we lose data.",
                "It costs more to run, than it saves.",
                "We prefer caching, than we do polling.",
                "Other options, than the default.",
            ],
        );
    }

    #[test]
    fn there_possessive_noun_phrases() {
        check(
            "ThereToTheir",
            &[
                ("They moved to there new office.", "there>their"),
                ("Users keep there current setup.", "there>their"),
                ("It depends on there team's goals.", "there>their"),
                ("The choice is there's.", "there's>theirs"),
                ("It is ours and there's.", "there's>theirs"),
            ],
            &[
                "Is there new data?",
                "Out there new ideas emerge.",
                "There's a new version.",
                "There's no current fix.",
                "Is there anyone's key left?",
                "We went there last year.",
                "There is no new data.",
                "Over there John's car is parked.",
                "If there's are no default constructors, use this.",
                "There's are three reasons we could have woken up.",
            ],
        );
    }

    #[test]
    fn effect_affect_in_context() {
        check(
            "AffectEffect",
            &[
                ("This will effect performance.", "effect>affect"),
                ("The change effects the result.", "effects>affects"),
                ("These settings effect the output.", "effect>affect"),
                ("They effect everyone.", "effect>affect"),
                ("It has a big affect on latency.", "affect>effect"),
                ("It had an affect.", "affect>effect"),
                ("Watch its affects on memory.", "affects>effects"),
                ("There were unintended affects.", "affects>effects"),
            ],
            &[
                "We effect a change.",
                "They effect the transfer tomorrow.",
                "Side effects are rare.",
                "The sound effects the game uses are loud.",
                "The change effects the team had were good.",
                "It has a side effect.",
                "Effect a change in policy.",
                "This setting affects performance.",
                "Large values affect latency.",
                "True affects all rows.",
                "The patient showed flat affect.",
            ],
        );
    }

    #[test]
    fn weather_whether_clauses() {
        check(
            "WeatherWhether",
            &[
                ("Weather the build passes or not, ship.", "Weather>Whether"),
                ("Weather it's ready is unclear.", "Weather>Whether"),
                ("We run it, weather you like it or not.", "weather>whether"),
                ("Decide weather the cache is warm.", "weather>whether"),
                (
                    "Weather we use Go or Rust matters little.",
                    "Weather>Whether",
                ),
            ],
            &[
                "Weather the storm and keep going.",
                "Weather data is updated hourly.",
                "The weather API is down.",
                "Weather or snow, we ship.",
                "Whether or not it matters, we log it.",
                "Check the weather or the news.",
                "Stocks and weather the market can't predict.",
                "The weather service reports rain or snow.",
            ],
        );
    }

    #[test]
    fn except_principal_write_in_context() {
        check(
            "ExceptAccept",
            &[
                ("The function excepts a string.", "excepts>accepts"),
                ("It is widely excepted.", "excepted>accepted"),
                (
                    "The patch was excepted by the maintainers.",
                    "excepted>accepted",
                ),
                ("We are excepting pull requests.", "excepting>accepting"),
                ("It does not except null values.", "except>accept"),
                ("Everything accept the key is copied.", "accept>except"),
            ],
            &[
                "Everything except the key is copied.",
                "All of you except the new hires.",
                "It is excepted from the rule.",
                "Excepting the first line, all is fine.",
                "Accept the terms first.",
                "Everyone can accept the invite.",
                "We accept for review any patch.",
                "Present company excepted.",
                "Let anyone accept the terms.",
            ],
        );
        check(
            "PrincipalPrinciple",
            &[
                ("The principle idea is simple.", "principle>principal"),
                ("Our principle problem is latency.", "principle>principal"),
                ("It is a matter of principal.", "principal>principle"),
                (
                    "The principal behind the design is simple.",
                    "principal>principle",
                ),
                ("Reason from first principals.", "principals>principles"),
            ],
            &[
                "The principle applies here.",
                "The principle of least privilege.",
                "A principal component analysis.",
                "The principal investigator signed.",
                "The following principals have access.",
                "The principle violations are logged.",
                "Repay the principal of the loan.",
                "The principle that guides us.",
            ],
        );
        check(
            "WriteRight",
            &[
                ("Do it write now.", "write>right"),
                ("The file is write here.", "write>right"),
                ("Write click the icon.", "Write>Right"),
                ("It's all write.", "write>right"),
                ("It works, write?", "write>right"),
                ("That's not write.", "write>right"),
                ("Is that write?", "write>right"),
                ("We are on the write track.", "write>right"),
            ],
            &[
                "Let them write here.",
                "Files that write now are slow.",
                "We all write.",
                "Write now, edit later.",
                "Write access is required.",
                "Right away, write the test.",
                "The write path is slow.",
                "Is it write-only?",
                "Write here your name.",
            ],
        );
    }

    #[test]
    fn your_youre() {
        check(
            "YourYoure",
            &[
                ("Always test you're own code.", "you're>your"),
                ("Security is you're responsibility.", "you're>your"),
                ("Don't commit you're secret key.", "you're>your"),
                ("If you're servers are down, page us.", "you're>your"),
                ("Update you're config first.", "you're>your"),
                ("Read it before you're first shift.", "you're>your"),
                ("You're code was wrong before.", "You're>Your"),
                ("I think your not ready.", "your>you're"),
                ("If your going to deploy, tag it.", "your>you're"),
                ("Thanks! And your welcome.", "your>you're"),
                ("When your sure, merge it.", "your>you're"),
                ("I think your probably using it.", "your>you're"),
                ("Honestly, your the best.", "your>you're"),
            ],
            &[
                "You're going to love it.",
                "If you're sure, merge it.",
                "You're welcome to contribute.",
                "You're first in line.",
                "You're first to arrive.",
                "You're first, then me.",
                "You're new here, so read this.",
                "If you're users of the old API, migrate.",
                "You're home and you're done.",
                "Your code, your config and your own keys.",
                "Your welcome message is shown on login.",
                "It is not your doing.",
                "Your not knowing is fine.",
                "Point your A record at the balancer.",
                "Your not-null constraint failed.",
                "You're most welcome.",
                "What you're after is the key.",
                "Refresh your already generated token.",
                "Replace your probably outdated copy.",
                "When you're first entering the module, read this.",
            ],
        );
    }

    #[test]
    fn their_there() {
        check(
            "ThereOwn",
            &[
                ("They wrote there own parser.", "there>their"),
                ("On they're own terms.", "they're>their"),
            ],
            &["There is no own goal here.", "They have their own tools."],
        );
        check(
            "ThereToTheir",
            &[
                ("Teams compare there behavior with ours.", "there>their"),
                ("Users keep there existing passwords.", "there>their"),
                (
                    "Because there upstreams answer fast, it hides.",
                    "there>their",
                ),
                ("Tests break and there value is low.", "there>their"),
                ("Keep them as there owners wrote them.", "there>their"),
            ],
            &[
                "Is there documentation for this?",
                "Are there users who rely on it?",
                "There are users who rely on it.",
                "Put the config there and restart.",
                "Is there existing support?",
                "The people there value quiet.",
                "We stayed there for a week.",
            ],
        );
        check(
            "TheirToThere",
            &[
                ("Their is no undo.", "Their>There"),
                ("I know their are three.", "their>there"),
                ("The file must stay their.", "their>there"),
            ],
            &[
                "Their team is small.",
                "They keep their promises.",
                "Pick his, her or their.",
                "Use they/them/their.",
                "Their is-a relationship holds.",
            ],
        );
        check(
            "TheirToTheyre",
            &[
                ("I bet their probably using it.", "their>they're"),
                ("I think their not ready.", "their>they're"),
                ("Their going to be late.", "Their>They're"),
            ],
            &[
                "Their not knowing hurt.",
                "Their going-away party was fun.",
                "Their almost-complete draft is here.",
                "Use their already generated token.",
                "Their almost always correct output helps.",
                "They lost their going rate.",
                "Their new API is fast.",
            ],
        );
        check(
            "TheyreToTheir",
            &[(
                "Customers must update they're configuration.",
                "they're>their",
            )],
            &[
                "They're users of the old API.",
                "They're tokens, not strings.",
                "They're done.",
                "They're data, so treat them carefully.",
            ],
        );
    }

    #[test]
    fn lose_loose() {
        check(
            "LoseLoose",
            &[
                ("You will not loose any data.", "loose>lose"),
                ("Don't loose it.", "loose>lose"),
                ("If you loose the key, rotate it.", "loose>lose"),
                ("It is easy to loose track of time.", "loose>lose"),
                ("We are loosing data.", "loosing>losing"),
                ("It looses the connection.", "looses>loses"),
                ("Callers never loose data.", "loose>lose"),
            ],
            &[
                "The screw is loose.",
                "The screw is not loose.",
                "The screw is never loose.",
                "Let the dog loose.",
                "The bolts came loose.",
                "We moved to loose coupling.",
                "It leads to loose ends.",
                "They cut it loose and moved on.",
                "Keep a loose grip.",
                "Use loose-fitting clothes.",
            ],
        );
    }

    #[test]
    fn affect_effect() {
        check(
            "AffectEffect",
            &[
                ("This doesn't effect performance.", "effect>affect"),
                ("It will not effect your users.", "effect>affect"),
                (
                    "Only crates effected by a change rebuild.",
                    "effected>affected",
                ),
                ("The option has no affect here.", "affect>effect"),
                ("Watch for side affects.", "affects>effects"),
                ("The change takes affect on restart.", "affect>effect"),
                ("It also effects the cache.", "effects>affects"),
                ("This effects only cold keys.", "effects>affects"),
            ],
            &[
                "This does not affect performance.",
                "The change had no effect.",
                "Side effects are rare.",
                "We want to effect change.",
                "The new law will effect a change in policy.",
                "The effects of caching are large.",
                "It takes effect on restart.",
                "Heat can affect the results.",
                "The only effects the change had were good.",
                "Effects like this are rare.",
            ],
        );
    }

    #[test]
    fn whose_whos() {
        check(
            "WhoseWhos",
            &[
                ("Reject requests who's timestamp is old.", "who's>whose"),
                ("The first route who's matcher accepts it.", "who's>whose"),
                ("The user who's code broke it.", "who's>whose"),
                ("Whose going to fix it?", "Whose>Who's"),
                ("Ask whose been on call.", "whose>who's"),
            ],
            &[
                "Who's on call today?",
                "Who's there?",
                "Who's responsible is unclear.",
                "Who's going to fix it?",
                "The user whose code broke it.",
                "Whose turn is it?",
                "Who's been here longest?",
                "Whoever who's free joins.",
                "Who's free accepts the ticket.",
                "Tell me who's next.",
            ],
        );
    }

    #[test]
    fn weather_whether() {
        check(
            "WeatherWhether",
            &[
                ("We never leak weather an id is valid.", "weather>whether"),
                ("Check weather it works.", "weather>whether"),
                ("It runs weather or not you like it.", "weather>whether"),
                ("Decide weather to retry.", "weather>whether"),
            ],
            &[
                "The weather is nice today.",
                "Check weather forecasts daily.",
                "We will weather the storm.",
                "Stocks and weather the market can't predict.",
                "Talk about weather in Helsinki.",
            ],
        );
    }

    #[test]
    fn principal_principle_and_complement() {
        check(
            "PrincipalPrinciple",
            &[
                ("The principle risk is the cutover.", "principle>principal"),
                ("Ask a principle engineer.", "principle>principal"),
                (
                    "Follow the principal of least privilege.",
                    "principal>principle",
                ),
                ("It works in principal.", "principal>principle"),
            ],
            &[
                "The principle of least privilege applies.",
                "A guiding principle behind the design.",
                "The design principle reasons are clear.",
                "The principal engineer approved it.",
                "Repay the principal first.",
                "Security principals of the tenant.",
                "Paid in principal and interest.",
            ],
        );
        check(
            "ComplimentComplement",
            &[
                (
                    "These tools compliment each other.",
                    "compliment>complement",
                ),
                (
                    "The colors complimented one another.",
                    "complimented>complemented",
                ),
            ],
            &[
                "Thanks for the compliment.",
                "She complimented the team.",
                "They complement each other.",
            ],
        );
    }

    #[test]
    fn then_than_both_ways() {
        check(
            "ThenThan",
            &[
                ("Wait and than retry.", "than>then"),
                ("It is more common then today.", "then>than"),
                ("Anything other then that fails.", "then>than"),
                ("Wait until than.", "than>then"),
                ("If it takes longer then `timeout`, give up.", "then>than"),
                ("It uses more memory then the old one.", "then>than"),
                ("It needs fewer lines of code then before.", "then>than"),
                ("Use less memory then usual.", "then>than"),
                ("A u8 field has a larger value then supported.", "then>than"),
                ("It's worse then the old one.", "then>than"),
                ("Better safe then sorry.", "then>than"),
                (
                    "If it has lower specificity then selectorB, abort.",
                    "then>than",
                ),
            ],
            &[
                "Back off then retry.",
                "Write more tests then the CI will pass.",
                "It is no worse in practice, then the old path is removed.",
                "Load more data then call flush.",
                "Create a larger batch then slice it.",
                "It matches one or more letters then whitespace.",
                "Check the pool first so we use fewer connections then idle pool.",
                "If the limit is a prefix of the lower limit then the earlier test lowerLimit has caught it.",
                "Wait and then retry.",
                "It is more common than before.",
                "Add more tests then run them.",
                "Back then we used SVN.",
                "It got better then.",
                "Make it faster, then smaller.",
            ],
        );
    }

    #[test]
    fn possessive_slots() {
        check(
            "YourYoure",
            &[
                ("Rotate you're existing credentials.", "you're>your"),
                ("Open you're pull request against main.", "you're>your"),
                ("Once you're pull request is merged, deploy.", "you're>your"),
                ("Check you're current branch.", "you're>your"),
            ],
            &[
                "You're 5 minutes late.",
                "You're new users, so read this.",
                "You're root now.",
                "You're part of the team.",
                "If you're done, merge it.",
                "You're using the new API.",
                "You're right about that.",
                "You're offline.",
            ],
        );
        check(
            "TheyreToTheir",
            &[("Update they're dependency versions.", "they're>their")],
            &[
                "They're great tools.",
                "They're good people.",
                "They're private keys.",
                "They're build tools for Rust.",
                "They're kind of slow.",
            ],
        );
        check(
            "WhoseWhos",
            &[
                ("Who's pull request is this?", "Who's>Whose"),
                (
                    "The catalog service, who's product pages get hammered.",
                    "who's>whose",
                ),
            ],
            &["Who's there?", "Who's next in line?"],
        );
        check(
            "ThereToTheir",
            &[
                ("Load there importers first.", "there>their"),
                ("We call there importers first.", "there>their"),
            ],
            &[
                "We moved there years ago.",
                "We moved there last week.",
                "Once we compile there will be a file.",
                "Paths with directory names there must be clean.",
                "In a few cases there may still be gaps.",
                "Put there the files you need.",
            ],
        );
    }

    #[test]
    fn repeated_words_context() {
        check(
            "RepeatedWords",
            &[
                ("The library is is synchronous.", "is is>is"),
                ("Log lines now now include it.", "now now>now"),
                ("We want no no surprises.", "no no>no"),
            ],
            &[
                "What it is is a cache.",
                "No no, that is fine.",
                "Now now, calm down.",
                "I had had enough.",
            ],
        );
    }

    #[test]
    fn a_an_skips_acronyms_and_identifiers() {
        check(
            "AnA",
            &[("Use an new API.", "an>a"), ("Use a apple.", "a>an")],
            &[
                "Use a SQLite library.",
                "Use an SQL query or a SQL query.",
                "Plan a FFT first.",
                "Take a u16 value.",
                "Parse a RST_STREAM frame.",
                "Open a foo.rs file.",
                "Use an HTML page.",
            ],
        );
    }

    #[test]
    fn pronoun_verb_agreement() {
        check(
            "PronounVerbAgreement",
            &[
                ("If you presses the button, it stops.", "presses>press"),
                ("The flag lets you sets the level.", "sets>set"),
                ("They runs the tests nightly.", "runs>run"),
                ("When they uses the cache, it is fast.", "uses>use"),
                ("We has two options.", "has>have"),
                ("You doesn't need it.", "doesn't>don't"),
                ("They is ready.", "is>are"),
                ("He run the script daily.", "run>runs"),
                ("She go to the office.", "go>goes"),
                ("It work on Linux.", "work>works"),
                ("It don't matter.", "don't>doesn't"),
                ("She have a copy.", "have>has"),
                ("He study hard.", "study>studies"),
                ("When it fail, retry.", "fail>fails"),
                ("If it return an error, log it.", "return>returns"),
                (
                    "The script checks the file, it return early.",
                    "return>returns",
                ),
                ("He fix the bug.", "fix>fixes"),
                ("She push the branch.", "push>pushes"),
                ("You passes the value.", "passes>pass"),
            ],
            &[
                "Does it work on Linux?",
                "Can you set the level?",
                "Let it run overnight.",
                "Make it work, then make it fast.",
                "This will help you get started.",
                "You and I run the tests.",
                "He and she run the shop.",
                "Did he run the script?",
                "We want him to run it.",
                "I suggested she go home.",
                "It is required that he attend.",
                "They insisted he stay.",
                "Watch it fail.",
                "I heard it break down.",
                "It works on Linux.",
                "He runs, she swims, they finish.",
                "The server gives you results in seconds.",
                "It will show you errors as they occur.",
                "We send you updates weekly.",
                "Thank you gifts are welcome.",
                "Part I covers the basics.",
                "World War I ends the chapter.",
                "It need not be fast.",
                "He dare not say it.",
                "Let it be.",
                "He set the flag yesterday.",
                "It cost us a day.",
                "She put it there.",
                "They read the docs.",
                "He/she runs the job.",
                "Are they colors or colours?",
                "What is he like?",
                "Leave it open.",
                "If it clear, continue.",
                "He fought hard.",
                "The pride in you is real.",
                "Either you or he runs it.",
                "It always works.",
                "It only runs once.",
                "It even works offline.",
                "They Run The Show",
                "The co-founder and he-man.",
                "It's fine; you're right.",
                "The words \"they runs\" are wrong.",
                "The two characters before it represent one byte.",
                "It sort of works.",
                "It further reduces load.",
                "These are all they types of groups.",
                "Thanks to all contributors, you guys rock!",
                "What the docs don't tell you is that it leaks.",
                "The top of he base.",
                "If the result is not failure, return he result.",
                "Valid since \"[t]he value false has the bit pattern\".",
                "In June she married Tom.",
                "It heap allocates a struct.",
                "It fork/execs the binary.",
                "It main use case is parsing.",
                "If I is zero, stop.",
                "Tell the compiler you closures must be boxed.",
                "The mime type is guessed fromt he file extension.",
                "Before he enter on the office, he shall take the oath.",
            ],
        );
    }

    #[test]
    fn subject_verb_agreement() {
        check_all(
            "SubjectVerbAgreement",
            &[
                ("If the server run out of memory, it restarts.", "run>runs"),
                ("Each file contain a header.", "contain>contains"),
                ("The files contains the schema.", "contains>contain"),
                ("Two tests fails the build.", "fails>fail"),
                ("Each of the files are checked.", "are>is"),
                ("One of the tests fail on CI.", "fail>fails"),
                ("The number of retries are limited.", "are>is"),
                ("The results is empty.", "is>are"),
                ("The config are loaded at startup.", "are>is"),
                ("These changes was merged yesterday.", "was>were"),
                ("The results was empty.", "was>were"),
                ("The config were wrong.", "were>was"),
                ("The tests was failing.", "was>were"),
                ("The job were retrying.", "were>was"),
                ("The server have crashed.", "have>has"),
                ("The files has been deleted.", "has>have"),
                ("The job were being retried.", "were>was"),
                ("The API key rotate every day.", "rotate>rotates"),
                (
                    "When the user sessions expires, log in again.",
                    "expires>expire",
                ),
                ("The file don't exist.", "don't>doesn't"),
                ("The tests doesn't cover it.", "doesn't>don't"),
                ("The server also run the migrations.", "run>runs"),
                ("Because the build fail the job, we retry.", "fail>fails"),
                ("The user's files contains secrets.", "contains>contain"),
                ("Each of them are optional.", "are>is"),
                ("The APIs returns JSON.", "returns>return"),
                ("The endpoint return a list of users.", "return>returns"),
                ("The response include a token.", "include>includes"),
                (
                    "If your applications needs to parse a string, use it.",
                    "needs>need",
                ),
                ("Each of the possible types are named.", "are>is"),
                (
                    "At least one of the required lines were missing.",
                    "were>was",
                ),
                (
                    "It is reset even if one of the drops panic.",
                    "panic>panics",
                ),
                ("When the number of RPCs go down to 0, stop.", "go>goes"),
                (
                    "If the new config changes reduces the cache, we flush.",
                    "reduces>reduce",
                ),
                (
                    "The service operate differently for each mode.",
                    "operate>operates",
                ),
                ("One example are graph algorithms.", "are>is"),
                ("If the logs shows the error, retry.", "shows>show"),
                ("Each of these are considered.", "are>is"),
                (
                    "It fails if the number of input bytes are less than 32.",
                    "are>is",
                ),
                (
                    "It waits until at least one of the operations have completed.",
                    "have>has",
                ),
                (
                    "Futures allocated when the arena are full move to the heap.",
                    "are>is",
                ),
                (
                    "Actually, this method perform the following code.",
                    "perform>performs",
                ),
                ("Email addresses is unique per workspace.", "is>are"),
                ("Error messages is empty when it passes.", "is>are"),
                ("Every time the server add a node, it logs.", "add>adds"),
                (
                    "Jobs are retried; one bad job do not stop the rest.",
                    "do>does",
                ),
                (
                    "The `{ retries: 0 }` option. The retry loop give up after five attempts.",
                    "give>gives",
                ),
                (
                    "Some internal implementations enables code reuse.",
                    "enables>enable",
                ),
                (
                    "All the examples assumes the directory exists.",
                    "assumes>assume",
                ),
                ("If all variants has no field, omit it.", "has>have"),
                ("In this version, the number of cells are fixed.", "are>is"),
                ("Each of the operations were measured.", "were>was"),
            ],
            &[
                "The tests pass.",
                "Each test passes.",
                "The data is stored in S3.",
                "The data are stored in S3.",
                "The user can run the script.",
                "The command to run is shown below.",
                "Make the server run faster.",
                "The files you changed are listed.",
                "The number of retries is limited.",
                "The list of files is long.",
                "News is slow today.",
                "The series of steps is short.",
                "If the server were down, we would know.",
                "It behaves as if the cache were empty.",
                "I wish the tests were faster.",
                "The team were happy.",
                "The team was happy.",
                "The staff are here.",
                "The test runs fail often.",
                "The config change broke the build.",
                "The config change is small.",
                "The server logs requests to stdout.",
                "The server process handles signals.",
                "The user request body is JSON.",
                "A list of files are attached.",
                "A number of tests are flaky.",
                "The server and the client run on the same host.",
                "The server and client run on the same host.",
                "The server or the client handle it.",
                "Does the server run on Linux?",
                "Let the tests run.",
                "It is important that the server restart cleanly.",
                "We recommend that the user have admin rights.",
                "The service that runs manages the queue.",
                "After the file change the tests pass.",
                "Before the test run, clean up.",
                "The status is green.",
                "The process exits.",
                "The class has two methods.",
                "The analysis shows a leak.",
                "Every time you run it, it works.",
                "Each time the server restarts, it logs.",
                "The first run failed.",
                "The following are supported.",
                "The rest are optional.",
                "The police are here.",
                "The physics is hard.",
                "All runs fine.",
                "If all goes well, ship it.",
                "The tests results show a pass.",
                "The United States is large.",
                "The Server Run",
                "The files `contains` it.",
                "The server run() method starts it.",
                "Thanks to the users who run it.",
                "The fish swim upstream.",
                "The people who use it like it.",
                "The file list shows every file.",
                "Every day the job run at noon.",
                "The more tests run, the better.",
                "The docs say the server run as root.",
                "The criteria is simple.",
                "The build set the flag.",
                "The file read the input.",
                "The user need not log in.",
                "Is the file open?",
                "Where does the server run?",
                // Found in public docs: numbers, measures, prepositions before the subject.
                "The two are equivalent.",
                "The first two are multiplied together.",
                "The latter two are tightly coupled.",
                "The three were all crowded together.",
                "However, the two have diverged in many ways.",
                "If the two do not match, the program will not link.",
                "The last three correspond to wire-format limits.",
                "Four spaces is too many.",
                "Four spaces gives us a code block.",
                "Three inches is such a wretched height.",
                "After parsing, the arguments after the flag are available.",
                "Items added since the last collection are evicted.",
                "The spaces after the list marker determine the indentation.",
                "Nodes before and after the change interoperate.",
                // Noun compounds read as subject and verb.
                "The maximum allow Unix timestamp.",
                "The resulting new commit SHA and push status.",
                "The benchmark CI job no longer causes failures.",
                "The default policy no longer looks for loops.",
                "The default port the OTLP exporter uses is updated.",
                "The current shell process ID.",
                "The task queue Arc is already shared.",
                "The sleepy thread S reads the queue.",
                "A simple builtin split DWARF loader.",
                "The parent type this struct extends, if any.",
                "The register element this selector governs.",
                "The usual enqueue and dequeue operations are provided.",
                "The general direct use of it is rare.",
                "However, the wire format API definition is here.",
                "A big shout out to our contributors!",
                "A big thank you to everyone.",
                "Last but not least, a special thank you to Tony.",
                "Many performance optimisations thanks to contributions.",
                "A full stack Web framework.",
                "A pure rust MQTT client.",
                "A new terminal UI dashboard.",
                "The HTTP transport APIs are asymmetrical.",
                "Here the outer list is loose, the inner list tight.",
                "It fired when the SSL verify, PSK client, or PSK server callback fired.",
                "This will even zero out the padding.",
                "The client MUST NOT retry the request.",
                "This change will help simplify dependency trees.",
                "This can help avoid symbol conflicts.",
                "A maintainer will likely do these cherry picks.",
                "The examples thus far have been using it.",
                "Gatsby's foot beat a short, restless tattoo.",
                // Relative clauses, mass nouns, collectives.
                "The syntax SQLite supports is a superset.",
                "This tool helps if your tokens definition stays constant.",
                "The encoded values use are little-endian.",
                "By default, all the optional middleware are disabled.",
                "The middleware were originally extracted from a project.",
                "All their public API have been redesigned.",
                "Then all the party were placed along the course.",
                "Jordan's party were calling impatiently.",
                // Bare plurals that are fine.
                "Sending emails is slow.",
                "More tests is better.",
                "GitHub Actions is a CI service.",
                "Extra spaces is a common typo.",
                "It works if happy eyeballs is enabled.",
                "Fake session tickets is a very nifty trick.",
                "Go protocol buffers is an open source project.",
                "Normally, all but one of the trailing newline characters are removed.",
                "Make sure the storage vs. the debt are well ordered.",
                "The candidates arguments for substitution are the keys.",
                "Reaffirm the bounds checks to avoid panics.",
                "The quartiles values for the X axis.",
                "Loop over the bands elements in the ith row.",
                "It is found if an attribute correspond with given name is found.",
                "The channel notify here is guaranteed to be safe.",
                "The callback function my halt the walk.",
                "If the closure panics, another build's item hid from the first lookup.",
                "Then the container init process is used.",
                "Once the number of references reaches zero, the entry is evicted.",
                "It returns true if at least one of the matchers returns true.",
                "To scale the model to that many replicas is guaranteed.",
                "Creating a map with that many entries also panics.",
                "Must validate that its contents is actually UTF-8.",
                "The smart Hir::repetition constructors does some basic work.",
                "It ensures that only one of these access specifiers can be applied.",
                "If omitted, the stale acquire work gets dropped.",
                "In this case, all but one receiver are able to receive values.",
                "For small arrays, where all but the last field are small.",
                "In a rare case where a graph algorithm were not applicable, stop.",
                "The key take away here is simple.",
                "If N is positive, the previous transition the one at idx.",
                "The main type you want to work with is TextDiff.",
            ],
        );
    }
    #[test]
    fn articles_before_numbers() {
        use Sound::*;
        for (d, want) in [
            ("8", Vowel),
            ("80", Vowel),
            ("800", Vowel),
            ("8000", Vowel),
            ("11", Vowel),
            ("18", Vowel),
            ("11000", Vowel),
            ("1", Consonant),
            ("100", Consonant),
            ("110", Consonant),
            ("429", Consonant),
            ("30", Consonant),
            ("12", Consonant),
            ("1024", Consonant),
            ("100000", Consonant),
            ("1100", Either),
            ("1800", Either),
            ("08", Either),
        ] {
            assert_eq!(number_sound(d), want, "{d}");
        }
        let fixes = |text: &str, orig: &str| -> Vec<String> {
            ana_numbers(text, orig)
                .into_iter()
                .map(|(r, fix)| format!("{}>{fix}", &text[r]))
                .collect()
        };
        let same = |s: &str| fixes(s, s);
        assert_eq!(same("Use a 8-byte key."), ["a>an"]);
        assert_eq!(same("It took a 80% cut."), ["a>an"]);
        assert_eq!(same("an 100 ms delay"), ["an>a"]);
        assert_eq!(same("A 8-core CPU."), ["A>An"]);
        assert_eq!(
            same("It is a 11 step plan and a 18 hour day."),
            ["a>an", "a>an"]
        );
        assert_eq!(same("Return an 429 or an 4xx code."), ["an>a", "an>a"]);
        assert_eq!(same("Wait an 30s timeout."), ["an>a"]);
        assert_eq!(same("Use a 8,000 row batch."), ["a>an"]);
        assert_eq!(
            same("Parse an 6LoWPAN header, a 8b word, an 4K page."),
            ["an>a", "a>an", "an>a"]
        );
        // Blanked noise tokens are read from the source.
        assert_eq!(fixes("Buy a      disk.", "Buy a 8GB  disk."), ["a>an"]);
        assert_eq!(fixes("Use a      pass.", "Use a 8th  pass."), ["a>an"]);
        for ok in [
            "Use an 8-byte key.",
            "It took an 80% cut.",
            "a 100 ms delay",
            "a 1 in 5 chance",
            "an 11 step plan",
            "a 429 response",
            "a 30s timeout",
            "an 1800s house or a 1800s house",
            "a 1/2 inch gap",
            "Row A 8 is empty.",
            "x.a 8",
            "a 3_000 limit",
            "the 8 cores",
            "a 89ab, b 89ab",
            "a 8f3c commit",
            "(8kb, an 16kb and 32kb)",
            "between 0 an 1",
        ] {
            assert!(same(ok).is_empty(), "{ok}: {:?}", same(ok));
        }
        // Code: the source has a backtick where the text has a space.
        assert!(fixes("Use a     value.", "Use a `8` value.").is_empty());
        // A comment marker between lines.
        assert!(fixes("Use a\n   8-byte key.", "Use a\n// 8-byte key.").is_empty());
    }

    #[test]
    fn number_suffixes() {
        let chars: Vec<char> = "the 2th and 3RD and 11th".chars().collect();
        let t = tokenize(&chars);
        let l = lint(&chars, &t, RULES, true);
        let at = |k: &str| -> Vec<String> {
            l.get(k)
                .into_iter()
                .flatten()
                .map(|l| chars[l.span.start..l.span.end].iter().collect())
                .collect()
        };
        assert_eq!(at("CorrectNumberSuffix"), ["th"]);
        assert_eq!(at("NumberSuffixCapitalization"), ["RD"]);
    }

    /// Correct English that none of the confusable rules may flag: prose, technical docs,
    /// code comments and idioms.
    const CORRECT: &[&str] = &[
        "There there, it will be fine.",
        "Yes, your honor.",
        "It's its own thing.",
        "Who's who in the project.",
        "We prefer loose coupling between crates.",
        "Tie up the loose ends before release.",
        "The weather station reports hourly.",
        "Ask the principal engineer.",
        "Follow the principle of least privilege.",
        "The two libraries are complementary.",
        "The new law will effect a change in policy.",
        "Your code, your rules.",
        "Your understanding of the problem helps.",
        "Your running shoes are by the door.",
        "Your building in the city is tall.",
        "Do your best to keep it short.",
        "It is your right to refuse.",
        "Turn to your right at the corner.",
        "Is this your kind of thing?",
        "Take your time.",
        "Your mileage may vary.",
        "Your welcome message is shown on login.",
        "You're right about that.",
        "You're welcome to open a PR.",
        "You're done.",
        "You're free to leave.",
        "You're toast if the build fails.",
        "If you're admin, you can delete it.",
        "When you're root, be careful.",
        "You're history.",
        "You're kidding.",
        "They're users of the old API.",
        "They're ready.",
        "The crate and its dependencies build fast.",
        "Each node keeps its offset and length.",
        "The type and its associated data.",
        "With its associated\ndata weight.",
        "If we see an opening parenthesis we must find its closing partner.",
        "Its value is cached.",
        "The cache lost its contents.",
        "At its best it is fast.",
        "The team did its best to fix it.",
        "It has its time to live set.",
        "The parser and its left and right children.",
        "It's a bug.",
        "It's been fixed.",
        "It's likely that we retry.",
        "It's time to release.",
        "I know it's reserved.",
        "It's encryption, not hashing.",
        "It's null terminated.",
        "It's worth noting.",
        "It's lunch time.",
        "Their code is clean.",
        "Their tests are slow.",
        "They lost their way.",
        "Their best is good enough.",
        "They did their best to fix it.",
        "There is a bug.",
        "There are two ways.",
        "Out there data is scarce.",
        "Here and there code is duplicated.",
        "Put it there.",
        "Is there documentation?",
        "We lived there years ago.",
        "Get out of there fast.",
        "North of there lies a town.",
        "The people there own nice cars.",
        "Companies there own the property.",
        "For there to be a fix, we need tests.",
        "There goes the build.",
        "There remains one issue.",
        "Whose code is this?",
        "The user whose account was locked.",
        "Who's there?",
        "Who's on call?",
        "Who's responsible is unclear.",
        "The witch is in the story.",
        "A good witch can help.",
        "The Blair Witch project scared us.",
        "Salem hanged a witch in 1692, and the witch is remembered.",
        "Everything except the key is copied.",
        "There is nothing to do except wait.",
        "All tests pass except the flaky one.",
        "We accept for review any patch.",
        "Accept the terms first.",
        "Write the code first.",
        "We all write code.",
        "The write path is slow.",
        "Turn write caching off.",
        "Write it down right now.",
        "Weather permitting, we ship Friday.",
        "Bad weather we can handle.",
        "The weather they forecast was wrong.",
        "We will weather the storm.",
        "Cold weather it seems is coming.",
        "The principal of the loan is due.",
        "First principal component analysis.",
        "Security principals of the tenant.",
        "She complimented the chef.",
        "A compliment to the team.",
        "Side effects are rare.",
        "The side effects of caching.",
        "Padding side affects the results.",
        "Changes in affect are a symptom.",
        "We want to effect change.",
        "Loose ends remain.",
        "The screw is loose.",
        "It came loose.",
        "Then we restart.",
        "More than we hoped.",
        "Back then we used SVN.",
        "Rather than that, restart.",
        "Wait until then.",
        "Even then it failed.",
        "Run a principal component analysis first.",
        "The principal investigator approved the grant.",
        "Fix it right away.",
        "You need write access to push.",
        "Whether or not it matters, we log it.",
        "Whether it works or not, we ship.",
        "The weather API returns JSON.",
        "The weather data or the map layer.",
        "The crate has its own allocator.",
        "Everything works except for the cache.",
        "Accept the terms to continue.",
        "They want to effect a change in policy.",
        "It has a side effect.",
        "They manage their own keys.",
        "The service keeps its state in memory.",
        "Their new office is nice.",
        "Its current size is small.",
        "The team and their potential.",
        "Update your current setup.",
        "Do your best, then rest.",
        "It is the principle that counts.",
        "Accepting all cookies is optional.",
        "We accept pull requests.",
        "Except for tests, nothing changed.",
        "Write the code, then test it.",
        "Right now, we write the docs.",
        "Is there new data or not?",
        "There's a bug in there.",
        "It's theirs, not ours.",
        "The effect was small.",
        "Effects on latency were minor.",
        "The change affects the result.",
        "Settings affect the output.",
        "Weather permitting, the event goes ahead.",
        "Check the weather before you leave.",
        "The principal amount is repaid monthly.",
        "Security principals can be users or groups.",
        "If it fails, then we retry.",
        "It is faster, than I expected, by far.",
        "Your idea is good.",
        "Your interested party list is empty.",
        "The widget and its automatic resizing.",
        "Its built in support is good.",
        "Once its buffer fills, it flushes.",
        "Its logged in user is shown.",
        // Found on public corpora.
        "Inherit your class from the base.",
        "Replay this recording in your terminal with asciinema.",
        "Embed time zones into your binary with a macro.",
        "Link your binary with the loader directly.",
        "Decrypt your encrypted      by setting the key.",
        "Pushing null ends only its readable half.",
        "If a flex item stretches, its computed cross size is auto.",
        "A dictionary has two pieces: Its header, and its content.",
        "Port Louis, its capital.",
        "Data per node, its color, which is red or black.",
        "And of course, its logic.",
        "Their gender.",
        "Permanently changing their offset from UTC.",
        "Assigned to an event when their cat, tid, and pid match.",
        "Implemented at their core with a few instructions.",
        "Formattable without giving up is more important, than that it be flawless.",
        "The change takes effect the next time a frame is decoded.",
        "Takes effect both immediately and later.",
        "Shut down the stream in the write direction.",
        "There's is always singular.",
        "Add the following to your executable to initialize it.",
        "Users know better about their terminal than the config allows.",
        "Prevent attributes from syncing with their related",
        "The customer is redirected back to your specified      .",
        "Required, including for your configured      .",
        "Give users time to do their",
        "The padding can then be effected by a simple combination.",
        "It is worth more than it's price suggests.",
        "It works better than it's ever been.",
        "It is more trouble than it's worth.",
        "It costs more than it's
worth.",
        "Keep attributes in sync with their related
DOM properties.",
    ];

    const CONFUSABLE_TESTS: &[&str] = &[
        "YourYoure",
        "ThereOwn",
        "ThereToTheir",
        "TheirToThere",
        "TheirToTheyre",
        "TheyreToTheir",
        "LoseLoose",
        "AffectEffect",
        "WhoseWhos",
        "WeatherWhether",
        "PrincipalPrinciple",
        "ComplimentComplement",
        "ItsContraction",
        "ItsPossessive",
        "WitchWhich",
        "ExceptAccept",
        "WriteRight",
        "ThenThan",
    ];

    #[test]
    fn confusables_leave_correct_english_alone() {
        for text in CORRECT {
            for rule in CONFUSABLE_TESTS {
                // `ThereOwn` is Harper's too; ours only checks `there own` + noun.
                if *rule == "ThereOwn" && text.contains("there own") {
                    continue;
                }
                let f = flagged(rule, text);
                assert!(f.is_empty(), "{rule} false positive: {text} -> {f:?}");
            }
        }
    }

    #[test]
    fn your_youre_by_word_class() {
        check(
            "YourYoure",
            &[
                ("I think your aware of it.", "your>you're"),
                ("If your happy with it, merge.", "your>you're"),
                ("Your done.", "Your>You're"),
                ("Let me know when your done with it.", "your>you're"),
                ("If your using the old API, upgrade.", "your>you're"),
                ("When your ready for review, ping me.", "your>you're"),
                ("Your right about that.", "Your>You're"),
                ("Great, your all set.", "your>you're"),
                ("I think your really sure.", "your>you're"),
                ("Put it in you're config.", "you're>your"),
                ("If you're car is parked there, move it.", "you're>your"),
                ("Check that you're account was created.", "you're>your"),
                ("You're tests are failing.", "You're>Your"),
                ("Thanks for you're help.", "you're>your"),
                ("So put you're most specific routes first.", "you're>your"),
                ("Update you're config.", "you're>your"),
            ],
            &[],
        );
    }

    #[test]
    fn their_there_by_word_class() {
        check(
            "ThereToTheir",
            &[
                ("Ask them about there code.", "there>their"),
                ("Because of there config, it fails.", "there>their"),
                ("There car is red.", "There>Their"),
                ("They left and there tests are broken.", "there>their"),
                ("It works with there parser.", "there>their"),
                ("Teams wanted autonomy over there tooling.", "there>their"),
                ("The third check catches there cousins.", "there>their"),
            ],
            &[],
        );
        check(
            "TheirToThere",
            &[
                ("Their will be a fix.", "Their>There"),
                ("I think their seems to be a bug.", "their>there"),
                ("Their exist many ways.", "Their>There"),
            ],
            &[
                "Their will is strong enough for them.",
                "Each keeps their own vector of fields for their",
            ],
        );
        check(
            "TheirToThere",
            &[],
            &[
                "Nested types need to create their own vector of fields for their",
                "Fields must be annotated in their .proto file.",
            ],
        );
        check(
            "TheirToTheyre",
            &[
                ("I think their aware of it.", "their>they're"),
                ("If their using the old API, it breaks.", "their>they're"),
                ("Their probably the best.", "Their>They're"),
                ("Raised after retries their exhausted.", "their>they're"),
            ],
            &[],
        );
        check(
            "TheyreToTheir",
            &[
                ("Check they're config is valid.", "they're>their"),
                ("It depends on they're setup.", "they're>their"),
            ],
            &[
                "They’re output in a correct order.",
                "They're set.",
                "They're home.",
                "Check the offsets and verify\nthey're in bounds.",
            ],
        );
    }

    #[test]
    fn its_both_ways() {
        check(
            "ItsContraction",
            &[
                ("Its a bug in the parser.", "Its>It's"),
                ("I think its been fixed.", "its>it's"),
                ("Maybe its not needed.", "its>it's"),
                ("Its important to test.", "Its>It's"),
                ("If its using the cache, clear it.", "its>it's"),
                ("Its going to fail.", "Its>It's"),
                ("So its time to release.", "its>it's"),
                ("Its always the same.", "Its>It's"),
                ("Its likely that we retry.", "Its>It's"),
                ("I guess its a broken pipe.", "its>it's"),
                ("If its still locked after that, retry.", "its>it's"),
                ("Ask. Its probably missing from the table.", "Its>It's"),
                ("We tried, and its how you get a bill.", "its>it's"),
            ],
            &[
                "The crate keeps its own copy.",
                "It lost its going rate.",
                "The file has its time to live.",
                "The model and its likely causes.",
                "Its not-null check fails.",
                "Its A record points here.",
            ],
        );
        check(
            "ItsPossessive",
            &[
                ("Each crate has it's own license.", "it's>its"),
                ("The size of it's buffer grows.", "it's>its"),
                ("It's value is cached.", "It's>Its"),
                ("The list and it's contents are freed.", "it's>its"),
                ("Due to it's design, it is fast.", "it's>its"),
                ("The tool verifies it's checksum.", "it's>its"),
                ("It will never let it's pods go.", "it's>its"),
                ("Fetch it again if you need it's current state.", "it's>its"),
                ("The job took longer than it's interval; retry.", "it's>its"),
                (
                    "Config is code and it's failure modes are worse.",
                    "it's>its",
                ),
            ],
            &[
                "It's likely we retry.",
                "It's position dependent.",
                "It's RSA.",
                "It's magic.",
                "It's code that runs.",
                "I think it's time to go.",
            ],
        );
    }

    #[test]
    fn whose_whos_by_word_class() {
        check(
            "WhoseWhos",
            &[
                ("A pointer who's value will be set.", "who's>whose"),
                ("The unit who's gender is needed.", "who's>whose"),
                ("Talk to who's owner?", "who's>whose"),
                ("Whose there?", "Whose>Who's"),
                ("Whose responsible for this?", "Whose>Who's"),
                ("I wonder whose coming today.", "whose>who's"),
            ],
            &[
                "Ciphers whose the block size is not 1.",
                "Who's admin here?",
                "Who's president now?",
                "The man whose responsible attitude helped.",
            ],
        );
    }

    #[test]
    fn witch_except_write() {
        check(
            "WitchWhich",
            &[
                ("Use the list, witch is sorted.", "witch>which"),
                ("A function witch returns a value.", "witch>which"),
                ("Witch one should I use?", "Witch>Which"),
                (
                    "It depends on the order in witch the tests run.",
                    "witch>which",
                ),
            ],
            &[
                "The witch hunts began.",
                "She dressed as a witch for Halloween.",
                "The wicked witch is gone.",
            ],
        );
        check(
            "ExceptAccept",
            &[
                ("We will except the terms.", "except>accept"),
                ("Please except my apology.", "except>accept"),
                ("We are happy to except contributions.", "except>accept"),
                ("Everything accept for the key is copied.", "accept>except"),
            ],
            &[
                "I didn't except it to work.",
                "Nothing to do except the laundry.",
                "It is identical to except for the name.",
                "We accept for publication only new work.",
            ],
        );
        check(
            "WriteRight",
            &[
                ("That's write.", "write>right"),
                ("That's exactly write!", "write>right"),
                ("It is the write way to do it.", "write>right"),
                ("Turn write at the corner.", "write>right"),
                ("You're write about that.", "write>right"),
            ],
            &[
                "That's write-only.",
                "That's write access.",
                "The write side of the pipe.",
            ],
        );
    }

    #[test]
    fn more_confusions() {
        check(
            "WeatherWhether",
            &[
                ("Weather it works or not, ship it.", "Weather>Whether"),
                ("We decide weather we retry.", "weather>whether"),
                ("It depends on weather it rains.", "weather>whether"),
            ],
            &[
                "Because of weather we canceled.",
                "The kind of weather we like.",
            ],
        );
        check(
            "PrincipalPrinciple",
            &[
                ("Follow these design principals.", "principals>principles"),
                ("A guiding principal of the project.", "principal>principle"),
            ],
            &[
                "The principal investigator signed it.",
                "The underlying principal must still have access.",
            ],
        );
        check(
            "ComplimentComplement",
            &[
                ("Store it in two's compliment.", "compliment>complement"),
                ("Pick complimentary colors.", "complimentary>complementary"),
            ],
            &["Coffee is complimentary for guests."],
        );
        check(
            "AffectEffect",
            &[
                ("The setting is still in affect.", "affect>effect"),
                ("It goes into affect tomorrow.", "affect>effect"),
                ("Cause and affect.", "affect>effect"),
                ("This is effecting performance.", "effecting>affecting"),
            ],
            &["Padding can be effected by a simple combination."],
        );
        check(
            "ThenThan",
            &[
                ("Since than it works.", "than>then"),
                ("From than on we test.", "than>then"),
                ("Than we restart the server.", "Than>Then"),
            ],
            &["It was more back than front."],
        );
    }

    /// Harper's findings at each `word` of `text`, after [`veto`].
    fn kept(name: &str, text: &str, word: &str) -> bool {
        let chars: Vec<char> = text.chars().collect();
        let tokens = tokenize(&chars);
        let start = text.find(word).expect("word in text");
        let start = text[..start].chars().count();
        let mut lints = BTreeMap::new();
        lints.insert(
            name.to_string(),
            vec![Lint {
                span: Span::new(start, start + word.chars().count()),
                ..Default::default()
            }],
        );
        veto(&chars, &tokens, &mut lints);
        !lints[name].is_empty()
    }

    #[test]
    fn veto_harper_confusions() {
        for (name, text, word, keep) in [
            (
                "ItsContraction",
                "This allows it and its associated\ndata to be deleted.",
                "its",
                false,
            ),
            (
                "ItsContraction",
                "Currently, its limit on the number of states.",
                "its",
                false,
            ),
            (
                "ItsContraction",
                "For each item its set of parameters.",
                "its",
                false,
            ),
            (
                "ItsContraction",
                "Its named mask values are specced.",
                "Its",
                false,
            ),
            (
                "ItsContraction",
                "The formatter and its buffer are discarded.",
                "its",
                false,
            ),
            (
                "ItsContraction",
                "Faster since its a power of 2.",
                "its",
                true,
            ),
            ("ItsContraction", "Assert its a power of two.", "its", true),
            (
                "ItsContraction",
                "Close the pool after its closed.",
                "its",
                true,
            ),
            (
                "ItsContraction",
                "It returned early (and its defer statement ran).",
                "its",
                false,
            ),
            (
                "ItsContraction",
                "Each time its called, it runs.",
                "its",
                true,
            ),
            (
                "ItsContraction",
                "Check it or, if not, its due to an error.",
                "its",
                true,
            ),
            (
                "ItsPossessive",
                "Specify all of it's component as a matrix.",
                "it's",
                true,
            ),
            ("ItsPossessive", "I know it's reserved.", "it's", false),
            (
                "ItsPossessive",
                "If cipher is set it's encryption.",
                "it's",
                false,
            ),
            (
                "ItsPossessive",
                "It's likely slower than that.",
                "It's",
                false,
            ),
            (
                "ItsPossessive",
                "Returning it's span if consumed.",
                "it's",
                true,
            ),
            (
                "ItsPossessive",
                "It's purpose is to store results.",
                "It's",
                true,
            ),
            (
                "ItsPossessive",
                "Return other primes; it's caller's job.",
                "it's",
                false,
            ),
            (
                "ItsPossessive",
                "Offset relative to it's parent's box.",
                "it's",
                true,
            ),
            (
                "ItsPossessive",
                "It's valid CSS, but it bypasses the flow.",
                "It's",
                false,
            ),
            (
                "ItsPossessive",
                "It's main purpose is caching.",
                "It's",
                true,
            ),
            (
                "ItsPossessive",
                "Dependent on it's binary form.",
                "it's",
                true,
            ),
            (
                "ItsPossessive",
                "Use a glyph to find it's bounding box.",
                "it's",
                true,
            ),
            (
                "ThenThan",
                "If we cannot represent the number then we emit zero.",
                "then",
                false,
            ),
            (
                "ThenThan",
                "If lower then buf is valid UTF-8.",
                "then",
                false,
            ),
            (
                "ThenThan",
                "We need to lower then simply lower it.",
                "then",
                false,
            ),
            ("ThenThan", "She loved me more even then.", "then", false),
            (
                "ThenThan",
                "If SET or OTHER then header is included.",
                "then",
                false,
            ),
            ("ThenThan", "It is smaller then the length.", "then", true),
            (
                "ItsPossessive",
                "Always pretend it's Jan 1, 1970 at midnight.",
                "it's",
                false,
            ),
            (
                "ItsPossessive",
                "When exp is NULL, it's NULL.",
                "it's",
                false,
            ),
            (
                "ItsPossessive",
                "- when exp is NULL, it's NULL",
                "it's",
                false,
            ),
            (
                "ThenThan",
                "If targeting P or later then passing these values will fail.",
                "then",
                false,
            ),
            (
                "ThenThan",
                "If it is larger then any remaining registers are ignored.",
                "then",
                false,
            ),
            (
                "ThenThan",
                "If the value is greater then no change will be made.",
                "then",
                false,
            ),
            (
                "ThenThan",
                "Fails if the buffer is smaller then the resulting data.",
                "then",
                true,
            ),
            (
                "ThenThan",
                "The total size is bigger then the other end can accept.",
                "then",
                true,
            ),
            (
                "ThenThan",
                "It returns anything other then 2xx.",
                "then",
                true,
            ),
            (
                "ThenThan",
                "We accept that rather then retry.",
                "then",
                true,
            ),
            (
                "ItsPossessive",
                "The job took longer than it's interval; retry.",
                "it's",
                true,
            ),
            (
                "ThereOwn",
                "The people there own nice cars.",
                "there",
                false,
            ),
            ("ThereOwn", "They wrote there own parser.", "there", true),
            (
                "ItsPossessive",
                "Indent the line to show it's part of a collection.",
                "it's",
                false,
            ),
            (
                "ItsContraction",
                "They should keep alive its embedded pointers (which would otherwise be).",
                "its",
                false,
            ),
            (
                "ItsContraction",
                "Wrap the returned          and its contained            with        .",
                "its",
                false,
            ),
        ] {
            assert_eq!(kept(name, text, word), keep, "{name}: {text}");
        }
    }
}
