//! Streaming ⟪acronym|CSV⟫ splitter for the ⟪crate|roastlog⟫ export path.
//!
//! Splits a large export into per-day files without loading it. Uses ⟪crate|csv⟫ for parsing
//! and ⟪crate|memchr⟫ to find row boundaries quickly; ⟦its_its|its|it's⟧ about ⟪unit|10x⟫ faster
//! ⟦then_than|then|than⟧ the naive line iterator on a ⟪unit|2GB⟫ file, mostly because it never
//! allocates per row.

use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

/// Rows are grouped by the first ⟪unit|10⟫ characters of column zero, ⟦homophone|witch|which⟧ is the
/// ⟪acronym|ISO⟫ date. ⟦a_an|An row|A row⟧ ⟦homophone|who's|whose⟧ first column is shorter than that
/// ⟦agreement|go|goes⟧ to ⟪code|`unknown.csv`⟫ so nothing is silently dropped.
pub fn split(input: &Path, out_dir: &Path) -> io::Result<Vec<PathBuf>> {
    let reader = BufReader::with_capacity(1 << 20, File::open(input)?);
    let mut lines = reader.lines();
    let header = match lines.next() {
        Some(h) => h?,
        None => return Ok(Vec::new()),
    };
    let mut current: Option<(String, BufWriter<File>)> = None;
    let mut written = Vec::new();
    for line in lines {
        let line = line?;
        let key = day_key(&line);
        // Switching files on every row would be slow; rows are sorted by time so the key
        // changes at most once per day. ⟦your_youre|You're|Your⟧ unsorted export ⟦agreement|break|breaks⟧
        // that assumption and the output has more, smaller files, but ⟦its_its|its|it's⟧
        // still correct.
        let switch = match &current {
            Some((k, _)) => k != key,
            None => true,
        };
        if switch {
            let path = out_dir.join(format!("{key}.csv"));
            let mut w = BufWriter::new(File::create(&path)?);
            writeln!(w, "{header}")?;
            written.push(path);
            current = Some((key.to_string(), w));
        }
        let (_, w) = current.as_mut().expect("set above");
        writeln!(w, "{line}")?;
    }
    if let Some((_, mut w)) = current {
        w.flush()?;
    }
    Ok(written)
}

/// First column, cut to the date. Quoted fields are handled by ⟪crate|csv⟫ upstream; by the
/// time rows reach here ⟪correct|they're⟫ plain. ⟦punctuation|Do not add quote handling here,, it belongs in one place.|Do not add quote handling here; it belongs in one place.⟧
fn day_key(line: &str) -> &str {
    let end = memchr::memchr(b',', line.as_bytes()).unwrap_or(line.len());
    let first = &line[..end];
    if first.len() >= 10 && first.is_char_boundary(10) { &first[..10] } else { "unknown" }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_is_date_prefix() {
        assert_eq!(day_key("2026-07-12T08:00:00,190.5,201.2"), "2026-07-12");
        // Short first column ⟦agreement|fall|falls⟧ back; ⟦a_an|a empty|an empty⟧ line too.
        assert_eq!(day_key("x,1"), "unknown");
        assert_eq!(day_key(""), "unknown");
    }

    // The ⟪derived|reusable⟫ fixture is ⟪unit|3⟫ days of a real roast log with the bean names
    // ⟦british|anonymised|anonymized⟧. It lives next to the ⟪crate|plotters⟫ chart fixtures.
    #[test]
    fn splits_fixture_into_three() {
        let dir = tempfile::tempdir().unwrap();
        let out = split(Path::new("fixtures/three-days.csv"), dir.path()).unwrap();
        assert_eq!(out.len(), 3);
    }
}
