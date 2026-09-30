//! Source files, languages and line/column lookup.

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Lang {
    Rust,
    Go,
    Python,
    JavaScript,
    TypeScript,
    Shell,
    Nix,
    Elixir,
    Zig,
    C,
    Cpp,
    Ruby,
    Java,
    CSharp,
    Php,
    Toml,
    Yaml,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileKind {
    Markdown,
    Code(Lang),
    /// GNU gettext PO/POT catalog.
    Gettext,
}

impl FileKind {
    pub fn detect(path: &Path) -> Option<FileKind> {
        let name = path.file_name()?.to_str()?;
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        Some(match ext.to_ascii_lowercase().as_str() {
            "md" | "markdown" | "mdx" => FileKind::Markdown,
            "po" | "pot" => FileKind::Gettext,
            "rs" => FileKind::Code(Lang::Rust),
            "go" => FileKind::Code(Lang::Go),
            "py" | "pyi" => FileKind::Code(Lang::Python),
            "js" | "jsx" | "mjs" | "cjs" => FileKind::Code(Lang::JavaScript),
            "ts" | "tsx" | "mts" | "cts" => FileKind::Code(Lang::TypeScript),
            "sh" | "bash" | "zsh" => FileKind::Code(Lang::Shell),
            "nix" => FileKind::Code(Lang::Nix),
            "ex" | "exs" => FileKind::Code(Lang::Elixir),
            "zig" => FileKind::Code(Lang::Zig),
            "c" | "h" => FileKind::Code(Lang::C),
            "cc" | "cpp" | "cxx" | "hpp" | "hh" | "hxx" => FileKind::Code(Lang::Cpp),
            "rb" => FileKind::Code(Lang::Ruby),
            "java" | "kt" | "kts" | "scala" | "swift" => FileKind::Code(Lang::Java),
            "cs" => FileKind::Code(Lang::CSharp),
            "php" => FileKind::Code(Lang::Php),
            "toml" => FileKind::Code(Lang::Toml),
            "yml" | "yaml" => FileKind::Code(Lang::Yaml),
            _ => match name {
                "Makefile" | "Dockerfile" | "Justfile" | "justfile" => FileKind::Code(Lang::Shell),
                _ => return None,
            },
        })
    }
}

#[derive(Debug, Clone)]
pub struct SourceFile {
    /// Absolute path.
    pub path: PathBuf,
    /// Path relative to the project root, used in output.
    pub rel: PathBuf,
    pub kind: FileKind,
    pub text: String,
    line_starts: Vec<usize>,
}

impl SourceFile {
    pub fn new(path: PathBuf, rel: PathBuf, kind: FileKind, text: String) -> Self {
        let mut line_starts = vec![0];
        line_starts.extend(text.match_indices('\n').map(|(i, _)| i + 1));
        SourceFile {
            path,
            rel,
            kind,
            text,
            line_starts,
        }
    }

    /// 1-based line and 1-based column (in chars) for a byte offset.
    pub fn line_col(&self, offset: usize) -> (usize, usize) {
        let offset = offset.min(self.text.len());
        let line = self.line_starts.partition_point(|&s| s <= offset) - 1;
        let start = self.line_starts[line];
        let col = self.text[start..floor_char(&self.text, offset)]
            .chars()
            .count()
            + 1;
        (line + 1, col)
    }

    /// Byte range of a 1-based line, without the trailing newline.
    pub fn line_range(&self, line: usize) -> std::ops::Range<usize> {
        let start = self.line_starts[line - 1];
        let end = self
            .line_starts
            .get(line)
            .map_or(self.text.len(), |&e| e - 1);
        let end = if end > start && self.text.as_bytes()[end - 1] == b'\r' {
            end - 1
        } else {
            end
        };
        start..end
    }

    pub fn line_count(&self) -> usize {
        self.line_starts.len()
    }
}

pub fn floor_char(s: &str, mut i: usize) -> usize {
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}
