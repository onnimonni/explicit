//! Scanners that find the end of opaque literals without consuming input.

use super::{Heredoc, Lexer};

impl Lexer<'_> {
    pub(super) fn quoted(&self, start: usize, count: usize, escapes: bool, doubled: bool) -> usize {
        let quote = self.bytes[start];
        let mut p = start + count;
        while p < self.bytes.len() {
            if escapes && self.bytes[p] == b'\\' {
                p = (p + 2).min(self.bytes.len());
            } else if self.bytes[p..].starts_with(&self.bytes[start..start + count]) {
                if doubled && self.bytes.get(p + 1) == Some(&quote) {
                    p += 2;
                } else {
                    return p + count;
                }
            } else {
                p += 1;
            }
        }
        p
    }

    pub(super) fn block_end(&self, start: usize) -> (usize, bool) {
        let mut depth = 1;
        let mut p = start + 2;
        while p + 1 < self.bytes.len() {
            if self.bytes[p..].starts_with(b"*/") {
                depth -= 1;
                p += 2;
                if depth == 0 {
                    return (p, true);
                }
            } else if self.spec.nested && self.bytes[p..].starts_with(b"/*") {
                depth += 1;
                p += 2;
            } else {
                p += 1;
            }
        }
        (self.bytes.len(), false)
    }

    pub(super) fn rust_raw(&self, start: usize) -> Option<usize> {
        let mut p = start;
        if matches!(self.bytes.get(p), Some(b'b' | b'c')) {
            p += 1;
        }
        if self.bytes.get(p) != Some(&b'r') {
            return None;
        }
        p += 1;
        let hashes = p;
        while self.bytes.get(p) == Some(&b'#') {
            p += 1;
        }
        let count = p - hashes;
        if self.bytes.get(p) != Some(&b'"') {
            return None;
        }
        p += 1;
        while p < self.bytes.len() {
            if self.bytes[p] == b'"' {
                let mut end = p + 1;
                while end - p - 1 < count && self.bytes.get(end) == Some(&b'#') {
                    end += 1;
                }
                if end - p - 1 == count {
                    return Some(end);
                }
                p = end;
            } else {
                p += 1;
            }
        }
        Some(p)
    }

    pub(super) fn rust_char(&self, start: usize) -> Option<usize> {
        let mut p = start + 1;
        if self.bytes.get(p) == Some(&b'\\') {
            p += 1;
            match self.bytes.get(p) {
                Some(b'u') if self.bytes.get(p + 1) == Some(&b'{') => {
                    p += 2;
                    while self
                        .bytes
                        .get(p)
                        .is_some_and(|b| b.is_ascii_hexdigit() || *b == b'_')
                    {
                        p += 1;
                    }
                    if self.bytes.get(p) != Some(&b'}') {
                        return None;
                    }
                    p += 1;
                }
                Some(b'x') => p = (p + 3).min(self.bytes.len()),
                Some(_) => p = self.char_after(p),
                None => return None,
            }
        } else {
            p = self.char_after(p);
        }
        (self.bytes.get(p) == Some(&b'\'')).then_some(p + 1)
    }

    pub(super) fn cpp_raw(&self, start: usize) -> Option<usize> {
        let mut p = start + 2;
        while p < self.bytes.len() && p - start <= 18 && self.bytes[p] != b'(' {
            if self.bytes[p].is_ascii_whitespace() || matches!(self.bytes[p], b'\\' | b')') {
                return None;
            }
            p += 1;
        }
        if self.bytes.get(p) != Some(&b'(') {
            return None;
        }
        let delimiter = &self.bytes[start + 2..p];
        p += 1;
        while p < self.bytes.len() {
            if self.bytes[p] == b')'
                && self.bytes[p + 1..].starts_with(delimiter)
                && self.bytes.get(p + 1 + delimiter.len()) == Some(&b'"')
            {
                return Some(p + 2 + delimiter.len());
            }
            p += 1;
        }
        Some(p)
    }

    pub(super) fn regex_end(&self, start: usize) -> usize {
        let mut p = start + 1;
        let mut class = false;
        while p < self.bytes.len() && !matches!(self.bytes[p], b'\n' | b'\r') {
            match self.bytes[p] {
                b'\\' => p = (p + 2).min(self.bytes.len()),
                b'[' => {
                    class = true;
                    p += 1;
                }
                b']' => {
                    class = false;
                    p += 1;
                }
                b'/' if !class => {
                    p += 1;
                    while self.bytes.get(p).is_some_and(u8::is_ascii_alphabetic) {
                        p += 1;
                    }
                    return p;
                }
                _ => p += 1,
            }
        }
        p
    }

    pub(super) fn sigil(&self, start: usize) -> Option<usize> {
        if !self
            .bytes
            .get(start + 1)
            .is_some_and(u8::is_ascii_alphabetic)
        {
            return None;
        }
        let p = start + 2;
        let &open = self.bytes.get(p)?;
        if matches!(open, b'\'' | b'"') {
            let count = if self.bytes[p..].starts_with(&[open, open, open]) {
                3
            } else {
                1
            };
            return Some(self.quoted(p, count, true, false));
        }
        let close = match open {
            b'(' => b')',
            b'[' => b']',
            b'{' => b'}',
            b'<' => b'>',
            b'/' | b'|' => open,
            _ => return None,
        };
        let mut depth = 1;
        let mut p = p + 1;
        while p < self.bytes.len() {
            let b = self.bytes[p];
            if b == b'\\' {
                p = (p + 2).min(self.bytes.len());
                continue;
            }
            p += 1;
            if b == close {
                depth -= 1;
                if depth == 0 {
                    return Some(p);
                }
            } else if open != close && b == open {
                depth += 1;
            }
        }
        Some(p)
    }

    pub(super) fn shell_parameter(&self, start: usize) -> usize {
        let mut depth = 1;
        let mut p = start + 2;
        while p < self.bytes.len() {
            match self.bytes[p] {
                b'\\' => p = (p + 2).min(self.bytes.len()),
                b'\'' | b'"' => p = self.quoted(p, 1, self.bytes[p] == b'"', false),
                b'{' => {
                    depth += 1;
                    p += 1;
                }
                b'}' => {
                    depth -= 1;
                    p += 1;
                    if depth == 0 {
                        return p;
                    }
                }
                _ => p += 1,
            }
        }
        p
    }

    pub(super) fn heredoc(&self, start: usize) -> Option<(usize, Heredoc)> {
        let mut p = start + 2;
        let tabs = self.bytes.get(p) == Some(&b'-');
        if tabs {
            p += 1;
        }
        while matches!(self.bytes.get(p), Some(b' ' | b'\t')) {
            p += 1;
        }
        let mut delimiter = Vec::new();
        while let Some(&b) = self.bytes.get(p) {
            if b.is_ascii_whitespace()
                || matches!(b, b';' | b'|' | b'&' | b'<' | b'>' | b'(' | b')')
            {
                break;
            }
            if matches!(b, b'\'' | b'"') {
                let end = self.quoted(p, 1, b == b'"', false);
                if self.bytes.get(end.wrapping_sub(1)) != Some(&b) {
                    return None;
                }
                delimiter.extend_from_slice(&self.bytes[p + 1..end - 1]);
                p = end;
            } else if b == b'\\' {
                p += 1;
                if let Some(&escaped) = self.bytes.get(p) {
                    delimiter.push(escaped);
                    p += 1;
                }
            } else {
                delimiter.push(b);
                p += 1;
            }
        }
        // `<<2)` is a shift inside an expression, not a heredoc word.
        if delimiter.is_empty() || self.bytes.get(p) == Some(&b')') {
            None
        } else {
            Some((p, Heredoc { delimiter, tabs }))
        }
    }
}
