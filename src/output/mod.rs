//! Output formats.

use std::collections::HashMap;
use std::io::{self, IsTerminal, Write};
use std::path::PathBuf;

use crate::diagnostic::{Diagnostic, Severity};

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Format {
    Human,
    Json,
    Sarif,
    Github,
}

pub fn write(
    format: Format,
    diags: &[Diagnostic],
    sources: &HashMap<PathBuf, String>,
    w: &mut dyn Write,
) -> io::Result<()> {
    match format {
        Format::Human => human(diags, sources, w, io::stdout().is_terminal()),
        Format::Json => {
            serde_json::to_writer_pretty(&mut *w, diags)?;
            writeln!(w)
        }
        Format::Sarif => {
            serde_json::to_writer_pretty(&mut *w, &sarif(diags))?;
            writeln!(w)
        }
        Format::Github => {
            for d in diags {
                let level = match d.severity {
                    Severity::Error => "error",
                    Severity::Warning => "warning",
                    Severity::Info => "notice",
                };
                let msg = d
                    .message
                    .replace('%', "%25")
                    .replace('\r', "%0D")
                    .replace('\n', "%0A");
                writeln!(
                    w,
                    "::{level} file={},line={},col={},title={}::{msg}",
                    d.path.display(),
                    d.line,
                    d.column,
                    d.rule
                )?;
            }
            Ok(())
        }
    }
}

fn human(
    diags: &[Diagnostic],
    sources: &HashMap<PathBuf, String>,
    w: &mut dyn Write,
    color: bool,
) -> io::Result<()> {
    let paint = |code: &str, s: &str| {
        if color {
            format!("\x1b[{code}m{s}\x1b[0m")
        } else {
            s.to_string()
        }
    };
    for d in diags {
        let (sev, code) = match d.severity {
            Severity::Error => ("error", "1;31"),
            Severity::Warning => ("warning", "1;33"),
            Severity::Info => ("info", "1;36"),
        };
        writeln!(
            w,
            "{}: {} {}",
            paint(code, sev),
            d.message,
            paint("2", &format!("[{}]", d.rule))
        )?;
        writeln!(
            w,
            "  {} {}:{}:{}",
            paint("34", "-->"),
            d.path.display(),
            d.line,
            d.column
        )?;
        if let Some(src) = sources.get(&d.path)
            && let Some(line) = src.lines().nth(d.line - 1)
        {
            let line = line.trim_end_matches('\r');
            let start_col = d.column - 1;
            let len = src.get(d.range.clone()).map_or(1, |s| {
                s.split('\n').next().unwrap_or("").chars().count().max(1)
            });
            let len = len.min(line.chars().count().saturating_sub(start_col).max(1));
            let pad: String = line
                .chars()
                .take(start_col)
                .map(|c| if c == '\t' { '\t' } else { ' ' })
                .collect();
            let num = d.line.to_string();
            writeln!(w, "{} {} ", " ".repeat(num.len()), paint("34", "|"))?;
            writeln!(w, "{} {} {}", paint("34", &num), paint("34", "|"), line)?;
            writeln!(
                w,
                "{} {} {}{}",
                " ".repeat(num.len()),
                paint("34", "|"),
                pad,
                paint(code, &"^".repeat(len))
            )?;
        }
        if !d.suggestions.is_empty() {
            let s: Vec<String> = d
                .suggestions
                .iter()
                .take(3)
                .map(|s| format!("\"{s}\""))
                .collect();
            writeln!(w, "  {} did you mean {}?", paint("32", "="), s.join(", "))?;
        }
        if let Some(h) = &d.help {
            writeln!(w, "  {} {}", paint("32", "help:"), h)?;
        }
        writeln!(w)?;
    }
    let count = |s| diags.iter().filter(|d| d.severity == s).count();
    let files = diags
        .iter()
        .map(|d| &d.path)
        .collect::<std::collections::HashSet<_>>()
        .len();
    if diags.is_empty() {
        writeln!(w, "{}", paint("32", "No problems found."))?;
    } else {
        writeln!(
            w,
            "{} errors, {} warnings, {} infos in {} files",
            count(Severity::Error),
            count(Severity::Warning),
            count(Severity::Info),
            files
        )?;
    }
    Ok(())
}

fn sarif(diags: &[Diagnostic]) -> serde_json::Value {
    use serde_json::json;
    let mut rules: Vec<&str> = diags.iter().map(|d| d.rule.as_str()).collect();
    rules.sort();
    rules.dedup();
    let results: Vec<_> = diags
        .iter()
        .map(|d| {
            json!({
                "ruleId": d.rule,
                "level": match d.severity { Severity::Error => "error", Severity::Warning => "warning", Severity::Info => "note" },
                "message": { "text": d.message },
                "locations": [{
                    "physicalLocation": {
                        "artifactLocation": { "uri": d.path.to_string_lossy().replace('\\', "/") },
                        "region": { "startLine": d.line, "startColumn": d.column, "byteOffset": d.range.start, "byteLength": d.range.len() }
                    }
                }]
            })
        })
        .collect();
    json!({
        "version": "2.1.0",
        "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
        "runs": [{
            "tool": { "driver": {
                "name": "explicit",
                "version": env!("CARGO_PKG_VERSION"),
                "rules": rules.iter().map(|r| json!({ "id": r })).collect::<Vec<_>>()
            }},
            "columnKind": "unicodeCodePoints",
            "results": results
        }]
    })
}
