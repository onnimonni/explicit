//! Definitions and anchors: duplicate reference definitions, footnotes, duplicate heading ids.

use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use regex::Regex;

use super::Ctx;
use crate::extract::markdown::normalize_label;
use crate::rules::Out;

static DEF_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^ {0,3}\[((?:[^\]\\]|\\.)+)\]:").expect("hardcoded regex is valid")
});
static FOOTNOTE_REF_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[\^([^\]\s]+)\]").expect("hardcoded regex is valid"));

pub fn check(c: &mut Ctx, out: &mut Out) {
    if c.on("md/no-duplicate-definition") {
        duplicate_defs(c, out);
    }
    if c.on("links/undefined-footnote") {
        undefined_footnotes(c, out);
    }
    if c.on("links/unused-footnote") {
        let used: HashSet<&str> = c.md.footnote_refs.iter().map(|(l, _)| l.as_str()).collect();
        for (label, r) in &c.md.footnote_defs {
            if !used.contains(label.as_str()) {
                let i = c.lines.index_of(r.start);
                let l = &c.lines.v[i];
                out.push(c.finding(
                    "links/unused-footnote",
                    r.start..l.end.max(r.start),
                    format!("Footnote [^{label}] is never referenced"),
                ));
            }
        }
    }
    if c.on("links/duplicate-anchor") {
        duplicate_anchors(c, out);
    }
}

fn duplicate_defs(c: &Ctx, out: &mut Out) {
    let known: HashSet<String> =
        c.md.ref_defs
            .iter()
            .map(|d| d.label.clone())
            .chain(c.md.footnote_defs.iter().map(|(l, _)| format!("^{l}")))
            .collect();
    let mut seen: HashMap<String, usize> = HashMap::new();
    let mut prev_def = false;
    for i in 0..c.lines.len() {
        let l = &c.lines.v[i];
        let may_start =
            i == 0 || c.lines.v[i - 1].blank || prev_def || c.lines.fm_end == Some(i - 1);
        prev_def = false;
        if !c.text_line(i) || !may_start {
            continue;
        }
        let body = &c.src[l.body..l.end];
        let Some(m) = DEF_RE.captures(body) else {
            continue;
        };
        let label = normalize_label(&m[1]);
        if !known.contains(&label) {
            continue;
        }
        prev_def = true;
        // `[//]: # (comment)` is a common comment idiom, repeated on purpose.
        let dest = body[m.get(0).expect("group 0 is the whole match").end()..].trim_start();
        if dest == "#" || dest.starts_with("# ") || dest.starts_with("#(") || dest.starts_with("<>")
        {
            continue;
        }
        if let Some(&first) = seen.get(&label) {
            out.push(
                c.finding(
                    "md/no-duplicate-definition",
                    l.body..l.end,
                    format!(
                        "Duplicate definition [{}] (first on line {})",
                        &m[1],
                        first + 1
                    ),
                )
                .help("Only the first definition is used; remove or rename this one"),
            );
        } else {
            seen.insert(label, i);
        }
    }
}

fn undefined_footnotes(c: &Ctx, out: &mut Out) {
    let defs: HashSet<&str> = c.md.footnote_defs.iter().map(|(l, _)| l.as_str()).collect();
    for i in 0..c.lines.len() {
        if !c.text_line(i) {
            continue;
        }
        let body = c.lines.v[i].body;
        let m = c.masked(i);
        for cap in FOOTNOTE_REF_RE.captures_iter(&m) {
            let all = cap.get(0).expect("regex group 0 is not optional");
            // A definition line `[^x]: ...`.
            if m[..all.start()].trim().is_empty() && m[all.end()..].starts_with(':') {
                continue;
            }
            if all.start() > 0 && m.as_bytes()[all.start() - 1] == b'\\' {
                continue;
            }
            let label = normalize_label(&cap[1]);
            if !defs.contains(label.as_str()) {
                out.push(c.finding(
                    "links/undefined-footnote",
                    body + all.start()..body + all.end(),
                    format!("Footnote [^{}] is not defined", &cap[1]),
                ));
            }
        }
    }
}

fn duplicate_anchors(c: &Ctx, out: &mut Out) {
    let md = c.md;
    let mut explicit: HashMap<&str, usize> = HashMap::new();
    for (i, h) in md.headings.iter().enumerate() {
        if !h.explicit_id {
            continue;
        }
        let a = h.anchor.as_str();
        let clash = if let Some(&j) = explicit.get(a) {
            let (line, _) = c.fc.a.file.line_col(md.headings[j].range.start);
            Some(format!("another heading on line {line}"))
        } else if let Some(o) = md.headings.iter().find(|o| !o.explicit_id && o.anchor == a) {
            let (line, _) = c.fc.a.file.line_col(o.range.start);
            Some(format!("the generated id of the heading on line {line}"))
        } else if md.html_anchors.iter().any(|x| x == a) {
            Some("an HTML id".to_string())
        } else {
            None
        };
        explicit.entry(a).or_insert(i);
        if let Some(what) = clash {
            out.push(c.finding(
                "links/duplicate-anchor",
                h.range.clone(),
                format!("Heading id #{a} duplicates {what}"),
            ));
        }
    }
}
