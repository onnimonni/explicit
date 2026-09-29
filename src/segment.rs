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
