//! Consistent marker styles: MD035 (thematic breaks), MD048 (code fences), MD049/MD050 (emphasis).

use std::ops::Range;

use super::Ctx;
use crate::rules::Out;

pub fn check(c: &mut Ctx, out: &mut Out) {
    if c.on("md/hr-style") {
        hr_style(c, out);
    }
    if c.on("md/code-fence-style") {
        fence_style(c, out);
    }
    if c.on("md/emphasis-style") {
        let cfg = c.fc.config.markdown.emphasis_style.clone();
        emphasis_style(c, out, "md/emphasis-style", &c.md.emphasis, 1, &cfg);
    }
    if c.on("md/strong-style") {
        let cfg = c.fc.config.markdown.strong_style.clone();
        emphasis_style(c, out, "md/strong-style", &c.md.strong, 2, &cfg);
    }
}

fn hr_style(c: &Ctx, out: &mut Out) {
    let cfg = c.fc.config.markdown.hr_style.as_str();
    let mut want: Option<String> = (cfg != "consistent").then(|| cfg.to_string());
    for t in &c.md.thematic_breaks {
        let i = c.lines.index_of(t.start);
        let l = &c.lines.v[i];
        let s = t.start.max(l.body);
        let raw = &c.src[s..l.end];
        let text = raw.trim();
        let start = s + (raw.len() - raw.trim_start().len());
        let r = start..start + text.len();
        let w = want.get_or_insert_with(|| text.to_string());
        if text != w {
            out.push(
                c.finding(
                    "md/hr-style",
                    r.clone(),
                    format!("Thematic break '{text}' should be '{w}'"),
                )
                .fix(r, w.clone()),
            );
        }
    }
}

/// The run of fence characters starting line `i` (after the block quote prefix and indentation).
fn fence_run(c: &Ctx, i: usize) -> Option<(Range<usize>, u8)> {
    let l = &c.lines.v[i];
    let raw = &c.src[l.body..l.end];
    let lead = raw.len() - raw.trim_start_matches(' ').len();
    let ch = *raw.as_bytes().get(lead)?;
    if ch != b'`' && ch != b'~' {
        return None;
    }
    let n = raw[lead..].bytes().take_while(|&b| b == ch).count();
    (n >= 3).then(|| (l.body + lead..l.body + lead + n, ch))
}

fn fence_style(c: &Ctx, out: &mut Out) {
    let mut want: Option<u8> = match c.fc.config.markdown.code_fence_style.as_str() {
        "backtick" => Some(b'`'),
        "tilde" => Some(b'~'),
        _ => None,
    };
    for cb in
        c.md.code_blocks
            .iter()
            .filter(|b| b.fenced && !b.range.is_empty())
    {
        let first = c.lines.index_of(cb.range.start);
        let Some((open, ch)) = fence_run(c, first) else {
            continue;
        };
        let w = *want.get_or_insert(ch);
        if ch == w {
            continue;
        }
        let last = c.lines.index_of(cb.range.end - 1);
        let close = (last > first)
            .then(|| fence_run(c, last))
            .flatten()
            .filter(|(r, x)| *x == ch && r.len() >= open.len());
        let inner_end = close.as_ref().map_or(c.lines.v[last].end, |(r, _)| r.start);
        let content = &c.src[open.end..inner_end];
        // Converting is unsafe when the new fence character appears in the info string or body.
        let fence3 = (w as char).to_string().repeat(3);
        let info = content.lines().next().unwrap_or("");
        let safe = !(w == b'`' && info.contains('`'))
            && !content
                .lines()
                .skip(1)
                .any(|ln| ln.trim_start().starts_with(&fence3));
        let (got, exp) = (
            if ch == b'`' { "```" } else { "~~~" },
            if w == b'`' { "```" } else { "~~~" },
        );
        let mut f = c.finding(
            "md/code-fence-style",
            open.clone(),
            format!("Code fence {got} should be {exp}"),
        );
        if safe && let Some((cr, _)) = close {
            let rep = |n: usize| (w as char).to_string().repeat(n);
            let text = format!("{}{}{}", rep(open.len()), content, rep(cr.len()));
            f = f.fix(open.start..cr.end, text);
        }
        out.push(f);
    }
}

fn emphasis_style(c: &Ctx, out: &mut Out, rule: &str, spans: &[Range<usize>], n: usize, cfg: &str) {
    let mut want: Option<u8> = match cfg {
        "asterisk" => Some(b'*'),
        "underscore" => Some(b'_'),
        _ => None,
    };
    let b = c.src.as_bytes();
    let mut spans: Vec<&Range<usize>> = spans.iter().collect();
    spans.sort_by_key(|r| r.start);
    for r in spans {
        if r.len() < 2 * n + 1 {
            continue;
        }
        let ch = b[r.start];
        if (ch != b'*' && ch != b'_') || b[r.end - 1] != ch {
            continue;
        }
        let w = *want.get_or_insert(ch);
        if ch == w {
            continue;
        }
        let name = |x: u8| {
            if x == b'*' {
                "asterisks"
            } else {
                "underscores"
            }
        };
        let mut f = c.finding(
            rule,
            r.clone(),
            format!(
                "Use {} instead of {} for {}",
                name(w),
                name(ch),
                if n == 1 {
                    "emphasis"
                } else {
                    "strong emphasis"
                }
            ),
        );
        // Underscores do not work inside words.
        let intraword = r.start > 0 && b[r.start - 1].is_ascii_alphanumeric()
            || b.get(r.end).is_some_and(u8::is_ascii_alphanumeric);
        if !(w == b'_' && intraword) {
            let m = (w as char).to_string().repeat(n);
            let inner = &c.src[r.start + n..r.end - n];
            f = f.fix(r.clone(), format!("{m}{inner}{m}"));
        }
        out.push(f);
    }
}
