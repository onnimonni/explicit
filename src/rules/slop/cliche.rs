// explicit-disable-file slop/* prose/* grammar/* spelling -- examples of the patterns this module detects
//! Structural LLM cliches: negation chains, "don't call it X. Call it Y.", echoing and
//! stacked sentences, repeated openers, colon triples, stranded auxiliaries, "not X but Y".
//!
//! Adapted from Simon Willison's llm-cliche-highlighter (Apache-2.0,
//! https://github.com/simonw/tools/blob/a48d9920ff1cf203cd0070c3e8321283df50bf11/llm-cliche-highlighter.html);
//! see NOTICE. The phrase-type patterns from the same tool live in `phrases.toml`, and the
//! upstream self-tests are ported in the tests below.
//!
//! All detectors work on one segment at a time. Single newlines (wrapped lines) are treated
//! as spaces; blank lines stay hard boundaries.

use std::ops::Range;
use std::sync::LazyLock;

use regex::{Captures, Regex};

use crate::diagnostic::{Finding, Severity};
use crate::segment::{Segment, SegmentKind};

fn re(p: &str) -> Regex {
    Regex::new(&p.replace('\'', "['’]")).expect("valid cliche regex")
}

/// Segment kinds that hold running prose (sentence-sequence rules).
pub fn is_prose_block(kind: SegmentKind) -> bool {
    matches!(
        kind,
        SegmentKind::Paragraph
            | SegmentKind::BlockQuote
            | SegmentKind::Comment
            | SegmentKind::DocComment
    )
}

/// Segment kinds where single-sentence constructions are checked.
pub fn is_sentence_block(kind: SegmentKind) -> bool {
    is_prose_block(kind) || kind == SegmentKind::ListItem
}

/// `text` with newlines inside a paragraph replaced by spaces (same byte length).
pub fn flatten(text: &str) -> String {
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    let mut out = String::with_capacity(text.len());
    for (i, line) in lines.iter().enumerate() {
        let next_blank = lines
            .get(i + 1)
            .is_none_or(|l| l.trim().is_empty() || starts_list_item(l));
        match line.strip_suffix('\n') {
            Some(body) if !body.trim().is_empty() && !next_blank => {
                out.push_str(body);
                out.push(' ');
            }
            _ => out.push_str(line),
        }
    }
    out
}

/// A line that starts a list item (`- x`, `* x`, `+ x`, `1. x`), e.g. inside a doc comment.
fn starts_list_item(line: &str) -> bool {
    let t = line.trim_start();
    let digits = t.bytes().take_while(u8::is_ascii_digit).count();
    let rest = &t[digits..];
    if digits > 0 {
        return rest.starts_with(". ") || rest.starts_with(") ");
    }
    ["- ", "* ", "+ "].iter().any(|m| t.starts_with(m))
}

/// Like `captures_iter`, but when `reject` refuses a match the search resumes one char
/// after its start (as a regex engine would after a failed lookahead).
fn captures_filtered<'t>(
    r: &Regex,
    text: &'t str,
    mut reject: impl FnMut(&Captures<'t>) -> bool,
) -> Vec<Captures<'t>> {
    let mut out = Vec::new();
    let mut pos = 0;
    while pos <= text.len() {
        let Some(c) = r.captures_at(text, pos) else {
            break;
        };
        let m = c.get(0).expect("group 0");
        if reject(&c) {
            pos = m.start() + text[m.start()..].chars().next().map_or(1, char::len_utf8);
            continue;
        }
        pos = if m.end() > m.start() {
            m.end()
        } else {
            m.end() + 1
        };
        out.push(c);
    }
    out
}

fn snippet(s: &str, max: usize) -> String {
    let s: String = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if s.chars().count() <= max {
        return s;
    }
    let t: String = s.chars().take(max).collect();
    format!("{}...", t.trim_end())
}

fn trim_end_ws(text: &str, start: usize, mut end: usize) -> usize {
    while end > start && text.as_bytes()[end - 1].is_ascii_whitespace() {
        end -= 1;
    }
    end
}

/// A local finding before it is anchored to a segment.
#[derive(Debug, Clone, PartialEq)]
pub struct Hit {
    pub range: Range<usize>,
    pub count: usize,
    pub label: String,
}

// Rule slop/negation-chain (upstream `no-chain`, `did-not-chain`).

// Items end at clause punctuation, brackets and quotes, so a chain stays in one clause.
const CHAIN_BODY: &str = r#"[^,.;:!?\n\u{2013}\u{2014}\u{2026}()\[\]"\u{201c}\u{201d}]*"#;
const CHAIN_SEP: &str = r"(?:\s*,\s*(?:and\s+|or\s+)?|\s+(?:and|or)\s+|\s*[;&\u{2013}\u{2014}]\s*(?:and\s+|or\s+)?|\s+-{1,2}\s+)";

struct Chain {
    chain: Regex,
    head: Regex,
    label: &'static str,
}

impl Chain {
    fn new(head: &str, label: &'static str) -> Chain {
        let item = format!("{head}{CHAIN_BODY}");
        Chain {
            chain: re(&format!(r"(?i)\b{item}(?:{CHAIN_SEP}{item})+")),
            head: re(&format!("(?i)^{head}")),
            label,
        }
    }

    /// Hits with the byte ranges of their counted items.
    fn find(&self, text: &str) -> Vec<(Hit, Vec<Range<usize>>)> {
        static SPLIT: LazyLock<Regex> = LazyLock::new(|| re(&format!("(?i){CHAIN_SEP}")));
        self.chain
            .find_iter(text)
            .map(|m| {
                let end = trim_end_ws(text, m.start(), m.end());
                let mut items = Vec::new();
                let mut from = m.start();
                let seps = SPLIT
                    .find_iter(&text[m.start()..end])
                    .map(|s| (m.start() + s.start(), m.start() + s.end()))
                    .chain([(end, end)]);
                for (a, b) in seps {
                    let part = &text[from..a];
                    let lead = part.len() - part.trim_start().len();
                    if self.head.is_match(part.trim()) {
                        items.push(from + lead..a);
                    } else if let Some(last) = items.last_mut() {
                        // "no X or Y": the non-negated part extends the previous item.
                        last.end = a;
                    }
                    from = b;
                }
                let hit = Hit {
                    range: m.start()..end,
                    count: items.len(),
                    label: self.label.to_string(),
                };
                (hit, items)
            })
            .collect()
    }
}

static NO_CHAIN: LazyLock<Chain> = LazyLock::new(|| Chain::new(r"no[-\s]", "No X, no Y"));
static DID_NOT_CHAIN: LazyLock<Chain> =
    LazyLock::new(|| Chain::new(r"(?:did\s+not|didn't)\s", "Did not X, did not Y"));

/// Nouns of slogan-style "no X, no Y" copy ("no fees, no fuss").
const SLOGAN_NOUNS: &[&str] = &[
    "ads",
    "apologies",
    "bells",
    "bloat",
    "bs",
    "buzzwords",
    "catch",
    "clutter",
    "commitment",
    "commitments",
    "compromise",
    "compromises",
    "contracts",
    "downloads",
    "drama",
    "excuses",
    "fees",
    "filler",
    "fluff",
    "frills",
    "friction",
    "fuss",
    "gimmicks",
    "guesswork",
    "hassle",
    "headaches",
    "hype",
    "jargon",
    "nonsense",
    "paywall",
    "paywalls",
    "pressure",
    "regrets",
    "sign-ups",
    "signup",
    "signups",
    "spam",
    "stress",
    "subscription",
    "subscriptions",
    "surprises",
    "tricks",
    "whistles",
    "worries",
];

/// Technical nouns: a "no X, no Y" list of these is a factual scope statement.
const TECH_NOUNS: &[&str] = &[
    "api",
    "backend",
    "character",
    "code",
    "error",
    "fallback",
    "frontend",
    "init",
    "newline",
    "path",
    "prefix",
    "reload",
    "suffix",
    "argument",
    "artifact",
    "attribute",
    "auth",
    "branch",
    "bucket",
    "build",
    "cache",
    "call",
    "callback",
    "check",
    "client",
    "clock",
    "cluster",
    "column",
    "commit",
    "component",
    "config",
    "configuration",
    "container",
    "cookie",
    "credential",
    "daemon",
    "database",
    "dependency",
    "deploy",
    "deployment",
    "digest",
    "directory",
    "endpoint",
    "entry",
    "env",
    "field",
    "file",
    "fixture",
    "flag",
    "header",
    "hook",
    "id",
    "image",
    "import",
    "index",
    "interpreter",
    "job",
    "key",
    "lease",
    "library",
    "lock",
    "log",
    "manifest",
    "method",
    "migration",
    "mock",
    "module",
    "network",
    "node",
    "package",
    "parser",
    "patch",
    "pipeline",
    "plugin",
    "pointer",
    "policy",
    "process",
    "proxy",
    "push",
    "query",
    "queue",
    "read",
    "record",
    "registry",
    "release",
    "request",
    "response",
    "role",
    "route",
    "row",
    "runtime",
    "schema",
    "script",
    "secret",
    "server",
    "service",
    "session",
    "socket",
    "stamp",
    "state",
    "step",
    "stub",
    "styling",
    "table",
    "tag",
    "test",
    "thread",
    "timer",
    "token",
    "tooling",
    "type",
    "upload",
    "url",
    "variable",
    "version",
    "wrapper",
    "write",
];

fn in_words(list: &[&str], word: &str) -> bool {
    let w = word.to_ascii_lowercase();
    let singular = w.strip_suffix('s').unwrap_or(&w);
    list.contains(&w.as_str()) || list.contains(&singular)
}

/// An identifier-looking token: `snake_case`, `a/b`, `a.b`, `a::b`, digits, `CamelCase`.
fn identifier_like(word: &str) -> bool {
    let w = word.trim_matches(|c: char| !c.is_alphanumeric());
    w.contains(['_', '/', '@', '#', '§', ':', '.', '='])
        || w.bytes().any(|b| b.is_ascii_digit())
        || (w.chars().any(char::is_lowercase) && w.chars().skip(1).any(char::is_uppercase))
}

/// Blanked code or markup in the item `raw[r]` (head included): a run of 3+ spaces within a
/// line, e.g. "no `x`" or "no `a`, no b". Runs that cross a newline are line indentation.
fn has_blank(raw: &str, r: Range<usize>) -> bool {
    let Some(item) = raw.get(r.start..) else {
        return false;
    };
    let len = r.end - r.start;
    let mut run = 0;
    let mut newline = false;
    for (i, c) in item.char_indices() {
        match c {
            ' ' => run += 1,
            '\n' => {
                newline = true;
                run += 1;
            }
            _ => {
                if run >= 3 && !newline {
                    return true;
                }
                if i >= len {
                    return false;
                }
                run = 0;
                newline = false;
            }
        }
    }
    run >= 3 && !newline
}

/// Whether a "no X, no Y" chain reads as a rhetorical slogan rather than a factual scope list
/// ("there is no registry, no `:global`, no lease"). Slogans have short everyday items and
/// either slogan nouns or a sentence-initial fragment; scope lists carry code, identifiers,
/// proper nouns, technical nouns or long items, usually inside a clause.
fn rhetorical_no_chain(raw: &str, flat: &str, hit: &Hit, items: &[Range<usize>]) -> bool {
    let chain = &flat[hit.range.clone()];
    let shouting = !chain.chars().any(char::is_lowercase);
    let mut slogan = false;
    let mut technical = false;
    let mut short = 0;
    for r in items {
        let item = &flat[r.clone()];
        // Skip the head ("no " / "no-"); an empty body is blanked code.
        let body = item.get(3..).unwrap_or("");
        let words: Vec<&str> = body
            .split_whitespace()
            .filter(|w| w.chars().any(char::is_alphanumeric))
            .collect();
        if words.is_empty() || has_blank(raw, r.clone()) {
            return false;
        }
        // "no such X", "no longer", "no one", "no more": not a list of absent things.
        const NOT_NOUN: &[&str] = &["such", "longer", "one", "more", "less", "matter", "doubt"];
        if words.iter().any(|w| identifier_like(w))
            || NOT_NOUN.iter().any(|n| words[0].eq_ignore_ascii_case(n))
        {
            return false;
        }
        if words.len() <= 3 {
            short += 1;
        }
        for w in &words {
            let w = w.trim_matches(|c: char| !c.is_alphanumeric() && c != '-');
            if in_words(SLOGAN_NOUNS, w) {
                slogan = true;
            }
            if in_words(TECH_NOUNS, w) || (!shouting && w.starts_with(char::is_uppercase)) {
                technical = true;
            }
        }
    }
    let n = items.len();
    if slogan && (!technical || short == n) {
        return true;
    }
    // Short items; three or more may end in a longer one ("no time, no money, no way to ...").
    let last_short = items
        .last()
        .is_some_and(|r| flat[r.clone()].split_whitespace().count() <= 4);
    if technical || !(short == n || (n >= 3 && short == n - 1 && !last_short)) {
        return false;
    }
    // Sentence-initial fragment: nothing but an opener, colon or dash before the chain.
    let before = flat[..hit.range.start].trim_end();
    before.is_empty()
        || before.ends_with([
            '.', '!', '?', ':', ';', '\u{2014}', '\u{2013}', '-', '"', '\u{201c}', '(',
        ])
}

/// Negation chains in `text` (raw segment text; wrapped lines are joined here).
#[cfg(test)]
pub fn negation_chains(text: &str) -> Vec<Hit> {
    let flat = flatten(text);
    negation_chains_in(text, &flat)
}

fn negation_chains_in(raw: &str, flat: &str) -> Vec<Hit> {
    let mut hits: Vec<Hit> = NO_CHAIN
        .find(flat)
        .into_iter()
        .filter(|(h, items)| rhetorical_no_chain(raw, flat, h, items))
        .map(|(h, _)| h)
        .collect();
    hits.extend(DID_NOT_CHAIN.find(flat).into_iter().map(|(h, _)| h));
    hits.sort_by_key(|h| h.range.start);
    hits
}

pub fn negation_chain(seg: &Segment, sev: Severity, out: &mut Vec<Finding>) {
    let text = flatten(&seg.text);
    // Chains in list items and table cells are usually terse spec notes: info by default.
    let sev = if sev == Severity::Warning
        && matches!(seg.kind, SegmentKind::ListItem | SegmentKind::TableCell)
    {
        Severity::Info
    } else {
        sev
    };
    for h in negation_chains_in(&seg.text, &text)
        .into_iter()
        .filter(|h| h.count >= 2)
    {
        out.push(
            Finding::new(
                "slop/negation-chain",
                sev,
                seg.abs(h.range.clone()),
                format!(
                    "\"{}\" chain with {} items: \"{}\"",
                    h.label,
                    h.count,
                    snippet(&text[h.range], 60)
                ),
            )
            .help("A list of what something is not sounds punchy but says little. State what it does or has instead (\"setup is one command\" rather than \"no sign-ups, no downloads\")."),
        );
    }
}

// Rule slop/dont-verb-it.

const DONT_HEAD: &str = r"(?i)\b(?:do\s+not|don't)\s+(?:just\s+|simply\s+|merely\s+)?(\w+)(?:\s+(?:of|about|at|on|for|with|to))?\s+it\b";

pub fn dont_verb_its(text: &str) -> Vec<Hit> {
    static HEAD: LazyLock<Regex> = LazyLock::new(|| re(DONT_HEAD));
    static PREFIX: LazyLock<Regex> =
        LazyLock::new(|| re(r#"(?i)^['"”’]*\s*(?:just\s+|simply\s+|merely\s+)?"#));
    static TAIL: LazyLock<Regex> =
        LazyLock::new(|| re(r"(?i)^(?:\s+(?:of|about|at|on|for|with|to))?\s+it\b"));
    let mut hits = Vec::new();
    let mut pos = 0;
    while let Some(c) = HEAD.captures_at(text, pos) {
        let m = c.get(0).expect("group 0");
        let verb = c[1].to_lowercase();
        let mut found = None;
        // `[^.!?\n]*?` then one clause-ending mark, tried left to right.
        for (i, ch) in text[m.end()..].char_indices() {
            let p = m.end() + i;
            if matches!(ch, '.' | '!' | '?' | ';' | ',' | ':' | '–' | '—') {
                let after = p + ch.len_utf8();
                let pre = PREFIX.find(&text[after..]).map_or(0, |x| x.end());
                let v = after + pre;
                let rest = &text[v..];
                let word_len = rest
                    .char_indices()
                    .find(|(_, c)| !(c.is_alphanumeric() || *c == '_'))
                    .map_or(rest.len(), |(i, _)| i);
                if word_len > 0
                    && rest[..word_len].to_lowercase() == verb
                    && let Some(t) = TAIL.find(&rest[word_len..])
                {
                    found = Some(v + word_len + t.end());
                    break;
                }
            }
            if matches!(ch, '.' | '!' | '?' | '\n') {
                break;
            }
        }
        match found {
            Some(end) => {
                hits.push(Hit {
                    range: m.start()..end,
                    count: 2,
                    label: verb,
                });
                pos = end;
            }
            None => pos = m.end(),
        }
    }
    hits
}

pub fn dont_verb_it(seg: &Segment, sev: Severity, out: &mut Vec<Finding>) {
    let text = flatten(&seg.text);
    for h in dont_verb_its(&text) {
        out.push(
            Finding::new(
                "slop/dont-verb-it",
                sev,
                seg.abs(h.range.clone()),
                format!(
                    "\"Don't {v} it ... {v} it\" reframe: \"{}\"",
                    snippet(&text[h.range], 60),
                    v = h.label
                ),
            )
            .help("State the second clause on its own (\"It is a return.\"); the negated setup is a stock rhetorical flourish."),
        );
    }
}

/// Sentence text. Upstream splits on every `.!?`; here a mark followed by a non-space
/// (`a.md`, `v1.2`, `e.g.x`) stays inside the sentence, which keeps file names and
/// versions in unquoted code comments from splitting sentences.
const SENT_BODY: &str = r"(?:[^.!?\n]|[.!?][^\s.!?])+";

// Rule slop/echo-sentences (upstream `echo-triad`).

fn lower_words(s: &str) -> Vec<String> {
    static W: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[a-z0-9'’-]+").expect("valid"));
    let lower = s.to_lowercase();
    W.find_iter(&lower)
        .map(|m| m.as_str().to_string())
        .collect()
}

/// The skeleton two sentences both end with: at least `n` shared trailing words, covering
/// at least half of the shorter sentence ("A cart is an object in the system. A room is an
/// object in the system."). Upstream accepts a shared n-gram anywhere, which flags every
/// pair of technical sentences that repeat a term ("the minimum supported version").
fn shared_skeleton(a: &str, b: &str, n: usize) -> Option<String> {
    let (a, b) = (lower_words(a), lower_words(b));
    let common = a
        .iter()
        .rev()
        .zip(b.iter().rev())
        .take_while(|(x, y)| x == y)
        .count();
    (common >= n && common * 2 >= a.len().min(b.len())).then(|| a[a.len() - common..].join(" "))
}

pub fn echoes(text: &str) -> Vec<Hit> {
    const MIN_GRAM: usize = 4;
    const MIN_RUN: usize = 2;
    static SENT: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(&format!("{SENT_BODY}[.!?]?")).expect("valid"));
    let sents: Vec<Range<usize>> = SENT
        .find_iter(text)
        .filter(|m| m.as_str().split_whitespace().count() >= 4)
        .map(|m| m.range())
        .collect();
    let mut hits = Vec::new();
    let mut i = 0;
    while i < sents.len() {
        let mut j = i;
        let mut shared: Option<String> = None;
        while j + 1 < sents.len() {
            if sents[j + 1].start - sents[j].end > 3 {
                break;
            }
            let Some(g) = shared_skeleton(
                &text[sents[j].clone()],
                &text[sents[j + 1].clone()],
                MIN_GRAM,
            ) else {
                break;
            };
            shared = Some(g);
            j += 1;
        }
        let run = j - i + 1;
        match shared {
            Some(g) if run >= MIN_RUN => {
                let end = trim_end_ws(text, sents[i].start, sents[j].end);
                let start = sents[i].start
                    + (text[sents[i].clone()].len() - text[sents[i].clone()].trim_start().len());
                hits.push(Hit {
                    range: start..end,
                    count: run,
                    label: g,
                });
                i = j + 1;
            }
            _ => i += 1,
        }
    }
    hits
}

pub fn echo_sentences(seg: &Segment, sev: Severity, out: &mut Vec<Finding>) {
    let text = flatten(&seg.text);
    for h in echoes(&text) {
        out.push(
            Finding::new(
                "slop/echo-sentences",
                sev,
                seg.abs(h.range),
                format!("{} consecutive sentences repeat the skeleton \"{}\"", h.count, h.label),
            )
            .help("Merge them into one sentence or a list (\"The parser, renderer and scheduler are state machines.\"), or vary the structure."),
        );
    }
}

// Rule slop/stacked-questions.

pub fn question_chains(text: &str) -> Vec<Hit> {
    static CHAIN: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(&format!(r"{SENT_BODY}\?(?:[ \t]+{SENT_BODY}\?)+")).expect("valid")
    });
    CHAIN
        .find_iter(text)
        .filter_map(|m| {
            let count = m.as_str().matches('?').count();
            // Grammar notation and code in comments (`EncodingDecl? SDDecl?`) is not prose:
            // every `?` must end a question (followed by a space or the end) and no
            // operator characters may appear.
            let bytes = text.as_bytes();
            let ends_ok = m.as_str().match_indices('?').all(|(i, _)| {
                bytes
                    .get(m.start() + i + 1)
                    .is_none_or(|b| b.is_ascii_whitespace())
            });
            if count < 2
                || !ends_ok
                || m.as_str()
                    .contains(['=', '|', '<', '>', '*', '[', ']', '{', '}'])
            {
                return None;
            }
            let start = m.end() - m.as_str().trim_start().len();
            Some(Hit {
                range: start..m.end(),
                count,
                label: String::new(),
            })
        })
        .collect()
}

pub fn stacked_questions(seg: &Segment, sev: Severity, out: &mut Vec<Finding>) {
    let text = flatten(&seg.text);
    for h in question_chains(&text) {
        out.push(
            Finding::new(
                "slop/stacked-questions",
                sev,
                seg.abs(h.range),
                format!("{} rhetorical questions in a row", h.count),
            )
            .help("Answer the question you care about, or turn the questions into a statement of what the reader will learn."),
        );
    }
}

// Rule slop/anaphora (upstream `sentence-anaphora`).

const ANAPHORA_SKIP: &[&str] = &[
    "i", "it", "the", "a", "an", "this", "that", "we", "you", "they", "he", "she", "there", "but",
    "and", "so", "in", "as", "if", "my", "his", "her", "their", "its", "these", "those", "for",
    "at", "on", "of", "to", "is", "was",
];

pub fn anaphoras(text: &str) -> Vec<Hit> {
    const MIN_RUN: usize = 3;
    static SENT: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(&format!("{SENT_BODY}[.!?]")).expect("valid"));
    static WORD: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"[A-Za-z][A-Za-z'’-]*").expect("valid"));
    let sents: Vec<(usize, usize, String)> = SENT
        .find_iter(text)
        .filter_map(|m| {
            let w = WORD.find(m.as_str())?;
            Some((m.start() + w.start(), m.end(), w.as_str().to_lowercase()))
        })
        .collect();
    let mut hits = Vec::new();
    let mut i = 0;
    while i < sents.len() {
        let mut j = i;
        while j + 1 < sents.len() && sents[j + 1].2 == sents[i].2 && sents[j + 1].0 - sents[j].1 < 4
        {
            j += 1;
        }
        let run = j - i + 1;
        if run >= MIN_RUN && !ANAPHORA_SKIP.contains(&sents[i].2.as_str()) {
            hits.push(Hit {
                range: sents[i].0..sents[j].1,
                count: run,
                label: sents[i].2.clone(),
            });
            i = j + 1;
        } else {
            i += 1;
        }
    }
    hits
}

pub fn anaphora(seg: &Segment, sev: Severity, out: &mut Vec<Finding>) {
    let text = flatten(&seg.text);
    for h in anaphoras(&text) {
        out.push(
            Finding::new(
                "slop/anaphora",
                sev,
                seg.abs(h.range),
                format!("{} consecutive sentences open with \"{}\"", h.count, h.label),
            )
            .help("Repeated openers are a cadence trick. Combine the sentences or vary how they start."),
        );
    }
}

// Rule slop/colon-triple (off by default: noisy in technical writing).

pub fn colon_triples(text: &str) -> Vec<Hit> {
    static R: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(:\s+[^.!?;:\n]{2,40},\s+[^.!?;:\n]{2,40},\s+(?:and\s+|or\s+)?[^.!?;:\n]{2,40})(?:[.!?\n]|$)")
            .expect("valid")
    });
    R.captures_iter(text)
        .filter_map(|c| c.get(1))
        .map(|m| Hit {
            range: m.range(),
            count: 3,
            label: String::new(),
        })
        .collect()
}

pub fn colon_triple(seg: &Segment, sev: Severity, out: &mut Vec<Finding>) {
    let text = flatten(&seg.text);
    for h in colon_triples(&text) {
        out.push(
            Finding::new(
                "slop/colon-triple",
                sev,
                seg.abs(h.range.clone()),
                format!("Colon into a triple: \"{}\"", snippet(&text[h.range], 60)),
            )
            .help("Check all three items are needed; keep the one that carries the point, or use a real list."),
        );
    }
}

// Rule slop/stranded-auxiliary.

pub fn stranded_auxiliaries(text: &str) -> Vec<Hit> {
    static R: LazyLock<Regex> = LazyLock::new(|| {
        re(
            r"[;:,]\s+[^.;:!?\n]{2,50}\s(?:did|does|do|was|were|is|are|has|have|had|can|could|would|will)(?:n't)?[.;]|\b(?:Maybe|Perhaps)\s+\w+[^.!?\n]{0,40}\s(?:would|could|might|should|did|had|was|is)(?:n't)?\s+(?:have\s*)?\.",
        )
    });
    R.find_iter(text)
        .filter(|m| !subordinate_clause(m.as_str()))
        .map(|m| Hit {
            range: m.range(),
            count: 1,
            label: String::new(),
        })
        .collect()
}

/// Tuning over upstream: ", if it does." / "items that don't." are ordinary conditional or
/// relative clauses, not the "X did; Y didn't" reversal.
fn subordinate_clause(m: &str) -> bool {
    const LEADS: &[&str] = &[
        "if", "when", "whenever", "unless", "as", "than", "because", "since", "whether", "where",
        "which", "that", "while", "so", "like", "until", "once", "before", "after", "though",
        "although", "even", "just", "rather",
    ];
    const RELATIVE: &[&str] = &["that", "which", "who", "what"];
    let words: Vec<String> = m
        .split(|c: char| !(c.is_alphanumeric() || c == '\'' || c == '’'))
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
        .collect();
    let first = words.first().map(String::as_str).unwrap_or("");
    let before_aux = words.len().checked_sub(2).map_or("", |i| words[i].as_str());
    LEADS.contains(&first) || RELATIVE.contains(&before_aux)
}

pub fn stranded_auxiliary(seg: &Segment, sev: Severity, out: &mut Vec<Finding>) {
    let text = flatten(&seg.text);
    for h in stranded_auxiliaries(&text) {
        out.push(
            Finding::new(
                "slop/stranded-auxiliary",
                sev,
                seg.abs(h.range.clone()),
                format!("Clause ends on a bare auxiliary for the reversal: \"{}\"", snippet(&text[h.range], 60)),
            )
            .help("Say the contrast with the full verb (\"the tool crashed but the data survived\") instead of the punchy \"X did; Y didn't\"."),
        );
    }
}

// Rule slop/not-but (off by default: plain "not X, but Y" is ordinary English).

pub fn not_buts(text: &str) -> Vec<Hit> {
    static R: LazyLock<Regex> = LazyLock::new(|| {
        re(
            r"(?i)\bnot\s+(?P<nf>)[^.!?\n;]{1,100}?\bbut\b|\b(?:isn't|is not)\s+[^.!?\n]{1,80}[.!?]\s*(?:it's|it is|this's|this is|that's|that is)\b",
        )
    });
    static INTENSIFIER: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?i)^(?:just|only|merely|simply)\b").expect("valid"));
    captures_filtered(&R, text, |c| {
        c.name("nf")
            .is_some_and(|g| INTENSIFIER.is_match(&text[g.end()..]))
    })
    .into_iter()
    .map(|c| Hit {
        range: c.get(0).expect("group 0").range(),
        count: 1,
        label: String::new(),
    })
    .collect()
}

pub fn not_but(seg: &Segment, sev: Severity, out: &mut Vec<Finding>) {
    let text = flatten(&seg.text);
    for h in not_buts(&text) {
        out.push(
            Finding::new(
                "slop/not-but",
                sev,
                seg.abs(h.range.clone()),
                format!("Negative parallelism \"{}\"", snippet(&text[h.range], 60)),
            )
            .help("State the positive claim directly; knock down X only if a reader would actually believe it."),
        );
    }
}

#[cfg(test)]
mod tests;
