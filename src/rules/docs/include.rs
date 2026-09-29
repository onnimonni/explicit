//! `docs/include-missing`: include directives pointing at files that do not exist.
//!
//! `mkdocs` snippets `--8<-- "path"`, Jekyll `{% include path %}` / `{% include_relative path %}`,
//! Hugo `{{< readfile file="path" >}}`, `<!-- include: path -->`, markdown-include `{!path!}`.

use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;

use super::{normalize, project_root};
use crate::diagnostic::{Finding, Severity};
use crate::extract::markdown::MdDoc;
use crate::links::has_scheme;
use crate::rules::{FileCtx, Out};

const RULE: &str = "docs/include-missing";

/// (pattern, also match inside fenced code blocks). Snippets are commonly used in code blocks.
static DIRECTIVES: LazyLock<Vec<(Regex, bool)>> = LazyLock::new(|| {
    [
        (r#"(?m)^[ \t]*-+8<-+[ \t]+["']([^"'\r\n]+)["']"#, true),
        (r"\{%-?\s*include(?:_relative)?\s+([^\s%{}]+)", false),
        (
            r#"\{\{[<%]\s*readfile\s+(?:file\s*=\s*)?["']([^"']+)["']"#,
            false,
        ),
        (r"(?i)<!--\s*include:\s*([^\s>]+?)\s*-->", false),
        (r"\{!\s*([^!{}\r\n]+?)\s*!\}", false),
    ]
    .into_iter()
    .map(|(re, code)| (Regex::new(re).expect("hardcoded regex is valid"), code))
    .collect()
});

/// Snippets block form: paths on the lines between two bare `--8<--` lines.
static SNIPPET_BLOCK_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)^[ \t]*-+8<-+[ \t]*\r?$").expect("hardcoded regex is valid"));

struct Resolver {
    bases: Vec<PathBuf>,
}

pub fn check(ctx: &FileCtx, out: &mut Out) {
    let Some(md) = &ctx.a.md else { return };
    let src = ctx.src();
    let root = project_root(ctx);
    let mut bases: Vec<PathBuf> = ctx
        .a
        .file
        .path
        .parent()
        .map(Path::to_path_buf)
        .into_iter()
        .collect();
    bases.push(root.clone());
    bases.extend(ctx.config.docs.include_dirs.iter().map(|d| {
        if d.is_absolute() {
            d.clone()
        } else {
            root.join(d)
        }
    }));
    let r = Resolver { bases };

    for (re, in_code) in DIRECTIVES.iter() {
        for c in re.captures_iter(src) {
            let (Some(whole), Some(m)) = (c.get(0), c.get(1)) else {
                continue;
            };
            if !skipped(md, src, whole.start(), *in_code) {
                r.report(m.as_str(), m.range(), out);
            }
        }
    }
    let marks: Vec<_> = SNIPPET_BLOCK_RE
        .find_iter(src)
        .filter(|m| !skipped(md, src, m.start(), true))
        .collect();
    for pair in marks.as_chunks::<2>().0 {
        let mut pos = pair[0].end();
        for line in src[pos..pair[1].start()].split_inclusive('\n') {
            let t = line.trim();
            if !t.is_empty() && !t.starts_with(';') {
                let off = pos + line.find(t).unwrap_or(0);
                r.report(t, off..off + t.len(), out);
            }
            pos += line.len();
        }
    }
}

/// Inside a code span, escaped with a leading `;` (snippets), or in a code block when not allowed.
fn skipped(md: &MdDoc, src: &str, at: usize, allow_code_blocks: bool) -> bool {
    let line_start = src[..at].rfind('\n').map_or(0, |i| i + 1);
    md.code_spans.iter().any(|r| r.contains(&at))
        || src[line_start..at].trim().ends_with(';')
        || (!allow_code_blocks && md.in_code_block(at))
}

impl Resolver {
    fn report(&self, raw: &str, range: Range<usize>, out: &mut Out) {
        let raw = raw.trim();
        if raw.is_empty() || has_scheme(raw) || raw.contains("{{") || raw.contains("${") {
            return;
        }
        // Snippets line/section selectors: `file.md:3:5`, `file.md:section`.
        let file_start = raw.rfind('/').map_or(0, |i| i + 1);
        let bare = raw[file_start..].find(':').map(|i| &raw[..file_start + i]);
        let exists = std::iter::once(raw).chain(bare).any(|c| {
            let rel = c.trim_start_matches('/');
            self.bases.iter().any(|b| normalize(&b.join(rel)).exists())
        });
        if !exists {
            out.push(
                Finding::new(
                    RULE,
                    Severity::Error,
                    range,
                    format!("Included file not found: {raw}"),
                )
                .help(
                    "Paths resolve relative to the file, the project root, then docs.include_dirs",
                ),
            );
        }
    }
}
