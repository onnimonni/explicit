//! Light Graphviz DOT validation: header, balanced delimiters and edge operators.

use super::point;
use crate::diagnostic::{Finding, Severity};
use crate::rules::Out;

const RULE: &str = "codeblock/dot";

pub fn check(body: &str, offset: usize, out: &mut Out) {
    let mut push = |pos: usize, msg: String| {
        out.push(
            Finding::new(RULE, Severity::Warning, point(body, offset, pos), msg)
                .help("Fix the DOT syntax, or add `skip-lint` to the info string"),
        );
    };

    let b = body.as_bytes();
    // Header: optional `strict`, then `graph` or `digraph`.
    let mut i = skip_trivia(body, 0);
    let mut word = read_word(body, i);
    if word.eq_ignore_ascii_case("strict") {
        i = skip_trivia(body, i + word.len());
        word = read_word(body, i);
    }
    let directed = if word.eq_ignore_ascii_case("digraph") {
        true
    } else if word.eq_ignore_ascii_case("graph") {
        false
    } else {
        push(i, "DOT graph must start with `graph` or `digraph`".into());
        return;
    };

    let mut stack: Vec<(u8, usize)> = Vec::new();
    let mut j = i + word.len();
    while j < b.len() {
        let c = b[j];
        match c {
            b'"' => {
                let start = j;
                j += 1;
                while j < b.len() && b[j] != b'"' {
                    j += if b[j] == b'\\' { 2 } else { 1 };
                }
                if j >= b.len() {
                    push(start, "Unterminated string".into());
                    return;
                }
                j += 1;
                continue;
            }
            b'/' if b.get(j + 1) == Some(&b'/') => {
                j = body[j..].find('\n').map_or(b.len(), |e| j + e);
                continue;
            }
            b'#' if body[..j]
                .rsplit('\n')
                .next()
                .is_some_and(|l| l.trim().is_empty()) =>
            {
                j = body[j..].find('\n').map_or(b.len(), |e| j + e);
                continue;
            }
            b'/' if b.get(j + 1) == Some(&b'*') => {
                match body[j + 2..].find("*/") {
                    Some(e) => j = j + 2 + e + 2,
                    None => {
                        push(j, "Unterminated comment".into());
                        return;
                    }
                }
                continue;
            }
            // HTML-like label: `label=<...>` with nested angle brackets.
            b'<' if prev_significant(b, j) == Some(b'=') => {
                let start = j;
                let mut depth = 0usize;
                while j < b.len() {
                    match b[j] {
                        b'<' => depth += 1,
                        b'>' => {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                        _ => {}
                    }
                    j += 1;
                }
                if j >= b.len() {
                    push(start, "Unterminated HTML label".into());
                    return;
                }
                j += 1;
                continue;
            }
            b'{' | b'[' => stack.push((c, j)),
            b'}' | b']' => {
                let open = if c == b'}' { b'{' } else { b'[' };
                match stack.pop() {
                    Some((o, _)) if o == open => {}
                    Some((o, _)) => {
                        let close = if o == b'{' { '}' } else { ']' };
                        push(j, format!("Mismatched `{}`, expected `{close}`", c as char));
                        return;
                    }
                    None => {
                        push(j, format!("Unmatched `{}`", c as char));
                        return;
                    }
                }
            }
            b'-' if b.get(j + 1) == Some(&b'>') && !directed => {
                push(
                    j,
                    "`->` edge in an undirected `graph`; use `--` or `digraph`".into(),
                );
                j += 2;
                continue;
            }
            b'-' if b.get(j + 1) == Some(&b'-') && directed => {
                push(j, "`--` edge in a `digraph`; use `->` or `graph`".into());
                j += 2;
                continue;
            }
            _ => {}
        }
        j += 1;
    }
    if let Some((o, p)) = stack.pop() {
        push(p, format!("Unclosed `{}`", o as char));
    }
}

fn prev_significant(b: &[u8], j: usize) -> Option<u8> {
    b[..j]
        .iter()
        .rev()
        .copied()
        .find(|c| !c.is_ascii_whitespace())
}

fn skip_trivia(s: &str, mut i: usize) -> usize {
    loop {
        let rest = &s[i..];
        let t = rest.trim_start();
        i += rest.len() - t.len();
        if t.starts_with("//") || t.starts_with('#') {
            i += t.find('\n').unwrap_or(t.len());
        } else if let Some(r) = t.strip_prefix("/*") {
            i += r.find("*/").map_or(t.len(), |e| e + 4);
        } else {
            return i;
        }
    }
}

fn read_word(s: &str, i: usize) -> &str {
    let rest = &s[i..];
    let end = rest
        .find(|c: char| !(c.is_alphanumeric() || c == '_'))
        .unwrap_or(rest.len());
    &rest[..end]
}
