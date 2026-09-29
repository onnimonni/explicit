//! Diagram validation in fenced code blocks (```mermaid, ```d2).

pub mod d2;
#[cfg(feature = "mermaid")]
pub mod mermaid;

use super::{FileCtx, Out};

pub fn check(ctx: &FileCtx, out: &mut Out) {
    let Some(md) = &ctx.a.md else { return };
    let src = ctx.src();
    for block in md.code_blocks.iter().filter(|b| b.fenced) {
        let Some(lang) = block.lang.as_deref() else {
            continue;
        };
        let Some((body, offset)) = fence_body(src, block.range.clone()) else {
            continue;
        };
        match lang.to_ascii_lowercase().as_str() {
            "d2" if ctx.enabled("diagram/d2") => d2::check(body, offset, out),
            #[cfg(feature = "mermaid")]
            "mermaid" if ctx.enabled("diagram/mermaid") => mermaid::check(body, offset, out),
            _ => {}
        }
    }
}

/// Content of a fenced code block and its absolute start offset (after the opening fence line).
pub fn fence_body(src: &str, range: std::ops::Range<usize>) -> Option<(&str, usize)> {
    let raw = &src[range.clone()];
    let first_nl = raw.find('\n')?;
    let body_start = first_nl + 1;
    let mut body = &raw[body_start..];
    // Drop the closing fence line if present.
    let trimmed = body.trim_end_matches(['\n', '\r', ' ']);
    if let Some(last_nl) = trimmed.rfind('\n').map(|i| i + 1).or(Some(0)) {
        let last = trimmed[last_nl..].trim_start();
        if last.starts_with("```") || last.starts_with("~~~") {
            body = &body[..last_nl];
        }
    }
    Some((body, range.start + body_start))
}
