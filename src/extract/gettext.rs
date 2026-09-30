//! GNU gettext PO/POT catalog parser.
//!
//! Hand-written and lossless enough for linting: every string piece keeps its source byte
//! range, and decoded string values keep a byte-by-byte map back to the source so findings
//! inside a (multi-line, escaped) string point at the right span. Syntax problems are
//! collected instead of aborting, so one bad line does not hide the rest of the catalog.

use std::ops::Range;

/// A decoded, possibly multi-line PO string (`"a" "b"` concatenated).
#[derive(Debug, Clone, Default)]
pub struct PoString {
    /// Byte ranges of each quoted piece's contents (between the quotes).
    pub pieces: Vec<Range<usize>>,
    /// Decoded value.
    pub value: String,
    /// For each byte of `value`: the source range of the character or escape producing it.
    /// Empty when the decoded bytes were not valid UTF-8 (then ranges fall back to the pieces).
    map: Vec<(usize, usize)>,
    /// Source ranges of escape sequences (`\n`, `\"`, `\t`, ...).
    escapes: Vec<Range<usize>>,
}

impl PoString {
    /// Source range covering all pieces (from the first opening quote's contents to the last
    /// closing quote's contents).
    pub fn span(&self) -> Range<usize> {
        match (self.pieces.first(), self.pieces.last()) {
            (Some(a), Some(b)) => a.start..b.end,
            _ => 0..0,
        }
    }

    /// Source range of `value[r]`.
    pub fn src_range(&self, r: Range<usize>) -> Range<usize> {
        if self.map.len() != self.value.len() || self.map.is_empty() {
            return self.span();
        }
        let r = r.start.min(self.map.len())..r.end.min(self.map.len());
        if r.start >= r.end {
            let at = self
                .map
                .get(r.start)
                .map_or_else(|| self.span().end, |m| m.0);
            return at..at;
        }
        self.map[r.start].0..self.map[r.end - 1].1
    }

    /// Source ranges of escape sequences (`\n`, `\"`, `\t`, ...).
    pub fn escape_ranges(&self) -> &[Range<usize>] {
        &self.escapes
    }
}

/// A keyword line with its string: `msgid "..."`.
#[derive(Debug, Clone)]
pub struct Field {
    /// Range of the keyword itself (`msgstr[1]`).
    pub keyword: Range<usize>,
    pub s: PoString,
}

impl Field {
    pub fn value(&self) -> &str {
        &self.s.value
    }

    /// From the keyword to the end of the last string piece (including its closing quote).
    pub fn range(&self) -> Range<usize> {
        let end = self.s.pieces.last().map_or(self.keyword.end, |p| p.end + 1);
        self.keyword.start..end
    }
}

/// One comment line of an entry: full line range and contents after the marker.
#[derive(Debug, Clone)]
pub struct CommentLine {
    pub raw: Range<usize>,
    pub content: Range<usize>,
}

#[derive(Debug, Clone, Default)]
pub struct Entry {
    /// From the first line (comment or keyword) to the end of the last line.
    pub range: Range<usize>,
    /// `# translator comment`
    pub translator: Vec<CommentLine>,
    /// `#. extracted comment`
    pub extracted: Vec<CommentLine>,
    /// `#: file:line`
    pub references: Vec<CommentLine>,
    /// `#, fuzzy, c-format`: each flag with its range.
    pub flags: Vec<(String, Range<usize>)>,
    /// `#| msgid "previous"`
    pub previous: Vec<CommentLine>,
    /// Keywords were written as `#~ msgid ...`.
    pub obsolete: bool,
    pub msgctxt: Option<Field>,
    pub msgid: Option<Field>,
    pub msgid_plural: Option<Field>,
    /// Singular `msgstr`.
    pub msgstr: Option<Field>,
    /// `msgstr[N]` in source order.
    pub msgstr_n: Vec<(usize, Field)>,
}

impl Entry {
    pub fn has_flag(&self, flag: &str) -> bool {
        self.flags.iter().any(|(f, _)| f == flag)
    }

    pub fn is_fuzzy(&self) -> bool {
        self.has_flag("fuzzy")
    }

    /// The header entry: `msgid ""` without context.
    pub fn is_header(&self) -> bool {
        self.msgctxt.is_none() && self.msgid.as_ref().is_some_and(|m| m.value().is_empty())
    }

    pub fn msgid_str(&self) -> &str {
        self.msgid.as_ref().map_or("", Field::value)
    }

    /// All translations: the singular `msgstr` or every `msgstr[N]`.
    pub fn translations(&self) -> Vec<&Field> {
        match &self.msgstr {
            Some(m) => vec![m],
            None => self.msgstr_n.iter().map(|(_, f)| f).collect(),
        }
    }

    /// Every translation is empty.
    pub fn is_untranslated(&self) -> bool {
        self.translations().iter().all(|f| f.value().is_empty())
    }

    /// Best range to point a finding about the whole entry at.
    pub fn anchor(&self) -> Range<usize> {
        self.msgid.as_ref().map_or(self.range.clone(), Field::range)
    }
}

#[derive(Debug, Clone)]
pub struct HeaderField {
    pub name: String,
    pub value: String,
    /// Source range of the `Name: value` line inside the header string.
    pub range: Range<usize>,
}

#[derive(Debug, Clone)]
pub struct Header {
    /// Index into [`Catalog::entries`].
    pub entry: usize,
    pub fields: Vec<HeaderField>,
}

impl Header {
    pub fn get(&self, name: &str) -> Option<&HeaderField> {
        self.fields
            .iter()
            .find(|f| f.name.eq_ignore_ascii_case(name))
    }

    /// `charset=` of Content-Type.
    pub fn charset(&self) -> Option<&str> {
        let ct = &self.get("Content-Type")?.value;
        let i = ct.to_ascii_lowercase().find("charset=")?;
        Some(ct[i + 8..].split([';', ' ']).next().unwrap_or("").trim())
    }
}

#[derive(Debug, Clone)]
pub struct SyntaxError {
    pub range: Range<usize>,
    pub message: String,
}

#[derive(Debug, Clone, Default)]
pub struct Catalog {
    pub entries: Vec<Entry>,
    pub header: Option<Header>,
    pub errors: Vec<SyntaxError>,
}

impl Catalog {
    pub fn header_entry(&self) -> Option<&Entry> {
        self.header.as_ref().map(|h| &self.entries[h.entry])
    }

    /// Source text that names things rather than being prose: references, flags, contexts and
    /// key-like msgids (`auth.login.title`). Spelling treats words here as known identifiers.
    pub fn identifier_ranges(&self) -> Vec<Range<usize>> {
        let mut v = Vec::new();
        for e in &self.entries {
            v.extend(e.references.iter().map(|c| c.content.clone()));
            v.extend(e.flags.iter().map(|(_, r)| r.clone()));
            v.extend(e.msgctxt.iter().map(|f| f.s.span()));
            if let Some(m) = &e.msgid
                && is_key(m.value())
            {
                v.push(m.s.span());
            }
        }
        v
    }
}

/// A msgid that is a lookup key (`shared.rendering.failure`, `LOGIN_TITLE`), not source text.
pub fn is_key(s: &str) -> bool {
    !s.is_empty()
        && !s.contains(char::is_whitespace)
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | ':' | '/'))
        && !s.ends_with(['.', ':'])
        && s.contains(['.', '_', ':'])
        && s.chars().any(|c| c.is_ascii_alphabetic())
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kw {
    Msgctxt,
    Msgid,
    MsgidPlural,
    Msgstr,
    MsgstrN(usize),
}

struct Parser<'a> {
    src: &'a str,
    cat: Catalog,
    cur: Entry,
    /// Current entry has any line.
    started: bool,
    /// The field that string continuation lines append to.
    last: Option<Kw>,
    /// Keyword lines seen in this entry (obsolete or not).
    any_keyword: bool,
}

/// Parse a PO/POT catalog. Never fails; problems end up in [`Catalog::errors`].
pub fn parse(src: &str) -> Catalog {
    let mut p = Parser {
        src,
        cat: Catalog::default(),
        cur: Entry::default(),
        started: false,
        last: None,
        any_keyword: false,
    };
    let mut pos = 0;
    // Skip a UTF-8 byte order mark.
    if src.starts_with('\u{feff}') {
        pos = 3;
    }
    while pos < src.len() {
        let nl = src[pos..].find('\n').map_or(src.len(), |i| pos + i);
        let mut end = nl;
        if end > pos && src.as_bytes()[end - 1] == b'\r' {
            end -= 1;
        }
        p.line(pos..end);
        pos = nl + 1;
    }
    p.finish();
    p.header();
    p.cat
}

impl Parser<'_> {
    fn err(&mut self, range: Range<usize>, message: impl Into<String>) {
        self.cat.errors.push(SyntaxError {
            range,
            message: message.into(),
        });
    }

    fn has_msgstr(&self) -> bool {
        self.cur.msgstr.is_some() || !self.cur.msgstr_n.is_empty()
    }

    fn touch(&mut self, line: &Range<usize>) {
        if !self.started {
            self.cur.range = line.clone();
            self.started = true;
        }
        self.cur.range.end = line.end;
    }

    fn line(&mut self, line: Range<usize>) {
        let src = self.src;
        let text = &src[line.clone()];
        let indent = text.len() - text.trim_start().len();
        let start = line.start + indent;
        let t = text.trim();
        if t.is_empty() {
            if self.has_msgstr() {
                self.finish();
            }
            return;
        }
        if let Some(rest) = t.strip_prefix("#~") {
            // Obsolete: `#~ msgid "x"`, `#~| msgid "prev"`.
            if rest.starts_with('|') {
                self.comment(line, start);
                return;
            }
            let off = start + 2 + (rest.len() - rest.trim_start().len());
            let body_end = start + t.len();
            if off >= body_end {
                return;
            }
            self.keyword_line(line, off..body_end, true);
            return;
        }
        if t.starts_with('#') {
            self.comment(line, start);
            return;
        }
        self.keyword_line(line, start..start + t.len(), false);
    }

    fn comment(&mut self, line: Range<usize>, start: usize) {
        if self.has_msgstr() {
            self.finish();
        } else if self.any_keyword {
            // A comment between msgid and msgstr (`msgid "a"` `# c` `msgstr "b"`).
            self.err(line.clone(), "comment inside an entry, before its msgstr");
        }
        self.touch(&line);
        let src = self.src;
        let end = start + src[start..line.end].trim_end().len();
        let body = &src[start..end];
        let marker = body.chars().nth(1);
        // `#~|` previous obsolete comments count as previous.
        let (kind, mlen) = if body.starts_with("#~|") {
            ('|', 3)
        } else {
            match marker {
                Some(c @ ('.' | ':' | ',' | '|')) => (c, 2),
                _ => (' ', 1),
            }
        };
        let mut cstart = start + mlen;
        // Elixir writes translator comments as `## text`.
        if kind == ' ' {
            while src[cstart..end].starts_with('#') {
                cstart += 1;
            }
        }
        while src[cstart..end].starts_with([' ', '\t']) {
            cstart += 1;
        }
        let cl = CommentLine {
            raw: line,
            content: cstart..end.max(cstart),
        };
        match kind {
            '.' => self.cur.extracted.push(cl),
            ':' => self.cur.references.push(cl),
            '|' => self.cur.previous.push(cl),
            ',' => {
                let mut off = cstart;
                for part in src[cstart..end].split(',') {
                    let lead = part.len() - part.trim_start().len();
                    let f = part.trim();
                    if !f.is_empty() {
                        let s = off + lead;
                        self.cur.flags.push((f.to_string(), s..s + f.len()));
                    }
                    off += part.len() + 1;
                }
            }
            _ => self.cur.translator.push(cl),
        }
    }

    /// `body` is the trimmed line (or the part after `#~`).
    fn keyword_line(&mut self, line: Range<usize>, body: Range<usize>, obsolete: bool) {
        let src = self.src;
        let t = &src[body.clone()];
        if t.starts_with('"') {
            // Continuation of the previous field.
            let Some(kw) = self.last else {
                self.err(body, "string without a keyword (msgid, msgstr, ...)");
                return;
            };
            self.touch(&line);
            let mut s = self
                .field_mut(kw)
                .map(|f| std::mem::take(&mut f.s))
                .unwrap_or_default();
            self.string(body.start, body.end, &mut s);
            if let Some(f) = self.field_mut(kw) {
                f.s = s;
            }
            return;
        }
        let word_len = t
            .find(|c: char| c.is_whitespace() || c == '"')
            .unwrap_or(t.len());
        let word = &t[..word_len];
        let kw = match word {
            "msgctxt" => Kw::Msgctxt,
            "msgid" => Kw::Msgid,
            "msgid_plural" => Kw::MsgidPlural,
            "msgstr" => Kw::Msgstr,
            w if w.starts_with("msgstr[") && w.ends_with(']') => {
                match w[7..w.len() - 1].parse::<usize>() {
                    Ok(n) => Kw::MsgstrN(n),
                    Err(_) => {
                        self.err(
                            body.start..body.start + word_len,
                            format!("bad plural index in `{w}`"),
                        );
                        return;
                    }
                }
            }
            w => {
                let r = body.start..body.start + word_len.max(1);
                if w.chars().next().is_some_and(|c| c.is_ascii_alphabetic()) {
                    self.err(r, format!("unknown keyword `{w}`"));
                } else {
                    self.err(
                        body,
                        "stray text; expected a keyword, a string or a comment",
                    );
                }
                self.last = None;
                return;
            }
        };
        // A new entry starts at msgctxt/msgid after a msgstr (or after a msgid lacking one).
        if matches!(kw, Kw::Msgctxt | Kw::Msgid) && (self.has_msgstr() || self.cur.msgid.is_some())
        {
            self.finish();
        }
        let kw_range = body.start..body.start + word_len;
        self.check_order(kw, kw_range.clone());
        if self.any_keyword && obsolete != self.cur.obsolete {
            self.err(kw_range.clone(), "entry mixes obsolete (#~) and live lines");
        }
        self.touch(&line);
        if obsolete {
            self.cur.obsolete = true;
        }
        self.any_keyword = true;
        let rest = &t[word_len..];
        let lead = rest.len() - rest.trim_start().len();
        let sstart = body.start + word_len + lead;
        let mut s = PoString::default();
        if sstart >= body.end {
            self.err(kw_range.clone(), format!("`{word}` without a string"));
        } else if src.as_bytes()[sstart] != b'"' {
            self.err(
                sstart..body.end,
                format!("`{word}` must be followed by a quoted string"),
            );
        } else {
            self.string(sstart, body.end, &mut s);
        }
        let field = Field {
            keyword: kw_range.clone(),
            s,
        };
        let dup = match kw {
            Kw::Msgctxt => self.cur.msgctxt.replace(field).is_some(),
            Kw::Msgid => self.cur.msgid.replace(field).is_some(),
            Kw::MsgidPlural => self.cur.msgid_plural.replace(field).is_some(),
            Kw::Msgstr => self.cur.msgstr.replace(field).is_some(),
            Kw::MsgstrN(n) => {
                let dup = self.cur.msgstr_n.iter().any(|(i, _)| *i == n);
                self.cur.msgstr_n.push((n, field));
                dup
            }
        };
        if dup {
            self.err(kw_range, format!("duplicate `{word}` in one entry"));
        }
        self.last = Some(kw);
    }

    fn check_order(&mut self, kw: Kw, r: Range<usize>) {
        let c = &self.cur;
        let msg = match kw {
            Kw::MsgidPlural if c.msgid.is_none() => Some("msgid_plural before msgid"),
            Kw::Msgstr | Kw::MsgstrN(_) if c.msgid.is_none() => Some("msgstr without msgid"),
            Kw::MsgidPlural if c.msgstr.is_some() || !c.msgstr_n.is_empty() => {
                Some("msgid_plural must come before msgstr")
            }
            Kw::Msgstr if c.msgid_plural.is_some() => {
                Some("entry with msgid_plural needs msgstr[N], not msgstr")
            }
            Kw::Msgstr if !c.msgstr_n.is_empty() => Some("msgstr mixed with msgstr[N]"),
            Kw::MsgstrN(_) if c.msgid_plural.is_none() => Some("msgstr[N] without msgid_plural"),
            Kw::MsgstrN(_) if c.msgstr.is_some() => Some("msgstr[N] mixed with msgstr"),
            _ => None,
        };
        if let Some(m) = msg {
            self.err(r, m);
        }
    }

    fn field_mut(&mut self, kw: Kw) -> Option<&mut Field> {
        match kw {
            Kw::Msgctxt => self.cur.msgctxt.as_mut(),
            Kw::Msgid => self.cur.msgid.as_mut(),
            Kw::MsgidPlural => self.cur.msgid_plural.as_mut(),
            Kw::Msgstr => self.cur.msgstr.as_mut(),
            Kw::MsgstrN(_) => self.cur.msgstr_n.last_mut().map(|(_, f)| f),
        }
    }

    /// Parse one quoted string starting at `start` (the opening quote), ending by `end`.
    fn string(&mut self, start: usize, end: usize, out: &mut PoString) {
        let src = self.src;
        let b = src.as_bytes();
        let mut bytes = std::mem::take(&mut out.value).into_bytes();
        let mut map = std::mem::take(&mut out.map);
        let mut i = start + 1;
        let mut closed = None;
        while i < end {
            match b[i] {
                b'"' => {
                    closed = Some(i);
                    break;
                }
                b'\\' => {
                    let Some(&c) = b.get(i + 1).filter(|_| i + 1 < end) else {
                        self.err(i..i + 1, "backslash at end of line");
                        i += 1;
                        continue;
                    };
                    let simple = match c {
                        b'n' => Some(b'\n'),
                        b't' => Some(b'\t'),
                        b'r' => Some(b'\r'),
                        b'"' => Some(b'"'),
                        b'\\' => Some(b'\\'),
                        b'a' => Some(7),
                        b'b' => Some(8),
                        b'f' => Some(12),
                        b'v' => Some(11),
                        b'\'' => Some(b'\''),
                        b'?' => Some(b'?'),
                        _ => None,
                    };
                    if let Some(v) = simple {
                        bytes.push(v);
                        map.push((i, i + 2));
                        out.escapes.push(i..i + 2);
                        i += 2;
                    } else if (b'0'..=b'7').contains(&c) {
                        let mut j = i + 1;
                        let mut v: u32 = 0;
                        while j < end && j < i + 4 && (b'0'..=b'7').contains(&b[j]) {
                            v = v * 8 + u32::from(b[j] - b'0');
                            j += 1;
                        }
                        bytes.push((v & 0xff) as u8);
                        map.push((i, j));
                        out.escapes.push(i..j);
                        i = j;
                    } else if c == b'x' {
                        let mut j = i + 2;
                        let mut v: u32 = 0;
                        while j < end && b[j].is_ascii_hexdigit() {
                            v = (v << 4) | char::from(b[j]).to_digit(16).unwrap_or(0);
                            j += 1;
                        }
                        if j == i + 2 {
                            self.err(i..i + 2, "`\\x` escape without hex digits");
                        } else {
                            bytes.push((v & 0xff) as u8);
                            map.push((i, j));
                            out.escapes.push(i..j);
                        }
                        i = j;
                    } else {
                        // Keep the text as written so the value stays distinct.
                        let w = src[i + 1..].chars().next().map_or(1, char::len_utf8);
                        self.err(
                            i..i + 1 + w,
                            format!("unknown escape `\\{}`", &src[i + 1..i + 1 + w]),
                        );
                        bytes.extend_from_slice(&b[i..i + 1 + w]);
                        for _ in 0..1 + w {
                            map.push((i, i + 1 + w));
                        }
                        i += 1 + w;
                    }
                }
                _ => {
                    let w = src[i..].chars().next().map_or(1, char::len_utf8);
                    bytes.extend_from_slice(&b[i..i + w]);
                    for _ in 0..w {
                        map.push((i, i + w));
                    }
                    i += w;
                }
            }
        }
        match closed {
            Some(q) => {
                out.pieces.push(start + 1..q);
                let tail = src[q + 1..end].trim();
                if !tail.is_empty() {
                    let ts = q + 1 + (src[q + 1..end].len() - src[q + 1..end].trim_start().len());
                    self.err(ts..end, "stray text after string");
                }
            }
            None => {
                out.pieces.push(start + 1..end);
                self.err(start..end, "unterminated string");
            }
        }
        match String::from_utf8(bytes) {
            Ok(s) => {
                out.value = s;
                out.map = map;
            }
            Err(e) => {
                out.value = String::from_utf8_lossy(e.as_bytes()).into_owned();
                out.map = Vec::new();
            }
        }
    }

    fn finish(&mut self) {
        let e = std::mem::take(&mut self.cur);
        let had_keyword = self.any_keyword;
        self.started = false;
        self.last = None;
        self.any_keyword = false;
        if !had_keyword {
            // Trailing comments without an entry are allowed.
            if !e.translator.is_empty() || !e.extracted.is_empty() || !e.flags.is_empty() {
                self.cat.entries.push(e);
            }
            return;
        }
        if e.msgid.is_none() {
            self.err(e.range.clone(), "entry without msgid");
        } else if e.msgstr.is_none() && e.msgstr_n.is_empty() {
            let msg = if e.msgid_plural.is_some() {
                "msgid_plural without msgstr[N]"
            } else {
                "msgid without msgstr"
            };
            self.err(e.anchor(), msg);
        } else if !e.msgstr_n.is_empty() {
            let mut idx: Vec<usize> = e.msgstr_n.iter().map(|(i, _)| *i).collect();
            idx.sort_unstable();
            idx.dedup();
            if idx.iter().enumerate().any(|(k, &i)| k != i) {
                let r =
                    e.msgstr_n[0].1.keyword.start..e.msgstr_n.last().map_or(0, |f| f.1.keyword.end);
                self.err(r, "msgstr[N] indices must run 0, 1, 2, ... without gaps");
            }
        }
        self.cat.entries.push(e);
    }

    fn header(&mut self) {
        let Some(idx) = self
            .cat
            .entries
            .iter()
            .position(|e| !e.obsolete && e.is_header())
        else {
            return;
        };
        let e = &self.cat.entries[idx];
        let mut fields = Vec::new();
        if let Some(ms) = e.msgstr.as_ref().or(e.msgstr_n.first().map(|f| &f.1)) {
            let v = &ms.s.value;
            let mut off = 0;
            for line in v.split_inclusive('\n') {
                let content = line.trim_end_matches('\n');
                if let Some((k, val)) = content.split_once(':') {
                    let k = k.trim();
                    if !k.is_empty() && !k.contains(char::is_whitespace) {
                        fields.push(HeaderField {
                            name: k.to_string(),
                            value: val.trim().to_string(),
                            range: ms.s.src_range(off..off + content.len()),
                        });
                    }
                }
                off += line.len();
            }
        }
        self.cat.header = Some(Header { entry: idx, fields });
    }
}

/// Parsed `Plural-Forms: nplurals=N; plural=EXPR;`.
#[derive(Debug, Clone)]
pub struct PluralForms {
    pub nplurals: usize,
    expr: Expr,
}

impl PluralForms {
    /// Plural form index for `n`, or `None` on division by zero.
    pub fn index(&self, n: u64) -> Option<u64> {
        self.expr.eval(n)
    }
}

/// Parse a Plural-Forms header value.
pub fn parse_plural_forms(v: &str) -> Result<PluralForms, String> {
    let mut nplurals = None;
    let mut plural = None;
    for part in v.split(';') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let Some((k, val)) = part.split_once('=') else {
            return Err(format!("`{part}` is not key=value"));
        };
        match k.trim() {
            "nplurals" => {
                nplurals = Some(
                    val.trim()
                        .parse::<usize>()
                        .map_err(|_| format!("nplurals `{}` is not a number", val.trim()))?,
                );
            }
            "plural" => plural = Some(val.trim().to_string()),
            other => return Err(format!("unknown Plural-Forms key `{other}`")),
        }
    }
    let nplurals = nplurals.ok_or("nplurals missing")?;
    if nplurals == 0 {
        return Err("nplurals must be at least 1".into());
    }
    let plural = plural.ok_or("plural expression missing")?;
    let expr = ExprParser::new(&plural).parse()?;
    Ok(PluralForms { nplurals, expr })
}

#[derive(Debug, Clone)]
enum Expr {
    N,
    Num(u64),
    Not(Box<Expr>),
    Bin(Op, Box<Expr>, Box<Expr>),
    Cond(Box<Expr>, Box<Expr>, Box<Expr>),
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Op {
    Or,
    And,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}

impl Expr {
    fn eval(&self, n: u64) -> Option<u64> {
        Some(match self {
            Expr::N => n,
            Expr::Num(v) => *v,
            Expr::Not(e) => u64::from(e.eval(n)? == 0),
            Expr::Cond(c, a, b) => {
                if c.eval(n)? != 0 {
                    a.eval(n)?
                } else {
                    b.eval(n)?
                }
            }
            Expr::Bin(op, a, b) => {
                let (x, y) = (a.eval(n)?, b.eval(n)?);
                match op {
                    Op::Or => u64::from(x != 0 || y != 0),
                    Op::And => u64::from(x != 0 && y != 0),
                    Op::Eq => u64::from(x == y),
                    Op::Ne => u64::from(x != y),
                    Op::Lt => u64::from(x < y),
                    Op::Le => u64::from(x <= y),
                    Op::Gt => u64::from(x > y),
                    Op::Ge => u64::from(x >= y),
                    Op::Add => x.wrapping_add(y),
                    Op::Sub => x.wrapping_sub(y),
                    Op::Mul => x.wrapping_mul(y),
                    Op::Div => x.checked_div(y)?,
                    Op::Mod => x.checked_rem(y)?,
                }
            }
        })
    }
}

/// Recursive-descent parser for the C subset gettext allows in `plural=`.
struct ExprParser<'a> {
    s: &'a [u8],
    i: usize,
}

impl<'a> ExprParser<'a> {
    fn new(s: &'a str) -> Self {
        ExprParser {
            s: s.as_bytes(),
            i: 0,
        }
    }

    fn parse(mut self) -> Result<Expr, String> {
        let e = self.cond()?;
        self.ws();
        if self.i < self.s.len() {
            return Err(format!(
                "unexpected `{}` in plural expression",
                char::from(self.s[self.i])
            ));
        }
        Ok(e)
    }

    fn ws(&mut self) {
        while self.i < self.s.len() && self.s[self.i].is_ascii_whitespace() {
            self.i += 1;
        }
    }

    fn eat(&mut self, t: &str) -> bool {
        self.ws();
        if self.s[self.i..].starts_with(t.as_bytes()) {
            self.i += t.len();
            true
        } else {
            false
        }
    }

    fn cond(&mut self) -> Result<Expr, String> {
        let c = self.bin(0)?;
        if self.eat("?") {
            let a = self.cond()?;
            if !self.eat(":") {
                return Err("`?` without `:` in plural expression".into());
            }
            let b = self.cond()?;
            return Ok(Expr::Cond(Box::new(c), Box::new(a), Box::new(b)));
        }
        Ok(c)
    }

    fn bin(&mut self, level: usize) -> Result<Expr, String> {
        const LEVELS: &[&[(&str, Op)]] = &[
            &[("||", Op::Or)],
            &[("&&", Op::And)],
            &[("==", Op::Eq), ("!=", Op::Ne)],
            &[("<=", Op::Le), (">=", Op::Ge), ("<", Op::Lt), (">", Op::Gt)],
            &[("+", Op::Add), ("-", Op::Sub)],
            &[("*", Op::Mul), ("/", Op::Div), ("%", Op::Mod)],
        ];
        if level == LEVELS.len() {
            return self.unary();
        }
        let mut left = self.bin(level + 1)?;
        'outer: loop {
            for (t, op) in LEVELS[level] {
                // `!` alone is unary; don't read `!=` as `!`.
                if self.eat(t) {
                    let right = self.bin(level + 1)?;
                    left = Expr::Bin(*op, Box::new(left), Box::new(right));
                    continue 'outer;
                }
            }
            return Ok(left);
        }
    }

    fn unary(&mut self) -> Result<Expr, String> {
        self.ws();
        if self.s[self.i..].starts_with(b"!") && !self.s[self.i..].starts_with(b"!=") {
            self.i += 1;
            return Ok(Expr::Not(Box::new(self.unary()?)));
        }
        if self.eat("(") {
            let e = self.cond()?;
            if !self.eat(")") {
                return Err("unbalanced `(` in plural expression".into());
            }
            return Ok(e);
        }
        self.ws();
        let start = self.i;
        while self.i < self.s.len() && self.s[self.i].is_ascii_digit() {
            self.i += 1;
        }
        if self.i > start {
            let v = std::str::from_utf8(&self.s[start..self.i])
                .ok()
                .and_then(|d| d.parse().ok())
                .ok_or("number too large in plural expression")?;
            return Ok(Expr::Num(v));
        }
        if self.eat("n") {
            return Ok(Expr::N);
        }
        Err(match self.s.get(self.i) {
            Some(c) => format!("unexpected `{}` in plural expression", char::from(*c)),
            None => "plural expression ends too early".into(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multiline_escapes_and_ranges() {
        let src = "msgid \"\"\n\"Hello \\\"x\\\"\\n\"\n\"wörld\\t!\"\nmsgstr \"Hei\"\n";
        let c = parse(src);
        assert!(c.errors.is_empty(), "{:?}", c.errors);
        let e = &c.entries[0];
        let m = e.msgid.as_ref().unwrap();
        assert_eq!(m.value(), "Hello \"x\"\nwörld\t!");
        assert_eq!(m.s.pieces.len(), 3);
        let v = m.value();
        let w = v.find("wörld").unwrap();
        assert_eq!(&src[m.s.src_range(w..w + "wörld".len())], "wörld");
        let q = v.find("\"x").unwrap();
        assert_eq!(&src[m.s.src_range(q..q + 3)], "\\\"x\\\"");
        assert_eq!(&src[m.range()], &src[0..src.find("\nmsgstr").unwrap()]);
    }

    #[test]
    fn octal_hex_crlf() {
        let src = "msgid \"a\\101\\x42\"\r\nmsgstr \"b\"\r\n";
        let c = parse(src);
        assert!(c.errors.is_empty(), "{:?}", c.errors);
        assert_eq!(c.entries[0].msgid_str(), "aAB");
        assert_eq!(c.entries[0].msgstr.as_ref().unwrap().value(), "b");
    }

    #[test]
    fn syntax_errors() {
        let src = "msgid \"a\nmsgstr \"b\\q\"\nfoo \"x\"\n\"stray\"\n\nmsgid \"c\"\n\nmsgid \"d\"\nmsgid_plural \"ds\"\nmsgstr \"x\"\n!!\n";
        let c = parse(src);
        let msgs: Vec<&str> = c.errors.iter().map(|e| e.message.as_str()).collect();
        assert!(msgs.contains(&"unterminated string"), "{msgs:?}");
        assert!(
            msgs.iter().any(|m| m.starts_with("unknown escape")),
            "{msgs:?}"
        );
        assert!(msgs.contains(&"unknown keyword `foo`"), "{msgs:?}");
        assert!(
            msgs.contains(&"string without a keyword (msgid, msgstr, ...)"),
            "{msgs:?}"
        );
        assert!(msgs.contains(&"msgid without msgstr"), "{msgs:?}");
        assert!(
            msgs.contains(&"entry with msgid_plural needs msgstr[N], not msgstr"),
            "{msgs:?}"
        );
        assert!(msgs.iter().any(|m| m.starts_with("stray text")), "{msgs:?}");
    }

    #[test]
    fn comments_flags_obsolete_header() {
        let src = "# Title\nmsgid \"\"\nmsgstr \"\"\n\"Language: fi\\n\"\n\"Plural-Forms: nplurals=2; plural=(n != 1);\\n\"\n\"Content-Type: text/plain; charset=UTF-8\\n\"\n\n# note\n#. dev\n#: a.ex:1 b.ex:2\n#, fuzzy, elixir-format\n#| msgid \"old\"\nmsgctxt \"ctx\"\nmsgid \"x\"\nmsgstr \"y\"\n\n#~ msgid \"gone\"\n#~ msgstr \"poissa\"\n";
        let c = parse(src);
        assert!(c.errors.is_empty(), "{:?}", c.errors);
        assert_eq!(c.entries.len(), 3);
        let h = c.header.as_ref().unwrap();
        assert_eq!(h.get("language").unwrap().value, "fi");
        assert_eq!(
            &src[h.get("Language").unwrap().range.clone()],
            "Language: fi"
        );
        assert_eq!(h.charset(), Some("UTF-8"));
        let e = &c.entries[1];
        assert!(e.is_fuzzy() && e.has_flag("elixir-format"));
        assert_eq!(&src[e.translator[0].content.clone()], "note");
        assert_eq!(&src[e.references[0].content.clone()], "a.ex:1 b.ex:2");
        assert_eq!(e.previous.len(), 1);
        assert_eq!(e.msgctxt.as_ref().unwrap().value(), "ctx");
        assert!(c.entries[2].obsolete);
        assert_eq!(c.entries[2].msgid_str(), "gone");
    }

    #[test]
    fn plural_forms() {
        let p = parse_plural_forms("nplurals=3; plural=(n%10==1 && n%100!=11 ? 0 : n%10>=2 && n%10<=4 && (n%100<10 || n%100>=20) ? 1 : 2);").unwrap();
        assert_eq!(p.nplurals, 3);
        assert_eq!(p.index(1), Some(0));
        assert_eq!(p.index(3), Some(1));
        assert_eq!(p.index(5), Some(2));
        assert_eq!(p.index(22), Some(1));
        let p = parse_plural_forms("nplurals=2; plural=n != 1;").unwrap();
        assert_eq!((p.index(1), p.index(0)), (Some(0), Some(1)));
        assert_eq!(
            parse_plural_forms("nplurals=1; plural=0;")
                .unwrap()
                .index(9),
            Some(0)
        );
        assert!(parse_plural_forms("nplurals=2; plural=(n != 1;").is_err());
        assert!(parse_plural_forms("nplurals=INTEGER; plural=EXPRESSION;").is_err());
        assert!(parse_plural_forms("nplurals=2;").is_err());
        assert!(parse_plural_forms("nplurals=2; plural=n x 1;").is_err());
    }

    #[test]
    fn keys() {
        assert!(is_key("shared.rendering.localizedFailure"));
        assert!(is_key("LOGIN_TITLE"));
        assert!(!is_key("Save"));
        assert!(!is_key("e.g."));
        assert!(!is_key("Hello world"));
        assert!(!is_key("%{count}"));
    }
}
