//! Format placeholders in gettext strings: printf (`%s`, `%1$d`), Python (`%(name)s`,
//! `{name}`), ICU MessageFormat (`{count, plural, one {…} other {…}}`), Elixir/Ruby
//! (`%{name}`, `%<name>s`) and Qt (`%1`).

use std::collections::BTreeSet;
use std::ops::Range;
use std::sync::LazyLock;

use regex::Regex;

use crate::extract::gettext::Entry;

/// Placeholder syntaxes to look for in one entry.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Styles {
    pub printf: bool,
    /// Allow the space flag (`% d`): only when the entry declares a printf format, since
    /// `50% of` would otherwise read as `% o`.
    pub printf_space: bool,
    pub py_named: bool,
    pub brace: bool,
    pub percent_brace: bool,
    pub ruby_angle: bool,
    pub qt: bool,
}

impl Styles {
    pub fn any(self) -> bool {
        self.printf
            || self.py_named
            || self.brace
            || self.percent_brace
            || self.ruby_angle
            || self.qt
    }
}

static AUTO_PRINTF: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"%(?:\d+\$)?[-+#0]*\d*(?:\.\d+)?(?:hh|h|ll|l|L|q|j|z|t)?[sdiufxXoeEgGcp]")
        .expect("hardcoded regex is valid")
});
static AUTO_PY_NAMED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"%\(\w+\)[-+#0 ]*\d*(?:\.\d+)?[sdiufxXoeEgGcr]").expect("valid"));
static AUTO_BRACE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:^|[^{%])\{(?:\w+(?:\.\w+)*)?(?:\s*,\s*\w+[^{}]*|[:!][^{}]*)?\}").expect("valid")
});
static AUTO_PERCENT_BRACE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"%\{\w+\}").expect("valid"));

/// Styles for `e`: from its `*-format` flags, else guessed from the msgid(s).
pub fn styles(e: &Entry) -> Styles {
    let mut s = Styles::default();
    let mut explicit = false;
    let mut denied: Vec<&str> = Vec::new();
    for (flag, _) in &e.flags {
        let Some(name) = flag.strip_suffix("-format") else {
            continue;
        };
        if let Some(no) = name.strip_prefix("no-") {
            denied.push(no);
            continue;
        }
        let before = s;
        match name {
            "c" | "objc" | "sh" | "php" | "perl" | "gcc-internal" | "gfc-internal" | "awk"
            | "lua" | "object-pascal" | "smalltalk" | "tcl" | "librep" | "scheme" | "lisp"
            | "javascript" | "d" => {
                s.printf = true;
                s.printf_space = true;
            }
            "python" => {
                s.printf = true;
                s.printf_space = true;
                s.py_named = true;
            }
            "python-brace" | "java" | "java-printf" | "csharp" | "perl-brace" | "icu" => {
                s.brace = true;
            }
            "elixir" => s.percent_brace = true,
            "ruby" => {
                s.printf = true;
                s.percent_brace = true;
                s.ruby_angle = true;
            }
            "qt" | "qt-plural" | "kde" | "kde-kuit" => s.qt = true,
            // An unknown format: keep guessing.
            _ => {}
        }
        explicit |= s != before;
    }
    if !explicit {
        let src = format!(
            "{}\n{}",
            e.msgid_str(),
            e.msgid_plural.as_ref().map_or("", |f| f.value())
        );
        let text = src.replace("%%", "");
        s.percent_brace = AUTO_PERCENT_BRACE.is_match(&text);
        s.py_named = AUTO_PY_NAMED.is_match(&text);
        s.printf = s.py_named || AUTO_PRINTF.is_match(&AUTO_PERCENT_BRACE.replace_all(&text, ""));
        s.brace = AUTO_BRACE.is_match(&AUTO_PERCENT_BRACE.replace_all(&text, ""));
    }
    for d in denied {
        match d {
            "c" | "python" | "php" | "sh" | "perl" | "objc" => {
                s.printf = false;
                s.py_named &= d != "python";
            }
            "python-brace" | "java" | "csharp" | "icu" => s.brace = false,
            "elixir" | "ruby" => s.percent_brace = false,
            "qt" | "kde" => s.qt = false,
            _ => {}
        }
    }
    s
}

/// One placeholder: comparison key and range in the decoded string.
#[derive(Debug, Clone)]
pub struct Ph {
    /// `%{count}`, `{name}`, `%(n)s`, or `%2$d` for printf argument 2 (unnumbered printf
    /// arguments get their position, so reordering with `%2$s %1$s` compares equal).
    pub key: String,
    pub range: Range<usize>,
}

static PRINTF_AT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^%(?:(\d+)\$)?([-+ #0']*)(\d+|\*)?(?:\.(\d+|\*))?(hh|h|ll|l|L|q|j|z|t|I64|I32)?([diouxXeEfFgGaAcspn@])")
        .expect("hardcoded regex is valid")
});
static PY_NAMED_AT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^%\((\w+)\)[-+ #0]*\d*(?:\.\d+)?([sdiufxXoeEgGcr])").expect("valid")
});
static PERCENT_BRACE_AT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^%\{(\w+)\}").expect("valid"));
static RUBY_ANGLE_AT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^%<(\w+)>[-+ #0]*\d*(?:\.\d+)?[sdiufxXoeEgGc]?").expect("valid"));
static QT_AT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^%L?(\d{1,2})").expect("valid"));

/// Printf conversion class, so `%d`/`%i` and `%f`/`%g` compare equal.
fn conv_class(len: &str, conv: &str) -> String {
    let c = match conv {
        "d" | "i" => "d",
        "o" | "u" | "x" | "X" => "u",
        "e" | "E" | "f" | "F" | "g" | "G" | "a" | "A" => "f",
        other => other,
    };
    format!("{len}{c}")
}

/// Placeholders in `s` under `st`.
pub fn extract(s: &str, st: Styles) -> Vec<Ph> {
    let mut out = Vec::new();
    let mut next_arg = 1usize;
    let mut next_brace = 0usize;
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() {
        let rest = &s[i..];
        if b[i] == b'%' {
            if rest.starts_with("%%") {
                i += 2;
                continue;
            }
            if st.percent_brace
                && let Some(c) = PERCENT_BRACE_AT.captures(rest)
            {
                let m = c.get(0).map_or(0, |m| m.end());
                out.push(Ph {
                    key: format!("%{{{}}}", &c[1]),
                    range: i..i + m,
                });
                i += m;
                continue;
            }
            if st.ruby_angle
                && let Some(m) = RUBY_ANGLE_AT.captures(rest)
            {
                let end = m.get(0).map_or(0, |m| m.end());
                out.push(Ph {
                    key: format!("%<{}>", &m[1]),
                    range: i..i + end,
                });
                i += end;
                continue;
            }
            if st.py_named
                && let Some(c) = PY_NAMED_AT.captures(rest)
            {
                let end = c.get(0).map_or(0, |m| m.end());
                out.push(Ph {
                    key: format!("%({}){}", &c[1], &c[2]),
                    range: i..i + end,
                });
                i += end;
                continue;
            }
            if st.qt
                && let Some(c) = QT_AT.captures(rest)
            {
                let end = c.get(0).map_or(0, |m| m.end());
                out.push(Ph {
                    key: format!("%{}", &c[1]),
                    range: i..i + end,
                });
                i += end;
                continue;
            }
            if st.printf
                && let Some(c) = PRINTF_AT.captures(rest)
                && (st.printf_space || !c[2].contains(' '))
            {
                let end = c.get(0).map_or(0, |m| m.end());
                let arg = match c.get(1) {
                    Some(n) => n.as_str().parse().unwrap_or(0),
                    None => {
                        let a = next_arg;
                        next_arg += 1;
                        a
                    }
                };
                let class = conv_class(c.get(5).map_or("", |m| m.as_str()), &c[6]);
                out.push(Ph {
                    key: format!("%{arg}${class}"),
                    range: i..i + end,
                });
                i += end;
                continue;
            }
        }
        if st.brace && b[i] == b'{' {
            if rest.starts_with("{{") {
                i += 2;
                continue;
            }
            if let Some(end) = matching_brace(s, i) {
                brace_arg(s, i, end, &mut next_brace, &mut out);
                i = end + 1;
                continue;
            }
        }
        if st.brace && rest.starts_with("}}") {
            i += 2;
            continue;
        }
        i += rest.chars().next().map_or(1, char::len_utf8);
    }
    out
}

/// Index of the `}` closing the `{` at `open`.
fn matching_brace(s: &str, open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (k, c) in s[open..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(open + k);
                }
            }
            _ => {}
        }
    }
    None
}

static SIMPLE_ARG: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*(\w*(?:\.\w+)*)\s*(?:[:!][^{}]*)?$").expect("valid"));
static ICU_ARG: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)^\s*(\w+)\s*,\s*(\w+)\s*(?:,(.*))?$").expect("valid"));

/// Record the brace argument `s[open..=close]`; ICU plural/select arguments recurse into
/// their sub-messages.
fn brace_arg(s: &str, open: usize, close: usize, next: &mut usize, out: &mut Vec<Ph>) {
    let inner = &s[open + 1..close];
    if let Some(c) = SIMPLE_ARG.captures(inner) {
        let name = if c[1].is_empty() {
            let n = *next;
            *next += 1;
            format!("#{n}")
        } else {
            c[1].to_string()
        };
        out.push(Ph {
            key: format!("{{{name}}}"),
            range: open..close + 1,
        });
        return;
    }
    let Some(c) = ICU_ARG.captures(inner) else {
        return;
    };
    let name = c[1].to_string();
    let kind = c[2].to_ascii_lowercase();
    let head_end = c.get(2).map_or(open + 1, |m| open + 1 + m.end());
    out.push(Ph {
        key: format!("{{{name}}}"),
        range: open..head_end,
    });
    if !matches!(kind.as_str(), "plural" | "select" | "selectordinal") {
        return;
    }
    let Some(opts) = c.get(3) else {
        return;
    };
    // Sub-messages: `one {…} other {…}`; placeholders inside count once per name.
    let base = open + 1 + opts.start();
    let body = opts.as_str();
    let mut k = 0;
    while let Some(off) = body[k..].find('{') {
        let at = base + k + off;
        let Some(end) = matching_brace(s, at) else {
            break;
        };
        let sub = &s[at + 1..end];
        let st = Styles {
            brace: true,
            ..Styles::default()
        };
        for mut p in extract(sub, st) {
            p.range = p.range.start + at + 1..p.range.end + at + 1;
            out.push(p);
        }
        k = end + 1 - base;
    }
}

/// Distinct keys.
pub fn keys(ph: &[Ph]) -> BTreeSet<String> {
    ph.iter().map(|p| p.key.clone()).collect()
}

/// Names that usually carry a plural count.
pub fn is_count_name(key: &str) -> bool {
    let name = key.trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
    matches!(
        name.to_ascii_lowercase().as_str(),
        "count" | "n" | "num" | "number" | "quantity" | "amount" | "total" | "cnt"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn k(s: &str, st: Styles) -> Vec<String> {
        extract(s, st).into_iter().map(|p| p.key).collect()
    }

    #[test]
    fn printf_positions() {
        let st = Styles {
            printf: true,
            printf_space: true,
            ..Styles::default()
        };
        assert_eq!(k("%s has %d items, 100%%", st), ["%1$s", "%2$d"]);
        assert_eq!(k("%2$i kohdetta: %1$s", st), ["%2$d", "%1$s"]);
        assert_eq!(k("%ld and %.2f", st), ["%1$ld", "%2$f"]);
    }

    #[test]
    fn named_styles() {
        let st = Styles {
            percent_brace: true,
            ..Styles::default()
        };
        assert_eq!(k("%{count} files in {dir}", st), ["%{count}"]);
        let st = Styles {
            printf: true,
            py_named: true,
            ..Styles::default()
        };
        assert_eq!(k("%(name)s and %(n)d", st), ["%(name)s", "%(n)d"]);
        let st = Styles {
            brace: true,
            ..Styles::default()
        };
        assert_eq!(
            k("{name} {0} {} {} {{literal}}", st),
            ["{name}", "{0}", "{#0}", "{#1}"]
        );
        let icu = "{count, plural, one {# file by {user}} other {# files by {user}}}";
        assert_eq!(k(icu, st), ["{count}", "{user}", "{user}"]);
        let got = extract(icu, st);
        assert_eq!(&icu[got[1].range.clone()], "{user}");
    }

    #[test]
    fn auto_detection() {
        let e = crate::extract::gettext::parse("msgid \"50% of {name}\"\nmsgstr \"\"\n");
        let st = styles(&e.entries[0]);
        assert!(!st.printf && st.brace && !st.percent_brace);
        let e = crate::extract::gettext::parse(
            "#, elixir-format\nmsgid \"%{count} of {x}\"\nmsgstr \"\"\n",
        );
        let st = styles(&e.entries[0]);
        assert!(st.percent_brace && !st.brace && !st.printf);
        let e = crate::extract::gettext::parse("msgid \"%{count} files\"\nmsgstr \"\"\n");
        let st = styles(&e.entries[0]);
        assert!(st.percent_brace && !st.brace && !st.printf, "{st:?}");
    }
}
