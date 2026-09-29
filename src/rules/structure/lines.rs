//! Line table with block context (code, front matter, tables, block quotes).

use std::sync::LazyLock;

use regex::Regex;

use crate::extract::markdown::MdDoc;

static TABLE_DELIM_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*\|?\s*:?-+:?\s*(\|\s*:?-+:?\s*)*\|?\s*$").expect("hardcoded regex is valid")
});

#[derive(Debug, Clone)]
pub struct Line {
    pub start: usize,
    /// Content end, excluding `\r\n` / `\n`.
    pub end: usize,
    /// Start of the next line (== end for the last line without newline).
    pub next: usize,
    /// Inside a code block or front matter.
    pub code: bool,
    pub front: bool,
    pub table: bool,
    /// Inside a block-level HTML block.
    pub html: bool,
    /// Block quote depth from `>` markers on this line.
    pub quote: usize,
    /// Offset after the block quote prefix.
    pub body: usize,
    /// Only whitespace after the block quote prefix.
    pub blank: bool,
}

#[derive(Debug)]
pub struct Lines {
    pub v: Vec<Line>,
    /// Index of the closing front matter delimiter line.
    pub fm_end: Option<usize>,
}

impl Lines {
    pub fn new(src: &str, md: &MdDoc) -> Lines {
        let bytes = src.as_bytes();
        let mut v = Vec::new();
        let mut s = 0;
        while s < src.len() {
            let (end, next) = match src[s..].find('\n') {
                Some(i) => (s + i, s + i + 1),
                None => (src.len(), src.len()),
            };
            let end = if end > s && bytes[end - 1] == b'\r' {
                end - 1
            } else {
                end
            };
            let (quote, off) = quote_prefix(&src[s..end]);
            let body = s + off;
            let blank = src[body..end].trim().is_empty();
            v.push(Line {
                start: s,
                end,
                next,
                code: false,
                front: false,
                table: false,
                html: false,
                quote,
                body,
                blank,
            });
            s = next;
        }
        let mut lines = Lines { v, fm_end: None };

        if md.front_matter.is_some()
            && lines
                .v
                .first()
                .is_some_and(|l| src[l.start..l.end].trim_end() == "---")
        {
            let close = (1..lines.v.len()).find(|&i| {
                matches!(
                    src[lines.v[i].start..lines.v[i].end].trim_end(),
                    "---" | "..."
                )
            });
            if let Some(k) = close {
                lines.fm_end = Some(k);
                for l in &mut lines.v[..=k] {
                    l.front = true;
                    l.code = true;
                }
            }
        }
        for cb in &md.code_blocks {
            if cb.range.is_empty() {
                continue;
            }
            let a = lines.index_of(cb.range.start);
            let b = lines.index_of(cb.range.end - 1);
            for l in &mut lines.v[a..=b] {
                l.code = true;
            }
        }
        for h in &md.html_blocks {
            if h.is_empty() {
                continue;
            }
            let a = lines.index_of(h.start);
            let b = lines.index_of(h.end - 1);
            for l in &mut lines.v[a..=b] {
                l.html = true;
            }
        }
        // GFM tables: header, delimiter row and following non-blank rows.
        let mut i = 1;
        while i < lines.v.len() {
            let l = &lines.v[i];
            let body = &src[l.body..l.end];
            let prev = &lines.v[i - 1];
            if !l.code
                && !prev.code
                && !prev.blank
                && body.contains('|')
                && TABLE_DELIM_RE.is_match(body)
            {
                let q = l.quote;
                lines.v[i - 1].table = true;
                let mut j = i;
                while j < lines.v.len()
                    && !lines.v[j].blank
                    && !lines.v[j].code
                    && lines.v[j].quote == q
                {
                    lines.v[j].table = true;
                    j += 1;
                }
                i = j;
            } else {
                i += 1;
            }
        }
        lines
    }

    /// Index of the line containing `offset`.
    pub fn index_of(&self, offset: usize) -> usize {
        self.v
            .partition_point(|l| l.start <= offset)
            .saturating_sub(1)
    }

    pub fn len(&self) -> usize {
        self.v.len()
    }
}

/// Block quote depth and byte offset of the content after the `>` markers.
pub fn quote_prefix(line: &str) -> (usize, usize) {
    let b = line.as_bytes();
    let (mut i, mut depth) = (0, 0);
    loop {
        let mut j = i;
        while j < b.len() && b[j] == b' ' && j - i < 3 {
            j += 1;
        }
        if j < b.len() && b[j] == b'>' {
            j += 1;
            if j < b.len() && (b[j] == b' ' || b[j] == b'\t') {
                j += 1;
            }
            depth += 1;
            i = j;
        } else {
            return (depth, i);
        }
    }
}

/// Visual width of leading text, expanding tabs to 4-column stops.
pub fn width(s: &str) -> usize {
    s.chars()
        .fold(0, |w, c| if c == '\t' { w + 4 - w % 4 } else { w + 1 })
}
