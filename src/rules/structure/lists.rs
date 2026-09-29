//! List rules: MD004, MD005/MD007, MD029, MD030, MD032, task list checkboxes.

use super::Ctx;
use super::lines::width;
use crate::extract::markdown::List;
use crate::rules::Out;

/// Offset of the list marker of an item.
fn marker(c: &Ctx, item_start: usize) -> usize {
    let b = c.src.as_bytes();
    let mut i = item_start;
    while i < b.len() && (b[i] == b' ' || b[i] == b'\t') {
        i += 1;
    }
    i
}

/// Indentation column of the marker, relative to the block quote prefix.
fn col(c: &Ctx, m: usize) -> usize {
    let l = &c.lines.v[c.lines.index_of(m)];
    if m < l.body {
        return 0;
    }
    width(&c.src[l.body..m])
}

/// Ordered item number and marker length (digits + delimiter).
fn number(c: &Ctx, m: usize) -> Option<(u64, usize)> {
    let digits: String = c.src[m..]
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    let n = digits.parse().ok()?;
    Some((n, digits.len() + 1))
}

pub fn check(c: &mut Ctx, out: &mut Out) {
    let md = c.md;
    let mut lists: Vec<&List> = md.lists.iter().collect();
    lists.sort_by_key(|l| l.range.start);

    if c.on("md/list-marker") {
        list_marker(c, &lists, out);
    }
    if c.on("md/list-indent") {
        list_indent(c, &lists, out);
    }
    if c.on("md/ol-prefix") {
        ol_prefix(c, &lists, out);
    }
    if c.on("md/blanks-around-lists") {
        blanks_around(c, &lists, out);
    }
    if c.on("md/list-marker-space") {
        marker_space(c, &lists, out);
    }
    if c.on("md/task-list-style") {
        task_style(c, &lists, out);
    }
}

/// Marker offset, marker length and the number of spaces after it, for items with content.
fn item_parts(c: &Ctx, item_start: usize) -> Option<(usize, usize, usize)> {
    let m = marker(c, item_start);
    let len = match c.src.as_bytes().get(m)? {
        b'-' | b'*' | b'+' => 1,
        b'0'..=b'9' => number(c, m)?.1,
        _ => return None,
    };
    let l = &c.lines.v[c.lines.index_of(m)];
    let after = c.src.get(m + len..l.end)?;
    let n = after.bytes().take_while(|&b| b == b' ').count();
    if after[n..].trim().is_empty() || after[n..].starts_with('\t') {
        return None;
    }
    Some((m, len, n))
}

fn marker_space(c: &Ctx, lists: &[&List], out: &mut Out) {
    for l in lists {
        let parts: Vec<(usize, usize, usize)> = l
            .items
            .iter()
            .filter_map(|it| item_parts(c, it.start))
            .collect();
        let content_col = |&(m, len, n): &(usize, usize, usize)| col(c, m) + len + n;
        let aligned = l.ordered
            && parts.len() > 1
            && parts
                .iter()
                .all(|p| content_col(p) == content_col(&parts[0]));
        for &(m, len, n) in &parts {
            // 5+ spaces start an indented code block inside the item.
            if n == 1 || n >= 5 || aligned {
                continue;
            }
            let r = m + len..m + len + n;
            out.push(
                c.finding(
                    "md/list-marker-space",
                    r.clone(),
                    format!("{n} spaces after list marker, expected 1"),
                )
                .fix(r, " "),
            );
        }
    }
}

fn task_style(c: &Ctx, lists: &[&List], out: &mut Out) {
    for l in lists {
        for it in &l.items {
            let Some((m, len, n)) = item_parts(c, it.start) else {
                continue;
            };
            let s = m + len + n;
            let rest = &c.src[s..];
            let ends = |k: usize| rest[k..].starts_with([' ', '\t', '\r', '\n']) || rest.len() == k;
            let (r, msg, rep) = if rest.starts_with("[X]") && ends(3) {
                (s..s + 3, "Use lowercase [x] for checked tasks", "[x]")
            } else if rest.starts_with("[]") && ends(2) {
                (s..s + 2, "Malformed task checkbox [], use [ ]", "[ ]")
            } else {
                continue;
            };
            out.push(c.finding("md/task-list-style", r.clone(), msg).fix(r, rep));
        }
    }
}

fn list_marker(c: &Ctx, lists: &[&List], out: &mut Out) {
    let fixed = match c.fc.config.markdown.list_marker.as_str() {
        "dash" => Some(b'-'),
        "asterisk" => Some(b'*'),
        "plus" => Some(b'+'),
        _ => None,
    };
    let mut expected = fixed;
    for l in lists.iter().filter(|l| !l.ordered) {
        for it in &l.items {
            let m = marker(c, it.start);
            let Some(&ch) = c.src.as_bytes().get(m) else {
                continue;
            };
            if !matches!(ch, b'-' | b'*' | b'+') {
                continue;
            }
            let want = *expected.get_or_insert(ch);
            if ch != want {
                let (got, want) = (ch as char, want as char);
                out.push(
                    c.finding(
                        "md/list-marker",
                        m..m + 1,
                        format!("List marker '{got}' should be '{want}'"),
                    )
                    .fix(m..m + 1, want.to_string()),
                );
            }
        }
    }
}

fn parent<'a>(lists: &[&'a List], l: &List) -> Option<&'a List> {
    if l.depth == 0 {
        return None;
    }
    lists
        .iter()
        .filter(|p| {
            p.depth + 1 == l.depth && p.range.start <= l.range.start && l.range.end <= p.range.end
        })
        .min_by_key(|p| p.range.len())
        .copied()
}

fn list_indent(c: &Ctx, lists: &[&List], out: &mut Out) {
    // MD005: items of one list share their indentation (ordered lists may right-align numbers).
    for l in lists {
        let Some(first) = l.items.first() else {
            continue;
        };
        let m0 = marker(c, first.start);
        let (c0, e0) = (col(c, m0), col(c, m0) + number(c, m0).map_or(1, |n| n.1));
        for it in &l.items[1..] {
            let m = marker(c, it.start);
            let ci = col(c, m);
            let ei = ci + number(c, m).map_or(1, |n| n.1);
            if ci != c0 && !(l.ordered && ei == e0) {
                out.push(c.finding(
                    "md/list-indent",
                    c.lines.v[c.lines.index_of(m)].start..m + 1,
                    format!("List item indented {ci} spaces, expected {c0} like the first item"),
                ));
            }
        }
    }

    // MD007: unordered lists inside unordered lists use one consistent indentation step.
    let mut step: Option<usize> = None;
    for l in lists.iter().filter(|l| !l.ordered) {
        let Some(first) = l.items.first() else {
            continue;
        };
        let m = marker(c, first.start);
        let ci = col(c, m);
        let line_start = c.lines.v[c.lines.index_of(m)].start;
        let Some(p) = parent(lists, l) else {
            if l.depth == 0 && ci > 0 {
                out.push(c.finding(
                    "md/list-indent",
                    line_start..m + 1,
                    format!("Top-level list indented {ci} spaces"),
                ));
            }
            continue;
        };
        let mut all_unordered = !p.ordered;
        let mut q = p;
        while all_unordered && let Some(pp) = parent(lists, q) {
            all_unordered = !pp.ordered;
            q = pp;
        }
        if !all_unordered {
            continue;
        }
        let Some(pi) = p.items.iter().rev().find(|it| it.start <= l.range.start) else {
            continue;
        };
        let pc = col(c, marker(c, pi.start));
        let s = ci.saturating_sub(pc);
        let want = *step.get_or_insert(s);
        if s != want {
            out.push(c.finding(
                "md/list-indent",
                line_start..m + 1,
                format!("Nested list indented {s} spaces from its parent, expected {want}"),
            ));
        }
    }
}

fn ol_prefix(c: &Ctx, lists: &[&List], out: &mut Out) {
    for l in lists.iter().filter(|l| l.ordered) {
        let nums: Vec<(u64, usize)> = l
            .items
            .iter()
            .filter_map(|it| number(c, marker(c, it.start)).map(|n| (n.0, marker(c, it.start))))
            .collect();
        if nums.len() < 2 {
            continue;
        }
        let (a, b) = (nums[0].0, nums[1].0);
        let same = a == b && a <= 1;
        for (i, &(n, m)) in nums.iter().enumerate() {
            let want = if same { a } else { a + i as u64 };
            if n != want {
                let len = n.to_string().len();
                let style = if same {
                    format!("all {a}s")
                } else {
                    "sequential".to_string()
                };
                out.push(
                    c.finding(
                        "md/ol-prefix",
                        m..m + len,
                        format!("Ordered list prefix {n}, expected {want}"),
                    )
                    .help(format!("Number items {style} (1/2/3 or all 1s)")),
                );
            }
        }
    }
}

fn blanks_around(c: &mut Ctx, lists: &[&List], out: &mut Out) {
    for l in lists.iter().filter(|l| l.depth == 0) {
        if l.range.is_empty() {
            continue;
        }
        let first = c.lines.index_of(marker(c, l.range.start));
        let mut last = c.lines.index_of(l.range.end - 1);
        while last > first && c.lines.v[last].blank {
            last -= 1;
        }
        if c.missing_blank_before(first) {
            let ln = &c.lines.v[first];
            let f = c.finding(
                "md/blanks-around-lists",
                ln.start..ln.end,
                "List should be preceded by a blank line",
            );
            let at = ln.start;
            let f = c.with_blank_line(f, at, first);
            out.push(f);
        }
        if c.missing_blank_after(last) {
            let at = c.lines.v[last + 1].start;
            let f = c.finding(
                "md/blanks-around-lists",
                c.eol(last),
                "List should be followed by a blank line",
            );
            let f = c.with_blank_line(f, at, last);
            out.push(f);
        }
    }
}
