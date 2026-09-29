//! D2 (terrastruct d2lang) validator for fenced ```d2 blocks.
//!
//! Pure Rust: a scannerless parser mirroring `d2parser` plus semantic checks
//! from `d2compiler` (reserved keywords, shapes, style values, classes, vars).

mod keywords;
mod parser;
mod validate;

use crate::diagnostic::Finding;
use crate::rules::Out;

const RULE: &str = "diagram/d2";

/// Validate D2 source `body` located at absolute byte `offset` in the file.
pub fn check(body: &str, offset: usize, out: &mut Out) {
    let (root, mut diags) = parser::parse(body);
    diags.extend(validate::validate(&root));
    diags.sort_by_key(|d| (d.range.start, d.range.end));
    diags.dedup_by(|a, b| a.range == b.range && a.msg == b.msg);
    for d in diags {
        let range = offset + d.range.start..offset + d.range.end;
        let mut f = Finding::new(RULE, d.severity, range.clone(), d.msg);
        if let Some(h) = d.help {
            f = f.help(h);
        }
        if let Some(s) = d.suggest {
            f = f.suggest(s);
        }
        if let Some(fix) = d.fix {
            f = f.fix(range, fix);
        }
        out.push(f);
    }
}

#[cfg(test)]
mod tests;
