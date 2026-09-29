//! Diagnostics shared by all rules.

use std::ops::Range;
use std::path::PathBuf;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Warning,
    Error,
}

impl Severity {
    pub fn as_str(self) -> &'static str {
        match self {
            Severity::Info => "info",
            Severity::Warning => "warning",
            Severity::Error => "error",
        }
    }
}

/// A text edit on the original file. Only attached when applying it is safe.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Fix {
    pub range: Range<usize>,
    pub replacement: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Diagnostic {
    pub path: PathBuf,
    pub rule: String,
    pub severity: Severity,
    /// Byte range in the original file.
    pub range: Range<usize>,
    pub line: usize,
    pub column: usize,
    /// Position just past the range (1-based line, column in characters).
    pub end_line: usize,
    pub end_column: usize,
    /// Flagged source text (the range), truncated to [`MAX_TEXT_CHARS`] characters.
    pub text: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub help: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub suggestions: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub fix: Option<Fix>,
}

/// Longest [`Diagnostic::text`] kept, in characters.
pub const MAX_TEXT_CHARS: usize = 200;

/// `src[range]` cut to [`MAX_TEXT_CHARS`] characters (with a trailing `…` when cut).
pub fn snippet(src: &str, range: &Range<usize>) -> String {
    let s = src.get(range.clone()).unwrap_or("");
    match s.char_indices().nth(MAX_TEXT_CHARS) {
        Some((i, _)) => format!("{}…", &s[..i]),
        None => s.to_string(),
    }
}

/// What a rule emits; path, line and column get filled in by the engine.
#[derive(Debug, Clone)]
pub struct Finding {
    pub rule: String,
    pub severity: Severity,
    pub range: Range<usize>,
    pub message: String,
    pub help: Option<String>,
    pub suggestions: Vec<String>,
    pub fix: Option<Fix>,
}

impl Finding {
    pub fn new(
        rule: impl Into<String>,
        severity: Severity,
        range: Range<usize>,
        message: impl Into<String>,
    ) -> Self {
        Finding {
            rule: rule.into(),
            severity,
            range,
            message: message.into(),
            help: None,
            suggestions: Vec::new(),
            fix: None,
        }
    }

    pub fn help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    pub fn suggest(mut self, s: impl Into<String>) -> Self {
        self.suggestions.push(s.into());
        self
    }

    pub fn fix(mut self, range: Range<usize>, replacement: impl Into<String>) -> Self {
        self.fix = Some(Fix {
            range,
            replacement: replacement.into(),
        });
        self
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn snippet_truncates_on_char_boundary() {
        let s = "ä".repeat(250);
        let t = super::snippet(&s, &(0..s.len()));
        assert_eq!(t.chars().count(), super::MAX_TEXT_CHARS + 1);
        assert!(t.ends_with('…'));
        assert_eq!(super::snippet("abc", &(1..3)), "bc");
        assert_eq!(super::snippet("abc", &(5..9)), "");
    }
}
