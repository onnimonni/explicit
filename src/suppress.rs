//! Inline suppression directives.
//!
//! Markdown: `<!-- explicit-disable rule1 rule2 -->`, `<!-- explicit-enable -->`,
//! `<!-- explicit-disable-next-line rule -->`, `<!-- explicit-disable-line -->`,
//! `<!-- explicit-disable-file -->`. Code: the same words at the start of a comment.
//! Rule lists accept globs; no list means all rules. Text after ` -- ` is a reason.
//! `explicit-lang` markers (see `crate::lang_marks`) are directives too: kept out of prose.

use std::ops::Range;
use std::sync::LazyLock;

use regex::Regex;

use crate::config::glob_match;
use crate::diagnostic::Finding;
use crate::rules::Analyzed;

/// Directive anchored at the start of comment content.
static DIRECTIVE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^\s*explicit-(disable-next-line|disable-line|disable-file|disable|enable|lang)\b([^\n]*)",
    )
    .expect("hardcoded regex is valid")
});

#[derive(Debug, Clone)]
enum Scope {
    /// Byte range where the rules are off.
    Range(Range<usize>),
}

#[derive(Debug, Clone)]
struct Suppression {
    /// Rule patterns; empty means all rules.
    rules: Vec<String>,
    /// Rule patterns re-enabled inside this scope.
    except: Vec<String>,
    scope: Scope,
}

#[derive(Debug, Default)]
pub struct Suppressions {
    items: Vec<Suppression>,
    /// Ranges of directive comments; kept out of prose checks.
    pub directive_ranges: Vec<Range<usize>>,
}

fn parse_rules(rest: &str) -> Vec<String> {
    let rest = rest.split(" -- ").next().unwrap_or("");
    let rest = rest
        .trim()
        .trim_end_matches("-->")
        .trim_end_matches("*/")
        .trim();
    rest.split(|c: char| c.is_whitespace() || c == ',')
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect()
}

fn rule_matches(pattern: &str, rule: &str) -> bool {
    glob_match(pattern, rule) || rule.starts_with(&format!("{pattern}/"))
}

/// Open `explicit-disable` region: rules (empty = all), re-enabled rules, start.
struct Open {
    rules: Vec<String>,
    except: Vec<String>,
    start: usize,
}

impl Suppressions {
    pub fn from(a: &Analyzed) -> Suppressions {
        let src = &a.file.text;
        // (comment range, directive kind, directive rest)
        let mut directives: Vec<(Range<usize>, &str, &str)> = Vec::new();
        if let Some(md) = &a.md {
            for r in &md.html {
                let raw = &src[r.clone()];
                // Each HTML comment body must start with the directive.
                for (i, _) in raw.match_indices("<!--") {
                    let body = &raw[i + 4..];
                    let body = body.find("-->").map_or(body, |e| &body[..e]);
                    if let Some(c) = DIRECTIVE_RE.captures(body) {
                        let (Some(kind), Some(rest)) = (c.get(1), c.get(2)) else {
                            continue;
                        };
                        directives.push((r.clone(), kind.as_str(), rest.as_str()));
                    }
                }
            }
        } else {
            for c in &a.comments {
                for l in &c.lines {
                    if let Some(cap) = DIRECTIVE_RE.captures(&src[l.content.clone()]) {
                        let (Some(kind), Some(rest)) = (cap.get(1), cap.get(2)) else {
                            continue;
                        };
                        directives.push((l.raw.clone(), kind.as_str(), rest.as_str()));
                    }
                }
            }
        }
        let mut s = Suppressions::default();
        let mut open: Vec<Open> = Vec::new();
        for (range, kind, rest) in directives {
            s.directive_ranges.push(range.clone());
            let rules = parse_rules(rest);
            let at = range.start;
            let mut push = |rules: Vec<String>, except: Vec<String>, r: Range<usize>| {
                s.items.push(Suppression {
                    rules,
                    except,
                    scope: Scope::Range(r),
                });
            };
            match kind {
                "disable" => open.push(Open {
                    rules,
                    except: Vec::new(),
                    start: range.end,
                }),
                "enable" if rules.is_empty() => {
                    for o in open.drain(..) {
                        push(o.rules, o.except, o.start..at);
                    }
                }
                "enable" => {
                    let mut keep = Vec::new();
                    for o in open.drain(..) {
                        let remaining: Vec<String> = o
                            .rules
                            .iter()
                            .filter(|r| !rules.contains(r))
                            .cloned()
                            .collect();
                        let all = o.rules.is_empty();
                        // Region before the enable stays fully suppressed.
                        push(o.rules, o.except.clone(), o.start..at);
                        if all || !remaining.is_empty() {
                            let mut except = o.except;
                            except.extend(rules.iter().cloned());
                            keep.push(Open {
                                rules: remaining,
                                except,
                                start: at,
                            });
                        }
                    }
                    open = keep;
                }
                "disable-file" => push(rules, Vec::new(), 0..src.len()),
                "disable-line" => {
                    let (ls, le) = line_bounds(src, range.start);
                    push(rules, Vec::new(), ls..le);
                }
                "disable-next-line" => {
                    let (_, le) = line_bounds(src, range.end.saturating_sub(1));
                    let next_start = (le + 1).min(src.len());
                    let (_, next_end) = line_bounds(src, next_start);
                    push(rules, Vec::new(), next_start..next_end.max(next_start));
                }
                _ => {}
            }
        }
        for o in open {
            s.items.push(Suppression {
                rules: o.rules,
                except: o.except,
                scope: Scope::Range(o.start..src.len()),
            });
        }
        s
    }

    pub fn is_suppressed(&self, f: &Finding) -> bool {
        self.items.iter().any(|s| {
            let Scope::Range(r) = &s.scope;
            let hit = f.range.start >= r.start && f.range.start <= r.end;
            hit && (s.rules.is_empty() || s.rules.iter().any(|p| rule_matches(p, &f.rule)))
                && !s.except.iter().any(|p| rule_matches(p, &f.rule))
        })
    }
}

/// Line containing byte `at`; byte-based so `at` may fall inside a multibyte char.
fn line_bounds(src: &str, at: usize) -> (usize, usize) {
    let bytes = src.as_bytes();
    let at = at.min(bytes.len());
    let start = bytes[..at]
        .iter()
        .rposition(|&b| b == b'\n')
        .map_or(0, |i| i + 1);
    let end = bytes[at..]
        .iter()
        .position(|&b| b == b'\n')
        .map_or(bytes.len(), |i| at + i);
    (start, end)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::Severity;
    use crate::source::{FileKind, SourceFile};

    fn analyzed(src: &str) -> Analyzed {
        Analyzed::new(SourceFile::new(
            "a.md".into(),
            "a.md".into(),
            FileKind::Markdown,
            src.into(),
        ))
    }

    fn f(rule: &str, at: usize) -> Finding {
        Finding::new(rule, Severity::Warning, at..at + 1, "x")
    }

    #[test]
    fn markdown_directives() {
        let src = "a\n<!-- explicit-disable-next-line spelling -->\nbad\nbad\n<!-- explicit-disable slop/* -->\nx\n<!-- explicit-enable -->\ny\n";
        let a = analyzed(src);
        let s = Suppressions::from(&a);
        let first_bad = src.find("bad").unwrap();
        let second_bad = src.rfind("bad").unwrap();
        assert!(s.is_suppressed(&f("spelling", first_bad)));
        assert!(!s.is_suppressed(&f("spelling", second_bad)));
        assert!(!s.is_suppressed(&f("md/x", first_bad)));
        assert!(s.is_suppressed(&f("slop/phrase", src.find("\nx").unwrap() + 1)));
        assert!(!s.is_suppressed(&f("slop/phrase", src.find("\ny").unwrap() + 1)));
    }

    fn code(src: &str) -> Analyzed {
        Analyzed::new(SourceFile::new(
            "a.rs".into(),
            "a.rs".into(),
            FileKind::Code(crate::source::Lang::Rust),
            src.into(),
        ))
    }

    #[test]
    fn disable_all_then_enable_one_keeps_others_disabled() {
        let src = "<!-- explicit-disable -->\na\n<!-- explicit-enable spelling -->\nb\n<!-- explicit-enable -->\nc\n";
        let s = Suppressions::from(&analyzed(src));
        let a = src.find("\na").unwrap() + 1;
        let b = src.find("\nb").unwrap() + 1;
        let c = src.find("\nc").unwrap() + 1;
        assert!(s.is_suppressed(&f("spelling", a)));
        assert!(s.is_suppressed(&f("slop/x", a)));
        assert!(!s.is_suppressed(&f("spelling", b)));
        assert!(s.is_suppressed(&f("slop/x", b)));
        assert!(!s.is_suppressed(&f("slop/x", c)));
        assert!(!s.is_suppressed(&f("spelling", c)));
    }

    #[test]
    fn disable_all_enable_one_until_eof() {
        let src = "// explicit-disable\n// explicit-enable spelling\nfn a() {}\n";
        let s = Suppressions::from(&code(src));
        let at = src.find("fn a").unwrap();
        assert!(!s.is_suppressed(&f("spelling", at)));
        assert!(s.is_suppressed(&f("slop/x", at)));
    }

    #[test]
    fn directive_must_start_comment() {
        let src = "// see explicit-disable-file for details\nfn a() {}\n";
        let s = Suppressions::from(&code(src));
        assert!(!s.is_suppressed(&f("spelling", src.find("fn a").unwrap())));
        assert!(s.directive_ranges.is_empty());
        let md = "Use <!-- note: explicit-disable-file --> here\nx\n";
        let s = Suppressions::from(&analyzed(md));
        assert!(!s.is_suppressed(&f("spelling", md.find("\nx").unwrap() + 1)));
        let md = "<!--explicit-disable-file-->\nx\n";
        let s = Suppressions::from(&analyzed(md));
        assert!(s.is_suppressed(&f("spelling", md.find("\nx").unwrap() + 1)));
    }

    #[test]
    fn next_line_with_multibyte_reason_does_not_panic() {
        let src = "// explicit-disable-next-line spelling -- 🦀\n// sentance\n";
        let s = Suppressions::from(&code(src));
        assert!(s.is_suppressed(&f("spelling", src.find("sentance").unwrap())));
        assert_eq!(line_bounds("a🦀\nb", 2), (0, 5));
    }
}
