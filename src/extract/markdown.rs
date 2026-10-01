//! Markdown extraction: headings, links, code blocks, lists and prose segments.

use std::collections::HashMap;
use std::ops::Range;
use std::sync::LazyLock;

use pulldown_cmark::{
    BrokenLink, CodeBlockKind, Event, HeadingLevel, LinkType, Options, Parser, Tag, TagEnd,
};
use regex::Regex;
use unicode_normalization::char::is_combining_mark;

use crate::segment::{Segment, SegmentKind};

pub static URL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"\b(?:https?://|www\.)[^\s<>()\[\]"'`]*[^\s<>()\[\]"'`.,;:!?]"#)
        .expect("hardcoded regex is valid")
});
/// File names and paths in prose (`explicit.toml`, `src/lib.rs`); Harper reads their dots as
/// sentence ends, so segments blank them like URLs. Without an extension a path needs two
/// slashes or a leading `./`, `../`, `~/` or `/`, so `and/or`, `TCP/IP` and `24/7` stay prose.
pub static PATH_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(concat!(
        r"\b[\w-]{2,}(?:/[\w.-]+)*\.[A-Za-z][A-Za-z0-9]{0,9}\b(?::\d+(?:[-–]\d+)?)?",
        // Path segments never end in `.`, so a sentence-ending period stays prose.
        r"|\b[\w.-]*[\w-](?:/[\w.-]*[\w-]){2,}/?",
        r"|(?:\.\.?/|~/|/)[\w.-]*[\w-](?:/[\w.-]*[\w-])*/?",
    ))
    .expect("hardcoded regex is valid")
});

/// Absolute ranges of URLs and paths inside a segment.
pub fn non_prose_ranges(seg: &Segment) -> Vec<std::ops::Range<usize>> {
    let text = seg.text.as_bytes();
    // A leading `/` right after a word char is the middle of `and/or`, not a root path.
    let mid_word = |i: usize| i > 0 && (text[i - 1].is_ascii_alphanumeric() || text[i - 1] == b'_');
    URL_RE
        .find_iter(&seg.text)
        .chain(
            PATH_RE
                .find_iter(&seg.text)
                .filter(|m| !(text[m.start()] == b'/' && mid_word(m.start()))),
        )
        .map(|m| seg.abs(m.range()))
        .collect()
}

#[cfg(test)]
mod path_re_tests {
    use super::*;

    fn paths(text: &str) -> Vec<&str> {
        let seg = Segment {
            range: 0..text.len(),
            text: text.into(),
            kind: SegmentKind::Paragraph,
        };
        non_prose_ranges(&seg)
            .into_iter()
            .map(|r| &text[r])
            .collect()
    }

    #[test]
    fn slashed_words_stay_prose() {
        assert!(paths("Use and/or with read/write access over TCP/IP, 24/7.").is_empty());
    }

    #[test]
    fn real_paths_are_found() {
        assert_eq!(paths("Edit explicit.toml now."), ["explicit.toml"]);
        assert_eq!(
            paths("See app.js:80-139 and lib.rs:7."),
            ["app.js:80-139", "lib.rs:7"]
        );
        assert_eq!(
            paths("See src/rules/mod and docs/x.md."),
            ["src/rules/mod", "docs/x.md"]
        );
        assert_eq!(
            paths("Run ./bin/tool or ../up then ~/cfg in /usr/local."),
            ["./bin/tool", "../up", "~/cfg", "/usr/local"]
        );
    }
}

static HTML_ID_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)\s(?:id|name)\s*=\s*["']([^"']+)["']"#).expect("hardcoded regex is valid")
});

#[derive(Debug, Clone)]
pub struct Heading {
    pub level: u8,
    pub text: String,
    pub range: Range<usize>,
    /// Anchor id: explicit `{#id}` or GitHub-style slug (deduplicated).
    pub anchor: String,
    pub setext: bool,
    /// The anchor comes from an explicit `{#id}` attribute.
    pub explicit_id: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkKind {
    Inline,
    /// `[text][label]`, `[label][]` or `[label]`.
    Reference,
    /// `<https://...>`
    Autolink,
    /// A URL written as plain text.
    Bare,
    /// `<a href>` / `<img src>` in HTML.
    Html,
}

#[derive(Debug, Clone)]
pub struct Link {
    pub dest: String,
    /// Range of the whole link in the source.
    pub range: Range<usize>,
    pub kind: LinkKind,
    pub is_image: bool,
    /// Visible link text (empty for images without alt).
    pub text: String,
    /// Normalized reference label for reference links.
    pub label: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RefDef {
    pub label: String,
    pub dest: String,
    pub range: Range<usize>,
}

#[derive(Debug, Clone)]
pub struct CodeBlock {
    pub range: Range<usize>,
    pub fenced: bool,
    pub lang: Option<String>,
}

#[derive(Debug, Clone)]
pub struct List {
    pub range: Range<usize>,
    pub ordered: bool,
    pub start: Option<u64>,
    pub depth: usize,
    /// Item ranges.
    pub items: Vec<Range<usize>>,
}

#[derive(Debug, Clone, Default)]
pub struct MdDoc {
    pub headings: Vec<Heading>,
    pub links: Vec<Link>,
    pub ref_defs: Vec<RefDef>,
    /// Full/collapsed reference links whose label has no definition.
    pub undefined_refs: Vec<(String, Range<usize>)>,
    pub code_blocks: Vec<CodeBlock>,
    /// Inline code span ranges.
    pub code_spans: Vec<Range<usize>>,
    /// Raw HTML (block and inline) ranges.
    pub html: Vec<Range<usize>>,
    pub lists: Vec<List>,
    /// IDs from `<a id>`/`<a name>` HTML anchors.
    pub html_anchors: Vec<String>,
    pub front_matter: Option<Range<usize>>,
    pub segments: Vec<Segment>,
    /// GFM table ranges.
    pub tables: Vec<Range<usize>>,
    /// Thematic break (`---`, `***`, `___`) ranges.
    pub thematic_breaks: Vec<Range<usize>>,
    /// Emphasis (`*x*` / `_x_`) ranges.
    pub emphasis: Vec<Range<usize>>,
    /// Strong emphasis (`**x**` / `__x__`) ranges.
    pub strong: Vec<Range<usize>>,
    /// Footnote references to defined footnotes: (normalized label, range).
    pub footnote_refs: Vec<(String, Range<usize>)>,
    /// Footnote definitions: (normalized label, range).
    pub footnote_defs: Vec<(String, Range<usize>)>,
    /// Block-level HTML ranges.
    pub html_blocks: Vec<Range<usize>>,
}

impl MdDoc {
    /// True if `offset` is inside a code block, code span, HTML or front matter.
    pub fn in_code(&self, offset: usize) -> bool {
        self.code_blocks.iter().any(|c| c.range.contains(&offset))
            || self.code_spans.iter().any(|c| c.contains(&offset))
            || self
                .front_matter
                .as_ref()
                .is_some_and(|f| f.contains(&offset))
    }

    pub fn in_code_block(&self, offset: usize) -> bool {
        self.code_blocks.iter().any(|c| c.range.contains(&offset))
            || self
                .front_matter
                .as_ref()
                .is_some_and(|f| f.contains(&offset))
    }

    /// All anchor ids a link fragment may target.
    pub fn anchors(&self) -> impl Iterator<Item = &str> {
        self.headings
            .iter()
            .map(|h| h.anchor.as_str())
            .chain(self.html_anchors.iter().map(String::as_str))
    }
}

pub fn options() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_HEADING_ATTRIBUTES
        | Options::ENABLE_YAML_STYLE_METADATA_BLOCKS
        | Options::ENABLE_GFM
}

/// Normalize a reference label the way CommonMark matches them.
pub fn normalize_label(s: &str) -> String {
    s.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// GitHub heading slug: lowercase, drop punctuation except `-` and `_`, spaces to `-`.
pub fn slugify(text: &str) -> String {
    text.trim()
        .to_lowercase()
        .chars()
        .filter_map(|c| {
            if c == ' ' {
                Some('-')
            } else if c.is_alphanumeric() || c == '-' || c == '_' {
                Some(c)
            } else {
                None
            }
        })
        .collect()
}

struct Block {
    start: usize,
    kind: SegmentKind,
    keep: Vec<Range<usize>>,
}

fn code_word_character(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || is_combining_mark(c)
}

/// Extend a masked source range through adjacent word characters without shifting bytes.
pub(crate) fn word_containing_range(src: &str, range: Range<usize>) -> Range<usize> {
    let start = src[..range.start]
        .char_indices()
        .rev()
        .take_while(|(_, c)| code_word_character(*c))
        .last()
        .map_or(range.start, |(i, _)| i);
    let end = src[range.end..]
        .char_indices()
        .take_while(|(_, c)| code_word_character(*c))
        .last()
        .map_or(range.end, |(i, c)| range.end + i + c.len_utf8());
    start..end
}

pub fn parse(src: &str) -> MdDoc {
    let mut doc = MdDoc::default();
    let mut undefined: Vec<(String, Range<usize>)> = Vec::new();
    let callback = |b: BrokenLink| {
        if matches!(b.link_type, LinkType::Reference | LinkType::Collapsed) {
            undefined.push((normalize_label(&b.reference), b.span.clone()));
        }
        None
    };
    let parser = Parser::new_with_broken_link_callback(src, options(), Some(callback));
    let mut iter = parser.into_offset_iter();

    let mut current: Option<Block> = None;
    let mut quote_depth = 0usize;
    let mut code_depth = 0usize;
    let mut autolink_depth = 0usize;
    let mut heading: Option<(u8, Range<usize>, Option<String>, String)> = None;
    let mut link_stack: Vec<Link> = Vec::new();
    let mut list_stack: Vec<List> = Vec::new();
    let mut slug_counts: HashMap<String, usize> = HashMap::new();
    let mut bare_run: Option<Range<usize>> = None;
    let mut masked_word_end = 0;

    fn flush(cur: &mut Option<Block>, end: usize, src: &str, out: &mut Vec<Segment>) {
        if let Some(b) = cur.take() {
            if b.keep.is_empty() || end <= b.start {
                return;
            }
            let mut seg = Segment::from_ranges(src, b.start..end, &b.keep, b.kind);
            let mut blanks = non_prose_ranges(&seg);
            blanks.extend(crate::segment::noise_ranges(&seg));
            seg.blank_all(&blanks);
            if !seg.is_blank() {
                out.push(seg);
            }
        }
    }

    let block_kind = |quote_depth: usize, k: SegmentKind| {
        if quote_depth > 0 && k == SegmentKind::Paragraph {
            SegmentKind::BlockQuote
        } else {
            k
        }
    };

    for (event, range) in iter.by_ref() {
        if !matches!(event, Event::Text(_))
            && let Some(r) = bare_run.take()
        {
            bare_urls(src, r, &mut doc.links);
        }
        match event {
            Event::Start(tag) => match tag {
                Tag::Paragraph => {
                    flush(&mut current, range.start, src, &mut doc.segments);
                    current = Some(Block {
                        start: range.start,
                        kind: block_kind(quote_depth, SegmentKind::Paragraph),
                        keep: vec![],
                    });
                }
                Tag::Heading { level, id, .. } => {
                    flush(&mut current, range.start, src, &mut doc.segments);
                    current = Some(Block {
                        start: range.start,
                        kind: SegmentKind::Heading,
                        keep: vec![],
                    });
                    heading = Some((
                        heading_level(level),
                        range.clone(),
                        id.map(|s| s.to_string()),
                        String::new(),
                    ));
                }
                Tag::TableCell => {
                    flush(&mut current, range.start, src, &mut doc.segments);
                    current = Some(Block {
                        start: range.start,
                        kind: SegmentKind::TableCell,
                        keep: vec![],
                    });
                }
                Tag::Item => {
                    flush(&mut current, range.start, src, &mut doc.segments);
                    current = Some(Block {
                        start: range.start,
                        kind: SegmentKind::ListItem,
                        keep: vec![],
                    });
                    if let Some(l) = list_stack.last_mut() {
                        l.items.push(range.clone());
                    }
                }
                Tag::List(start) => {
                    flush(&mut current, range.start, src, &mut doc.segments);
                    list_stack.push(List {
                        range: range.clone(),
                        ordered: start.is_some(),
                        start,
                        depth: list_stack.len(),
                        items: vec![],
                    });
                }
                Tag::BlockQuote(_) => {
                    flush(&mut current, range.start, src, &mut doc.segments);
                    quote_depth += 1;
                }
                Tag::CodeBlock(kind) => {
                    flush(&mut current, range.start, src, &mut doc.segments);
                    code_depth += 1;
                    let (fenced, lang) = match kind {
                        CodeBlockKind::Fenced(info) => {
                            let lang = info
                                .split(|c: char| c.is_whitespace() || c == ',' || c == '{')
                                .next()
                                .unwrap_or("")
                                .to_string();
                            (true, (!lang.is_empty()).then_some(lang))
                        }
                        CodeBlockKind::Indented => (false, None),
                    };
                    doc.code_blocks.push(CodeBlock {
                        range: range.clone(),
                        fenced,
                        lang,
                    });
                }
                Tag::HtmlBlock => {
                    flush(&mut current, range.start, src, &mut doc.segments);
                    doc.html_blocks.push(range.clone());
                }
                Tag::Table(_) => doc.tables.push(range.clone()),
                Tag::Emphasis => doc.emphasis.push(range.clone()),
                Tag::Strong => doc.strong.push(range.clone()),
                Tag::FootnoteDefinition(ref label) => {
                    doc.footnote_defs
                        .push((normalize_label(label), range.clone()));
                }
                Tag::MetadataBlock(_) => {
                    doc.front_matter = Some(range.clone());
                    code_depth += 1;
                }
                Tag::Link {
                    link_type,
                    dest_url,
                    id,
                    ..
                }
                | Tag::Image {
                    link_type,
                    dest_url,
                    id,
                    ..
                } => {
                    let is_image = tag_is_image(&src[range.clone()]);
                    let kind = match link_type {
                        LinkType::Autolink | LinkType::Email => {
                            autolink_depth += 1;
                            LinkKind::Autolink
                        }
                        LinkType::Reference | LinkType::Collapsed | LinkType::Shortcut => {
                            LinkKind::Reference
                        }
                        _ => LinkKind::Inline,
                    };
                    let label = (kind == LinkKind::Reference).then(|| {
                        if id.is_empty() {
                            normalize_label(link_text_guess(&src[range.clone()]))
                        } else {
                            normalize_label(&id)
                        }
                    });
                    link_stack.push(Link {
                        dest: dest_url.to_string(),
                        range: range.clone(),
                        kind,
                        is_image,
                        text: String::new(),
                        label,
                    });
                }
                _ => {}
            },
            Event::End(tag) => match tag {
                TagEnd::Paragraph | TagEnd::TableCell | TagEnd::Item => {
                    flush(&mut current, range.end, src, &mut doc.segments);
                }
                TagEnd::Heading(_) => {
                    flush(&mut current, range.end, src, &mut doc.segments);
                    if let Some((level, hrange, id, text)) = heading.take() {
                        let setext = !src[hrange.clone()].trim_start().starts_with('#');
                        let explicit_id = id.is_some();
                        let anchor = match id {
                            Some(id) => id,
                            None => {
                                let base = slugify(&text);
                                let n = slug_counts.entry(base.clone()).or_insert(0);
                                let a = if *n == 0 {
                                    base.clone()
                                } else {
                                    format!("{base}-{n}")
                                };
                                *n += 1;
                                a
                            }
                        };
                        doc.headings.push(Heading {
                            level,
                            text: text.trim().to_string(),
                            range: hrange,
                            anchor,
                            setext,
                            explicit_id,
                        });
                    }
                }
                TagEnd::List(_) => {
                    flush(&mut current, range.end, src, &mut doc.segments);
                    if let Some(l) = list_stack.pop() {
                        doc.lists.push(l);
                    }
                }
                TagEnd::BlockQuote(_) => {
                    flush(&mut current, range.end, src, &mut doc.segments);
                    quote_depth = quote_depth.saturating_sub(1);
                }
                TagEnd::CodeBlock | TagEnd::MetadataBlock(_) => {
                    code_depth = code_depth.saturating_sub(1)
                }
                TagEnd::Link | TagEnd::Image => {
                    if let Some(link) = link_stack.pop() {
                        if link.kind == LinkKind::Autolink {
                            autolink_depth = autolink_depth.saturating_sub(1);
                        }
                        doc.links.push(link);
                    }
                }
                _ => {}
            },
            Event::Text(t) => {
                if code_depth > 0 {
                    continue;
                }
                if let Some(h) = heading.as_mut() {
                    h.3.push_str(&t);
                }
                for l in &mut link_stack {
                    l.text.push_str(&t);
                }
                if autolink_depth > 0 {
                    continue;
                }
                // Bare URLs in plain text: pulldown-cmark splits text at delimiter runs (`*`, `_`), so
                // scan the raw source of each contiguous run of Text events, not each fragment.
                if link_stack.is_empty() {
                    match bare_run.as_mut() {
                        Some(r) if r.end == range.start => r.end = range.end,
                        _ => {
                            if let Some(r) = bare_run.replace(range.clone()) {
                                bare_urls(src, r, &mut doc.links);
                            }
                        }
                    }
                }
                let cur = current.get_or_insert_with(|| Block {
                    start: range.start,
                    kind: block_kind(quote_depth, SegmentKind::Paragraph),
                    keep: vec![],
                });
                // Text may be a decoded entity/escape; only keep it when it is the raw source.
                if src.get(range.clone()) == Some(&*t) {
                    let start = range.start.max(masked_word_end).min(range.end);
                    if start < range.end {
                        cur.keep.push(start..range.end);
                    }
                }
            }
            Event::Code(t) => {
                if let Some(h) = heading.as_mut() {
                    h.3.push_str(&t);
                }
                for l in &mut link_stack {
                    l.text.push_str(&t);
                }
                // Inline code inside an orthographic word leaves no standalone
                // prose token on either side. Keep syntax ranges exact; trim only prose.
                let word_range = word_containing_range(src, range.clone());
                if let Some(cur) = current.as_mut() {
                    for keep in cur.keep.iter_mut().rev() {
                        if keep.end <= word_range.start {
                            break;
                        }
                        keep.end = keep.end.min(word_range.start).max(keep.start);
                    }
                }
                masked_word_end = word_range.end;
                doc.code_spans.push(range);
            }
            Event::Rule => doc.thematic_breaks.push(range),
            Event::FootnoteReference(label) => {
                doc.footnote_refs.push((normalize_label(&label), range));
            }
            Event::Html(_) | Event::InlineHtml(_) => {
                let raw = &src[range.clone()];
                for c in HTML_ID_RE.captures_iter(raw) {
                    doc.html_anchors.push(c[1].to_string());
                }
                for (dest, is_image) in html_links(raw) {
                    doc.links.push(Link {
                        dest,
                        range: range.clone(),
                        kind: LinkKind::Html,
                        is_image,
                        text: String::new(),
                        label: None,
                    });
                }
                doc.html.push(range);
            }
            _ => {}
        }
    }
    flush(&mut current, src.len(), src, &mut doc.segments);
    if let Some(r) = bare_run.take() {
        bare_urls(src, r, &mut doc.links);
    }

    doc.ref_defs = iter
        .reference_definitions()
        .iter()
        .map(|(label, def)| RefDef {
            label: normalize_label(label),
            dest: def.dest.to_string(),
            range: def.span.clone(),
        })
        .collect();
    doc.ref_defs.sort_by_key(|d| d.range.start);
    drop(iter);
    doc.undefined_refs = undefined;
    doc.links.sort_by_key(|l| l.range.start);
    doc
}

/// Record bare URLs found in the raw source `run` of contiguous plain text.
fn bare_urls(src: &str, run: Range<usize>, links: &mut Vec<Link>) {
    for m in URL_RE.find_iter(&src[run.clone()]) {
        let r = run.start + m.start()..run.start + m.end();
        let dest = if m.as_str().starts_with("www.") {
            format!("https://{}", m.as_str())
        } else {
            m.as_str().to_string()
        };
        links.push(Link {
            dest,
            range: r,
            kind: LinkKind::Bare,
            is_image: false,
            text: m.as_str().to_string(),
            label: None,
        });
    }
}

/// Whether the bare URL at `range` continues in the source past markdown syntax (e.g.
/// `https://example.org/a*b*c`, where `*b*` parses as emphasis), so `range` holds only a prefix.
/// Trailing emphasis closers (`**https://example.org**`) do not count.
pub fn bare_url_truncated(src: &str, range: &Range<usize>) -> bool {
    let line_end = src[range.start..]
        .find('\n')
        .map_or(src.len(), |i| range.start + i);
    let Some(m) = URL_RE.find(&src[range.start..line_end]) else {
        return false;
    };
    let full_end = range.start + m.end();
    m.start() == 0
        && full_end > range.end
        && src[range.end..full_end]
            .trim_end_matches(['*', '_', '~'])
            .contains(|c: char| !matches!(c, '*' | '_' | '~'))
}

fn heading_level(l: HeadingLevel) -> u8 {
    match l {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

fn tag_is_image(raw: &str) -> bool {
    raw.starts_with("![")
}

/// For `[label]` / `[label][]` the label is the bracketed text.
fn link_text_guess(raw: &str) -> &str {
    let raw = raw.strip_prefix('!').unwrap_or(raw);
    let raw = raw.strip_prefix('[').unwrap_or(raw);
    raw.split(']').next().unwrap_or(raw)
}

static HTML_HREF_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)<(a|img|source)\b[^>]*?\s(href|src)\s*=\s*["']([^"']+)["']"#)
        .expect("hardcoded regex is valid")
});

fn html_links(raw: &str) -> Vec<(String, bool)> {
    HTML_HREF_RE
        .captures_iter(raw)
        .map(|c| (c[3].to_string(), !c[1].eq_ignore_ascii_case("a")))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_structure() {
        let src = "# Title\n\nSome `code` and [a link](./x.md#part) and https://example.com/x.\n\n## Title\n\n```rust\nlet x = 1;\n```\n\n- one\n- two\n\n[ref]: https://a.b\n[unused]: https://c.d\n\nSee [x][ref] and [y][missing].\n";
        let d = parse(src);
        assert_eq!(d.headings.len(), 2);
        assert_eq!(d.headings[0].anchor, "title");
        assert_eq!(d.headings[1].anchor, "title-1");
        assert!(
            d.links
                .iter()
                .any(|l| l.dest == "./x.md#part" && l.kind == LinkKind::Inline)
        );
        assert!(
            d.links
                .iter()
                .any(|l| l.dest == "https://example.com/x" && l.kind == LinkKind::Bare)
        );
        assert!(d.links.iter().any(|l| l.label.as_deref() == Some("ref")));
        assert_eq!(d.undefined_refs.len(), 1);
        assert_eq!(d.ref_defs.len(), 2);
        assert_eq!(d.code_blocks[0].lang.as_deref(), Some("rust"));
        assert_eq!(d.lists[0].items.len(), 2);
        for s in &d.segments {
            assert_eq!(s.text.len(), s.range.len());
            assert!(!s.text.contains("code"), "code span blanked: {:?}", s.text);
            assert!(!s.text.contains("https"), "url blanked: {:?}", s.text);
        }
    }

    #[test]
    fn bare_url_across_delimiters() {
        // Unmatched `*` splits Text events; the URL must still be whole.
        let src = "See https://example.org/a*b now.\n";
        let d = parse(src);
        let l: Vec<_> = d
            .links
            .iter()
            .filter(|l| l.kind == LinkKind::Bare)
            .collect();
        assert_eq!(l.len(), 1);
        assert_eq!(&src[l[0].range.clone()], "https://example.org/a*b");
        // `*b*` is emphasis: the Text fragment is a truncated URL.
        let src = "See https://example.org/a*b*c now.\n";
        let d = parse(src);
        let l = d.links.iter().find(|l| l.kind == LinkKind::Bare).unwrap();
        assert!(bare_url_truncated(src, &l.range));
        let src = "See **https://example.org/x** now.\n";
        let d = parse(src);
        let l = d.links.iter().find(|l| l.kind == LinkKind::Bare).unwrap();
        assert_eq!(&src[l.range.clone()], "https://example.org/x");
        assert!(!bare_url_truncated(src, &l.range));
    }

    #[test]
    fn slugs() {
        assert_eq!(slugify("Hello, World! 2.0"), "hello-world-20");
        assert_eq!(slugify("Use `foo_bar` now"), "use-foo_bar-now");
    }
}
