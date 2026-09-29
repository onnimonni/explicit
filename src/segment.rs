//! Prose segments.
//!
//! A segment covers a byte range of the original file. Its `text` has the same
//! byte length as that range, with everything that is not prose (markup, code,
//! comment markers, URLs) replaced by spaces. Newlines are kept. This keeps
//! offsets identical to the source, so no offset mapping tables are needed:
//! `file_offset = segment.range.start + text_offset`.

use std::ops::Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SegmentKind {
    Heading,
    Paragraph,
    ListItem,
    TableCell,
    BlockQuote,
    ImageAlt,
    /// Ordinary line or block comment.
    Comment,
    /// Doc comment (`///`, `/** */`, Go doc, docstring).
    DocComment,
}

impl SegmentKind {
    pub fn is_comment(self) -> bool {
        matches!(self, SegmentKind::Comment | SegmentKind::DocComment)
    }
}

#[derive(Debug, Clone)]
pub struct Segment {
    pub range: Range<usize>,
    pub text: String,
    pub kind: SegmentKind,
}

impl Segment {
    /// Build a segment from `src[range]` keeping only `keep` sub-ranges (absolute offsets).
    pub fn from_ranges(
        src: &str,
        range: Range<usize>,
        keep: &[Range<usize>],
        kind: SegmentKind,
    ) -> Segment {
        let mut bytes: Vec<u8> = src.as_bytes()[range.clone()]
            .iter()
            .map(|&b| if b == b'\n' { b'\n' } else { b' ' })
            .collect();
        for k in keep {
            let Some(r) = clamp(k, &range) else {
                continue;
            };
            bytes[r.clone()]
                .copy_from_slice(&src.as_bytes()[r.start + range.start..r.end + range.start]);
        }
        // Every kept range is on char boundaries, the rest is ASCII, so this is valid UTF-8.
        let text = String::from_utf8(bytes)
            .unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned());
        Segment { range, text, kind }
    }

    /// Blank `range` (absolute offsets) inside this segment. Costs O(range length).
    pub fn blank(&mut self, range: Range<usize>) {
        let Some(r) = self.local_char_range(&range) else {
            return;
        };
        let spaces: String = self.text[r.clone()]
            .bytes()
            .map(|b| if b == b'\n' { '\n' } else { ' ' })
            .collect();
        self.text.replace_range(r, &spaces);
    }

    /// Blank several absolute ranges with a single pass over the text.
    pub fn blank_all<'a>(&mut self, ranges: impl IntoIterator<Item = &'a Range<usize>>) {
        let local: Vec<Range<usize>> = ranges
            .into_iter()
            .filter_map(|r| self.local_char_range(r))
            .collect();
        if local.is_empty() {
            return;
        }
        let mut bytes = std::mem::take(&mut self.text).into_bytes();
        for r in local {
            for b in &mut bytes[r] {
                if *b != b'\n' {
                    *b = b' ';
                }
            }
        }
        // Whole chars were replaced by ASCII spaces, so the text stays valid UTF-8.
        self.text = String::from_utf8(bytes)
            .unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned());
    }

    /// `range` (absolute) clamped to this segment, in local offsets widened to char boundaries.
    fn local_char_range(&self, range: &Range<usize>) -> Option<Range<usize>> {
        let mut r = clamp(range, &self.range)?;
        while !self.text.is_char_boundary(r.start) {
            r.start -= 1;
        }
        while !self.text.is_char_boundary(r.end) {
            r.end += 1;
        }
        Some(r)
    }

    pub fn is_blank(&self) -> bool {
        self.text.trim().is_empty()
    }

    /// Absolute range of a match inside `text`.
    pub fn abs(&self, r: Range<usize>) -> Range<usize> {
        self.range.start + r.start..self.range.start + r.end
    }
}

/// Tokens that are prose-adjacent but not words: numbers with units (`10.5s`, `200ms`, `4GB`,
/// `12.5 s`), list enumerators in brackets (`(a)`, `(iv)`, `[b]`), task boxes (`[ ]`) and
/// letter-number ids (`S-3`, `RFC-042`, `#594`, `§3.6`). Harper splits them into letters it
/// then flags as misspellings (`s`, `f`) or grammar slips (`(i)` -> `(I)`, `(a)` -> `an`).
static NOISE_RE: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
    regex::Regex::new(concat!(
        // Number immediately followed by a unit or letters: 10.5s, 200ms, 3x, 4GB, 2nd.
        r"\b\d+(?:[.,]\d+)*[A-Za-zµ]+\b",
        // Number, one space, a common unit: `12.5 s`, `3 ms`.
        r"|\b\d+(?:[.,]\d+)*[ \u{a0}](?:ns|[uµ]s|ms|s|min|h|[kKMGT]i?B|px|em|rem)\b",
        // Enumerators in brackets: (a) (iv) [b] (XII) and task boxes [ ] [x].
        r"|[(\[](?:[A-Za-z]|x{0,3}(?:ix|iv|v?i{0,3})|X{0,3}(?:IX|IV|V?I{0,3})|\d{1,3})[)\]]",
        r"|\[ \]",
        // Ids: S-3, RFC-042, UC-1a, #594, §3.6.
        r"|\b[A-Z][A-Za-z]*-\d+[A-Za-z]?\b|#\d+\b|§\s?\d+(?:\.\d+)*",
    ))
    .expect("hardcoded regex is valid")
});

/// Absolute ranges of non-word tokens (units, enumerators, ids) inside `seg`; see [`NOISE_RE`].
pub fn noise_ranges(seg: &Segment) -> Vec<Range<usize>> {
    NOISE_RE
        .find_iter(&seg.text)
        // `()` / `[]` match the empty numeral alternatives; they are punctuation, keep them.
        .filter(|m| !matches!(m.as_str(), "()" | "[]" | "(]" | "[)"))
        .map(|m| seg.abs(m.range()))
        .collect()
}

/// `r` (absolute) intersected with `outer`, in offsets local to `outer`; `None` when empty.
fn clamp(r: &Range<usize>, outer: &Range<usize>) -> Option<Range<usize>> {
    let s = r.start.max(outer.start).saturating_sub(outer.start);
    let e = r.end.min(outer.end).saturating_sub(outer.start);
    (s < e).then_some(s..e)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keep_ranges_outside_segment_are_ignored() {
        let src = "aaa bbb\nccc";
        let s = Segment::from_ranges(src, 4..11, &[0..2, 4..7, 20..30], SegmentKind::Paragraph);
        assert_eq!(s.text, "bbb\n   ");
    }

    #[test]
    fn noise_tokens() {
        let src = "Took 10.5s and 200ms (3x, 4GB, 12.5 s) per (f) and (iv) [b] [ ] S-3 RFC-042 #594 §3.6 (mix) (a b) () 3 sheep";
        let all = 0..src.len();
        let seg = Segment::from_ranges(
            src,
            all.clone(),
            std::slice::from_ref(&all),
            SegmentKind::Paragraph,
        );
        let found: Vec<&str> = noise_ranges(&seg).into_iter().map(|r| &src[r]).collect();
        assert_eq!(
            found,
            [
                "10.5s", "200ms", "3x", "4GB", "12.5 s", "(f)", "(iv)", "[b]", "[ ]", "S-3",
                "RFC-042", "#594", "§3.6"
            ]
        );
    }

    #[test]
    fn blank_single_and_all() {
        let src = "héllo wörld\nnext";
        let all = 0..src.len();
        let mut a = Segment::from_ranges(
            src,
            all.clone(),
            std::slice::from_ref(&all),
            SegmentKind::Paragraph,
        );
        let mut b = a.clone();
        // Out-of-range and mid-char ranges are safe.
        a.blank(100..200);
        a.blank(2..3); // inside 'é' -> whole char blanked
        a.blank(8..src.len());
        b.blank_all(&[100..200, 2..3, 8..src.len()]);
        assert_eq!(a.text, b.text);
        assert_eq!(a.text.len(), src.len());
        assert!(a.text.starts_with("h  llo w"));
        assert!(a.text.contains('\n'));
        assert!(!a.text.contains("next"));
    }
}
