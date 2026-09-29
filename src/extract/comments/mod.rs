//! Language-aware comment extraction without a parser dependency.

use std::ops::Range;

use crate::segment::Segment;
use crate::source::Lang;

mod lexer;
mod python;
#[cfg(test)]
mod tests;

use lexer::Lexer;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CommentKind {
    /// `// ...`, `# ...`
    Line,
    /// `/* ... */`
    Block,
    /// `///`, `//!`, `/** */`, Elixir `@doc`, Zig `///`
    Doc,
    /// Python docstring.
    Docstring,
}

#[derive(Debug, Clone)]
pub struct CommentLine {
    /// Whole physical comment line including markers (absolute byte range).
    pub raw: Range<usize>,
    /// Text after stripping markers/decoration like ` * ` (absolute byte range, may be empty).
    pub content: Range<usize>,
}

#[derive(Debug, Clone)]
pub struct CommentBlock {
    /// From the first comment marker to the end of the last comment line.
    pub range: Range<usize>,
    pub kind: CommentKind,
    pub lines: Vec<CommentLine>,
    /// 1-based line of the first comment line.
    pub start_line: usize,
    /// First non-blank code line after the block, trimmed.
    pub next_code_line: Option<String>,
    /// The comment follows code on the same line (`x = 1 // note`).
    pub trailing: bool,
}

impl CommentBlock {
    /// Content lines as strings.
    pub fn prose_lines<'a>(&self, src: &'a str) -> Vec<&'a str> {
        self.lines.iter().map(|l| &src[l.content.clone()]).collect()
    }

    pub fn is_doc(&self) -> bool {
        matches!(self.kind, CommentKind::Doc | CommentKind::Docstring)
    }
}

/// Extract comments with absolute UTF-8 byte ranges and physical line metadata.
pub fn extract(lang: Lang, src: &str) -> Vec<CommentBlock> {
    Lexer::new(lang, src).run()
}

/// One prose segment per comment block.
///
/// Fenced and indented code examples, commented-out code, inline code spans and
/// URLs are blanked so prose rules only see prose.
pub fn segments(src: &str, blocks: &[CommentBlock]) -> Vec<Segment> {
    use crate::segment::SegmentKind;
    blocks
        .iter()
        .map(|b| {
            let mut keep = Vec::new();
            let mut in_fence = false;
            for l in &b.lines {
                let content = &src[l.content.clone()];
                let trimmed = content.trim_start();
                if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
                    in_fence = !in_fence;
                    continue;
                }
                let indented_code = b.is_doc() && content.starts_with("    ");
                if in_fence || indented_code || looks_like_code(trimmed) {
                    continue;
                }
                keep.push(l.content.clone());
            }
            let kind = if b.is_doc() {
                SegmentKind::DocComment
            } else {
                SegmentKind::Comment
            };
            let mut seg = Segment::from_ranges(src, b.range.clone(), &keep, kind);
            let mut blanks: Vec<Range<usize>> = CODE_SPAN_RE
                .find_iter(&seg.text)
                .map(|m| seg.abs(m.range()))
                .collect();
            blanks.extend(crate::extract::markdown::non_prose_ranges(&seg));
            for r in blanks {
                seg.blank(r);
            }
            seg
        })
        .filter(|s| !s.is_blank())
        .collect()
}

static CODE_SPAN_RE: std::sync::LazyLock<regex::Regex> =
    std::sync::LazyLock::new(|| regex::Regex::new(r"`[^`\n]+`").expect("hardcoded regex is valid"));

static CODE_LINE_RE: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
    regex::Regex::new(
        r"(?x)
        ^(let|const|var|fn|def|func|pub|use|import|from|return|if|elif|else|for|while|match|
          impl|struct|enum|class|package|include|export|local|alias|set|unset)\b.*[;{}()=:\]]\s*$
        | ^[\w.$\[\]\x22'-]+\s*(=|\+=|:=|=>)\s*\S.*$
        | ^[\w.:$]+\(.*\)\s*[;,]?\s*$
        | [;{}]\s*$
        | ^[})\]]",
    )
    .expect("hardcoded regex is valid")
});

/// Commented-out code rather than prose.
fn looks_like_code(line: &str) -> bool {
    let line = line.trim();
    if line.is_empty() {
        return false;
    }
    // Prose with a trailing brace is rare; a sentence ends with punctuation and has several words.
    let words = line.split_whitespace().count();
    CODE_LINE_RE.is_match(line) && !(words > 6 && line.ends_with('.'))
}
