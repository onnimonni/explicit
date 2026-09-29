//! Markdown structure rules (markdownlint equivalents).

mod atx;
mod blocks;
mod consistency;
mod headings;
mod inline;
mod languages;
mod lines;
mod lists;
mod misc;
mod quotes;
mod refs;
mod spans;
mod tables;
mod whitespace;

#[cfg(test)]
mod tests;

use std::collections::HashSet;
use std::ops::Range;

use super::{FileCtx, Out, RULES};
use crate::diagnostic::{Finding, Severity};
use crate::extract::markdown::MdDoc;
use lines::Lines;

pub fn check(ctx: &FileCtx, out: &mut Out) {
    let Some(md) = ctx.a.md.as_ref() else { return };
    if !ctx.family_enabled("md/") && !ctx.family_enabled("links/") {
        return;
    }
    let mut c = Ctx::new(ctx, md);
    whitespace::check(&mut c, out);
    headings::check(&mut c, out);
    atx::check(&mut c, out);
    lists::check(&mut c, out);
    blocks::check(&mut c, out);
    tables::check(&mut c, out);
    quotes::check(&mut c, out);
    inline::check(&mut c, out);
    spans::check(&mut c, out);
    consistency::check(&mut c, out);
    refs::check(&mut c, out);
    misc::check(&mut c, out);
}

/// Shared state for the structure rules of one file.
pub(crate) struct Ctx<'a> {
    pub fc: &'a FileCtx<'a>,
    pub src: &'a str,
    pub md: &'a MdDoc,
    pub lines: Lines,
    /// Line ending used by the file.
    pub nl: &'static str,
    /// Offsets where a blank-line insertion fix was already attached.
    inserted: HashSet<usize>,
}

impl<'a> Ctx<'a> {
    fn new(fc: &'a FileCtx<'a>, md: &'a MdDoc) -> Self {
        let src = fc.src();
        let nl = if src.contains("\r\n") { "\r\n" } else { "\n" };
        Ctx {
            fc,
            src,
            md,
            lines: Lines::new(src, md),
            nl,
            inserted: HashSet::new(),
        }
    }

    pub fn on(&self, rule: &str) -> bool {
        self.fc.enabled(rule)
    }

    /// A finding with the rule's default severity (warning for rules off by default).
    pub fn finding(&self, rule: &str, range: Range<usize>, msg: impl Into<String>) -> Finding {
        let sev = RULES
            .iter()
            .find(|r| r.id == rule)
            .and_then(|r| r.default)
            .unwrap_or(Severity::Warning);
        Finding::new(rule, sev, range, msg)
    }

    /// Attach a fix inserting a blank line at `at` (start of a line), once per offset.
    pub fn with_blank_line(&mut self, f: Finding, at: usize, line: usize) -> Finding {
        if !self.inserted.insert(at) {
            return f;
        }
        let l = &self.lines.v[line];
        let prefix = self.src[l.start..l.body].trim_end();
        let text = format!("{prefix}{}", self.nl);
        f.fix(at..at, text)
    }

    pub fn in_list(&self, offset: usize) -> bool {
        self.md.lists.iter().any(|l| l.range.contains(&offset))
    }

    /// Whether line `i` should be preceded by a blank line but is not.
    pub fn missing_blank_before(&self, i: usize) -> bool {
        if i == 0 || self.lines.fm_end == Some(i - 1) {
            return false;
        }
        let (p, l) = (&self.lines.v[i - 1], &self.lines.v[i]);
        !p.blank && p.quote >= l.quote
    }

    /// Whether line `i` should be followed by a blank line but is not.
    pub fn missing_blank_after(&self, i: usize) -> bool {
        let Some(n) = self.lines.v.get(i + 1) else {
            return false;
        };
        !n.blank && n.quote >= self.lines.v[i].quote
    }

    /// Line `i` from its body start with code spans and raw HTML blanked to spaces (same byte offsets).
    pub fn masked(&self, i: usize) -> String {
        let l = &self.lines.v[i];
        let r = l.body..l.end;
        let mut b = self.src.as_bytes()[r.clone()].to_vec();
        for sp in self.md.code_spans.iter().chain(&self.md.html) {
            let (s, e) = (sp.start.max(r.start), sp.end.min(r.end));
            if s < e {
                b[s - r.start..e - r.start].fill(b' ');
            }
        }
        String::from_utf8(b).unwrap_or_default()
    }

    /// Whether line `i` holds normal Markdown text (not code, front matter or an HTML block).
    pub fn text_line(&self, i: usize) -> bool {
        let l = &self.lines.v[i];
        !l.code && !l.html
    }

    /// Range of the newline ending line `i` (used for "after" findings).
    pub fn eol(&self, i: usize) -> Range<usize> {
        let l = &self.lines.v[i];
        l.end..l.next.max(l.end)
    }
}
