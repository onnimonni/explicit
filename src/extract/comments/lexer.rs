//! Byte-level comment lexer shared by all supported languages.

mod scan;

use super::python::Python;
use super::{CommentBlock, CommentKind, CommentLine};
use crate::source::Lang;

// Byte sets kept as constants so function bodies have no unbalanced brace literals.
/// Opens an interpolation in Shell, Nix and JavaScript strings.
const INTERPOLATION_OPEN: &[u8] = b"${";
/// Bytes after which a YAML quote starts a quoted scalar.
const YAML_FLOW_INDICATORS: &[u8] = b":[{,-";
/// Bytes after which a `/` starts a regex literal, not a division.
const REGEX_PRECEDING_PUNCTUATION: &[u8] = b"([{,=:;!?&|+-*%^~<>/";

#[derive(Clone, Copy)]
pub(super) struct Spec {
    slash: bool,
    hash: bool,
    block: bool,
    nested: bool,
    backtick: bool,
    triple: bool,
}

// Shared delimiters live in this table; special literal forms are recognized
// before the common quote handling below.
pub(super) const SPECS: &[(Lang, Spec)] = &[
    (Lang::Rust, Spec::new(true, false, true, true, false, false)),
    (Lang::Go, Spec::new(true, false, true, false, true, false)),
    (
        Lang::Python,
        Spec::new(false, true, false, false, false, true),
    ),
    (
        Lang::JavaScript,
        Spec::new(true, false, true, false, true, false),
    ),
    (
        Lang::TypeScript,
        Spec::new(true, false, true, false, true, false),
    ),
    (
        Lang::Shell,
        Spec::new(false, true, false, false, true, false),
    ),
    (Lang::Nix, Spec::new(false, true, true, false, false, false)),
    (
        Lang::Elixir,
        Spec::new(false, true, false, false, false, true),
    ),
    (
        Lang::Zig,
        Spec::new(true, false, false, false, false, false),
    ),
    (Lang::C, Spec::new(true, false, true, false, false, false)),
    (Lang::Cpp, Spec::new(true, false, true, false, false, false)),
    (
        Lang::Ruby,
        Spec::new(false, true, false, false, true, false),
    ),
    (Lang::Java, Spec::new(true, false, true, false, false, true)),
    (
        Lang::CSharp,
        Spec::new(true, false, true, false, false, true),
    ),
    (Lang::Php, Spec::new(true, true, true, false, true, false)),
    (
        Lang::Toml,
        Spec::new(false, true, false, false, false, true),
    ),
    (
        Lang::Yaml,
        Spec::new(false, true, false, false, false, false),
    ),
];

impl Spec {
    const fn new(
        slash: bool,
        hash: bool,
        block: bool,
        nested: bool,
        backtick: bool,
        triple: bool,
    ) -> Self {
        Self {
            slash,
            hash,
            block,
            nested,
            backtick,
            triple,
        }
    }
}

#[derive(Clone, Copy)]
enum Mode {
    Code(Option<usize>),
    Template,
    NixQuote,
    NixIndented,
}

struct Found {
    block: CommentBlock,
    end_line: usize,
    line_comment: bool,
}

struct Heredoc {
    delimiter: Vec<u8>,
    tabs: bool,
}

pub(super) struct Lexer<'a> {
    lang: Lang,
    spec: Spec,
    src: &'a str,
    bytes: &'a [u8],
    pos: usize,
    line: usize,
    starts: Vec<usize>,
    code: Vec<bool>,
    found: Vec<Found>,
    modes: Vec<Mode>,
    regex_allowed: bool,
    shell_word: bool,
    indents: Vec<usize>,
    heredocs: std::collections::VecDeque<Heredoc>,
    /// Open parentheses inside shell arithmetic `((...))`; 0 outside.
    arith: usize,
    /// Parent indent of a YAML block scalar whose body starts on the next line.
    yaml_block: Option<usize>,
    python: Python,
}

impl<'a> Lexer<'a> {
    pub(super) fn new(lang: Lang, src: &'a str) -> Self {
        let mut starts = vec![0];
        starts.extend(src.match_indices('\n').map(|(p, _)| p + 1));
        let indents = starts
            .iter()
            .map(|&start| {
                let mut indent = 0;
                for &b in &src.as_bytes()[start..] {
                    match b {
                        b' ' => indent += 1,
                        b'\t' => indent = (indent / 8 + 1) * 8,
                        _ => break,
                    }
                }
                indent
            })
            .collect();
        Self {
            lang,
            spec: SPECS
                .iter()
                .find(|(l, _)| *l == lang)
                .expect("every language has a spec")
                .1,
            src,
            bytes: src.as_bytes(),
            pos: 0,
            line: 0,
            code: vec![false; starts.len()],
            starts,
            found: Vec::new(),
            modes: vec![Mode::Code(None)],
            regex_allowed: true,
            shell_word: false,
            indents,
            heredocs: std::collections::VecDeque::new(),
            arith: 0,
            yaml_block: None,
            python: Python::new(),
        }
    }

    fn at(&self, text: &[u8]) -> bool {
        self.bytes[self.pos..].starts_with(text)
    }

    // Consumed bytes pass here once, including opaque literal bodies.
    fn advance(&mut self, end: usize, code: bool) {
        if code {
            self.shell_word = true;
        }
        for &byte in &self.bytes[self.pos..end] {
            if byte == b'\n' {
                self.line += 1;
            } else if code && !byte.is_ascii_whitespace() {
                self.code[self.line] = true;
            }
        }
        self.pos = end;
    }

    fn line_end(&self, start: usize) -> usize {
        let end = self.bytes[start..]
            .iter()
            .position(|&b| b == b'\n')
            .map_or(self.bytes.len(), |n| start + n);
        if end > start && self.bytes[end - 1] == b'\r' {
            end - 1
        } else {
            end
        }
    }

    pub(super) fn run(mut self) -> Vec<CommentBlock> {
        while self.pos < self.bytes.len() {
            self.step();
        }
        self.finish()
    }

    /// Consume one token (or string fragment) at `self.pos`.
    fn step(&mut self) {
        if !matches!(self.modes.last(), Some(Mode::Code(_))) {
            self.string_fragment();
            return;
        }
        let p = self.pos;
        let b = self.bytes[p];
        if self.whitespace(p, b)
            || self.comment(p, b)
            || self.language_literal(p, b)
            || self.quote_literal(p, b)
            || self.identifier(p, b)
        {
            return;
        }
        self.punctuation(p, b);
    }

    fn whitespace(&mut self, p: usize, b: u8) -> bool {
        if b == b'\n' {
            self.shell_word = false;
            self.advance(p + 1, false);
            if self.lang == Lang::Python {
                self.python.newline();
            }
            self.consume_heredocs();
            self.consume_yaml_block();
            self.shell_word = false;
            return true;
        }
        if b.is_ascii_whitespace() {
            self.shell_word = false;
            self.advance(p + 1, false);
            return true;
        }
        false
    }

    fn comment(&mut self, p: usize, b: u8) -> bool {
        if p == 0 && self.at(b"#!") {
            self.advance(self.line_end(p), false);
            return true;
        }
        if self.lang == Lang::Ruby
            && p == self.starts[self.line]
            && self.at(b"=begin")
            && self.word_end(p + 6)
        {
            self.ruby_block();
            return true;
        }
        if self.spec.slash && self.at(b"//") {
            let doc = self.at(b"///") || self.at(b"//!");
            let kind = if doc {
                CommentKind::Doc
            } else {
                CommentKind::Line
            };
            self.emit(self.line_end(p), kind, if doc { 3 } else { 2 }, 0, true);
            return true;
        }
        if self.spec.hash && b == b'#' && self.hash_allowed() {
            self.emit(self.line_end(p), CommentKind::Line, 1, 0, true);
            return true;
        }
        if self.spec.block && self.at(b"/*") {
            self.block_comment(p);
            return true;
        }
        false
    }

    fn block_comment(&mut self, p: usize) {
        let doc = self.at(b"/**") || self.at(b"/*!");
        let (end, closed) = self.block_end(p);
        // /**/ shares its middle star between the delimiters.
        let opening = if doc && !self.at(b"/**/") { 3 } else { 2 };
        let kind = if doc {
            CommentKind::Doc
        } else {
            CommentKind::Block
        };
        self.emit(end, kind, opening, if closed { 2 } else { 0 }, false);
    }

    /// Literal forms specific to one language, recognized before common quotes.
    fn language_literal(&mut self, p: usize, b: u8) -> bool {
        match self.lang {
            Lang::Elixir => self.elixir_literal(p, b),
            Lang::Zig => self.skip_if(self.at(b"\\\\").then(|| self.line_end(p))),
            Lang::Rust => self.rust_literal(p, b),
            Lang::Cpp => {
                let raw = (b == b'R' && self.bytes.get(p + 1) == Some(&b'"'))
                    .then(|| self.cpp_raw(p))
                    .flatten();
                self.skip_if(raw)
            }
            Lang::Shell => self.shell_literal(p, b),
            Lang::Nix => self.nix_string(p, b),
            Lang::JavaScript | Lang::TypeScript => self.js_literal(p, b),
            Lang::CSharp => self.csharp_verbatim(p),
            Lang::Yaml => self.yaml_block_header(p, b),
            _ => false,
        }
    }

    /// Advance over code up to `end` when a literal was found.
    fn skip_if(&mut self, end: Option<usize>) -> bool {
        let Some(end) = end else { return false };
        self.advance(end, true);
        true
    }

    fn elixir_literal(&mut self, p: usize, b: u8) -> bool {
        if (self.at(b"@doc") || self.at(b"@moduledoc")) && self.elixir_doc() {
            return true;
        }
        if b == b'~'
            && let Some(end) = self.sigil(p)
        {
            self.advance(end, true);
            return true;
        }
        if b != b'?' {
            return false;
        }
        let Some(ch) = self.src[p + 1..].chars().next() else {
            return false;
        };
        let mut end = p + 1 + ch.len_utf8();
        if ch == '\\'
            && let Some(escaped) = self.src[end..].chars().next()
        {
            end += escaped.len_utf8();
        }
        self.advance(end, true);
        true
    }

    fn rust_literal(&mut self, p: usize, b: u8) -> bool {
        if let Some(end) = self.rust_raw(p) {
            self.advance(end, true);
            return true;
        }
        if b != b'\'' {
            return false;
        }
        let end = self.rust_char(p).unwrap_or(p + 1);
        self.advance(end, true);
        true
    }

    fn shell_literal(&mut self, p: usize, b: u8) -> bool {
        if self.at(b"<<<") {
            self.advance(p + 3, true);
            self.shell_word = false;
            return true;
        }
        // Arithmetic `$(( 1 << 2 ))` / `(( x <<= 1 ))`: `<<` is a shift, not a heredoc.
        if self.arith > 0 && matches!(b, b'(' | b')') {
            if b == b'(' {
                self.arith += 1;
            } else {
                self.arith -= 1;
            }
            self.advance(p + 1, true);
            self.shell_word = false;
            return true;
        }
        if self.arith == 0 && self.at(b"((") {
            self.arith = 2;
            self.advance(p + 2, true);
            self.shell_word = false;
            return true;
        }
        if self.arith == 0
            && self.at(b"<<")
            && let Some((end, here)) = self.heredoc(p)
        {
            self.heredocs.push_back(here);
            self.advance(end, true);
            return true;
        }
        if self.at(INTERPOLATION_OPEN) {
            let end = self.shell_parameter(p);
            self.advance(end, true);
            return true;
        }
        if b != b'\\' {
            return false;
        }
        let was_word = self.shell_word;
        self.advance(self.char_after(p + 1), true);
        if self.bytes.get(p + 1) == Some(&b'\n') {
            self.shell_word = was_word;
        }
        true
    }

    fn nix_string(&mut self, p: usize, b: u8) -> bool {
        let (mode, len) = if b == b'"' {
            (Mode::NixQuote, 1)
        } else if self.at(b"''") {
            (Mode::NixIndented, 2)
        } else {
            return false;
        };
        self.advance(p + len, true);
        self.modes.push(mode);
        true
    }

    fn js_literal(&mut self, p: usize, b: u8) -> bool {
        if b == b'`' {
            self.advance(p + 1, true);
            self.modes.push(Mode::Template);
            return true;
        }
        if b == b'/' && self.regex_allowed {
            let end = self.regex_end(p);
            self.advance(end, true);
            self.regex_allowed = false;
            return true;
        }
        false
    }

    fn csharp_verbatim(&mut self, p: usize) -> bool {
        let prefix = if self.at(b"$@\"") || self.at(b"@$\"") {
            2
        } else if self.at(b"@\"") {
            1
        } else {
            return false;
        };
        let end = self.quoted(p + prefix, 1, false, true);
        self.advance(end, true);
        true
    }

    /// Position of the opening quote after a Python string prefix (`r`, `b`, `f`, ...),
    /// and whether the prefix still allows a docstring.
    fn python_string_prefix(&self, p: usize, b: u8) -> (usize, bool) {
        if self.lang != Lang::Python || !b.is_ascii_alphabetic() {
            return (p, true);
        }
        let mut quote = p;
        let mut doc_prefix = true;
        while quote < self.bytes.len()
            && matches!(
                self.bytes[quote],
                b'r' | b'R' | b'u' | b'U' | b'f' | b'F' | b'b' | b'B'
            )
        {
            doc_prefix &= !matches!(self.bytes[quote], b'f' | b'F' | b'b' | b'B');
            quote += 1;
        }
        if quote - p > 2 || !matches!(self.bytes.get(quote), Some(b'\'' | b'"')) {
            quote = p;
        }
        (quote, doc_prefix)
    }

    fn quote_literal(&mut self, p: usize, b: u8) -> bool {
        let (quote, doc_prefix) = self.python_string_prefix(p, b);
        let q = self.bytes[quote];
        let is_quote = (q == b'"' || (q == b'\'' && self.lang != Lang::Nix))
            || (q == b'`' && self.spec.backtick);
        if !is_quote || (self.lang == Lang::Yaml && !self.yaml_quote_allowed()) {
            return false;
        }
        let count = if self.spec.triple && self.bytes[quote..].starts_with(&[q, q, q]) {
            3
        } else {
            1
        };
        let escapes = !(q == b'`' && self.lang == Lang::Go
            || q == b'\'' && matches!(self.lang, Lang::Shell | Lang::Toml | Lang::Yaml));
        let end = self.quoted(quote, count, escapes, self.lang == Lang::Yaml && q == b'\'');
        let doc = self.lang == Lang::Python
            && self.python_token(&[])
            && doc_prefix
            && self.python_standalone(end);
        if doc {
            let closed =
                end >= quote + count * 2 && self.bytes[end - count..end].iter().all(|&c| c == q);
            self.emit(
                end,
                CommentKind::Docstring,
                quote - p + count,
                if closed { count } else { 0 },
                false,
            );
        } else {
            self.advance(end, true);
        }
        self.regex_allowed = false;
        true
    }

    fn identifier(&mut self, p: usize, b: u8) -> bool {
        if !(b.is_ascii_alphabetic() || b == b'_' || b == b'$' || b >= 128) {
            return false;
        }
        let mut end = p + 1;
        while end < self.bytes.len()
            && (self.bytes[end].is_ascii_alphanumeric()
                || matches!(self.bytes[end], b'_' | b'$')
                || self.bytes[end] >= 128)
        {
            // Leave a prefixed C++ raw string opener visible.
            if self.lang == Lang::Cpp
                && self.bytes[end] == b'R'
                && self.bytes.get(end + 1) == Some(&b'"')
            {
                break;
            }
            end += 1;
        }
        if self.lang == Lang::Python {
            self.python_token(&self.bytes[p..end]);
        }
        self.regex_allowed = regex_follows_keyword(&self.bytes[p..end]);
        self.advance(end, true);
        true
    }

    fn punctuation(&mut self, p: usize, b: u8) {
        if self.lang == Lang::Python {
            self.python_token(&[]);
            self.python.punctuation(b);
        }
        self.track_interpolation_brace(b);
        self.regex_allowed = regex_follows_punctuation(b);
        self.advance(p + 1, true);
        if self.lang == Lang::Shell && matches!(b, b';' | b'|' | b'&' | b'(' | b')' | b'<' | b'>') {
            self.shell_word = false;
        }
    }

    /// Balance braces inside `${...}` so the closing brace ends the interpolation.
    fn track_interpolation_brace(&mut self, b: u8) {
        let Some(Mode::Code(Some(depth))) = self.modes.last_mut() else {
            return;
        };
        if b == b'{' {
            *depth += 1;
        } else if b == b'}' {
            if *depth == 0 {
                self.modes.pop();
            } else {
                *depth -= 1;
            }
        }
    }

    fn word_end(&self, p: usize) -> bool {
        self.bytes.get(p).is_none_or(|b| b.is_ascii_whitespace())
    }

    fn char_after(&self, p: usize) -> usize {
        p + self.src[p..].chars().next().map_or(0, char::len_utf8)
    }

    fn hash_allowed(&self) -> bool {
        match self.lang {
            Lang::Shell => !self.shell_word,
            // PHP 8 attributes `#[Route(...)]` are code.
            Lang::Php => self.bytes.get(self.pos + 1) != Some(&b'['),
            Lang::Yaml => self.pos == 0 || self.bytes[self.pos - 1].is_ascii_whitespace(),
            _ => true,
        }
    }

    fn yaml_quote_allowed(&self) -> bool {
        self.pos == 0
            || self.bytes[self.pos - 1].is_ascii_whitespace()
            || YAML_FLOW_INDICATORS.contains(&self.bytes[self.pos - 1])
    }

    fn python_token(&mut self, word: &[u8]) -> bool {
        self.python.token(self.indents[self.line], word)
    }

    fn python_standalone(&self, mut p: usize) -> bool {
        while matches!(self.bytes.get(p), Some(b' ' | b'\t' | b'\r')) {
            p += 1;
        }
        self.bytes
            .get(p)
            .is_none_or(|b| matches!(b, b'\n' | b'#' | b';'))
    }

    fn string_fragment(&mut self) {
        let p = self.pos;
        let mode = *self.modes.last().expect("base code mode is never popped");
        let indented = matches!(mode, Mode::NixIndented);
        if indented && (self.at(b"'''") || self.at(b"''$")) {
            self.advance(p + 3, true);
        } else if indented && self.at(b"''\\") {
            self.advance(self.char_after(p + 3), true);
        } else if !indented && self.bytes[p] == b'\\' {
            self.advance(self.char_after(p + 1), true);
        } else if self.at(INTERPOLATION_OPEN) {
            self.advance(p + 2, true);
            self.modes.push(Mode::Code(Some(0)));
            self.regex_allowed = true;
        } else if (matches!(mode, Mode::Template) && self.bytes[p] == b'`')
            || (matches!(mode, Mode::NixQuote) && self.bytes[p] == b'"')
            || (indented && self.at(b"''"))
        {
            self.advance(p + if indented { 2 } else { 1 }, true);
            self.modes.pop();
            self.regex_allowed = false;
        } else {
            self.advance(self.char_after(p), true);
        }
    }

    fn elixir_doc(&mut self) -> bool {
        let p = self.pos;
        let marker = if self.at(b"@moduledoc") { 10 } else { 4 };
        let mut quote = p + marker;
        if !self.word_end(quote) {
            return false;
        }
        while self.bytes.get(quote).is_some_and(u8::is_ascii_whitespace) {
            quote += 1;
        }
        if !self.bytes[quote..].starts_with(b"\"\"\"") {
            return false;
        }
        let end = self.quoted(quote, 3, true, false);
        let closed = end >= quote + 6 && self.bytes[end - 3..end] == *b"\"\"\"";
        self.emit(
            end,
            CommentKind::Doc,
            quote - p + 3,
            if closed { 3 } else { 0 },
            false,
        );
        true
    }

    /// `key: |`, `- >-`, `key: &a |2`: the header of a YAML block scalar.
    fn yaml_block_header(&mut self, p: usize, b: u8) -> bool {
        if !matches!(b, b'|' | b'>') {
            return false;
        }
        let line_start = self.starts[self.line];
        let before = &self.bytes[line_start..p];
        if before.last().is_some_and(|c| !c.is_ascii_whitespace()) {
            return false;
        }
        // Previous token must end a key / sequence entry, or be an anchor/tag.
        let prev = before
            .split(u8::is_ascii_whitespace)
            .rfind(|w| !w.is_empty());
        let header_ok = match prev {
            None => true,
            Some(w) => {
                matches!(w.last(), Some(b':' | b'-')) || matches!(w.first(), Some(b'&' | b'!'))
            }
        };
        if !header_ok {
            return false;
        }
        let mut end = p + 1;
        while self
            .bytes
            .get(end)
            .is_some_and(|c| matches!(c, b'-' | b'+') || c.is_ascii_digit())
        {
            end += 1;
        }
        let mut q = end;
        while matches!(self.bytes.get(q), Some(b' ' | b'\t')) {
            q += 1;
        }
        let rest_ok = match self.bytes.get(q) {
            None | Some(b'\n' | b'\r') => true,
            Some(b'#') => q > end,
            _ => false,
        };
        if !rest_ok {
            return false;
        }
        self.yaml_block = Some(self.indents[self.line]);
        self.advance(end, true);
        true
    }

    /// Skip block scalar body lines: blank or indented deeper than the parent.
    fn consume_yaml_block(&mut self) {
        let Some(parent) = self.yaml_block.take() else {
            return;
        };
        while self.pos < self.bytes.len() {
            let end = self.line_end(self.pos);
            let blank = self.bytes[self.pos..end]
                .iter()
                .all(u8::is_ascii_whitespace);
            if !blank && self.indents[self.line] <= parent {
                break;
            }
            let next = self.bytes[end..]
                .iter()
                .position(|&c| c == b'\n')
                .map_or(self.bytes.len(), |n| end + n + 1);
            self.advance(next, !blank);
        }
    }

    fn consume_heredocs(&mut self) {
        while let Some(here) = self.heredocs.pop_front() {
            while self.pos < self.bytes.len() {
                let end = self.line_end(self.pos);
                let mut start = self.pos;
                if here.tabs {
                    while self.bytes.get(start) == Some(&b'\t') {
                        start += 1;
                    }
                }
                let done = self.bytes[start..end] == here.delimiter;
                let next = if self.bytes.get(end) == Some(&b'\r') {
                    end + 1
                } else {
                    end
                };
                let next = if self.bytes.get(next) == Some(&b'\n') {
                    next + 1
                } else {
                    next
                };
                self.advance(next, true);
                if done {
                    break;
                }
            }
        }
    }

    fn ruby_block(&mut self) {
        let start = self.pos;
        let mut p = self.line_end(start);
        let mut closing = 0;
        let mut end = self.bytes.len();
        while p < self.bytes.len() {
            if self.bytes[p] == b'\n' {
                p += 1;
                if self.bytes[p..].starts_with(b"=end") && self.word_end(p + 4) {
                    end = self.line_end(p);
                    closing = end - p;
                    break;
                }
            } else {
                p += 1;
            }
        }
        self.emit(end, CommentKind::Block, 6, closing, false);
    }

    fn emit(
        &mut self,
        end: usize,
        kind: CommentKind,
        opening: usize,
        closing: usize,
        line_comment: bool,
    ) {
        let start = self.pos;
        let start_line = self.line;
        let trailing = self.code[start_line];
        let line_comment = line_comment
            && self.bytes[self.starts[start_line]..start]
                .iter()
                .all(u8::is_ascii_whitespace);
        let body_start = start + opening;
        let body_end = end.saturating_sub(closing).max(body_start);
        let decorated = self.bytes[start..].starts_with(b"/*");
        let mut lines = Vec::new();
        let mut p = start;
        loop {
            let newline = self.bytes[p..end]
                .iter()
                .position(|&b| b == b'\n')
                .map_or(end, |n| p + n);
            let raw_end = if newline > p && self.bytes[newline - 1] == b'\r' {
                newline - 1
            } else {
                newline
            };
            let mut lo = p.max(body_start).min(raw_end);
            let hi = raw_end.min(body_end).max(lo);
            let mut strip_space = body_start >= p && body_start <= raw_end;
            if decorated && p > start {
                let mut star = lo;
                while star < hi && matches!(self.bytes[star], b' ' | b'\t') {
                    star += 1;
                }
                if star < hi && self.bytes[star] == b'*' {
                    lo = star + 1;
                    strip_space = true;
                } else if self.bytes[lo..hi].iter().all(u8::is_ascii_whitespace) {
                    lo = hi;
                }
            }
            if strip_space && lo < hi && self.bytes[lo] == b' ' {
                lo += 1;
            }
            lines.push(CommentLine {
                raw: p..raw_end,
                content: lo..hi,
            });
            if newline == end {
                break;
            }
            p = newline + 1;
        }
        self.advance(end, false);
        if line_comment
            && !trailing
            && let Some(last) = self.found.last_mut()
            && last.line_comment
            && !last.block.trailing
            && last.block.kind == kind
            && last.end_line + 1 == start_line
            && self.bytes[last.block.range.end..start]
                .iter()
                .all(u8::is_ascii_whitespace)
        {
            last.block.range.end = end;
            last.block.lines.extend(lines);
            last.end_line = self.line;
            return;
        }
        self.found.push(Found {
            block: CommentBlock {
                range: start..end,
                kind,
                lines,
                start_line: start_line + 1,
                next_code_line: None,
                trailing,
            },
            end_line: self.line,
            line_comment,
        });
    }

    fn finish(mut self) -> Vec<CommentBlock> {
        let mut next = None;
        let mut next_code = vec![None; self.starts.len()];
        for line in (0..self.starts.len()).rev() {
            next_code[line] = next;
            if self.code[line] {
                next = Some(line);
            }
        }
        for found in &mut self.found {
            if let Some(line) = next_code[found.end_line] {
                let end = self.starts.get(line + 1).copied().unwrap_or(self.src.len());
                found.block.next_code_line =
                    Some(self.src[self.starts[line]..end].trim().to_owned());
            }
        }
        self.found.into_iter().map(|f| f.block).collect()
    }
}

/// After these keywords a `/` starts a regex literal, not a division.
fn regex_follows_keyword(word: &[u8]) -> bool {
    matches!(
        word,
        b"return"
            | b"throw"
            | b"case"
            | b"delete"
            | b"void"
            | b"typeof"
            | b"new"
            | b"in"
            | b"of"
            | b"yield"
            | b"await"
            | b"instanceof"
            | b"else"
            | b"do"
    )
}

/// After these punctuation bytes a `/` starts a regex literal, not a division.
fn regex_follows_punctuation(b: u8) -> bool {
    REGEX_PRECEDING_PUNCTUATION.contains(&b)
}
