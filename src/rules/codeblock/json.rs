//! ```json (strict) and ```jsonc (comments and trailing commas allowed).

use serde::de::IgnoredAny;

use super::{has_placeholder, line_col_to_byte, point};
use crate::diagnostic::{Finding, Severity};
use crate::rules::Out;

const RULE: &str = "codeblock/json";

pub fn check(body: &str, offset: usize, jsonc: bool, out: &mut Out) {
    if has_placeholder(body, false) {
        return;
    }
    let cleaned;
    let text = if jsonc {
        cleaned = strip_jsonc(body);
        cleaned.as_str()
    } else {
        body
    };
    let Err(e) = serde_json::from_str::<IgnoredAny>(text) else {
        return;
    };
    let pos = if e.line() == 0 {
        0
    } else {
        line_col_to_byte(text, e.line(), e.column())
    };
    let msg = e.to_string();
    let msg = msg.split(" at line ").next().unwrap_or(&msg);
    let lang = if jsonc { "JSONC" } else { "JSON" };
    let mut f = Finding::new(
        RULE,
        Severity::Error,
        point(body, offset, pos),
        format!("Invalid {lang}: {msg}"),
    );
    if !jsonc && (body.contains("//") || body.contains("/*")) {
        f = f.help("Use ```jsonc for JSON with comments, or add `skip-lint` to the info string");
    } else {
        f = f.help("Fix the syntax, or add `skip-lint` to the info string");
    }
    out.push(f);
}

/// Blank out `//` and `/* */` comments and trailing commas, keeping byte offsets.
pub(crate) fn strip_jsonc(src: &str) -> String {
    let b = src.as_bytes();
    let mut out = b.to_vec();
    let mut i = 0;
    let mut in_str = false;
    // Last significant (non-space, non-comment) byte index outside strings.
    let mut last_comma: Option<usize> = None;
    while i < b.len() {
        let c = b[i];
        if in_str {
            if c == b'\\' {
                i += 2;
                continue;
            }
            if c == b'"' {
                in_str = false;
            }
            i += 1;
            continue;
        }
        match c {
            b'"' => {
                in_str = true;
                last_comma = None;
                i += 1;
            }
            b'/' if b.get(i + 1) == Some(&b'/') => {
                while i < b.len() && b[i] != b'\n' {
                    if b[i] != b'\r' {
                        out[i] = b' ';
                    }
                    i += 1;
                }
            }
            b'/' if b.get(i + 1) == Some(&b'*') => {
                let end = src[i + 2..].find("*/").map_or(b.len(), |e| i + 2 + e + 2);
                for (j, o) in out.iter_mut().enumerate().take(end).skip(i) {
                    if !matches!(b[j], b'\n' | b'\r') {
                        *o = b' ';
                    }
                }
                i = end;
            }
            b',' => {
                last_comma = Some(i);
                i += 1;
            }
            b'}' | b']' => {
                if let Some(ci) = last_comma.take() {
                    out[ci] = b' ';
                }
                i += 1;
            }
            c if c.is_ascii_whitespace() => i += 1,
            _ => {
                last_comma = None;
                i += 1;
            }
        }
    }
    // Only whole characters (ASCII comment bytes or full UTF-8 runs) were replaced.
    String::from_utf8(out).unwrap_or_else(|_| src.to_string())
}
