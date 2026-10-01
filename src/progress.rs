//! A one-line progress bar on stderr for interactive runs.
//!
//! Shown only when stderr is a terminal and no agent or CI is driving the process (see
//! [`wanted`]). Agents such as Claude Code and Codex read stderr through a pipe and also set
//! environment variables; both hide the bar, so their transcripts stay clean. `EXPLICIT_PROGRESS=0`
//! or `=1` overrides the detection.

use std::io::{IsTerminal, Write};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Redraw at most this often; the last item of a phase always draws.
const MIN_REDRAW: Duration = Duration::from_millis(80);
const BAR_WIDTH: usize = 20;
const DEFAULT_COLUMNS: usize = 80;

/// Progress reporter; a disabled one costs an atomic check per tick.
#[derive(Debug, Default)]
pub struct Progress {
    state: Option<Mutex<State>>,
}

#[derive(Debug, Default)]
struct State {
    phase: String,
    done: usize,
    total: usize,
    columns: usize,
    last_draw: Option<Instant>,
    drawn: bool,
}

impl Progress {
    /// Never draws.
    pub fn disabled() -> Self {
        Progress::default()
    }

    /// Draws when [`wanted`] says so.
    pub fn auto() -> Self {
        if wanted(|k| std::env::var(k).ok(), std::io::stderr().is_terminal()) {
            Progress::always()
        } else {
            Progress::disabled()
        }
    }

    /// Draws regardless of the terminal.
    pub fn always() -> Self {
        Progress {
            state: Some(Mutex::new(State {
                columns: columns(),
                ..State::default()
            })),
        }
    }

    pub fn enabled(&self) -> bool {
        self.state.is_some()
    }

    /// Begin a phase of `total` steps; `phase` is a short verb phrase like `Checking files`.
    pub fn start(&self, phase: &str, total: usize) {
        let Some(m) = &self.state else { return };
        let mut s = m.lock().unwrap_or_else(|e| e.into_inner());
        s.phase = phase.to_string();
        s.done = 0;
        s.total = total;
        s.last_draw = None;
        draw(&mut s, "");
    }

    /// One step done; `item` names it (a path or URL) and may be empty.
    pub fn tick(&self, item: &str) {
        let Some(m) = &self.state else { return };
        let mut s = m.lock().unwrap_or_else(|e| e.into_inner());
        s.done += 1;
        let due = s
            .last_draw
            .is_none_or(|t| t.elapsed() >= MIN_REDRAW || s.done >= s.total);
        if due {
            draw(&mut s, item);
        }
    }

    /// Clear the line, so regular output starts at the left margin.
    pub fn finish(&self) {
        let Some(m) = &self.state else { return };
        let mut s = m.lock().unwrap_or_else(|e| e.into_inner());
        if s.drawn {
            let mut err = std::io::stderr().lock();
            let _ = err.write_all(b"\r\x1b[2K");
            let _ = err.flush();
            s.drawn = false;
        }
    }
}

impl Drop for Progress {
    fn drop(&mut self) {
        self.finish();
    }
}

fn draw(s: &mut State, item: &str) {
    let line = render(&s.phase, s.done, s.total, item, s.columns);
    let mut err = std::io::stderr().lock();
    let _ = err.write_all(b"\r\x1b[2K");
    let _ = err.write_all(line.as_bytes());
    let _ = err.flush();
    s.drawn = true;
    s.last_draw = Some(Instant::now());
}

/// The bar line, without the carriage return: `Checking files ████░░ 12/30 README.md`,
/// truncated to `columns` characters.
fn render(phase: &str, done: usize, total: usize, item: &str, columns: usize) -> String {
    let filled = if total == 0 {
        BAR_WIDTH
    } else {
        (done.min(total) * BAR_WIDTH).div_ceil(total)
    };
    let mut line = format!(
        "{phase} {}{} {done}/{total}",
        "█".repeat(filled),
        "░".repeat(BAR_WIDTH - filled)
    );
    let used = line.chars().count();
    let room = columns.saturating_sub(used + 1);
    if !item.is_empty() && room >= 4 {
        line.push(' ');
        let n = item.chars().count();
        if n <= room {
            line.push_str(item);
        } else {
            line.push('…');
            line.extend(item.chars().skip(n - (room - 1)));
        }
    }
    line
}

/// Whether to show the bar: stderr is a terminal, `TERM` is not `dumb`, and no CI or coding
/// agent is running the process. `EXPLICIT_PROGRESS=0`/`1` overrides everything else.
///
/// Agents detected by environment: Claude Code (`CLAUDECODE`), Codex (`CODEX_*`), Cursor
/// (`CURSOR_AGENT`), plus the common `CI` flag.
pub fn wanted(env: impl Fn(&str) -> Option<String>, stderr_is_terminal: bool) -> bool {
    match env("EXPLICIT_PROGRESS").as_deref().map(str::trim) {
        Some("0") | Some("false") | Some("no") | Some("off") => return false,
        Some("1") | Some("true") | Some("yes") | Some("on") => return true,
        _ => {}
    }
    if !stderr_is_terminal {
        return false;
    }
    if env("TERM").is_some_and(|t| t == "dumb") {
        return false;
    }
    const AGENTS: [&str; 6] = [
        "CI",
        "CLAUDECODE",
        "CODEX_SANDBOX",
        "CODEX_SANDBOX_NETWORK_DISABLED",
        "CODEX_THREAD_ID",
        "CURSOR_AGENT",
    ];
    !AGENTS.iter().any(|k| env(k).is_some_and(|v| !v.is_empty()))
}

fn columns() -> usize {
    std::env::var("COLUMNS")
        .ok()
        .and_then(|c| c.trim().parse().ok())
        .filter(|&c: &usize| c >= 20)
        .unwrap_or(DEFAULT_COLUMNS)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env_of<'a>(vars: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |k| {
            vars.iter()
                .find(|(n, _)| *n == k)
                .map(|(_, v)| v.to_string())
        }
    }

    #[test]
    fn shown_only_on_a_terminal_without_agents() {
        assert!(wanted(env_of(&[]), true));
        assert!(!wanted(env_of(&[]), false));
        assert!(!wanted(env_of(&[("TERM", "dumb")]), true));
        assert!(!wanted(env_of(&[("CI", "true")]), true));
        assert!(!wanted(env_of(&[("CLAUDECODE", "1")]), true));
        assert!(!wanted(
            env_of(&[("CODEX_SANDBOX_NETWORK_DISABLED", "1")]),
            true
        ));
        assert!(!wanted(env_of(&[("CURSOR_AGENT", "1")]), true));
        // Empty values don't count as set.
        assert!(wanted(env_of(&[("CI", "")]), true));
    }

    #[test]
    fn explicit_progress_overrides_detection() {
        assert!(wanted(
            env_of(&[("EXPLICIT_PROGRESS", "1"), ("CI", "1")]),
            false
        ));
        assert!(!wanted(env_of(&[("EXPLICIT_PROGRESS", "0")]), true));
        assert!(!wanted(env_of(&[("EXPLICIT_PROGRESS", "off")]), true));
    }

    #[test]
    fn renders_bar_and_counts() {
        let line = render("Checking files", 0, 4, "", 80);
        assert_eq!(line, format!("Checking files {} 0/4", "░".repeat(20)));
        let line = render("Checking files", 2, 4, "README.md", 80);
        assert_eq!(
            line,
            format!(
                "Checking files {}{} 2/4 README.md",
                "█".repeat(10),
                "░".repeat(10)
            )
        );
        let line = render("Checking files", 4, 4, "", 80);
        assert!(line.ends_with(&format!("{} 4/4", "█".repeat(20))));
    }

    #[test]
    fn truncates_long_items_from_the_left() {
        let item = "src/some/deeply/nested/directory/file.rs";
        let line = render("Checking files", 1, 9, item, 50);
        assert!(line.chars().count() <= 50, "{line}");
        assert!(line.contains('…'));
        assert!(line.ends_with("file.rs"));
        // No room at all: the item is dropped, the bar stays.
        let line = render("Checking files", 1, 9, item, 42);
        assert!(line.ends_with("1/9"), "{line}");
    }

    #[test]
    fn empty_total_fills_the_bar() {
        assert_eq!(
            render("Reading", 0, 0, "", 80),
            format!("Reading {} 0/0", "█".repeat(20))
        );
    }

    #[test]
    fn disabled_progress_is_a_no_op() {
        let p = Progress::disabled();
        assert!(!p.enabled());
        p.start("x", 3);
        p.tick("a");
        p.finish();
    }
}
