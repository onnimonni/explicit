//! Code block rules: MD031, MD040, MD046, known fence languages.

use super::{Ctx, languages};
use crate::rules::Out;

pub fn check(c: &mut Ctx, out: &mut Out) {
    let md = c.md;
    let fences = c.on("md/blanks-around-fences");
    let lang = c.on("md/fenced-code-language");
    let style = c.on("md/code-block-style");
    let known = c.on("md/code-language-known");
    if !(fences || lang || style || known) {
        return;
    }
    for cb in &md.code_blocks {
        if cb.range.is_empty() {
            continue;
        }
        let first = c.lines.index_of(cb.range.start);
        let fl = &c.lines.v[first];
        let open = fl.body.max(cb.range.start)..fl.end;
        if !cb.fenced {
            if style {
                out.push(
                    c.finding("md/code-block-style", open, "Indented code block")
                        .help("Use a fenced code block with a language"),
                );
            }
            continue;
        }
        if lang && cb.lang.is_none() {
            out.push(
                c.finding(
                    "md/fenced-code-language",
                    open.clone(),
                    "Fenced code block has no language",
                )
                .help("Add a language after the opening fence, e.g. ```text"),
            );
        }
        if let (true, Some(l)) = (known, cb.lang.as_deref())
            && !languages::is_known(l, &c.fc.config.markdown.allowed_languages)
        {
            let at = c.src[open.clone()]
                .find(l)
                .map_or(open.clone(), |i| open.start + i..open.start + i + l.len());
            let mut f = c
                .finding("md/code-language-known", at, format!("Unknown code block language '{l}'"))
                .help("Use a language identifier GitHub Linguist recognizes, or add it to markdown.allowed_languages");
            for s in languages::suggest(l) {
                f = f.suggest(s);
            }
            out.push(f);
        }
        if !fences {
            continue;
        }
        if c.missing_blank_before(first) {
            let f = c.finding(
                "md/blanks-around-fences",
                open.clone(),
                "Fenced code block should be preceded by a blank line",
            );
            let at = c.lines.v[first].start;
            let f = c.with_blank_line(f, at, first);
            out.push(f);
        }
        let last = c.lines.index_of(cb.range.end - 1);
        let ll = &c.lines.v[last];
        let closed = last > first && {
            let t = c.src[ll.body..ll.end].trim_start();
            t.starts_with("```") || t.starts_with("~~~")
        };
        if closed && c.missing_blank_after(last) {
            let at = c.lines.v[last + 1].start;
            let f = c.finding(
                "md/blanks-around-fences",
                c.eol(last),
                "Fenced code block should be followed by a blank line",
            );
            let f = c.with_blank_line(f, at, last);
            out.push(f);
        }
    }
}
