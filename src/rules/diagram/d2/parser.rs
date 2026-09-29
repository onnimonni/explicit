//! Scannerless recursive-descent D2 parser.
//!
//! Mirrors `d2parser/parse.go` (terrastruct/d2 @ 01bc7ecd) closely so that syntax
//! accepted by D2 is accepted here: the grammar is context sensitive (unquoted
//! strings end on different characters in keys vs values), which is why there
//! is no separate token stream. All ranges are byte offsets into the body.

use std::ops::Range;

use crate::diagnostic::Severity;

#[derive(Debug, Clone)]
pub struct Diag {
    pub range: Range<usize>,
    pub severity: Severity,
    pub msg: String,
    pub help: Option<String>,
    pub suggest: Option<String>,
    /// Safe replacement for `range`.
    pub fix: Option<String>,
}

impl Diag {
    pub fn error(range: Range<usize>, msg: impl Into<String>) -> Self {
        Diag {
            range,
            severity: Severity::Error,
            msg: msg.into(),
            help: None,
            suggest: None,
            fix: None,
        }
    }
    pub fn warning(range: Range<usize>, msg: impl Into<String>) -> Self {
        Diag {
            severity: Severity::Warning,
            ..Diag::error(range, msg)
        }
    }
    pub fn help(mut self, h: impl Into<String>) -> Self {
        self.help = Some(h.into());
        self
    }
    pub fn suggest(mut self, s: Option<&str>) -> Self {
        self.suggest = s.map(str::to_string);
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrKind {
    Unquoted,
    Double,
    Single,
    Block,
}

#[derive(Debug, Clone)]
pub struct Subst {
    pub path: Vec<String>,
    pub range: Range<usize>,
}

#[derive(Debug, Clone)]
pub struct Str {
    /// Decoded text (substitutions removed).
    pub text: String,
    pub range: Range<usize>,
    pub kind: StrKind,
    pub substs: Vec<Subst>,
}

impl Str {
    pub fn unquoted(&self) -> bool {
        self.kind == StrKind::Unquoted
    }
    pub fn is_glob(&self) -> bool {
        self.unquoted() && self.text.contains('*')
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScalarKind {
    Null,
    Bool,
    Number,
    Suspension,
    Str,
}

#[derive(Debug, Clone)]
pub struct Scalar {
    pub kind: ScalarKind,
    pub s: Str,
}

#[derive(Debug, Clone)]
pub enum Value {
    Scalar(Scalar),
    Array(Array),
    Map(Map),
    Import(Range<usize>),
}

impl Value {
    pub fn range(&self) -> Range<usize> {
        match self {
            Value::Scalar(s) => s.s.range.clone(),
            Value::Array(a) => a.range.clone(),
            Value::Map(m) => m.range.clone(),
            Value::Import(r) => r.clone(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Array {
    pub range: Range<usize>,
    pub items: Vec<Value>,
    pub substs: Vec<Subst>,
}

#[derive(Debug, Clone, Default)]
pub struct Map {
    pub range: Range<usize>,
    pub nodes: Vec<Node>,
}

#[derive(Debug, Clone)]
pub enum Node {
    Key(Box<Key>),
    SpreadSubst(Subst),
    SpreadImport,
}

#[derive(Debug, Clone)]
pub struct KeyPath {
    pub parts: Vec<Str>,
}

#[derive(Debug, Clone, Default)]
pub struct Key {
    pub range: Range<usize>,
    /// `&` or `!&` filter.
    pub filter: bool,
    pub key: Option<KeyPath>,
    /// Number of connections in the key (`a -> b -> c` = 2).
    pub edges: usize,
    pub edge_key: Option<KeyPath>,
    pub colon: Option<usize>,
    pub primary: Option<Scalar>,
    pub value: Option<Value>,
}

impl Key {
    /// Primary scalar, else the value if it is a scalar.
    pub fn scalar(&self) -> Option<&Scalar> {
        self.primary.as_ref().or(match &self.value {
            Some(Value::Scalar(s)) => Some(s),
            _ => None,
        })
    }
    pub fn map(&self) -> Option<&Map> {
        match &self.value {
            Some(Value::Map(m)) => Some(m),
            _ => None,
        }
    }
}

enum Parsed {
    Nothing,
    Comment,
    BlockComment,
    Key(Box<Key>),
    Node(Node),
}

const MAX_DEPTH: usize = 256;

pub struct Parser<'a> {
    src: &'a str,
    pos: usize,
    depth: usize,
    in_edge_group: bool,
    pub diags: Vec<Diag>,
}

pub fn parse(src: &str) -> (Map, Vec<Diag>) {
    let mut p = Parser {
        src,
        pos: 0,
        depth: 0,
        in_edge_group: false,
        diags: Vec::new(),
    };
    let m = p.parse_map(true, 0);
    (m, p.diags)
}

fn char_at(s: &str, i: usize) -> Option<char> {
    s.get(i..).and_then(|r| r.chars().next())
}

impl Parser<'_> {
    fn at(&self, i: usize) -> Option<char> {
        char_at(self.src, i)
    }

    /// Skip whitespace from `from`; returns (next position, newlines skipped).
    fn skip_ws(&self, from: usize) -> (usize, usize) {
        let mut i = from;
        let mut nl = 0;
        while let Some(c) = self.at(i) {
            if !c.is_whitespace() {
                break;
            }
            if c == '\n' {
                nl += 1;
            }
            i += c.len_utf8();
        }
        (i, nl)
    }

    /// Next non-space char on the same line (None on EOF or newline).
    fn peek_same_line(&self) -> Option<(usize, char)> {
        let (q, nl) = self.skip_ws(self.pos);
        if nl > 0 {
            return None;
        }
        self.at(q).map(|c| (q, c))
    }

    fn line_end(&self, from: usize) -> usize {
        self.src[from..]
            .find('\n')
            .map_or(self.src.len(), |i| from + i)
    }

    fn err(&mut self, d: Diag) {
        self.diags.push(d);
    }

    fn parse_map(&mut self, is_file: bool, open: usize) -> Map {
        let mut m = Map {
            range: open..open,
            nodes: Vec::new(),
        };
        if !is_file {
            self.depth += 1;
            if self.depth > MAX_DEPTH {
                self.err(Diag::error(open..open + 1, "D2 maps are nested too deeply"));
                self.pos = self.src.len();
                self.depth -= 1;
                return m;
            }
        }
        loop {
            let (q, _) = self.skip_ws(self.pos);
            self.pos = q;
            let Some(c) = self.at(q) else {
                if !is_file {
                    self.err(
                        Diag::error(open..open + 1, "unclosed `{`")
                            .help("maps must be terminated with `}`"),
                    );
                }
                break;
            };
            match c {
                ';' => {
                    self.pos += 1;
                    continue;
                }
                '}' => {
                    self.pos += 1;
                    if is_file {
                        self.err(Diag::error(q..q + 1, "unexpected `}` without matching `{`"));
                        continue;
                    }
                    break;
                }
                _ => {}
            }
            let before = self.diags.len();
            let parsed = self.parse_map_node(c);
            let what = match &parsed {
                Parsed::BlockComment => continue,
                Parsed::Nothing => None,
                Parsed::Comment => Some("comment"),
                Parsed::Key(k) => Some(match &k.value {
                    Some(Value::Scalar(s)) => match s.s.kind {
                        StrKind::Block => "block string",
                        StrKind::Double => "double quoted string",
                        StrKind::Single => "single quoted string",
                        StrKind::Unquoted => "value",
                    },
                    Some(Value::Map(_)) => "map",
                    Some(Value::Array(_)) => "array",
                    Some(Value::Import(_)) => "import",
                    None => "key",
                }),
                Parsed::Node(_) => Some("spread"),
            };
            match parsed {
                Parsed::Key(k) => m.nodes.push(Node::Key(k)),
                Parsed::Node(n) => m.nodes.push(n),
                _ => {}
            }
            self.trailing_garbage(what, &[';', '}', '#'], before);
        }
        if !is_file {
            self.depth -= 1;
        }
        m.range = open..self.pos;
        m
    }

    /// Consume and report text left on the line after a node.
    /// Only reported if the node itself produced no diagnostics (avoids cascades).
    fn trailing_garbage(&mut self, what: Option<&str>, stops: &[char], before: usize) {
        let after = self.pos;
        loop {
            let (q, nl) = self.skip_ws(self.pos);
            match self.at(q) {
                Some(c) if nl == 0 && !stops.contains(&c) => self.pos = q + c.len_utf8(),
                _ => break,
            }
        }
        if after == self.pos || self.diags.len() > before {
            return;
        }
        let start = self.skip_ws(after).0;
        let range = start..self.pos;
        let text = &self.src[range.clone()];
        let d = match (what, text.chars().next()) {
            (_, Some(']')) => Diag::error(start..start + 1, "unexpected `]` without matching `[`"),
            (_, Some(')')) => Diag::error(start..start + 1, "unexpected `)` without matching `(`"),
            (None, Some('>')) => Diag::error(range, "malformed connection")
                .help("connections are written `->`, `<-`, `<->` or `--` between two keys"),
            (Some("block string"), _) => Diag::error(range, "unexpected text after block string")
                .help("see https://d2lang.com/tour/text#advanced-block-strings"),
            (Some(w), _) => Diag::error(range, format!("unexpected text after {w}")),
            (None, _) => Diag::error(range, "invalid text beginning unquoted key"),
        };
        self.err(d);
    }

    fn parse_map_node(&mut self, c: char) -> Parsed {
        let start = self.pos;
        match c {
            '#' => {
                self.pos = self.line_end(start);
                return Parsed::Comment;
            }
            '"' if self.src[start..].starts_with("\"\"\"") => {
                return self.parse_block_comment(start);
            }
            '.' if self.src[start..].starts_with("...$") => {
                self.pos = start + 4;
                return match self.parse_substitution(start) {
                    Some(s) => Parsed::Node(Node::SpreadSubst(s)),
                    None => Parsed::Nothing,
                };
            }
            '.' if self.src[start..].starts_with("...@") => {
                self.pos = start + 4;
                self.parse_import_path();
                return Parsed::Node(Node::SpreadImport);
            }
            _ => {}
        }
        match self.parse_map_key() {
            Some(k) => Parsed::Key(Box::new(k)),
            None => Parsed::Nothing,
        }
    }

    fn parse_block_comment(&mut self, start: usize) -> Parsed {
        let body = start + 3;
        match self.src[body..].find("\"\"\"") {
            Some(i) => self.pos = body + i + 3,
            None => {
                self.err(
                    Diag::error(start..body, "unterminated block comment")
                        .help("block comments must be terminated with `\"\"\"`"),
                );
                self.pos = self.src.len();
            }
        }
        Parsed::BlockComment
    }

    fn parse_map_key(&mut self) -> Option<Key> {
        let start = self.pos;
        let mut mk = Key {
            range: start..start,
            ..Default::default()
        };
        if self.src[start..].starts_with("!&") {
            self.pos += 2;
            mk.filter = true;
        } else if self.src[start..].starts_with('&') {
            self.pos += 1;
            mk.filter = true;
        }
        if self.at(self.pos) == Some('(') {
            let open = self.pos;
            self.pos += 1;
            self.parse_edge_group(&mut mk, open);
        } else {
            let k = self.parse_key();
            match self.peek_same_line() {
                Some((q, '(')) => {
                    mk.key = k;
                    self.pos = q + 1;
                    self.parse_edge_group(&mut mk, q);
                }
                Some((_, '<' | '>' | '-')) => {
                    self.parse_edges(&mut mk, k.map(|k| k.parts[0].range.start));
                    self.parse_map_key_value(&mut mk);
                }
                _ => {
                    mk.key = k;
                    self.parse_map_key_value(&mut mk);
                }
            }
        }
        mk.range = start..self.pos;
        if mk.key.is_none() && mk.edges == 0 {
            return None;
        }
        Some(mk)
    }

    fn parse_map_key_value(&mut self, mk: &mut Key) {
        let Some((q, c)) = self.peek_same_line() else {
            return;
        };
        let keyless = mk.key.is_none() && mk.edges == 0;
        match c {
            '{' if keyless => return,
            '{' => {}
            ':' => {
                self.pos = q + 1;
                mk.colon = Some(q);
                if keyless {
                    self.err(Diag::error(
                        mk.range.start..self.pos,
                        "map value without key",
                    ));
                }
            }
            _ => return,
        }
        mk.value = self.parse_value();
        if mk.value.is_none() {
            let colon = mk.colon.unwrap_or(q);
            let mut d = Diag::error(colon..colon + 1, "missing value after colon");
            let (n, nl) = self.skip_ws(self.pos);
            if nl == 0 && self.at(n) == Some('#') {
                let word: String = self.src[n + 1..]
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric())
                    .collect();
                if !word.is_empty() && word.chars().all(|c| c.is_ascii_hexdigit()) {
                    d = d.help(format!(
                        "`#` starts a comment in D2; quote hex colors: \"#{word}\""
                    ));
                }
            }
            self.err(d);
            return;
        }
        if let Some(Value::Scalar(s)) = &mk.value
            && let Some((q, '{')) = self.peek_same_line()
        {
            let s = s.clone();
            self.pos = q;
            mk.primary = Some(s);
            mk.value = self.parse_value();
        }
    }

    fn parse_edge_group(&mut self, mk: &mut Key, open: usize) {
        self.in_edge_group = true;
        let src = self.parse_key();
        self.parse_edges(mk, src.map(|k| k.parts[0].range.start));
        match self.peek_same_line() {
            Some((q, ')')) => self.pos = q + 1,
            _ => {
                self.in_edge_group = false;
                self.err(
                    Diag::error(open..open + 1, "unclosed `(`")
                        .help("edge groups must be terminated with `)`"),
                );
                return;
            }
        }
        if let Some((q, '[')) = self.peek_same_line() {
            self.pos = q + 1;
            self.parse_edge_index(q);
        }
        if let Some((q, '.')) = self.peek_same_line() {
            self.pos = q + 1;
            mk.edge_key = self.parse_key();
        }
        self.in_edge_group = false;
        self.parse_map_key_value(mk);
    }

    fn parse_edge_index(&mut self, open: usize) {
        let unterminated = |p: &mut Self| {
            p.err(Diag::error(open..open + 1, "unterminated edge index").help("expected `]`"));
        };
        match self.peek_same_line() {
            None => return unterminated(self),
            Some((q, '*')) => self.pos = q + 1,
            Some((_, c)) if c.is_ascii_digit() => loop {
                match self.peek_same_line() {
                    None => return unterminated(self),
                    Some((_, ']')) => break,
                    Some((q, c)) => {
                        self.pos = q + c.len_utf8();
                        if !c.is_ascii_digit() {
                            self.err(Diag::error(
                                q..self.pos,
                                "unexpected character in edge index",
                            ));
                        }
                    }
                }
            },
            Some((q, c)) => {
                self.err(
                    Diag::error(q..q + c.len_utf8(), "unexpected character in edge index")
                        .help("edge indexes are integers or `*`, e.g. `(a -> b)[0]`"),
                );
                return;
            }
        }
        match self.peek_same_line() {
            Some((q, ']')) => self.pos = q + 1,
            _ => unterminated(self),
        }
    }

    fn parse_edges(&mut self, mk: &mut Key, mut src: Option<usize>) {
        loop {
            let Some((q, c)) = self.peek_same_line() else {
                return;
            };
            if !matches!(c, '<' | '*' | '-') {
                return;
            }
            let start = src.unwrap_or(q);
            if src.is_none() {
                self.err(Diag::error(q..q + 1, "connection missing source"));
            }
            self.pos = q + 1;
            if !self.parse_edge(start) {
                return;
            }
            let arrow_end = self.pos;
            match self.parse_key() {
                None => {
                    self.err(Diag::error(
                        start..arrow_end,
                        "connection missing destination",
                    ));
                    src = None;
                }
                Some(dst) => {
                    mk.edges += 1;
                    src = Some(dst.parts[0].range.start);
                }
            }
        }
    }

    fn parse_edge(&mut self, start: usize) -> bool {
        loop {
            match self.at(self.pos) {
                None => {
                    self.err(Diag::error(start..self.pos, "unterminated connection"));
                    return false;
                }
                Some('>' | '*') => {
                    self.pos += 1;
                    return true;
                }
                Some('\\') => {
                    self.pos += 1;
                    let (q, nl) = self.skip_ws(self.pos);
                    if self.at(q).is_none() {
                        continue;
                    }
                    if nl == 0 {
                        self.err(Diag::error(
                            start..self.pos,
                            "only newline escapes are allowed in connections",
                        ));
                        return false;
                    }
                    if nl == 1 {
                        self.pos = q;
                    }
                }
                Some('-') => self.pos += 1,
                Some(_) => return true,
            }
        }
    }

    fn parse_key(&mut self) -> Option<KeyPath> {
        let mut parts = Vec::new();
        while let Some((q, c)) = self.peek_same_line() {
            if c == '(' || c == '.' {
                break;
            }
            self.pos = q;
            let Some(s) = self.parse_string(true) else {
                break;
            };
            if s.unquoted() && s.text.starts_with('@') {
                let t = s.text.clone();
                self.err(Diag::error(
                    s.range.clone(),
                    format!("`{t}` is not a valid import, did you mean `...{t}`?"),
                ));
            }
            parts.push(s);
            match self.peek_same_line() {
                Some((q, '.')) => self.pos = q + 1,
                _ => break,
            }
        }
        (!parts.is_empty()).then_some(KeyPath { parts })
    }

    fn parse_string(&mut self, in_key: bool) -> Option<Str> {
        let (q, c) = self.peek_same_line()?;
        self.pos = q + c.len_utf8();
        match c {
            '"' => Some(self.parse_double(q, in_key)),
            '\'' => Some(self.parse_single(q)),
            '|' => Some(self.parse_block(q)),
            _ => {
                self.pos = q;
                self.parse_unquoted(in_key)
            }
        }
    }

    fn parse_unquoted(&mut self, in_key: bool) -> Option<Str> {
        let start = self.pos;
        let mut last = start;
        let mut text = String::new();
        let mut substs = Vec::new();
        if self.src[start..].starts_with("...@") {
            self.err(Diag::error(
                start..start + 4,
                "unquoted strings cannot begin with `...@` as that's import spread syntax",
            ));
        }
        while let Some(mut c) = self.at(self.pos) {
            if self.in_edge_group && c == ')' {
                let (q, nl) = self.skip_ws(self.pos + 1);
                match self.at(q) {
                    None => break,
                    _ if nl > 0 => break,
                    Some('#' | '{' | '}' | '[' | ']' | ':' | '.') => break,
                    _ => {}
                }
                text.push(')');
                self.pos += 1;
                last = self.pos;
                continue;
            }
            if matches!(c, '\n' | ';' | '#' | '{' | '}' | '[' | ']') {
                break;
            }
            if in_key {
                match c {
                    ':' | '.' | '<' | '>' | '&' => break,
                    '-' => {
                        let Some(c2) = self.at(self.pos + 1) else {
                            break;
                        };
                        if matches!(c2, '\n' | ';' | '#' | '{' | '}' | '[' | ']') {
                            text.push('-');
                            self.pos += 1;
                            last = self.pos;
                            break;
                        }
                        if matches!(c2, '-' | '>' | '*') {
                            break;
                        }
                        text.push('-');
                        self.pos += 1;
                        last = self.pos;
                        c = c2;
                    }
                    _ => {}
                }
            }
            let cstart = self.pos;
            self.pos += c.len_utf8();
            if !c.is_whitespace() {
                last = self.pos;
            }
            if !in_key && c == '$' {
                if let Some(s) = self.parse_substitution(cstart) {
                    substs.push(s);
                }
                last = last.max(self.pos);
                continue;
            }
            if c != '\\' {
                text.push(c);
                continue;
            }
            let Some(c2) = self.at(self.pos) else {
                self.err(Diag::error(cstart..self.pos, "unfinished escape sequence"));
                break;
            };
            self.pos += c2.len_utf8();
            last = self.pos;
            if c2 == '\n' {
                let (q, nl) = self.skip_ws(self.pos);
                if self.at(q).is_none() || nl > 0 {
                    break;
                }
                self.pos = q;
                continue;
            }
            text.push(decode_escape(c2));
        }
        let trimmed = text.trim_end().to_string();
        if trimmed.is_empty() && substs.is_empty() {
            self.pos = self.pos.max(start);
            return None;
        }
        Some(Str {
            text: trimmed,
            range: start..last,
            kind: StrKind::Unquoted,
            substs,
        })
    }

    fn unterminated_string(&mut self, open: usize, q: char) {
        let end = self.line_end(open);
        self.err(Diag::error(open..end, "unterminated string").help(format!(
            "strings opened with `{q}` must be closed with `{q}` on the same line"
        )));
    }

    fn parse_double(&mut self, open: usize, in_key: bool) -> Str {
        let mut text = String::new();
        let mut substs = Vec::new();
        loop {
            let Some(c) = self.at(self.pos) else {
                self.unterminated_string(open, '"');
                break;
            };
            if c == '\n' {
                self.unterminated_string(open, '"');
                break;
            }
            let cstart = self.pos;
            self.pos += c.len_utf8();
            if !in_key
                && c == '$'
                && let Some(s) = self.parse_substitution(cstart)
            {
                substs.push(s);
                continue;
            }
            if c == '"' {
                break;
            }
            if c != '\\' {
                text.push(c);
                continue;
            }
            let Some(c2) = self.at(self.pos) else {
                self.unterminated_string(open, '"');
                break;
            };
            self.pos += c2.len_utf8();
            if c2 != '\n' {
                text.push(decode_escape(c2));
            }
        }
        Str {
            text,
            range: open..self.pos,
            kind: StrKind::Double,
            substs,
        }
    }

    fn parse_single(&mut self, open: usize) -> Str {
        let mut text = String::new();
        loop {
            let Some(c) = self.at(self.pos) else {
                self.unterminated_string(open, '\'');
                break;
            };
            if c == '\n' {
                self.unterminated_string(open, '\'');
                break;
            }
            self.pos += c.len_utf8();
            if c == '\'' {
                if self.at(self.pos) == Some('\'') {
                    self.pos += 1;
                    text.push('\'');
                    continue;
                }
                break;
            }
            if c == '\\' && self.at(self.pos) == Some('\n') {
                self.pos += 1;
                continue;
            }
            text.push(c);
        }
        Str {
            text,
            range: open..self.pos,
            kind: StrKind::Single,
            substs: Vec::new(),
        }
    }

    fn parse_block(&mut self, open: usize) -> Str {
        let mut quote = String::new();
        let mut text = String::new();
        let unterminated = |p: &mut Self, quote: &str| {
            let head_end = p.src[open..]
                .find(char::is_whitespace)
                .map_or(p.src.len(), |i| open + i);
            p.err(
                Diag::error(open..head_end, "unterminated block string")
                    .help(format!("block string must be terminated with `{quote}|`")),
            );
            p.pos = p.src.len();
        };
        // Extra quote symbols, e.g. `|||` or "|`".
        loop {
            let Some(c) = self.at(self.pos) else {
                unterminated(self, &quote);
                return self.block_str(open, text);
            };
            if c.is_whitespace() || c.is_alphanumeric() || c == '_' {
                break;
            }
            quote.push(c);
            self.pos += c.len_utf8();
        }
        // Language tag.
        loop {
            let Some(c) = self.at(self.pos) else {
                unterminated(self, &quote);
                return self.block_str(open, text);
            };
            if c.is_whitespace() {
                break;
            }
            self.pos += c.len_utf8();
        }
        let (end_hint, end_rest) = match quote.chars().last() {
            None => ('|', String::new()),
            Some(last) => (
                last,
                format!("{}|", &quote[..quote.len() - last.len_utf8()]),
            ),
        };
        loop {
            let Some(c) = self.at(self.pos) else {
                unterminated(self, &quote);
                return self.block_str(open, text);
            };
            self.pos += c.len_utf8();
            if c == end_hint && self.src[self.pos..].starts_with(&end_rest) {
                self.pos += end_rest.len();
                break;
            }
            text.push(c);
        }
        self.block_str(open, text)
    }

    fn block_str(&self, open: usize, text: String) -> Str {
        Str {
            text,
            range: open..self.pos,
            kind: StrKind::Block,
            substs: Vec::new(),
        }
    }

    fn parse_array(&mut self, open: usize) -> Array {
        let mut a = Array {
            range: open..open,
            ..Default::default()
        };
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            self.err(Diag::error(
                open..open + 1,
                "D2 arrays are nested too deeply",
            ));
            self.pos = self.src.len();
            self.depth -= 1;
            return a;
        }
        loop {
            let (q, _) = self.skip_ws(self.pos);
            self.pos = q;
            let Some(c) = self.at(q) else {
                self.err(
                    Diag::error(open..open + 1, "unclosed `[`")
                        .help("arrays must be terminated with `]`"),
                );
                break;
            };
            match c {
                ';' => {
                    self.pos += 1;
                    continue;
                }
                ']' => {
                    self.pos += 1;
                    break;
                }
                '#' => {
                    self.pos = self.line_end(q);
                    continue;
                }
                '"' if self.src[q..].starts_with("\"\"\"") => {
                    self.parse_block_comment(q);
                    continue;
                }
                '.' if self.src[q..].starts_with("...$") => {
                    self.pos = q + 4;
                    if let Some(s) = self.parse_substitution(q) {
                        a.substs.push(s);
                    }
                }
                '.' if self.src[q..].starts_with("...@") => {
                    self.pos = q + 4;
                    self.parse_import_path();
                    a.items.push(Value::Import(q..self.pos));
                }
                _ => {
                    let before = self.diags.len();
                    let v = self.parse_value();
                    let what = v.as_ref().map(|_| "array item");
                    if let Some(v) = v {
                        a.items.push(v);
                    }
                    self.trailing_garbage_array(what, before);
                    continue;
                }
            }
            let before = self.diags.len();
            self.trailing_garbage_array(Some("array item"), before);
        }
        self.depth -= 1;
        a.range = open..self.pos;
        a
    }

    fn trailing_garbage_array(&mut self, what: Option<&str>, before: usize) {
        self.trailing_garbage(what, &[';', ']', '#'], before);
        if what.is_none()
            && let Some(d) = self.diags.get_mut(before)
            && d.msg == "invalid text beginning unquoted key"
        {
            d.msg = "invalid text beginning unquoted string".into();
        }
    }

    fn parse_value(&mut self) -> Option<Value> {
        let (q, c) = self.peek_same_line()?;
        match c {
            '[' => {
                self.pos = q + 1;
                return Some(Value::Array(self.parse_array(q)));
            }
            '{' => {
                self.pos = q + 1;
                return Some(Value::Map(self.parse_map(false, q)));
            }
            '@' => {
                self.pos = q + 1;
                self.parse_import_path();
                return Some(Value::Import(q..self.pos));
            }
            _ => {}
        }
        let s = self.parse_string(false)?;
        let kind = if s.unquoted() && s.substs.is_empty() {
            let t = s.text.to_ascii_lowercase();
            match t.as_str() {
                "null" => ScalarKind::Null,
                "suspend" | "unsuspend" => ScalarKind::Suspension,
                "true" | "false" => ScalarKind::Bool,
                _ if is_number(&t) => ScalarKind::Number,
                _ => ScalarKind::Str,
            }
        } else {
            ScalarKind::Str
        };
        Some(Value::Scalar(Scalar { kind, s }))
    }

    /// `${path}`; `self.pos` is just after `$` (or `...$`).
    fn parse_substitution(&mut self, start: usize) -> Option<Subst> {
        let (q, nl) = self.skip_ws(self.pos);
        let c = self.at(q)?;
        if nl > 0 {
            return None;
        }
        if c != '{' {
            self.err(
                Diag::warning(start..self.pos, "`$` must start a `${...}` substitution").help(
                    "escape a literal dollar sign as `\\$`, or wrap the value in single quotes",
                ),
            );
            return None;
        }
        self.pos = q + 1;
        let path = self
            .parse_key()
            .map(|k| k.parts.into_iter().map(|p| p.text).collect())
            .unwrap_or_default();
        match self.peek_same_line() {
            Some((q, '}')) => {
                self.pos = q + 1;
                Some(Subst {
                    path,
                    range: start..self.pos,
                })
            }
            _ => {
                self.err(
                    Diag::error(start..q + 1, "unterminated substitution `${`")
                        .help("substitutions must be terminated by `}`"),
                );
                None
            }
        }
    }

    fn parse_import_path(&mut self) {
        while matches!(self.at(self.pos), Some('.' | '/')) {
            self.pos += 1;
        }
        self.parse_key();
    }
}

fn decode_escape(c: char) -> char {
    match c {
        'a' => '\u{7}',
        'b' => '\u{8}',
        'f' => '\u{c}',
        'n' => '\n',
        'r' => '\r',
        't' => '\t',
        'v' => '\u{b}',
        other => other,
    }
}

/// Approximates Go `big.Rat.SetString`: decimals, exponents and fractions.
fn is_number(s: &str) -> bool {
    let body = s.strip_prefix(['+', '-']).unwrap_or(s);
    if !body.starts_with(|c: char| c.is_ascii_digit() || c == '.') {
        return false;
    }
    if let Some((a, b)) = body.split_once('/') {
        return a.bytes().all(|c| c.is_ascii_digit())
            && !a.is_empty()
            && !b.is_empty()
            && b.bytes().all(|c| c.is_ascii_digit());
    }
    body.parse::<f64>().is_ok()
}
