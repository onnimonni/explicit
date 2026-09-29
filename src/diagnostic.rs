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
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Fix {
    pub range: Range<usize>,
    pub replacement: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Diagnostic {
    pub path: PathBuf,
    pub rule: String,
    pub severity: Severity,
    /// Byte range in the original file.
    pub range: Range<usize>,
    pub line: usize,
    pub column: usize,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub suggestions: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fix: Option<Fix>,
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
