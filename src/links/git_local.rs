//! Read-only questions to the local git repository for same-repo links.
//!
//! Object lookups go through one `git cat-file --batch-check` process per run (the only
//! external programs explicit runs are `git` and `gh`, never through a shell). Git metadata
//! files (`.git`, `commondir`, `shallow`, `config`) are read directly.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

/// Longest a `git` or `gh` process may run before it is killed.
pub const TIMEOUT: Duration = Duration::from_secs(30);

/// Where git keeps a working tree's metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitDirs {
    /// Working tree top.
    pub top: PathBuf,
    /// Per-worktree git dir (`.git`, or `gitdir:` of a `.git` file); `None` if unreadable.
    pub git_dir: Option<PathBuf>,
    /// Shared git dir (`commondir` of linked worktrees, else `git_dir`).
    pub common: Option<PathBuf>,
}

/// Find `.git` at or above `start`, following worktree/submodule `.git` files and `commondir`.
pub fn git_dirs(start: &Path) -> Option<GitDirs> {
    let (top, dot_git) = start
        .ancestors()
        .map(|d| (d, d.join(".git")))
        .find(|(_, g)| g.exists())?;
    let git_dir = if dot_git.is_file() {
        std::fs::read_to_string(&dot_git).ok().and_then(|t| {
            t.lines()
                .find_map(|l| l.strip_prefix("gitdir:"))
                .map(|p| top.join(p.trim()))
        })
    } else {
        Some(dot_git)
    };
    let common = git_dir.as_ref().map(|g| {
        std::fs::read_to_string(g.join("commondir"))
            .ok()
            .map_or_else(|| g.clone(), |c| g.join(c.trim()))
    });
    Some(GitDirs {
        top: top.to_path_buf(),
        git_dir,
        common,
    })
}

/// Clone properties that make missing objects inconclusive.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CloneKind {
    /// History is cut off (`shallow` file): old commits and tags may be absent.
    pub shallow: bool,
    /// Partial clone (promisor remote): blobs may be absent, so path lookups are skipped.
    pub partial: bool,
}

pub fn clone_kind(dirs: &GitDirs) -> CloneKind {
    let dirs_iter = || dirs.git_dir.iter().chain(dirs.common.iter());
    let shallow = dirs_iter().any(|d| d.join("shallow").is_file());
    let partial = dirs_iter().any(|d| {
        std::fs::read_to_string(d.join("config")).is_ok_and(|c| {
            c.lines().any(|l| {
                let l = l.trim().to_ascii_lowercase().replace(' ', "");
                l.starts_with("partialclone=") || l == "promisor=true"
            })
        })
    });
    CloneKind { shallow, partial }
}

/// Answer of `git cat-file --batch-check` for one object name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Obj {
    /// Exists; holds the object type (`commit`, `tree`, `blob`, `tag`).
    Found(String),
    Missing,
    /// Abbreviated name matching several objects.
    Ambiguous,
}

/// Look up object names (`<rev>`, `<rev>^{commit}`, `<rev>:<path>`) in the repository at `top`
/// with one `git cat-file --batch-check`; `None` when git is unavailable or fails.
pub fn batch_check(top: &Path, specs: &[String]) -> Option<HashMap<String, Obj>> {
    let specs: Vec<&String> = specs
        .iter()
        .filter(|s| !s.is_empty() && !s.contains(['\n', '\r', '\0']))
        .collect();
    if specs.is_empty() {
        return Some(HashMap::new());
    }
    let mut input = Vec::new();
    for s in &specs {
        input.extend_from_slice(s.as_bytes());
        input.push(b'\n');
    }
    let mut cmd = Command::new("git");
    cmd.arg("-C")
        .arg(top)
        .args(["cat-file", "--batch-check"])
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_OPTIONAL_LOCKS", "0")
        // Partial clones would otherwise fetch missing objects from the network.
        .env("GIT_NO_LAZY_FETCH", "1")
        .env("LC_ALL", "C");
    let (status, out) = run(&mut cmd, Some(input), TIMEOUT)?;
    if !status.success() {
        return None;
    }
    parse_batch_check(&specs, &String::from_utf8_lossy(&out))
}

fn parse_batch_check(specs: &[&String], out: &str) -> Option<HashMap<String, Obj>> {
    let lines: Vec<&str> = out.lines().collect();
    if lines.len() != specs.len() {
        return None;
    }
    let mut m = HashMap::new();
    for (spec, line) in specs.iter().zip(lines) {
        let obj = if line.ends_with(" missing") {
            Obj::Missing
        } else if line.ends_with(" ambiguous") {
            Obj::Ambiguous
        } else {
            let mut parts = line.split(' ');
            match (parts.next(), parts.next()) {
                (Some(sha), Some(ty)) if sha.chars().all(|c| c.is_ascii_hexdigit()) => {
                    Obj::Found(ty.to_string())
                }
                _ => return None,
            }
        };
        m.insert((*spec).clone(), obj);
    }
    Some(m)
}

/// Run `cmd` (never through a shell) with optional stdin, capturing stdout; stderr is discarded.
/// Kills it after `timeout`. `None` when it cannot be spawned or times out.
pub fn run(
    cmd: &mut Command,
    input: Option<Vec<u8>>,
    timeout: Duration,
) -> Option<(ExitStatus, Vec<u8>)> {
    cmd.stdin(if input.is_some() {
        Stdio::piped()
    } else {
        Stdio::null()
    })
    .stdout(Stdio::piped())
    .stderr(Stdio::null());
    let mut child = cmd.spawn().ok()?;
    let stdin = child.stdin.take();
    let mut stdout = child.stdout.take()?;
    // Separate writer and reader threads: a full pipe on either side must not deadlock.
    let writer = std::thread::spawn(move || {
        if let (Some(mut w), Some(data)) = (stdin, input) {
            let _ = w.write_all(&data);
        }
    });
    let reader = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stdout.read_to_end(&mut buf);
        buf
    });
    let start = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) => break Some(s),
            Ok(None) if start.elapsed() < timeout => {
                std::thread::sleep(Duration::from_millis(5));
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
        }
    };
    let _ = writer.join();
    let out = reader.join().ok()?;
    Some((status?, out))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_batch_output() {
        let specs: Vec<String> = ["a^{commit}", "b:x.md", "abc", "t"]
            .map(String::from)
            .to_vec();
        let refs: Vec<&String> = specs.iter().collect();
        let out = "0123abcd commit 200\nb:x.md missing\nabc ambiguous\n89ab tag 150\n";
        let m = parse_batch_check(&refs, out).unwrap();
        assert_eq!(m["a^{commit}"], Obj::Found("commit".into()));
        assert_eq!(m["b:x.md"], Obj::Missing);
        assert_eq!(m["abc"], Obj::Ambiguous);
        assert_eq!(m["t"], Obj::Found("tag".into()));
        assert!(parse_batch_check(&refs, "0123 commit 1\n").is_none());
        assert!(parse_batch_check(&refs[..1], "fatal: bad\n").is_none());
    }

    #[test]
    fn clone_kinds_and_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("main/.git/worktrees/wt")).unwrap();
        std::fs::write(root.join("main/.git/worktrees/wt/commondir"), "../..\n").unwrap();
        std::fs::create_dir_all(root.join("wt/sub")).unwrap();
        std::fs::write(root.join("wt/.git"), "gitdir: ../main/.git/worktrees/wt\n").unwrap();
        let d = git_dirs(&root.join("wt/sub")).unwrap();
        assert_eq!(d.top, root.join("wt"));
        assert!(d.common.unwrap().join("worktrees").is_dir());
        let d = git_dirs(&root.join("wt")).unwrap();
        assert_eq!(clone_kind(&d), CloneKind::default());
        std::fs::write(root.join("main/.git/shallow"), "0123\n").unwrap();
        std::fs::write(
            root.join("main/.git/config"),
            "[remote \"origin\"]\n\tpromisor = true\n",
        )
        .unwrap();
        assert_eq!(
            clone_kind(&d),
            CloneKind {
                shallow: true,
                partial: true
            }
        );
    }

    #[test]
    fn run_times_out_and_missing_binary() {
        assert!(run(&mut Command::new("explicit-no-such-binary"), None, TIMEOUT).is_none());
    }
}
