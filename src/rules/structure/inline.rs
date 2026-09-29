//! Inline rules: MD034, MD036, MD038, MD042, MD045.

use std::sync::LazyLock;

use regex::Regex;

use super::Ctx;
use crate::extract::markdown::{LinkKind, bare_url_truncated};
use crate::rules::Out;

static IMG_TAG_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?is)<img\b[^>]*>").expect("hardcoded regex is valid"));
static ALT_ATTR_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\salt\s*=").expect("hardcoded regex is valid"));

const END_PUNCT: &str = ".,;:!?。，；：！？";

pub fn check(c: &mut Ctx, out: &mut Out) {
    let md = c.md;
    if c.on("md/no-bare-urls") {
        for l in md.links.iter().filter(|l| l.kind == LinkKind::Bare) {
            let raw = &c.src[l.range.clone()];
            if !(raw.starts_with("http://") || raw.starts_with("https://"))
                || md.in_code(l.range.start)
            {
                continue;
            }
            let f = c.finding(
                "md/no-bare-urls",
                l.range.clone(),
                format!("Bare URL {raw}"),
            );
            // Never wrap a prefix of a URL that continues past markdown syntax.
            out.push(if bare_url_truncated(c.src, &l.range) {
                f
            } else {
                f.fix(l.range.clone(), format!("<{raw}>"))
            });
        }
    }

    if c.on("md/no-empty-links") {
        for l in &md.links {
            if l.is_image || !matches!(l.kind, LinkKind::Inline | LinkKind::Reference) {
                continue;
            }
            let d = l.dest.trim();
            if d.is_empty() || d == "#" {
                out.push(c.finding(
                    "md/no-empty-links",
                    l.range.clone(),
                    "Link has no destination",
                ));
            }
        }
    }

    if c.on("md/image-alt") {
        for l in &md.links {
            if l.is_image && l.kind != LinkKind::Html && l.text.trim().is_empty() {
                out.push(c.finding("md/image-alt", l.range.clone(), "Image has no alt text"));
            }
        }
        for h in &md.html {
            for m in IMG_TAG_RE.find_iter(&c.src[h.clone()]) {
                if !ALT_ATTR_RE.is_match(m.as_str()) {
                    let r = h.start + m.start()..h.start + m.end();
                    out.push(c.finding("md/image-alt", r, "Image has no alt attribute"));
                }
            }
        }
    }

    if c.on("md/no-space-in-code") {
        code_spaces(c, out);
    }

    if c.on("md/no-emphasis-as-heading") {
        emphasis_heading(c, out);
    }
}

fn code_spaces(c: &Ctx, out: &mut Out) {
    for sp in &c.md.code_spans {
        let raw = &c.src[sp.clone()];
        let n = raw.bytes().take_while(|&b| b == b'`').count();
        if n == 0 || raw.len() < 2 * n || !raw[raw.len() - n..].bytes().all(|b| b == b'`') {
            continue;
        }
        let inner = &raw[n..raw.len() - n];
        let is_sp = |b: u8| b == b' ' || b == b'\t';
        if inner.bytes().all(is_sp) {
            continue;
        }
        let lead = inner.bytes().take_while(|&b| is_sp(b)).count();
        let trail = inner.bytes().rev().take_while(|&b| is_sp(b)).count();
        if lead == 0 && trail == 0 {
            continue;
        }
        let core = inner.trim_matches([' ', '\t']);
        // CommonMark strips one space on each side; needed around content that starts/ends with a backtick.
        if lead == 1 && trail == 1 && (core.starts_with('`') || core.ends_with('`')) {
            continue;
        }
        let f = c.finding("md/no-space-in-code", sp.clone(), "Spaces inside code span");
        let s = sp.start + n;
        let e = sp.end - n;
        out.push(if core.starts_with('`') || core.ends_with('`') {
            f
        } else {
            f.fix(s..e, core)
        });
    }
}

fn emphasis_heading(c: &Ctx, out: &mut Out) {
    let v = &c.lines.v;
    for (i, l) in v.iter().enumerate() {
        if l.code || l.table || l.blank || l.quote > 0 || c.in_list(l.start) {
            continue;
        }
        let before_ok = i == 0 || v[i - 1].blank || c.lines.fm_end == Some(i - 1);
        let after_ok = v.get(i + 1).is_none_or(|n| n.blank);
        if !before_ok || !after_ok {
            continue;
        }
        let raw = &c.src[l.start..l.end];
        let t = raw.trim();
        let Some(inner) = ["**", "__", "*", "_"].iter().find_map(|m| {
            let inner = t.strip_prefix(m)?.strip_suffix(m)?;
            let ch = m.chars().next()?;
            let ok = !inner.is_empty() && !inner.contains(ch) && inner.trim() == inner;
            ok.then_some(inner)
        }) else {
            continue;
        };
        if inner
            .chars()
            .last()
            .is_some_and(|ch| END_PUNCT.contains(ch))
        {
            continue;
        }
        let s = l.start + (raw.len() - raw.trim_start().len());
        out.push(
            c.finding(
                "md/no-emphasis-as-heading",
                s..s + t.len(),
                "Emphasis used instead of a heading",
            )
            .help("Use a heading (e.g. `## Title`)"),
        );
    }
}
