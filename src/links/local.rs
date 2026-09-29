//! Local link checks: relative files, anchors, reference definitions and `http://` links.

use std::collections::HashSet;
use std::ops::Range;
use std::path::{Component, Path, PathBuf};

use super::{
    dest_range, encode_path_segment, has_scheme, is_excluded, near_misses, percent_decode,
    report_range, sev,
};
use crate::diagnostic::Finding;
use crate::engine::Workspace;
use crate::extract::markdown::{LinkKind, MdDoc, URL_RE};
use crate::rules::{Analyzed, FileCtx, Out};
use crate::source::FileKind;

/// A local link destination split into its parts.
#[derive(Debug)]
enum Target<'a> {
    /// `#frag`
    SameFile { frag: &'a str },
    /// `path[?query][#frag]` resolved to an absolute, lexically normalized path.
    Path {
        path: PathBuf,
        raw_path: &'a str,
        query: Option<&'a str>,
        frag: Option<&'a str>,
        root_relative: bool,
    },
}

fn resolve<'a>(dest: &'a str, base_dir: &Path, root: &Path) -> Option<Target<'a>> {
    let dest = dest.trim();
    if dest.is_empty()
        || dest.contains("{{")
        || dest.contains("{%")
        || dest.contains("${")
        || has_scheme(dest)
        || dest.starts_with("//")
    {
        return None;
    }
    let (before_frag, frag) = match dest.split_once('#') {
        Some((p, f)) => (p, Some(f)),
        None => (dest, None),
    };
    let (raw_path, query) = match before_frag.split_once('?') {
        Some((p, q)) => (p, Some(q)),
        None => (before_frag, None),
    };
    if raw_path.is_empty() {
        return frag
            .filter(|f| !f.is_empty() && query.is_none())
            .map(|frag| Target::SameFile { frag });
    }
    let decoded = percent_decode(raw_path);
    if decoded.contains('\0') {
        return None;
    }
    let root_relative = decoded.starts_with('/');
    let joined = if root_relative {
        root.join(decoded.trim_start_matches('/'))
    } else {
        base_dir.join(&decoded)
    };
    Some(Target::Path {
        path: normalize(&joined),
        raw_path,
        query,
        frag,
        root_relative,
    })
}

/// Resolve `.` and `..` without touching the filesystem.
fn normalize(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                if matches!(out.components().next_back(), Some(Component::Normal(_))) {
                    out.pop();
                } else if !out.has_root() {
                    out.push("..");
                }
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

#[derive(Debug, PartialEq, Eq)]
enum Exists {
    File,
    Dir,
    Missing,
    /// Exists only case-insensitively; holds the path with on-disk casing.
    CaseMismatch(PathBuf),
}

/// Case-sensitive lookups over the workspace's shared directory listings.
struct FsCache<'a> {
    ws: &'a Workspace,
}

impl FsCache<'_> {
    fn list(&self, dir: &Path) -> crate::engine::DirListing {
        self.ws.list_dir(dir)
    }

    /// Case-sensitive existence check. Components shared with `trusted` are assumed correct.
    fn lookup(&self, path: &Path, trusted: &Path) -> Exists {
        let pc: Vec<Component> = path.components().collect();
        let common = pc
            .iter()
            .zip(trusted.components())
            .take_while(|(a, b)| **a == *b)
            .count();
        let mut cur: PathBuf = pc[..common].iter().map(|c| c.as_os_str()).collect();
        let mut mismatch = false;
        for c in &pc[common..] {
            match c {
                Component::Normal(name) => {
                    let Some(entries) = self.list(&cur) else {
                        return Exists::Missing;
                    };
                    if entries.iter().any(|e| e == name) {
                        cur.push(name);
                    } else {
                        let lower = name.to_string_lossy().to_lowercase();
                        match entries
                            .iter()
                            .find(|e| e.to_string_lossy().to_lowercase() == lower)
                        {
                            Some(actual) => {
                                mismatch = true;
                                cur.push(actual);
                            }
                            None => return Exists::Missing,
                        }
                    }
                }
                other => cur.push(other.as_os_str()),
            }
        }
        match std::fs::metadata(&cur) {
            Err(_) => Exists::Missing,
            Ok(_) if mismatch => Exists::CaseMismatch(cur),
            Ok(m) if m.is_dir() => Exists::Dir,
            Ok(_) => Exists::File,
        }
    }
}

fn is_markdown(p: &Path) -> bool {
    FileKind::detect(p) == Some(FileKind::Markdown)
}

/// `#L10`, `#L10-L20`, `#L10C5`.
fn is_line_anchor(frag: &str) -> bool {
    let re = |s: &str| {
        let s = s.strip_prefix('L').unwrap_or("");
        let digits = s.bytes().take_while(u8::is_ascii_digit).count();
        digits > 0
            && s[digits..]
                .strip_prefix('C')
                .map_or(s.len() == digits, |c| {
                    !c.is_empty() && c.bytes().all(|b| b.is_ascii_digit())
                })
    };
    match frag.split_once('-') {
        Some((a, b)) => re(a) && re(b),
        None => re(frag),
    }
}

fn root_dir(config: &crate::config::Config) -> PathBuf {
    let r = match &config.links.root_dir {
        Some(d) if d.is_absolute() => d.clone(),
        Some(d) => config.root.join(d),
        None => config.root.clone(),
    };
    r.canonicalize().unwrap_or(r)
}

fn display(p: &Path, root: &Path) -> String {
    p.strip_prefix(root).unwrap_or(p).display().to_string()
}

struct Checker<'a> {
    ws: &'a Workspace,
    fs: FsCache<'a>,
    base: PathBuf,
    link_root: PathBuf,
    project_root: PathBuf,
    want_file: bool,
    want_anchor: bool,
}

pub fn check(ctx: &FileCtx, ws: &Workspace, out: &mut Out) {
    let Some(md) = &ctx.a.md else {
        if ctx.enabled("links/insecure") {
            for (url, range) in comment_urls(ctx.a) {
                insecure(ctx, &url, range, out);
            }
        }
        return;
    };
    let base = ctx
        .a
        .file
        .path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_default();
    let project_root = ctx
        .config
        .root
        .canonicalize()
        .unwrap_or_else(|_| ctx.config.root.clone());
    let mut c = Checker {
        ws,
        fs: FsCache { ws },
        base,
        link_root: root_dir(ctx.config),
        project_root,
        want_file: ctx.enabled("links/missing-file"),
        want_anchor: ctx.enabled("links/missing-anchor"),
    };
    if c.want_file || c.want_anchor {
        for link in &md.links {
            if matches!(
                link.kind,
                LinkKind::Inline | LinkKind::Reference | LinkKind::Html
            ) {
                c.link(md, &link.dest, report_range(ctx.src(), link), out);
            }
        }
    }
    refs(ctx, md, out);
    if ctx.enabled("links/insecure") {
        let src = ctx.src();
        for link in md.links.iter().filter(|l| l.kind != LinkKind::Reference) {
            insecure(
                ctx,
                &link.dest,
                dest_range(src, link).unwrap_or_else(|| report_range(src, link)),
                out,
            );
        }
        for d in &md.ref_defs {
            let r = src
                .get(d.range.clone())
                .and_then(|raw| raw.find(d.dest.as_str()))
                .map_or(d.range.clone(), |i| {
                    d.range.start + i..d.range.start + i + d.dest.len()
                });
            insecure(ctx, &d.dest, r, out);
        }
    }
}

impl Checker<'_> {
    fn link(&mut self, md: &MdDoc, dest: &str, range: Range<usize>, out: &mut Out) {
        match resolve(dest, &self.base, &self.link_root) {
            None => {}
            Some(Target::SameFile { frag }) => {
                if self.want_anchor {
                    anchor(md, frag, None, "this file", range, out);
                }
            }
            Some(Target::Path {
                path,
                raw_path,
                query,
                frag,
                root_relative,
            }) => {
                let trusted = if root_relative {
                    self.link_root.clone()
                } else {
                    self.base.clone()
                };
                match self.fs.lookup(&path, &trusted) {
                    Exists::Missing => {
                        if self.want_file {
                            out.push(self.missing(dest, raw_path, &path, range));
                        }
                    }
                    Exists::CaseMismatch(actual) => {
                        if self.want_file {
                            let name = actual
                                .file_name()
                                .map(|n| n.to_string_lossy().into_owned())
                                .unwrap_or_default();
                            let mut f = Finding::new(
                                "links/missing-file",
                                sev("links/missing-file"),
                                range,
                                format!(
                                    "Link target `{dest}` matches `{}` only case-insensitively; it breaks on case-sensitive filesystems",
                                    display(&actual, &self.project_root)
                                ),
                            );
                            if let Some(s) = replace_last_segment(dest, raw_path, &name)
                                .filter(|_| actual.parent() == path.parent())
                            {
                                f = f.suggest(s);
                            }
                            out.push(f);
                        }
                    }
                    Exists::Dir => {}
                    Exists::File => {
                        let Some(frag) = frag.filter(|f| !f.is_empty()) else {
                            return;
                        };
                        if !self.want_anchor || !is_markdown(&path) {
                            return;
                        }
                        if is_line_anchor(frag)
                            && query.is_some_and(|q| q.split('&').any(|kv| kv == "plain=1"))
                        {
                            return;
                        }
                        let canon = path.canonicalize().unwrap_or_else(|_| path.clone());
                        if let Some(a) = self.ws.markdown(&canon)
                            && let Some(tmd) = &a.md
                        {
                            let prefix = &dest[..dest.find('#').unwrap_or(dest.len())];
                            anchor(
                                tmd,
                                frag,
                                Some(prefix),
                                &format!("`{}`", display(&canon, &self.project_root)),
                                range,
                                out,
                            );
                        }
                    }
                }
            }
        }
    }

    fn missing(&mut self, dest: &str, raw_path: &str, path: &Path, range: Range<usize>) -> Finding {
        let shown = display(path, &self.project_root);
        let mut f = Finding::new(
            "links/missing-file",
            sev("links/missing-file"),
            range,
            format!("Link target `{dest}` does not exist (resolved to `{shown}`)"),
        );
        let (Some(parent), Some(name)) = (path.parent(), path.file_name()) else {
            return f;
        };
        let name = name.to_string_lossy().into_owned();
        let Some(entries) = self.fs.list(parent) else {
            return f.help(format!(
                "directory `{}` does not exist",
                display(parent, &self.project_root)
            ));
        };
        let names: Vec<String> = entries
            .iter()
            .map(|e| e.to_string_lossy().into_owned())
            .collect();
        let mut cands = near_misses(&name, names.iter().map(String::as_str));
        for ext in [".md", ".markdown"] {
            let with = format!("{name}{ext}");
            if names.contains(&with) && !cands.contains(&with) {
                cands.insert(0, with);
            }
        }
        for c in cands {
            if let Some(s) = replace_last_segment(dest, raw_path, &c) {
                f = f.suggest(s);
            }
        }
        f
    }
}

/// `dest` with the last path segment of `raw_path` replaced by `name`, keeping `?query#frag`.
fn replace_last_segment(dest: &str, raw_path: &str, name: &str) -> Option<String> {
    let start = dest.find(raw_path)?;
    let suffix = &dest[start + raw_path.len()..];
    let trimmed = raw_path.trim_end_matches('/');
    let slash = &raw_path[trimmed.len()..];
    let dir = trimmed.rfind('/').map_or("", |i| &trimmed[..=i]);
    Some(format!(
        "{}{dir}{}{slash}{suffix}",
        &dest[..start],
        encode_path_segment(name)
    ))
}

fn anchor(
    md: &MdDoc,
    frag: &str,
    prefix: Option<&str>,
    target: &str,
    range: Range<usize>,
    out: &mut Out,
) {
    let decoded = percent_decode(frag);
    let want = decoded
        .strip_prefix("user-content-")
        .unwrap_or(&decoded)
        .to_lowercase();
    if want == "top" || md.anchors().any(|a| a.to_lowercase() == want) {
        return;
    }
    let mut f = Finding::new(
        "links/missing-anchor",
        sev("links/missing-anchor"),
        range,
        format!("Anchor `#{frag}` not found in {target}"),
    );
    let anchors: Vec<&str> = md.anchors().collect();
    let mut cands = near_misses(&want, anchors.iter().copied());
    if cands.is_empty() {
        // Heading text typed instead of its slug, or a slug missing a word.
        let slug = crate::extract::markdown::slugify(&decoded);
        cands.extend(
            anchors
                .iter()
                .filter(|a| **a == slug || (want.len() >= 4 && a.contains(want.as_str())))
                .take(3)
                .map(|a| a.to_string()),
        );
    }
    for c in cands {
        f = f.suggest(format!("{}#{c}", prefix.unwrap_or("")));
    }
    out.push(f);
}

fn refs(ctx: &FileCtx, md: &MdDoc, out: &mut Out) {
    if ctx.enabled("links/undefined-ref") {
        for (label, range) in &md.undefined_refs {
            let mut f = Finding::new(
                "links/undefined-ref",
                sev("links/undefined-ref"),
                range.clone(),
                format!("Reference `[{label}]` is not defined"),
            );
            for c in near_misses(label, md.ref_defs.iter().map(|d| d.label.as_str())) {
                f = f.suggest(c);
            }
            out.push(f);
        }
    }
    if ctx.enabled("links/unused-ref") {
        let used: HashSet<&str> = md.links.iter().filter_map(|l| l.label.as_deref()).collect();
        for d in md
            .ref_defs
            .iter()
            .filter(|d| !used.contains(d.label.as_str()))
        {
            out.push(
                Finding::new(
                    "links/unused-ref",
                    sev("links/unused-ref"),
                    d.range.clone(),
                    format!("Reference definition `[{}]` is never used", d.label),
                )
                .help("remove it or link to it with `[text][label]`"),
            );
        }
    }
}

fn insecure(ctx: &FileCtx, url: &str, range: Range<usize>, out: &mut Out) {
    let Some(rest) = url
        .get(..7)
        .filter(|p| p.eq_ignore_ascii_case("http://"))
        .map(|_| &url[7..])
    else {
        return;
    };
    if rest.is_empty() || is_excluded(ctx.config, url) {
        return;
    }
    out.push(
        Finding::new(
            "links/insecure",
            sev("links/insecure"),
            range,
            format!("Link `{url}` uses insecure http://"),
        )
        .suggest(format!("https://{rest}")),
    );
}

/// URLs in code comments with their exact source ranges.
pub(crate) fn comment_urls(a: &Analyzed) -> Vec<(String, Range<usize>)> {
    let src = &a.file.text;
    let mut v = Vec::new();
    for b in &a.comments {
        for l in &b.lines {
            let Some(text) = src.get(l.content.clone()) else {
                continue;
            };
            for m in URL_RE.find_iter(text) {
                let url = if m.as_str().starts_with("www.") {
                    format!("https://{}", m.as_str())
                } else {
                    m.as_str().to_string()
                };
                v.push((url, l.content.start + m.start()..l.content.start + m.end()));
            }
        }
    }
    v
}

/// Absolute local paths this file links to (existing or not), for watch invalidation.
pub fn targets(a: &Analyzed, config: &crate::config::Config) -> Vec<PathBuf> {
    let Some(md) = &a.md else { return Vec::new() };
    let base = a
        .file
        .path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_default();
    let link_root = root_dir(config);
    let mut out: Vec<PathBuf> = Vec::new();
    for link in md.links.iter().filter(|l| {
        matches!(
            l.kind,
            LinkKind::Inline | LinkKind::Reference | LinkKind::Html
        )
    }) {
        let dest = link.dest.trim();
        let root = if dest.starts_with('/') {
            link_root.clone()
        } else {
            base.clone()
        };
        if let Some(Target::Path { path, .. }) = resolve(dest, &base, &root) {
            out.push(path);
        }
    }
    out.sort();
    out.dedup();
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::source::SourceFile;
    use std::fs;
    use std::sync::Arc;

    struct Env {
        _dir: tempfile::TempDir,
        root: PathBuf,
        config: Config,
    }

    fn env(files: &[(&str, &str)]) -> Env {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
        for (p, text) in files {
            let full = root.join(p);
            fs::create_dir_all(full.parent().unwrap()).unwrap();
            fs::write(full, text).unwrap();
        }
        let config = Config {
            root: root.clone(),
            ..Config::default()
        };
        Env {
            _dir: dir,
            root,
            config,
        }
    }

    fn run(e: &Env, file: &str) -> Vec<Finding> {
        let path = e.root.join(file);
        let text = fs::read_to_string(&path).unwrap();
        let kind = FileKind::detect(&path).unwrap();
        let a = Arc::new(Analyzed::new(SourceFile::new(
            path.clone(),
            PathBuf::from(file),
            kind,
            text,
        )));
        let mut ws = Workspace::default();
        ws.root = e.root.clone();
        ws.files.insert(path, a.clone());
        let ctx = FileCtx {
            a: &a,
            config: &e.config,
        };
        let mut out = Vec::new();
        check(&ctx, &ws, &mut out);
        out
    }

    fn rules(f: &[Finding]) -> Vec<&str> {
        f.iter().map(|f| f.rule.as_str()).collect()
    }

    #[test]
    fn missing_file_and_suggestion() {
        let e = env(&[
            (
                "README.md",
                "See [install](docs/instal.md) and [ok](docs/install.md).\n",
            ),
            ("docs/install.md", "# Install\n"),
        ]);
        let f = run(&e, "README.md");
        assert_eq!(rules(&f), vec!["links/missing-file"]);
        assert!(f[0].message.contains("docs/instal.md"), "{}", f[0].message);
        assert_eq!(f[0].suggestions, vec!["docs/install.md"]);
    }

    #[test]
    fn missing_extension_suggested() {
        let e = env(&[("README.md", "[x](guide)\n"), ("guide.md", "# G\n")]);
        let f = run(&e, "README.md");
        assert_eq!(f[0].suggestions[0], "guide.md");
    }

    #[test]
    fn existing_directory_ok() {
        let e = env(&[
            ("README.md", "[docs](docs/) and [docs2](./docs) [up](../)\n"),
            ("docs/a.md", "x\n"),
        ]);
        assert!(run(&e, "README.md").is_empty());
    }

    #[test]
    fn case_mismatch() {
        let e = env(&[
            ("README.md", "[c](docs/Contributing.md)\n"),
            ("docs/CONTRIBUTING.md", "# C\n"),
        ]);
        let f = run(&e, "README.md");
        assert_eq!(rules(&f), vec!["links/missing-file"]);
        assert!(f[0].message.contains("case"), "{}", f[0].message);
        assert_eq!(f[0].suggestions, vec!["docs/CONTRIBUTING.md"]);
        let e = env(&[("README.md", "[c](Docs/a.md)\n"), ("docs/a.md", "# C\n")]);
        assert_eq!(rules(&run(&e, "README.md")), vec!["links/missing-file"]);
    }

    #[test]
    fn anchors_across_files() {
        let e = env(&[
            (
                "README.md",
                "# Top\n\n[a](#top) [b](#missing-one) [c](guide.md#setup) [d](guide.md#SETUP) [e](guide.md#nope) [f](guide.md#setp)\n",
            ),
            ("guide.md", "# Guide\n\n## Setup\n\n<a id=\"custom\"></a>\n"),
        ]);
        let f = run(&e, "README.md");
        assert_eq!(
            rules(&f),
            vec![
                "links/missing-anchor",
                "links/missing-anchor",
                "links/missing-anchor"
            ],
            "{f:?}"
        );
        assert!(f[0].message.contains("this file"));
        assert!(f[1].message.contains("guide.md"));
        assert_eq!(f[2].suggestions, vec!["guide.md#setup"]);
    }

    #[test]
    fn html_anchor_and_duplicate_headings() {
        let e = env(&[(
            "a.md",
            "# A\n\n## Usage\n\n## Usage\n\n<a name=\"x-y\"></a>\n\n[1](#usage-1) [2](#usage-2) [3](#x-y) [4](#USAGE)\n",
        )]);
        let f = run(&e, "a.md");
        assert_eq!(rules(&f), vec!["links/missing-anchor"]);
        assert!(f[0].message.contains("usage-2"));
    }

    #[test]
    fn line_anchors_and_non_markdown() {
        let e = env(&[
            (
                "a.md",
                "[1](main.rs#L10) [2](main.rs#L10-L20) [3](b.md?plain=1#L3) [4](b.md#L3)\n",
            ),
            ("main.rs", "fn main() {}\n"),
            ("b.md", "# B\n"),
        ]);
        let f = run(&e, "a.md");
        assert_eq!(rules(&f), vec!["links/missing-anchor"]);
        assert!(f[0].message.contains("#L3"));
    }

    #[test]
    fn percent_encoded_paths() {
        let e = env(&[
            (
                "a.md",
                "[1](my%20doc.md) [2](<my doc.md>) [3](my%20dok.md) [4](my%20doc.md#t%C3%A4st)\n",
            ),
            ("my doc.md", "# Täst\n"),
        ]);
        let f = run(&e, "a.md");
        assert_eq!(rules(&f), vec!["links/missing-file"]);
        assert_eq!(f[0].suggestions, vec!["my%20doc.md"]);
    }

    #[test]
    fn root_relative() {
        let mut e = env(&[
            (
                "docs/sub/a.md",
                "[1](/docs/b.md) [2](/b.md) [3](/nope.md)\n",
            ),
            ("docs/b.md", "x\n"),
        ]);
        let f = run(&e, "docs/sub/a.md");
        assert_eq!(rules(&f), vec!["links/missing-file", "links/missing-file"]);
        e.config.links.root_dir = Some(PathBuf::from("docs"));
        let f = run(&e, "docs/sub/a.md");
        assert_eq!(rules(&f), vec!["links/missing-file", "links/missing-file"]);
        assert!(f[0].message.contains("/docs/b.md"), "{}", f[0].message);
    }

    #[test]
    fn reference_links() {
        let e = env(&[
            (
                "a.md",
                "See [x][good] and [y][bad] and [z][gone].\n\n[good]: ./b.md#b\n[bad]: ./missing.md\n[unused]: https://example.com\n",
            ),
            ("b.md", "# B\n"),
        ]);
        let f = run(&e, "a.md");
        let r = rules(&f);
        assert!(r.contains(&"links/missing-file"), "{f:?}");
        assert!(r.contains(&"links/undefined-ref"));
        assert!(r.contains(&"links/unused-ref"));
        assert_eq!(r.len(), 3, "{f:?}");
    }

    #[test]
    fn skips_schemes_templates() {
        let e = env(&[(
            "a.md",
            "[1](mailto:a@b.c) [2](tel:123) [3]({{ site.url }}/x) [4](data:text/plain,hi) [5](//cdn.x/y) [6](javascript:void(0))\n",
        )]);
        assert!(run(&e, "a.md").is_empty());
    }

    #[test]
    fn insecure_links() {
        let e = env(&[(
            "a.md",
            "[a](http://foo.org/x) <http://localhost:8080> http://bar.org\n",
        )]);
        let f = run(&e, "a.md");
        assert_eq!(rules(&f), vec!["links/insecure", "links/insecure"]);
        assert_eq!(f[0].suggestions, vec!["https://foo.org/x"]);
    }

    #[test]
    fn html_links() {
        let e = env(&[
            (
                "a.md",
                "<p><img src=\"logo.png\"> <a href=\"gone.md\">x</a></p>\n",
            ),
            ("logo.png", ""),
        ]);
        let f = run(&e, "a.md");
        assert_eq!(rules(&f), vec!["links/missing-file"]);
    }

    #[test]
    fn targets_listed() {
        let e = env(&[
            (
                "d/a.md",
                "[1](../b.md#x) [2](/c.md) [3](https://x.org) [4](#y)\n",
            ),
            (".git/HEAD", ""),
        ]);
        let path = e.root.join("d/a.md");
        let a = Analyzed::new(SourceFile::new(
            path.clone(),
            PathBuf::from("d/a.md"),
            FileKind::Markdown,
            fs::read_to_string(&path).unwrap(),
        ));
        assert_eq!(
            targets(&a, &e.config),
            vec![e.root.join("b.md"), e.root.join("c.md")]
        );
    }

    #[test]
    fn line_anchor_detection() {
        assert!(is_line_anchor("L10"));
        assert!(is_line_anchor("L10-L20"));
        assert!(is_line_anchor("L1C2-L3C4"));
        assert!(!is_line_anchor("Label"));
        assert!(!is_line_anchor("L"));
    }
}
