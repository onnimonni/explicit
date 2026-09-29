//! Block quote rules: MD027, MD028 and GitHub alert syntax.

use std::sync::LazyLock;

use regex::Regex;

use super::Ctx;
use crate::rules::Out;

static ALERT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\[!([A-Za-z]+)\]([ \t]*)(.*)$").expect("hardcoded regex is valid")
});

/// Alert types GitHub renders (matched case-insensitively, like GitHub and pulldown-cmark).
const ALERTS: &[&str] = &["NOTE", "TIP", "IMPORTANT", "WARNING", "CAUTION"];

/// Common non-GitHub alert names and their closest GitHub type.
const ALIASES: &[(&str, &str)] = &[
    ("INFO", "NOTE"),
    ("INFORMATION", "NOTE"),
    ("HINT", "TIP"),
    ("SUCCESS", "TIP"),
    ("CHECK", "TIP"),
    ("ATTENTION", "WARNING"),
    ("WARN", "WARNING"),
    ("DANGER", "CAUTION"),
    ("ERROR", "CAUTION"),
    ("BUG", "CAUTION"),
    ("FAILURE", "CAUTION"),
];

pub fn check(c: &mut Ctx, out: &mut Out) {
    if c.on("md/no-multiple-space-blockquote") {
        multiple_space(c, out);
    }
    if c.on("md/no-blanks-blockquote") {
        blanks(c, out);
    }
    if c.on("md/alert-syntax") {
        alerts(c, out);
    }
}

fn multiple_space(c: &Ctx, out: &mut Out) {
    for (i, l) in c.lines.v.iter().enumerate() {
        if l.quote == 0 || l.blank || !c.text_line(i) || c.in_list(l.body) {
            continue;
        }
        // `quote_prefix` consumed one space after the last `>`.
        if l.body == l.start || c.src.as_bytes()[l.body - 1] != b' ' {
            continue;
        }
        let rest = &c.src[l.body..l.end];
        let n = rest.bytes().take_while(|&b| b == b' ').count();
        // 4+ extra spaces would be an indented code block.
        if n == 0 || n >= 4 {
            continue;
        }
        let r = l.body..l.body + n;
        out.push(
            c.finding(
                "md/no-multiple-space-blockquote",
                r.clone(),
                "Multiple spaces after block quote marker",
            )
            .fix(r, ""),
        );
    }
}

fn blanks(c: &Ctx, out: &mut Out) {
    let v = &c.lines.v;
    let mut i = 1;
    while i < v.len() {
        let empty = |k: usize| v[k].quote == 0 && v[k].blank && !v[k].code;
        if !empty(i) || v[i - 1].quote == 0 || v[i - 1].code {
            i += 1;
            continue;
        }
        let s = i;
        while i < v.len() && empty(i) {
            i += 1;
        }
        if i < v.len() && v[i].quote > 0 && !v[i].code {
            out.push(
                c.finding(
                    "md/no-blanks-blockquote",
                    v[s].start..v[i - 1].next,
                    "Blank line inside block quote splits it in two",
                )
                .help("Start the blank line with '>' or separate the quotes with text"),
            );
        }
    }
}

/// Closest GitHub alert type for an unknown one.
fn nearest(name: &str) -> &'static str {
    let up = name.to_ascii_uppercase();
    if let Some((_, to)) = ALIASES.iter().find(|(a, _)| *a == up) {
        return to;
    }
    ALERTS
        .iter()
        .find(|a| a.starts_with(&up[..up.len().min(2)]))
        .copied()
        .unwrap_or("NOTE")
}

fn alerts(c: &Ctx, out: &mut Out) {
    let v = &c.lines.v;
    for (i, l) in v.iter().enumerate() {
        if l.quote == 0 || l.code {
            continue;
        }
        let first = i == 0 || v[i - 1].quote < l.quote || (v[i - 1].blank && v[i - 1].quote == 0);
        if !first {
            continue;
        }
        let body = &c.src[l.body..l.end];
        let lead = body.len() - body.trim_start().len();
        let Some(m) = ALERT_RE.captures(&body[lead..]) else {
            continue;
        };
        let at = l.body + lead;
        let name = &m[1];
        let marker = at..at + 3 + name.len();
        if !ALERTS.iter().any(|a| a.eq_ignore_ascii_case(name)) {
            let want = nearest(name);
            out.push(
                c.finding(
                    "md/alert-syntax",
                    marker,
                    format!("Unknown GitHub alert type [!{name}]"),
                )
                .help("GitHub supports NOTE, TIP, IMPORTANT, WARNING and CAUTION")
                .suggest(format!("[!{want}]")),
            );
            continue;
        }
        let content = m.get(3).expect("regex group 3 is not optional");
        if content.as_str().trim().is_empty() {
            continue;
        }
        let sp = m.get(2).expect("regex group 2 is not optional");
        let prefix = c.src[l.start..l.body].trim_end();
        let r = at + sp.start()..at + content.start();
        out.push(
            c.finding(
                "md/alert-syntax",
                marker,
                format!("Alert text must start on the line after [!{name}]"),
            )
            .help("GitHub renders the marker as plain text otherwise")
            .fix(r, format!("{}{prefix} ", c.nl)),
        );
    }
}
