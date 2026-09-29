use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::diagnostic::Finding;
use crate::engine::{Workspace, build_workspace};
use crate::rules::FileCtx;

struct Env {
    _dir: tempfile::TempDir,
    root: PathBuf,
    config: Config,
}

impl Env {
    fn new(files: &[(&str, &str)]) -> Env {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path().canonicalize().expect("canonical");
        for (p, text) in files {
            let path = root.join(p);
            std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
            std::fs::write(&path, text).expect("write");
        }
        let mut config = Config {
            root: root.clone(),
            ..Config::default()
        };
        config
            .rules
            .insert("docs/orphan-page".into(), crate::config::Level::Warn);
        Env {
            _dir: dir,
            root,
            config,
        }
    }

    fn ws(&self) -> Workspace {
        let paths: Vec<PathBuf> = walk(&self.root)
            .into_iter()
            .filter(|p| p.extension().is_some_and(|e| e == "md"))
            .collect();
        build_workspace(&paths, &self.config)
    }

    fn run(&self, file: &str) -> Vec<Finding> {
        let ws = self.ws();
        let a = ws
            .files
            .get(&self.root.join(file))
            .expect("file in workspace")
            .clone();
        let ctx = FileCtx {
            a: &a,
            config: &self.config,
        };
        let mut out = Vec::new();
        super::check(&ctx, &ws, &mut out);
        out
    }
}

fn walk(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for e in std::fs::read_dir(dir).expect("read_dir").flatten() {
        let p = e.path();
        if p.is_dir() {
            out.extend(walk(&p));
        } else {
            out.push(p);
        }
    }
    out
}

fn only<'a>(f: &'a [Finding], rule: &str) -> Vec<&'a Finding> {
    f.iter().filter(|x| x.rule == rule).collect()
}

fn apply(src: &str, f: &Finding) -> String {
    let fix = f.fix.as_ref().expect("fix");
    format!(
        "{}{}{}",
        &src[..fix.range.start],
        fix.replacement,
        &src[fix.range.end..]
    )
}

const TOC: &str = "docs/toc-sync";

#[test]
fn toc_heading_form_in_sync() {
    let src = "# Title\n\n## Contents\n\n- [Intro](#intro)\n  - [Setup ü](#setup-ü)\n- [Usage](#usage)\n\n## Intro\n\n### Setup ü\n\n## Usage\n";
    let e = Env::new(&[("a.md", src)]);
    assert!(only(&e.run("a.md"), TOC).is_empty(), "{:?}", e.run("a.md"));
}

#[test]
fn toc_heading_form_missing_stale_order() {
    let src = "# Title\n\n## Table of Contents\n\n- [Usage](#usage)\n- [Intro](#intro)\n- [Gone](#gone)\n\n## Intro\n\n## Usage\n\n## Äpfel\n";
    let e = Env::new(&[("a.md", src)]);
    let f = e.run("a.md");
    let f = only(&f, TOC);
    let msgs: Vec<&str> = f.iter().map(|x| x.message.as_str()).collect();
    assert_eq!(f.len(), 3, "{msgs:?}");
    let stale = f
        .iter()
        .find(|x| x.message.contains("Stale"))
        .expect("stale");
    assert_eq!(&src[stale.range.clone()], "[Gone](#gone)");
    let missing = f
        .iter()
        .find(|x| x.message.contains("missing"))
        .expect("missing");
    assert_eq!(&src[missing.range.clone()], "## Äpfel\n".trim_end());
    let order = f
        .iter()
        .find(|x| x.message.contains("out of order"))
        .expect("order");
    assert_eq!(&src[order.range.clone()], "[Usage](#usage)");
    // No fix for the heading form.
    assert!(f.iter().all(|x| x.fix.is_none()));
}

#[test]
fn toc_marker_form_fix() {
    let src = "# T\r\n\r\n<!-- toc -->\r\n\r\n- [A](#a)\r\n  - [B](#b-x)\r\n\r\n<!-- tocstop -->\r\n\r\n## A\r\n\r\n### B [x]\r\n\r\n## C\r\n";
    let e = Env::new(&[("a.md", src)]);
    let f = e.run("a.md");
    let f = only(&f, TOC);
    assert_eq!(f.len(), 1, "{f:?}");
    let with_fix: Vec<_> = f.iter().filter(|x| x.fix.is_some()).collect();
    assert_eq!(with_fix.len(), 1);
    let fixed = apply(src, with_fix[0]);
    assert!(
        fixed.contains("<!-- toc -->\r\n\r\n- [A](#a)\r\n  - [B \\[x\\]](#b-x)\r\n- [C](#c)\r\n\r\n<!-- tocstop -->"),
        "{fixed:?}"
    );
    let e = Env::new(&[("a.md", &fixed)]);
    assert!(only(&e.run("a.md"), TOC).is_empty());
}

#[test]
fn toc_not_a_toc() {
    // "Contents" section without anchor links, and markers inside code.
    let src = "# T\n\n## Contents\n\n- apples\n- pears\n\n```\n<!-- toc -->\n<!-- tocstop -->\n```\n\n## Other\n";
    let e = Env::new(&[("a.md", src)]);
    assert!(only(&e.run("a.md"), TOC).is_empty());
}

const ORPHAN: &str = "docs/orphan-page";

#[test]
fn orphan_pages() {
    let e = Env::new(&[
        (
            "README.md",
            "See [guide](docs/guide.md) and [api](/docs/sub/api).\n",
        ),
        ("docs/index.md", "# Index\n"),
        ("docs/guide.md", "# Guide\n\n[Ünï](./ünï%20page.md)\n"),
        ("docs/sub/api.md", "# API\n"),
        ("docs/ünï page.md", "# U\n"),
        ("docs/lonely.md", "# Lonely ö\r\nbody\r\n"),
        ("docs/_partial.md", "x\n"),
        ("docs/drafts/wip.md", "x\n"),
        ("other/free.md", "x\n"),
    ]);
    let mut e = e;
    e.config.docs.orphan_exempt = vec!["docs/drafts/*".into()];
    for ok in [
        "README.md",
        "docs/index.md",
        "docs/guide.md",
        "docs/sub/api.md",
        "docs/ünï page.md",
        "docs/_partial.md",
        "docs/drafts/wip.md",
        "other/free.md",
    ] {
        assert!(only(&e.run(ok), ORPHAN).is_empty(), "{ok}");
    }
    let f = e.run("docs/lonely.md");
    let f = only(&f, ORPHAN);
    assert_eq!(f.len(), 1);
    assert_eq!(f[0].range, 0.."# Lonely ö".len());
}

#[test]
fn orphan_mkdocs_nav() {
    let e = Env::new(&[
        (
            "mkdocs.yml",
            "nav:\n  - Home: index.md\n  - Guide: 'guide/start.md'\n",
        ),
        ("docs/guide/start.md", "# Start\n"),
    ]);
    assert!(only(&e.run("docs/guide/start.md"), ORPHAN).is_empty());
}

#[test]
fn orphan_off_by_default() {
    let mut e = Env::new(&[("docs/lonely.md", "# L\n")]);
    e.config.rules.clear();
    assert!(only(&e.run("docs/lonely.md"), ORPHAN).is_empty());
}

const INC: &str = "docs/include-missing";

#[test]
fn includes() {
    let src = concat!(
        "# Inc\n\n",
        "--8<-- \"docs/snip.md\"\n",
        "--8<-- \"missing-ä.md\"\n",
        ";--8<-- \"escaped.md\"\n\n",
        "```py\n--8<-- \"code.py:3:5\"\n--8<-- \"nope.py\"\n```\n\n",
        "{% include header.html %}\n",
        "{% include_relative ./rel.md %}\n",
        "{% include {{ page.var }} %}\n",
        "{{< readfile file=\"data/x.txt\" >}}\n",
        "<!-- include: gone.md -->\n\n",
        "{!docs/snip.md!}\n\n",
        "`{! inspan.md !}`\n\n",
        "--8<--\ndocs/snip.md\nblock-missing.md\n--8<--\n",
    );
    let e = Env::new(&[
        ("docs/a.md", src),
        ("docs/snip.md", "x\n"),
        ("docs/code.py", "x\n"),
        ("_includes/header.html", "x\n"),
        ("docs/rel.md", "x\n"),
        ("data/x.txt", "x\n"),
    ]);
    let f = e.run("docs/a.md");
    let mut got: Vec<&str> = only(&f, INC)
        .iter()
        .map(|x| &src[x.range.clone()])
        .collect();
    got.sort_unstable();
    assert_eq!(
        got,
        ["block-missing.md", "gone.md", "missing-ä.md", "nope.py"]
    );
}

#[test]
fn includes_crlf_and_include_dirs() {
    let src = "--8<-- \"part.md\"\r\n\r\n--8<--\r\nother.md\r\n--8<--\r\n";
    let mut e = Env::new(&[("a.md", src), ("snippets/part.md", "x\n")]);
    let f = e.run("a.md");
    let got: Vec<&str> = only(&f, INC)
        .iter()
        .map(|x| &src[x.range.clone()])
        .collect();
    assert_eq!(got, ["part.md", "other.md"]);
    e.config.docs.include_dirs = vec!["snippets".into()];
    let f = e.run("a.md");
    let got: Vec<&str> = only(&f, INC)
        .iter()
        .map(|x| &src[x.range.clone()])
        .collect();
    assert_eq!(got, ["other.md"]);
}

const IMG: &str = "docs/readme-absolute-image";

#[test]
fn readme_images() {
    let src = concat!(
        "# Ä\n\n",
        "![logo](https://github.com/me/proj/raw/main/assets/logo.png)\n",
        "![shot](https://raw.githubusercontent.com/me/proj/refs/heads/main/assets/shot%201.png)\n",
        "![blob](https://github.com/me/proj/blob/main/assets/logo.png?raw=true)\n",
        "![gone](https://github.com/me/proj/raw/main/assets/none.png)\n",
        "![other](https://github.com/else/lib/raw/main/assets/logo.png)\n",
        "<img src=\"https://raw.githubusercontent.com/me/proj/main/assets/logo.png\">\n",
    );
    let e = Env::new(&[
        ("README.md", src),
        ("assets/logo.png", "x"),
        ("assets/shot 1.png", "x"),
        (
            ".git/config",
            "[remote \"origin\"]\n\turl = git@github.com:me/proj.git\n",
        ),
    ]);
    let f = e.run("README.md");
    let f = only(&f, IMG);
    assert_eq!(f.len(), 4, "{f:?}");
    let mut fixed = src.to_string();
    for x in f.iter().rev() {
        fixed = apply(&fixed, x);
    }
    assert!(fixed.contains("![logo](assets/logo.png)"));
    assert!(fixed.contains("![shot](assets/shot%201.png)"));
    assert!(fixed.contains("![blob](assets/logo.png)"));
    assert!(fixed.contains("<img src=\"assets/logo.png\">"));
    assert!(fixed.contains("none.png") && fixed.contains("else/lib"));
}

#[test]
fn readme_image_subdir_and_non_readme() {
    let src = "![l](https://github.com/me/proj/raw/main/assets/logo.png)\n";
    let e = Env::new(&[
        ("pkg/README.md", src),
        ("pkg/guide.md", src),
        ("assets/logo.png", "x"),
    ]);
    let f = e.run("pkg/README.md");
    let f = only(&f, IMG);
    assert_eq!(f.len(), 1);
    assert_eq!(
        f[0].fix.as_ref().expect("fix").replacement,
        "../assets/logo.png"
    );
    assert!(only(&e.run("pkg/guide.md"), IMG).is_empty());
}

#[test]
fn toc_marker_fix_keeps_inline_markup() {
    let src = "# T\n\n<!-- toc -->\n<!-- tocstop -->\n\n## The `foo` command\n\n## *Fast* mode ##\n\n## Custom {#cid}\n\nSetext **bold**\n--------------\n";
    let e = Env::new(&[("a.md", src)]);
    let f = e.run("a.md");
    let f = only(&f, TOC);
    let with_fix: Vec<_> = f.iter().filter(|x| x.fix.is_some()).collect();
    assert_eq!(with_fix.len(), 1, "{f:?}");
    let fixed = apply(src, with_fix[0]);
    assert!(
        fixed.contains(
            "<!-- toc -->\n\n- [The `foo` command](#the-foo-command)\n- [*Fast* mode](#fast-mode)\n- [Custom](#cid)\n- [Setext **bold**](#setext-bold)\n\n<!-- tocstop -->"
        ),
        "{fixed:?}"
    );
    let e = Env::new(&[("a.md", &fixed)]);
    assert!(only(&e.run("a.md"), TOC).is_empty());
}
