//! Mermaid validation via merman-core.
//!
//! The fenced block body is parsed with merman-core's headless parser (the same grammar
//! families as mermaid.js). Parse failures become `diagram/mermaid` findings mapped back to
//! absolute file offsets.

use std::ops::Range;
use std::sync::LazyLock;

use merman_core::{Engine, Error, ParseDiagnosticSpanKind, ParseOptions};

use crate::diagnostic::{Finding, Severity};
use crate::rules::Out;

const RULE: &str = "diagram/mermaid";
const HELP: &str = "fix the diagram syntax so Mermaid can render it; \
                    see https://mermaid.js.org/intro/syntax-reference.html";

/// Building the engine assembles detector/parser registries and the default config: do it once.
static ENGINE: LazyLock<Engine> = LazyLock::new(Engine::new);

/// Header keywords not (or not literally) present in merman's header facts.
const EXTRA_KEYWORDS: &[&str] = &[
    "flowchart",
    "graph",
    "stateDiagram",
    "sankey-beta",
    "block",
    "architecture",
    "treemap",
];

/// Validate Mermaid source `body` located at absolute byte `offset` in the file.
pub fn check(body: &str, offset: usize, out: &mut Out) {
    if body.trim().is_empty() {
        return;
    }
    // merman-core is alpha: never let a parser panic take down the linter.
    let res = std::panic::catch_unwind(|| ENGINE.parse_diagram_sync(body, ParseOptions::strict()));
    let Ok(res) = res else { return };
    let Err(err) = res else { return };

    let finding = match err {
        Error::DiagramParse {
            diagram_type,
            diagnostic,
        } => {
            let range = match diagnostic.span() {
                Some(sp) => span_range(body, sp.start, sp.end, diagnostic.span_kind()),
                None => locate_from_message(body, diagnostic.message())
                    .unwrap_or_else(|| first_line_range(body)),
            };
            Finding::new(
                RULE,
                Severity::Error,
                shift(range, offset),
                format!(
                    "Mermaid {}: {}",
                    pretty_type(&diagram_type),
                    clean_message(diagnostic.message())
                ),
            )
            .help(HELP)
        }
        Error::DetectType(_) => header_finding(body, offset),
        Error::OperationCancelled(_) => return,
        other => Finding::new(
            RULE,
            Severity::Error,
            shift(first_line_range(body), offset),
            format!("Mermaid: {}", clean_message(&other.to_string())),
        )
        .help(HELP),
    };
    out.push(finding);
}

/// Unknown or misspelled diagram header (e.g. `flowchrt`).
fn header_finding(body: &str, offset: usize) -> Finding {
    let Some(line) = header_line(body) else {
        return Finding::new(
            RULE,
            Severity::Error,
            shift(first_line_range(body), offset),
            "Mermaid: no diagram type header found",
        )
        .help("start the diagram with a type keyword such as `flowchart TD` or `sequenceDiagram`");
    };
    let text = &body[line.clone()];
    let lead = text.len() - text.trim_start().len();
    let word_len = text[lead..]
        .find(|c: char| c.is_whitespace() || c == ':' || c == ';')
        .unwrap_or(text.len() - lead);
    let word = &text[lead..lead + word_len];
    let range = line.start + lead..line.start + lead + word_len.max(1);
    let mut f = Finding::new(
        RULE,
        Severity::Error,
        shift(fit(body, range), offset),
        format!("Mermaid: unknown diagram type `{word}`"),
    );
    match nearest_keyword(word) {
        Some(kw) => {
            f = f.help(format!("did you mean `{kw}`?")).suggest(kw);
        }
        None => {
            f = f.help(
                "start the diagram with a known type keyword, e.g. `flowchart TD`, \
                 `sequenceDiagram`, `classDiagram`, `erDiagram`, `gantt`, `pie`",
            );
        }
    }
    f
}

/// Byte range of the header line: skips frontmatter, `%%` comments/directives and blank lines.
fn header_line(body: &str) -> Option<Range<usize>> {
    let mut in_frontmatter = false;
    let mut seen_content = false;
    let mut pos = 0;
    for raw in body.split_inclusive('\n') {
        let start = pos;
        pos += raw.len();
        let line = raw.trim_end_matches(['\n', '\r']);
        let t = line.trim();
        if in_frontmatter {
            if t == "---" {
                in_frontmatter = false;
            }
            continue;
        }
        if t.is_empty() || t.starts_with("%%") {
            continue;
        }
        if t == "---" && !seen_content {
            in_frontmatter = true;
            seen_content = true;
            continue;
        }
        return Some(start..start + line.len());
    }
    None
}

fn keywords() -> impl Iterator<Item = &'static str> {
    merman_core::diagram_header_facts()
        .iter()
        .map(|h| h.label.split_whitespace().next().unwrap_or(h.label))
        .chain(EXTRA_KEYWORDS.iter().copied())
}

fn nearest_keyword(word: &str) -> Option<&'static str> {
    let w = word.to_ascii_lowercase();
    if w.is_empty() {
        return None;
    }
    let (kw, dist) = keywords()
        .map(|k| (k, levenshtein(&w, &k.to_ascii_lowercase())))
        .min_by_key(|&(k, d)| (d, k.len()))?;
    let limit = (w.chars().count() / 3).max(2);
    (dist <= limit).then_some(kw)
}

fn levenshtein(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut cur = vec![i + 1; b.len() + 1];
        for (j, &cb) in b.iter().enumerate() {
            cur[j + 1] = (prev[j] + usize::from(ca != cb))
                .min(prev[j + 1] + 1)
                .min(cur[j] + 1);
        }
        prev = cur;
    }
    prev[b.len()]
}

/// Map a merman span (body-relative) to a non-empty, char-aligned body range.
fn span_range(body: &str, start: usize, end: usize, kind: ParseDiagnosticSpanKind) -> Range<usize> {
    let len = body.len();
    let start = floor(body, start.min(len));
    let end = ceil(body, end.min(len)).max(start);
    if end > start {
        return start..end;
    }
    let content_end = body.trim_end().len();
    if start >= content_end || kind == ParseDiagnosticSpanKind::InsertionPoint && start >= len {
        // At (or past) the last token: point at the final visible character.
        if content_end == 0 {
            return 0..0;
        }
        let s = floor(body, content_end - 1);
        return s..content_end;
    }
    fit(body, start..start)
}

/// Widen an empty range to one character and align to char boundaries.
fn fit(body: &str, r: Range<usize>) -> Range<usize> {
    let len = body.len();
    let start = floor(body, r.start.min(len));
    let mut end = ceil(body, r.end.min(len)).max(start);
    if end == start {
        if start < len {
            end = ceil(body, start + 1);
        } else if start > 0 {
            return floor(body, start - 1)..start;
        }
    }
    start..end
}

/// Without a span, try `line N` or a quoted offending token from the message.
fn locate_from_message(body: &str, msg: &str) -> Option<Range<usize>> {
    let lower = msg.to_ascii_lowercase();
    for marker in ["got ", "unexpected ", "found ", "token "] {
        let Some(i) = lower.find(marker) else {
            continue;
        };
        let rest = &msg[i + marker.len()..];
        let Some(q) = rest
            .chars()
            .next()
            .filter(|c| matches!(c, '\'' | '"' | '`'))
        else {
            continue;
        };
        let inner = &rest[1..];
        let Some(close) = inner.find(q) else { continue };
        let token = &inner[..close];
        if !token.trim().is_empty()
            && let Some(p) = body.find(token)
        {
            return Some(p..p + token.len());
        }
    }
    let i = lower.find("line ")?;
    let n: usize = lower[i + 5..]
        .chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>()
        .parse()
        .ok()?;
    let (start, line) = body
        .split_inclusive('\n')
        .scan(0, |pos, l| {
            let s = *pos;
            *pos += l.len();
            Some((s, l))
        })
        .nth(n.checked_sub(1)?)?;
    let line = line.trim_end_matches(['\n', '\r']);
    let lead = line.len() - line.trim_start().len();
    (line.len() > lead).then(|| start + lead..start + line.len())
}

/// First non-empty line (trimmed), or the first char of the body.
fn first_line_range(body: &str) -> Range<usize> {
    let mut pos = 0;
    for raw in body.split_inclusive('\n') {
        let line = raw.trim_end_matches(['\n', '\r']);
        let lead = line.len() - line.trim_start().len();
        if line.len() > lead {
            return pos + lead..pos + line.trim_end().len();
        }
        pos += raw.len();
    }
    fit(body, 0..0)
}

/// Single-line message: drop jison-style excerpt/caret lines and join the rest.
fn clean_message(msg: &str) -> String {
    let parts: Vec<&str> = msg
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.chars().all(|c| matches!(c, '-' | '^' | ' ')))
        .collect();
    if parts.len() <= 1 {
        return parts.first().copied().unwrap_or(msg).to_string();
    }
    // A DetectType-style message echoes the whole source; keep only its first line.
    if msg.contains("for text:") {
        return parts[0].to_string();
    }
    parts.join(" ")
}

fn pretty_type(t: &str) -> &str {
    t.strip_suffix("-v2").unwrap_or(t)
}

fn shift(r: Range<usize>, offset: usize) -> Range<usize> {
    r.start + offset..r.end + offset
}

fn floor(s: &str, mut i: usize) -> usize {
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

fn ceil(s: &str, mut i: usize) -> usize {
    while i < s.len() && !s.is_char_boundary(i) {
        i += 1;
    }
    i
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(body: &str) -> Vec<Finding> {
        let mut out = Vec::new();
        check(body, 0, &mut out);
        out
    }

    fn one(body: &str) -> (Finding, &str) {
        let out = run(body);
        assert_eq!(out.len(), 1, "expected one finding for {body:?}: {out:?}");
        let f = out.into_iter().next().unwrap();
        assert!(body.is_char_boundary(f.range.start) && body.is_char_boundary(f.range.end));
        assert!(
            f.range.end <= body.len() && f.range.start < f.range.end,
            "{f:?}"
        );
        let text = &body[f.range.clone()];
        (f, text)
    }

    #[test]
    fn valid_diagrams() {
        for body in [
            "flowchart TD\n  A[Start] --> B{Ok?}\n  B -->|yes| C[Done]\n",
            "graph LR\n  subgraph one\n    a --> b\n  end\n",
            "sequenceDiagram\n  Alice->>Bob: Hello\n  Bob-->>Alice: Hi\n",
            "classDiagram\n  class Animal {\n    +String name\n    +eat()\n  }\n  Animal <|-- Dog\n",
            "gantt\n  dateFormat YYYY-MM-DD\n  section S\n  Task :a1, 2024-01-01, 3d\n",
            "---\ntitle: Pets\n---\n%%{init: {'theme':'dark'}}%%\npie title Pets\n  \"Dogs\" : 3\n  \"Cats\" : 2\n",
            "",
            "\n  \n",
        ] {
            assert!(run(body).is_empty(), "{body:?}: {:?}", run(body));
        }
    }

    #[test]
    fn unterminated_node_label() {
        let (f, text) = one("flowchart TD\n  A[Start --> B\n");
        assert_eq!(text, "[Start --> B");
        assert!(
            f.message.starts_with("Mermaid flowchart: "),
            "{}",
            f.message
        );
        assert_eq!(f.rule, "diagram/mermaid");
        assert_eq!(f.severity, Severity::Error);
        assert!(f.help.is_some());
    }

    #[test]
    fn subgraph_without_end() {
        let body = "flowchart TD\n  subgraph one\n    A --> B\n";
        let (f, text) = one(body);
        // Insertion point at EOF is widened onto the last visible character.
        assert_eq!(text, "B");
        assert_eq!(f.range.start, body.rfind('B').unwrap());
        assert!(f.message.contains("end"), "{}", f.message);
    }

    #[test]
    fn bad_sequence_arrow() {
        let body = "sequenceDiagram\n  Alice -x-> Bob: hi\n";
        let (f, text) = one(body);
        assert_eq!(text, ">");
        assert!(body[..f.range.start].ends_with("Alice -x-"));
        assert!(f.message.starts_with("Mermaid sequence: "), "{}", f.message);
    }

    #[test]
    fn header_typo() {
        let (f, text) = one("flowchrt TD\n  A --> B\n");
        assert_eq!(text, "flowchrt");
        assert_eq!(f.message, "Mermaid: unknown diagram type `flowchrt`");
        assert_eq!(f.suggestions, vec!["flowchart".to_string()]);

        let (f, text) = one("---\ntitle: x\n---\n%% note\nsequenceDiagam\n  A->>B: hi\n");
        assert_eq!(text, "sequenceDiagam");
        assert_eq!(f.suggestions, vec!["sequenceDiagram".to_string()]);

        let (f, text) = one("zzzzqqqq\n");
        assert_eq!(text, "zzzzqqqq");
        assert!(f.suggestions.is_empty());
    }

    #[test]
    fn spans_after_frontmatter_are_body_relative() {
        let body = "---\ntitle: x\n---\nflowchart TD\n  A[Start --> B\n";
        let (_, text) = one(body);
        assert_eq!(text, "[Start --> B");
    }

    #[test]
    fn offset_is_applied_and_unicode_safe() {
        let body = "flowchart TD\n  A[Ä ö --> B\n";
        let mut out = Vec::new();
        check(body, 100, &mut out);
        assert_eq!(out.len(), 1);
        let r = out[0].range.clone();
        assert_eq!(&body[r.start - 100..r.end - 100], "[Ä ö --> B");
    }

    #[test]
    fn helpers() {
        assert_eq!(fit("ab", 2..2), 1..2);
        assert_eq!(fit("äb", 1..1), 0..2);
        assert_eq!(first_line_range("\n  x y \n"), 3..6);
        assert_eq!(
            locate_from_message("a\n  bad thing\n", "Parse error on line 2: nope"),
            Some(4..13)
        );
        assert_eq!(locate_from_message("A --> Q\n", "got 'Q'"), Some(6..7));
        assert_eq!(nearest_keyword("sequencediagram"), Some("sequenceDiagram"));
        assert_eq!(nearest_keyword("gant"), Some("gantt"));
    }

    #[test]
    fn end_to_end_markdown() {
        use crate::config::Config;
        use crate::rules::{Analyzed, FileCtx};
        use crate::source::{FileKind, SourceFile};

        let src = "# Title\n\nSome text.\n\n```mermaid\nflowchart TD\n  A[Start --> B\n```\n\n```mermaid\ngraph LR\n  a --> b\n```\n";
        let a = Analyzed::new(SourceFile::new(
            "t.md".into(),
            "t.md".into(),
            FileKind::Markdown,
            src.to_string(),
        ));
        let cfg = Config::default();
        let ctx = FileCtx {
            a: &a,
            config: &cfg,
        };
        let mut out = Vec::new();
        crate::rules::diagram::check(&ctx, &mut out);
        let out: Vec<_> = out.into_iter().filter(|f| f.rule == RULE).collect();
        assert_eq!(out.len(), 1, "{out:?}");
        let f = &out[0];
        assert_eq!(&src[f.range.clone()], "[Start --> B");
        assert_eq!(a.file.line_col(f.range.start), (7, 4));
    }
}
