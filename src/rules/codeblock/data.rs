//! ```toml and ```yaml syntax checks.

use super::{has_placeholder, point};
use crate::diagnostic::{Finding, Severity};
use crate::rules::Out;

pub fn check_toml(body: &str, offset: usize, out: &mut Out) {
    if has_placeholder(body, false) {
        return;
    }
    let Err(e) = body.parse::<toml::Table>() else {
        return;
    };
    let range = match e.span() {
        Some(s) if s.end > s.start && s.end <= body.len() => {
            let start = crate::source::floor_char(body, s.start);
            // Keep the range on one line.
            let end = body[start..s.end]
                .find(['\r', '\n'])
                .map_or(s.end, |i| start + i)
                .max(start);
            if end > start {
                offset + start..offset + end
            } else {
                point(body, offset, start)
            }
        }
        Some(s) => point(body, offset, s.start),
        None => point(body, offset, 0),
    };
    out.push(
        Finding::new(
            "codeblock/toml",
            Severity::Error,
            range,
            format!("Invalid TOML: {}", e.message().trim()),
        )
        .help("Fix the syntax, or add `skip-lint` to the info string"),
    );
}

pub fn check_yaml(body: &str, offset: usize, out: &mut Out) {
    if has_placeholder(body, true) {
        return;
    }
    let parser = saphyr_parser::Parser::new_from_str(body);
    for ev in parser {
        let Err(e) = ev else { continue };
        let pos = char_to_byte(body, e.marker().index());
        out.push(
            Finding::new(
                "codeblock/yaml",
                Severity::Error,
                point(body, offset, pos),
                format!("Invalid YAML: {}", e.info()),
            )
            .help("Fix the syntax, or add `skip-lint` to the info string"),
        );
        return;
    }
}

fn char_to_byte(s: &str, chars: usize) -> usize {
    s.char_indices().nth(chars).map_or(s.len(), |(i, _)| i)
}
