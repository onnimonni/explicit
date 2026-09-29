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

const CHAIN_BODY: &str = r"[^,.;:!?\n\u{2013}\u{2014}\u{2026}]*";
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

    fn find(&self, text: &str) -> Vec<Hit> {
        static SPLIT: LazyLock<Regex> = LazyLock::new(|| re(&format!("(?i){CHAIN_SEP}")));
        self.chain
            .find_iter(text)
            .map(|m| {
                let end = trim_end_ws(text, m.start(), m.end());
                let count = SPLIT
                    .split(m.as_str())
                    .filter(|p| self.head.is_match(p.trim()))
                    .count();
                Hit {
                    range: m.start()..end,
                    count,
                    label: self.label.to_string(),
                }
            })
            .collect()
    }
}

static NO_CHAIN: LazyLock<Chain> = LazyLock::new(|| Chain::new(r"no[-\s]", "No X, no Y"));
static DID_NOT_CHAIN: LazyLock<Chain> =
    LazyLock::new(|| Chain::new(r"(?:did\s+not|didn't)\s", "Did not X, did not Y"));

pub fn negation_chains(text: &str) -> Vec<Hit> {
    let mut hits = NO_CHAIN.find(text);
    hits.extend(DID_NOT_CHAIN.find(text));
    hits.sort_by_key(|h| h.range.start);
    hits
}

pub fn negation_chain(seg: &Segment, sev: Severity, out: &mut Vec<Finding>) {
    let text = flatten(&seg.text);
    // Blanked code can leave an empty item ("no `x`"), so require two counted items.
    for h in negation_chains(&text).into_iter().filter(|h| h.count >= 2) {
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
