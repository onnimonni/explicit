//! Table rules: MD055, MD056, MD058.

use super::Ctx;
use crate::rules::Out;

/// Positions of unescaped `|` in a row.
fn pipes(row: &str) -> Vec<usize> {
    let b = row.as_bytes();
    let mut v = Vec::new();
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'\\' => i += 1,
            b'|' => v.push(i),
            _ => {}
        }
        i += 1;
    }
    v
}

struct Row {
    line: usize,
    leading: bool,
    trailing: bool,
    cells: usize,
}

fn row(c: &Ctx, i: usize) -> Row {
    let l = &c.lines.v[i];
    let raw = &c.src[l.body..l.end];
    let t = raw.trim();
    let p = pipes(t);
    let leading = t.starts_with('|');
    let trailing = t.len() > 1 && p.last() == Some(&(t.len() - 1));
    let cells = p.len() + 1 - usize::from(leading) - usize::from(trailing);
    Row {
        line: i,
        leading,
        trailing,
        cells,
    }
}

fn style(r: &Row) -> &'static str {
    match (r.leading, r.trailing) {
        (true, true) => "leading and trailing pipes",
        (true, false) => "leading pipe only",
        (false, true) => "trailing pipe only",
        (false, false) => "no leading or trailing pipes",
    }
}

pub fn check(c: &mut Ctx, out: &mut Out) {
    let count = c.on("md/table-column-count");
    let pipe = c.on("md/table-pipe-style");
    let blanks = c.on("md/blanks-around-tables");
    if !(count || pipe || blanks) {
        return;
    }
    let mut want_style: Option<(bool, bool)> = None;
    for t in &c.md.tables {
        if t.is_empty() {
            continue;
        }
        let first = c.lines.index_of(t.start);
        let mut last = c.lines.index_of(t.end - 1);
        while last > first && c.lines.v[last].blank {
            last -= 1;
        }
        let rows: Vec<Row> = (first..=last).map(|i| row(c, i)).collect();
        let head = rows[0].cells;
        for r in &rows {
            let l = &c.lines.v[r.line];
            if count && r.cells != head {
                let what = if r.cells > head { "extra" } else { "missing" };
                out.push(c.finding(
                    "md/table-column-count",
                    l.body..l.end,
                    format!(
                        "Table row has {} cells, header has {head} ({what} cells)",
                        r.cells
                    ),
                ));
            }
            if pipe {
                let want = *want_style.get_or_insert((r.leading, r.trailing));
                if (r.leading, r.trailing) != want {
                    let exp = Row {
                        line: 0,
                        leading: want.0,
                        trailing: want.1,
                        cells: 0,
                    };
                    out.push(c.finding(
                        "md/table-pipe-style",
                        l.body..l.end,
                        format!("Table row has {}, expected {}", style(r), style(&exp)),
                    ));
                }
            }
        }
        if !blanks {
            continue;
        }
        if c.missing_blank_before(first) {
            let ln = &c.lines.v[first];
            let f = c.finding(
                "md/blanks-around-tables",
                ln.body..ln.end,
                "Table should be preceded by a blank line",
            );
            let at = ln.start;
            let f = c.with_blank_line(f, at, first);
            out.push(f);
        }
        if c.missing_blank_after(last) {
            let at = c.lines.v[last + 1].start;
            let f = c.finding(
                "md/blanks-around-tables",
                c.eol(last),
                "Table should be followed by a blank line",
            );
            let f = c.with_blank_line(f, at, last);
            out.push(f);
        }
    }
}
