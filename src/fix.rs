//! Apply safe fixes.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::diagnostic::{Diagnostic, Fix};

/// Apply non-overlapping fixes to `text`. Returns the new text and how many were applied.
///
/// An insertion (empty range) touching a replacement is skipped: both usually come from
/// different rules repairing the same spot (e.g. a regenerated TOC that already ends with a
/// blank line plus a "blank line around list" insertion). Identical insertions at one offset
/// apply once. Skipped fixes get another chance on the next run.
pub fn apply(text: &str, fixes: &[&Fix]) -> (String, usize) {
    let mut fixes: Vec<&Fix> = fixes.to_vec();
    // Replacements before insertions at the same offset, so the insertion counts as touching.
    fixes.sort_by_key(|f| (f.range.start, f.range.is_empty()));
    let mut out = String::with_capacity(text.len());
    let mut pos = 0;
    let mut applied = 0;
    let mut last: Option<&Fix> = None;
    for f in fixes {
        if f.range.start < pos
            || f.range.end > text.len()
            || !text.is_char_boundary(f.range.start)
            || !text.is_char_boundary(f.range.end)
        {
            continue;
        }
        if f.range.is_empty()
            && let Some(l) = last
            && l.range.end == f.range.start
            && (!l.range.is_empty() || l.replacement == f.replacement)
        {
            continue;
        }
        out.push_str(&text[pos..f.range.start]);
        out.push_str(&f.replacement);
        pos = f.range.end;
        applied += 1;
        last = Some(f);
    }
    out.push_str(&text[pos..]);
    (out, applied)
}

/// Write fixes for all diagnostics to disk. Returns number of fixes applied.
pub fn write_fixes(root: &Path, diags: &[Diagnostic]) -> std::io::Result<usize> {
    let mut by_file: BTreeMap<&PathBuf, Vec<&Fix>> = BTreeMap::new();
    for d in diags {
        if let Some(f) = &d.fix {
            by_file.entry(&d.path).or_default().push(f);
        }
    }
    let mut total = 0;
    for (rel, fixes) in by_file {
        let path = root.join(rel);
        let text = std::fs::read_to_string(&path)?;
        let (new, n) = apply(&text, &fixes);
        if n > 0 && new != text {
            replace_file(&path, new.as_bytes())?;
            total += n;
        }
    }
    Ok(total)
}

/// Atomically replace `path` with `data`, keeping its permissions. The temp file beside it
/// is created exclusively under a fresh name, so an existing file or symlink is never
/// followed or clobbered.
fn replace_file(path: &Path, data: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let perms = std::fs::metadata(path)?.permissions();
    let dir = path.parent().unwrap_or(Path::new("."));
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.subsec_nanos());
    let (tmp, mut file) = loop {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let tmp = dir.join(format!(
            ".{name}.explicit-{}-{nanos:x}-{n}.tmp",
            std::process::id()
        ));
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)
        {
            Ok(f) => break (tmp, f),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists && n < 1000 => {}
            Err(e) => return Err(e),
        }
    };
    let res = file
        .write_all(data)
        .and_then(|()| file.sync_all())
        .and_then(|()| std::fs::set_permissions(&tmp, perms))
        .and_then(|()| std::fs::rename(&tmp, path));
    if res.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    res
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlapping_fixes_skipped() {
        let a = Fix {
            range: 0..3,
            replacement: "x".into(),
        };
        let b = Fix {
            range: 2..4,
            replacement: "y".into(),
        };
        let c = Fix {
            range: 5..6,
            replacement: "".into(),
        };
        let (out, n) = apply("abcdefg", &[&b, &a, &c]);
        assert_eq!(out, "xdeg");
        assert_eq!(n, 2);
    }

    fn fx(range: std::ops::Range<usize>, r: &str) -> Fix {
        Fix {
            range,
            replacement: r.into(),
        }
    }

    #[test]
    fn insertion_touching_replacement_skipped() {
        // Regenerated block ending in a blank line + a blank-line insertion at its end.
        let regen = fx(2..5, "\n\nX\n\n");
        let blank = fx(5..5, "\n");
        let (out, n) = apply("A\nold\nB", &[&blank, &regen]);
        assert_eq!(out, "A\n\n\nX\n\n\nB");
        assert_eq!(n, 1);
        // Insertion at a replacement's start: replacement wins.
        let (out, n) = apply("abc", &[&fx(1..1, "-"), &fx(1..2, "B")]);
        assert_eq!((out.as_str(), n), ("aBc", 1));
        // Identical insertions apply once; different ones both apply.
        let (out, n) = apply("ab", &[&fx(1..1, "\n"), &fx(1..1, "\n")]);
        assert_eq!((out.as_str(), n), ("a\nb", 1));
        let (out, n) = apply("ab", &[&fx(1..1, "x"), &fx(1..1, "y")]);
        assert_eq!((out.as_str(), n), ("axyb", 2));
    }

    #[cfg(unix)]
    #[test]
    fn write_fixes_keeps_permissions_and_ignores_stale_tmp() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("a.md");
        std::fs::write(&p, "hello").unwrap();
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o750)).unwrap();
        // A pre-existing file at the old predictable temp path must stay untouched.
        let old_tmp = dir.path().join("a.explicit-tmp");
        std::fs::write(&old_tmp, "keep").unwrap();
        replace_file(&p, b"bye").unwrap();
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "bye");
        let mode = std::fs::metadata(&p).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o750);
        assert_eq!(std::fs::read_to_string(&old_tmp).unwrap(), "keep");
        let left: Vec<_> = std::fs::read_dir(dir.path()).unwrap().collect();
        assert_eq!(left.len(), 2, "no temp files left behind");
    }
}
