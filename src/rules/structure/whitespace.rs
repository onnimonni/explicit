//! Line-level whitespace rules: MD009, MD010, MD012, MD013, MD047.

use std::sync::LazyLock;

use regex::Regex;

use super::Ctx;
use crate::rules::Out;

static REF_DEF_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s{0,3}\[[^\]]+\]:\s").expect("hardcoded regex is valid"));

pub fn check(c: &mut Ctx, out: &mut Out) {
    if c.on("md/no-trailing-spaces") {
        trailing_spaces(c, out);
    }
    if c.on("md/no-hard-tabs") {
        hard_tabs(c, out);
    }
    if c.on("md/no-multiple-blanks") {
        multiple_blanks(c, out);
    }
    if c.on("md/line-length") {
        line_length(c, out);
    }
    if c.on("md/single-trailing-newline") {
        trailing_newline(c, out);
    }
}

fn trailing_spaces(c: &Ctx, out: &mut Out) {
    for (i, l) in c.lines.v.iter().enumerate() {
        if l.code {
            continue;
        }
        let t = &c.src[l.start..l.end];
        let trimmed = t.trim_end_matches([' ', '\t']);
        if trimmed.len() == t.len() {
            continue;
        }
        let ws = &t[trimmed.len()..];
        let blank = trimmed.trim().is_empty();
        let spaces_only = ws.bytes().all(|b| b == b' ');
        // Exactly two spaces is a hard line break.
        if !blank && ws == "  " {
            continue;
        }
        let next_text = c.lines.v.get(i + 1).is_some_and(|n| !n.blank && !n.code);
        let keep_break = !blank && spaces_only && ws.len() > 2 && next_text;
        let range = l.start + trimmed.len()..l.end;
        // Keeping a hard break: delete only the spaces beyond two.
        let fix_range = if keep_break {
            range.start + 2..range.end
        } else {
            range.clone()
        };
        let msg = if spaces_only {
            format!("{} trailing spaces", ws.len())
        } else {
            "Trailing whitespace".to_string()
        };
        let f = c
            .finding("md/no-trailing-spaces", range.clone(), msg)
            .fix(fix_range, "");
        out.push(if keep_break {
            f.help("Use exactly two spaces for a hard line break")
        } else {
            f
        });
    }
}

fn hard_tabs(c: &Ctx, out: &mut Out) {
    for l in &c.lines.v {
        if l.code {
            continue;
        }
        let t = &c.src[l.start..l.end];
        let Some(pos) = t
            .char_indices()
            .map(|(i, _)| i)
            .find(|&i| t.as_bytes()[i] == b'\t' && !c.md.in_code(l.start + i))
        else {
            continue;
        };
        let run = t[pos..].bytes().take_while(|&b| b == b'\t').count();
        let s = l.start + pos;
        out.push(
            c.finding("md/no-hard-tabs", s..s + run, "Hard tab")
                .help("Use spaces for indentation and alignment"),
        );
    }
}

fn multiple_blanks(c: &Ctx, out: &mut Out) {
    let v = &c.lines.v;
    let is_blank = |i: usize| !v[i].code && c.src[v[i].start..v[i].end].trim().is_empty();
    let mut i = 0;
    while i < v.len() {
        if !is_blank(i) {
            i += 1;
            continue;
        }
        let start = i;
        while i < v.len() && is_blank(i) {
            i += 1;
        }
        let count = i - start;
        // Trailing blank lines at EOF belong to md/single-trailing-newline.
        if count < 2 || i == v.len() {
            continue;
        }
        let range = v[start + 1].start..v[i - 1].next;
        out.push(
            c.finding(
                "md/no-multiple-blanks",
                range.clone(),
                format!("{count} consecutive blank lines"),
            )
            .fix(range, ""),
        );
    }
}

fn line_length(c: &Ctx, out: &mut Out) {
    let limit = c.fc.config.markdown.line_length;
    if limit == 0 {
        return;
    }
    for l in &c.lines.v {
        if l.code || l.table {
            continue;
        }
        let t = &c.src[l.start..l.end];
        let n = t.chars().count();
        if n <= limit || REF_DEF_RE.is_match(t) {
            continue;
        }
        let off = t.char_indices().nth(limit).map_or(t.len(), |(i, _)| i);
        // Allow long unbreakable tails such as URLs.
        if !t[off..].contains([' ', '\t']) {
            continue;
        }
        out.push(c.finding(
            "md/line-length",
            l.start + off..l.end,
            format!("Line is {n} characters, limit is {limit}"),
        ));
    }
}

fn trailing_newline(c: &Ctx, out: &mut Out) {
    let src = c.src;
    let Some(last) = c
        .lines
        .v
        .iter()
        .rposition(|l| !src[l.start..l.end].trim().is_empty())
    else {
        return;
    };
    let from = c.lines.v[last].end;
    let tail = &src[from..];
    if tail == "\n" || tail == "\r\n" {
        return;
    }
    if tail.is_empty() {
        let e = src.len();
        out.push(
            c.finding(
                "md/single-trailing-newline",
                e..e,
                "File does not end with a newline",
            )
            .fix(e..e, c.nl),
        );
    } else {
        let nl = if tail.starts_with("\r\n") {
            "\r\n"
        } else {
            "\n"
        };
        out.push(
            c.finding(
                "md/single-trailing-newline",
                from..src.len(),
                "File ends with extra blank lines",
            )
            .fix(from..src.len(), nl),
        );
    }
}
