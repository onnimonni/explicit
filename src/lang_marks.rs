//! Explicit language markers for mixed documents. A marked region is checked in its language
//! whatever detection says:
//!
//! - HTML `lang` attributes in Markdown: `<span lang="fi">…</span>`, `<div lang="sv">`,
//!   `<p lang=fi>` up to the matching closing tag;
//! - block markers: `<!-- explicit-lang fi -->` in Markdown, `// explicit-lang fi` (any comment
//!   syntax) in code, up to `explicit-lang end`, the next marker or the end of the file;
//! - gettext translations, in the catalog's language (see [`crate::rules::gettext`]).
//!
//! Nested markers: the innermost wins.

use std::ops::Range;
use std::sync::LazyLock;

use regex::Regex;

use crate::rules::Analyzed;

/// A marked byte range and its primary language subtag (`fi`, `sv`, `en`, or another code).
pub type Region = (Range<usize>, String);

/// `explicit-lang <tag>` at the start of a comment.
static MARKER_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*explicit-lang\s+([A-Za-z][\w-]*)").expect("hardcoded regex is valid")
});

/// An HTML start or end tag: `/`, name, attributes, self-closing `/`.
static TAG_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"<(/?)([A-Za-z][A-Za-z0-9-]*)((?:\s[^<>]*?)?)\s*(/?)>")
        .expect("hardcoded regex is valid")
});

/// A `lang` / `xml:lang` attribute value.
static LANG_ATTR_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)(?:^|\s)(?:xml:)?lang\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s"'=<>`]+))"#)
        .expect("hardcoded regex is valid")
});

/// Elements without an end tag.
const VOID: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source", "track",
    "wbr",
];

/// Marked regions of `a` as sorted, non-overlapping file ranges.
pub fn regions(a: &Analyzed) -> Vec<Region> {
    let src = a.file.text.as_str();
    let mut raw = Vec::new();
    if let Some(md) = &a.md {
        markdown(src, &md.html, &mut raw);
    } else {
        comments(a, &mut raw);
        if let Some(po) = &a.po {
            raw.extend(crate::rules::gettext::translation_regions(&a.file, po));
        }
    }
    flatten(raw)
}

/// The language a marker names: `end` closes the open region (`None`).
fn marker_lang(tag: &str) -> Option<String> {
    (!tag.eq_ignore_ascii_case("end")).then(|| crate::rules::spell_lang::primary(tag))
}

/// Block markers apply from the end of the marker to the start of the next one.
#[derive(Default)]
struct Blocks {
    open: Option<(usize, String)>,
}

impl Blocks {
    fn marker(&mut self, tag: &str, at: Range<usize>, out: &mut Vec<Region>) {
        if let Some((start, l)) = self.open.take() {
            out.push((start..at.start, l));
        }
        self.open = marker_lang(tag).map(|l| (at.end, l));
    }

    fn finish(self, end: usize, out: &mut Vec<Region>) {
        if let Some((start, l)) = self.open {
            out.push((start..end, l));
        }
    }
}

/// Markers in a Markdown file's raw HTML: comments with `explicit-lang` and elements with a
/// `lang` attribute (from the end of the start tag to the start of the matching end tag).
fn markdown(src: &str, html: &[Range<usize>], out: &mut Vec<Region>) {
    let mut blocks = Blocks::default();
    // Open elements: lowercase name, language, content start.
    let mut stack: Vec<(String, Option<String>, usize)> = Vec::new();
    let mut chunks: Vec<&Range<usize>> = html.iter().collect();
    chunks.sort_by_key(|r| r.start);
    for r in chunks {
        let Some(raw) = src.get(r.clone()) else {
            continue;
        };
        let mut pos = 0;
        while pos < raw.len() {
            let comment = raw[pos..].find("<!--").map(|i| pos + i);
            let tag = TAG_RE.captures_at(raw, pos);
            let tag_start = tag
                .as_ref()
                .and_then(|c| c.get(0))
                .map_or(usize::MAX, |m| m.start());
            match comment {
                Some(c) if c < tag_start => {
                    let body_start = c + 4;
                    let end = raw[body_start..]
                        .find("-->")
                        .map_or(raw.len(), |e| body_start + e + 3);
                    let body = &raw[body_start..end.saturating_sub(3).max(body_start)];
                    if let Some(cap) = MARKER_RE.captures(body) {
                        let at = r.start + c..r.start + end;
                        blocks.marker(&cap[1], at, out);
                    }
                    pos = end;
                }
                _ => {
                    let Some(cap) = tag else {
                        break;
                    };
                    let m = cap.get(0).map_or(pos..raw.len(), |m| m.range());
                    let name = cap[2].to_ascii_lowercase();
                    if &cap[1] == "/" {
                        if let Some(i) = stack.iter().rposition(|(n, _, _)| *n == name) {
                            for (_, lang, start) in stack.drain(i..).rev() {
                                if let Some(l) = lang {
                                    out.push((start..r.start + m.start, l));
                                }
                            }
                        }
                    } else if cap[4].is_empty() && !VOID.contains(&name.as_str()) {
                        let lang = LANG_ATTR_RE.captures(&cap[3]).and_then(|a| {
                            let v = a.get(1).or(a.get(2)).or(a.get(3))?.as_str().trim();
                            (!v.is_empty()).then(|| crate::rules::spell_lang::primary(v))
                        });
                        stack.push((name, lang, r.start + m.end));
                    }
                    pos = m.end.max(pos + 1);
                }
            }
        }
    }
    for (_, lang, start) in stack {
        if let Some(l) = lang {
            out.push((start..src.len(), l));
        }
    }
    blocks.finish(src.len(), out);
}

/// `explicit-lang` markers at the start of code comment lines.
fn comments(a: &Analyzed, out: &mut Vec<Region>) {
    let src = a.file.text.as_str();
    let mut blocks = Blocks::default();
    let mut lines: Vec<_> = a.comments.iter().flat_map(|c| &c.lines).collect();
    lines.sort_by_key(|l| l.raw.start);
    for l in lines {
        if let Some(cap) = src
            .get(l.content.clone())
            .and_then(|c| MARKER_RE.captures(c))
        {
            blocks.marker(&cap[1], l.raw.clone(), out);
        }
    }
    blocks.finish(src.len(), out);
}

/// Sorted, non-overlapping pieces of `raw`, each taking the language of the shortest region
/// that covers it; adjacent pieces of one language merge.
fn flatten(raw: Vec<Region>) -> Vec<Region> {
    let raw: Vec<Region> = raw.into_iter().filter(|(r, _)| !r.is_empty()).collect();
    if raw.len() <= 1 {
        return raw;
    }
    let mut cuts: Vec<usize> = raw.iter().flat_map(|(r, _)| [r.start, r.end]).collect();
    cuts.sort_unstable();
    cuts.dedup();
    let mut out: Vec<Region> = Vec::new();
    for w in cuts.windows(2) {
        let (s, e) = (w[0], w[1]);
        let Some((_, l)) = raw
            .iter()
            .filter(|(r, _)| r.start <= s && e <= r.end)
            .min_by_key(|(r, _)| r.len())
        else {
            continue;
        };
        match out.last_mut() {
            Some((last, ll)) if last.end == s && ll == l => last.end = e,
            _ => out.push((s..e, l.clone())),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::{FileKind, Lang, SourceFile};
    use std::path::PathBuf;

    fn analyzed(name: &str, kind: FileKind, text: &str) -> Analyzed {
        Analyzed::new(SourceFile::new(
            PathBuf::from("/x").join(name),
            PathBuf::from(name),
            kind,
            text.to_string(),
        ))
    }

    /// `(marked text, language)` pairs of a Markdown text.
    fn marked(text: &str) -> Vec<(String, String)> {
        let a = analyzed("a.md", FileKind::Markdown, text);
        regions(&a)
            .into_iter()
            .map(|(r, l)| (text[r].trim().to_string(), l))
            .collect()
    }

    fn pair(t: &str, l: &str) -> (String, String) {
        (t.to_string(), l.to_string())
    }

    #[test]
    fn html_lang_attributes() {
        assert_eq!(
            marked("# T\n\nSee <span lang=\"fi-FI\">hyvä <b>päivä</b></span> here.\n"),
            [pair("hyvä <b>päivä</b>", "fi")]
        );
        assert_eq!(
            marked("# T\n\n<div lang='sv'>\n\nHej då.\n\n</div>\n\nEnd.\n"),
            [pair("Hej då.", "sv")]
        );
        // Nested: the inner language wins; unquoted values; void elements do not nest.
        assert_eq!(
            marked("# T\n\n<div lang=fi>\n\nA <br> <span lang=en>b</span> c\n\n</div>\n"),
            [
                pair("A <br> <span lang=en>", "fi"),
                pair("b", "en"),
                pair("</span> c", "fi")
            ]
        );
        // `lang` in inline code is no markup.
        assert!(marked("# T\n\nUse `<span lang=\"fi\">x</span>` here.\n").is_empty());
    }

    #[test]
    fn block_markers() {
        let text = "# T\n\nEnglish.\n\n<!-- explicit-lang fi -->\n\nSuomea.\n\n\
            <!-- explicit-lang sv -->\n\nSvenska.\n\n<!-- explicit-lang end -->\n\nEnglish.\n\n\
            <!-- explicit-lang xx -->\n\nOther.\n";
        assert_eq!(
            marked(text),
            [
                pair("Suomea.", "fi"),
                pair("Svenska.", "sv"),
                pair("Other.", "xx")
            ]
        );
    }

    #[test]
    fn code_comment_markers() {
        let text = "// explicit-lang fi\n// Tämä on suomea.\nfn a() {}\n\
            // explicit-lang end\n// English.\n";
        let a = analyzed("a.rs", FileKind::Code(Lang::Rust), text);
        let got = regions(&a);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].1, "fi");
        let r = got[0].0.clone();
        assert!(text[r.clone()].contains("Tämä") && !text[r].contains("English"));
    }
}
