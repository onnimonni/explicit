// explicit-disable-file slop/* prose/* -- examples of the patterns this module detects
//! Comment slop rules ported from aislop (MIT License, Copyright (c) 2025 heavykenny),
//! https://github.com/scanaislop/aislop at commit 7d50952bf26a450e94244760b73eec15efad3724:
//! `src/engines/ai-slop/{meta-comment,narrative-comments,narrative-comments-patterns,comment-blocks}.ts`.
//! See the NOTICE file for the full license text.
//!
//! Changes from the original: JS regexes translated to Rust syntax (no lookaround),
//! declaration patterns added for Nix, Elixir, Zig, shell and C/C++, cross-reference
//! phrases ("used by", "called from") dropped because they are useful context, and the
//! findings split into `slop/comment-plan`, `slop/comment-history`, `slop/comment-narrative`
//! and `slop/comment-banner`.

use std::ops::Range;
use std::sync::LazyLock;

use regex::Regex;

use super::{FileCtx, Out, sev};
use crate::diagnostic::Finding;
use crate::extract::comments::{CommentBlock, CommentKind};
use crate::source::{FileKind, Lang};

fn res(ps: &[&str]) -> Vec<Regex> {
    ps.iter()
        .map(|p| Regex::new(p).expect("valid comment regex"))
        .collect()
}

fn re(p: &str) -> Regex {
    Regex::new(p).expect("valid comment regex")
}

// Patterns ported from upstream meta-comment.ts.

/// Numbered plan markers at the start of a comment. Reported as a banner when the
/// comment is only a section header, so kept apart from the other plan patterns.
static PLAN_STEP_RE: LazyLock<Regex> =
    LazyLock::new(|| re(r"(?i)^(?:stage|step|phase)\s+\d+\s*[:.\-–—]"));

static PLAN_REFERENCE_RES: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    res(&[
        r"(?i)\bstep\s+\d+\s+of\s+the\s+plan\b",
        r"(?i)\bas\s+(?:per|requested)\s+(?:the\s+)?(?:requirements?|spec|task|ticket|prompt|instructions?)\b",
        r"(?i)\bper\s+the\s+(?:spec|requirements?|task|ticket|plan|prompt|instructions?)\b",
        r"(?i)\bfrom\s+the\s+(?:task|todo|plan|spec|ticket|prompt|requirements?)\b",
        r"(?i)\bimplement(?:ing|s|ed)?\s+use\s*case\s+\d*",
        r"(?i)\b(?:requirements?\s+doc|requirement\s+\d+)\b",
        r"(?i)\bas\s+(?:instructed|specified|outlined)\s+(?:above|below|in\s+the)\b",
    ])
});

static BEFORE_AFTER_RES: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    res(&[
        r"(?i)\bpreviously[,:]?\s+(?:this|we|it|the)\b",
        r"(?i)\bused\s+to\s+(?:be|use|call|return|do|have|rely)\b",
        r"(?i)\bchanged\s+(?:\w+\s+){0,3}from\s+.+\bto\b",
        r"(?i)\bno\s+longer\s+(?:needed|used|required|necessary|calls?|returns?|does)\b",
        r"(?i)\bthis\s+was\s+.+\bbut\s+now\b",
        r"(?i)\bwe\s+(?:now|used\s+to)\s+(?:no\s+longer\s+)?(?:use|call|return|do|have)\b",
        r"(?i)\breplaced\s+the\s+(?:old|previous|former)\b",
        r"(?i)\b(?:was|were)\s+(?:renamed|moved|removed|refactored|extracted)\s+(?:from|to|out\s+of)\b",
        // From aislop's cross-reference list: these narrate history, not structure.
        r"(?i)\bwe\s+moved\b",
        r"(?i)\brefactor(?:ed)?\s+from\b",
    ])
});

/// Real WHY context and to-do/fix-me markers exempt a comment from every rule.
static WHY_OR_TODO_RE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i)\b(?:because|since|otherwise|todo|fixme|hack|xxx|workaround|safety|invariant)\b|\b(?:note|reason|why):|\bsee\s+(?:issue|#)",
    )
});

static SUPPRESS_RE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i)\b(?:biome-ignore|eslint-disable|eslint-enable|ts-ignore|ts-expect-error|ts-nocheck|noqa|pylint:\s*disable|rubocop:(?:disable|enable|todo)|noinspection|phpcs:(?:disable|ignore)|nolint|shellcheck\s+(?:disable|source)|explicit-(?:disable|enable)|prettier-ignore|istanbul\s+ignore|c8\s+ignore|type:\s*ignore|nosec|deno-lint-ignore|clippy::\w+|go:(?:build|generate|embed|linkname|noinline)|frozen_string_literal)\b|@ts-\w+|#\[allow|\+build\b",
    )
});

static LICENSE_RE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i)\bcopyright\b|\blicen[sc]e[sd]?\b|spdx-license-identifier|all rights reserved|\(c\)\s*\d{4}",
    )
});

// Patterns ported from upstream narrative-comments-patterns.ts.

static DECORATIVE_SEPARATOR: LazyLock<Regex> = LazyLock::new(|| re(r"^[-=─━═~_*#/+]{6,}$"));
static DECORATIVE_SECTION_HEADER: LazyLock<Regex> =
    LazyLock::new(|| re(r"^[-=─━═~_*#/+]{3,}[\s\S]+?[-=─━═~_*#/+]{3,}$"));
static SECTION_HEADER: LazyLock<Regex> =
    LazyLock::new(|| re(r"(?i)^(?:Phase|Step|Section|Part)\s+\d+[:.\-]"));

static JUSTIFICATION_OPENERS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    res(&[
        r"(?i)^(?:The idea here|The trick is|This was needed|Originally,?)",
        r"(?i)^This\s+(?:function|method|class|module|component|hook|util|helper|handler|service|struct|trait|enum)\b",
        r"(?i)^It\s+(?:does|handles|takes|returns|processes|reads|writes|sends|fetches|loads|creates|deletes|updates|parses|validates)\b",
        r"(?i)^(?:First|Then|Finally|Next|Lastly|Subsequently),?\s+(?:it|we|the\s+(?:function|method|class))\b",
    ])
});

static EXPLANATORY_OPENERS: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r#"^(?:Matches|Detects|Represents|Holds|Stores|Tracks|Handles|Manages|Controls|Contains|Captures|Encapsulates|Wraps|Describes)\s+[A-Za-z`'"]"#,
    )
});

static EXPLANATORY_WHY_MARKERS: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i)\b(?:because|since|otherwise|workaround|caveat|warning|important|assumes?|bug|issue|see\s+(?:issue|above|below)|in\s+prod|in\s+production|breaks?\s+when|fails?\s+when|must\s+run|must\s+be|has\s+to\s+be|hack\s+for|fix\s+for|to\s+avoid|to\s+ensure|to\s+prevent|in\s+order\s+to|necessary|guarantee[sd]?|prevents?|regardless\s+of|required\s+(?:for|to|by)|for\s+example|useful\s+(?:for|when)|intended\s+to|on\s+purpose|by\s+design|ideally|however|although|even\s+though|despite|whereas|unfortunately|trade-?off|first\s+need)\b|\b(?:note|reason):|\be\.g\.|\bi\.e\.",
    )
});

const MEANINGFUL_DOC_TAGS: &[&str] = &[
    "deprecated",
    "see",
    "example",
    "type",
    "returns",
    "return",
    "param",
    "throws",
    "typedef",
    "callback",
    "override",
    "template",
    "internal",
    "public",
    "private",
    "protected",
    "experimental",
    "alpha",
    "beta",
    "since",
    "todo",
    "link",
    "license",
    "preserve",
    "swagger",
    "openapi",
    "route",
    "group",
    "summary",
    "description",
    "operationid",
    "response",
    "responses",
    "request",
    "requestbody",
    "security",
    "tag",
    "tags",
    "path",
    "body",
    "query",
    "queryparam",
    "header",
    "headers",
    "produces",
    "accept",
    "middleware",
    "api",
    "apiname",
    "apidefine",
    "apigroup",
    "apiparam",
    "apiquery",
    "apibody",
    "apiheader",
    "apisuccess",
    "apierror",
    "apiexample",
    "apiversion",
    "apidescription",
    "apipermission",
    "apiuse",
    "apiignore",
    "apiprivate",
    "namespace",
    "category",
    "spec",
    "doc",
    "moduledoc",
    "impl",
];

static DECL_START: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"^\s*(?:export\s+)?(?:async\s+)?(?:const|let|var|function|class|type|interface|enum|abstract\s+class)\s+",
    )
});
static EXPORT_DEFAULT: LazyLock<Regex> = LazyLock::new(|| re(r"^\s*export\s+default\b"));
static TS_MEMBER_DECL_START: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"^\s*(?:readonly\s+|static\s+|public\s+|private\s+|protected\s+|abstract\s+|override\s+)*[\w$]+\??\s*:",
    )
});
static PY_DECL_START: LazyLock<Regex> =
    LazyLock::new(|| re(r"^\s*(?:async\s+def|def|class)\s+|^\s*@\w"));
static GO_DECL_START: LazyLock<Regex> =
    LazyLock::new(|| re(r"^\s*(?:func|type|var|const|import)\b"));
static RUST_DECL_START: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?(?:unsafe\s+)?(?:fn|struct|enum|trait|impl|const|static|type|mod)\b|^\s*macro_rules!|^\s*#\[",
    )
});
static RUBY_DECL_START: LazyLock<Regex> = LazyLock::new(|| re(r"^\s*(?:class|module|def)\s+"));
static JAVA_DECL_START: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"^\s*(?:public|private|protected|static|final|abstract|sealed|non-sealed|\s)+(?:class|interface|enum|record|@interface|\w[^(){};=]*\s+\w+\s*\()|^\s*(?:class|interface|enum|record|@interface)\s+",
    )
});
static CSHARP_DECL_START: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"^\s*(?:\[[^\]]*\]\s*)*(?:(?:public|private|protected|internal|static|async|sealed|abstract|virtual|override|partial|readonly|extern|unsafe|file|new)\s+)+(?:class|interface|struct|record|enum|[\w<>\[\],.?]+\s+\w+)|^\s*(?:class|interface|struct|record|enum|namespace)\s+",
    )
});
static PHP_DECL_START: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"^\s*(?:(?:public|private|protected|static|final|abstract|readonly)\s+)*(?:function|class|interface|trait|enum|const)\s+",
    )
});
static NIX_DECL_START: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"^\s*(?:[A-Za-z_][\w'-]*(?:\.[A-Za-z_\x22][\w'\x22-]*)*\s*=|\{[^}]*\}\s*:|[a-z]\w*\s*:\s*$|let\b|inherit\b)",
    )
});
static ELIXIR_DECL_START: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"^\s*(?:defp?|defmodule|defmacrop?|defstruct|defprotocol|defimpl|defguardp?|defdelegate|defexception|@spec|@type|@typep|@callback|@impl)\b",
    )
});
static ZIG_DECL_START: LazyLock<Regex> = LazyLock::new(|| {
    re(r"^\s*(?:pub\s+)?(?:export\s+|extern\s+|inline\s+|noinline\s+)*(?:fn|const|var|test)\b")
});
static SHELL_DECL_START: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"^\s*(?:function\s+[\w:.-]+|[\w:.-]+\s*\(\s*\)\s*\{?|(?:readonly|export|local|declare|typeset)\s+(?:-\w+\s+)*\w+|[A-Za-z_]\w*=)",
    )
});
static C_DECL_START: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"^\s*(?:typedef|struct|enum|union|class|namespace|template\s*<|#\s*define)\b|^\s*(?:(?:static|extern|inline|const|unsigned|signed|virtual|constexpr|explicit)\s+)*[A-Za-z_][\w:<>,]*[\s*&]+[*&]?[A-Za-z_][\w:]*\s*\([^;]*$",
    )
});

fn looks_like_declaration(next: Option<&str>, lang: Option<Lang>) -> bool {
    let Some(next) = next else { return false };
    if DECL_START.is_match(next) || EXPORT_DEFAULT.is_match(next) {
        return true;
    }
    match lang {
        Some(Lang::Python) => PY_DECL_START.is_match(next),
        Some(Lang::Go) => GO_DECL_START.is_match(next),
        Some(Lang::Rust) => RUST_DECL_START.is_match(next),
        Some(Lang::Ruby) => RUBY_DECL_START.is_match(next),
        Some(Lang::Java) => JAVA_DECL_START.is_match(next),
        Some(Lang::Php) => PHP_DECL_START.is_match(next),
        Some(Lang::CSharp) => CSHARP_DECL_START.is_match(next),
        Some(Lang::JavaScript | Lang::TypeScript) => TS_MEMBER_DECL_START.is_match(next),
        Some(Lang::Nix) => NIX_DECL_START.is_match(next),
        Some(Lang::Elixir) => ELIXIR_DECL_START.is_match(next),
        Some(Lang::Zig) => ZIG_DECL_START.is_match(next),
        Some(Lang::Shell) => SHELL_DECL_START.is_match(next),
        Some(Lang::C | Lang::Cpp) => C_DECL_START.is_match(next),
        _ => false,
    }
}

/// A comment block prepared for the heuristics, mirroring aislop's `CommentBlock`.
struct View<'a> {
    block: &'a CommentBlock,
    /// Non-doc comments (`//`, `#`, `/* */`); aislop's "line" blocks.
    line_like: bool,
    /// Rust `///` / `//!` doc comments.
    rust_doc: bool,
    raw: Vec<&'a str>,
    /// Trimmed content lines; for doc comments, blank and `@tag` lines are dropped.
    prose: Vec<(&'a str, Range<usize>)>,
    tags: Vec<String>,
    /// Prose joined with spaces, and (joined offset, absolute offset, len) per line.
    joined: String,
    map: Vec<(usize, usize, usize)>,
}

impl<'a> View<'a> {
    fn new(src: &'a str, block: &'a CommentBlock, lang: Option<Lang>) -> View<'a> {
        let line_like = matches!(block.kind, CommentKind::Line | CommentKind::Block);
        let rust_doc = lang == Some(Lang::Rust) && block.kind == CommentKind::Doc;
        let raw = block.lines.iter().map(|l| &src[l.raw.clone()]).collect();
        let mut prose = Vec::new();
        let mut tags = Vec::new();
        for l in &block.lines {
            let full = &src[l.content.clone()];
            let lead = full.len() - full.trim_start().len();
            let t = full.trim();
            let r = l.content.start + lead..l.content.start + lead + t.len();
            if let Some(tag) = t.strip_prefix('@') {
                let name: String = tag
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect();
                tags.push(name.to_lowercase());
                if !line_like {
                    continue;
                }
            }
            if !line_like && t.is_empty() {
                continue;
            }
            prose.push((t, r));
        }
        let mut joined = String::new();
        let mut map = Vec::new();
        for (t, r) in &prose {
            if t.is_empty() {
                continue;
            }
            if !joined.is_empty() {
                joined.push(' ');
            }
            map.push((joined.len(), r.start, t.len()));
            joined.push_str(t);
        }
        View {
            block,
            line_like,
            rust_doc,
            raw,
            prose,
            tags,
            joined,
            map,
        }
    }

    /// Absolute range of a range inside `joined`.
    fn abs(&self, r: Range<usize>) -> Range<usize> {
        let at = |pos: usize, end: bool| {
            let mut best = self.map.first().map_or(self.block.range.start, |m| m.1);
            for &(j, a, len) in &self.map {
                if pos >= j && pos <= j + len {
                    return a + (pos - j);
                }
                if pos >= j {
                    best = if end { a + len } else { a };
                }
            }
            best
        };
        let s = at(r.start, false);
        let e = at(r.end, true).max(s);
        s..e
    }

    fn prose_text(&self) -> impl Iterator<Item = &str> {
        self.prose.iter().map(|(t, _)| *t)
    }

    fn non_empty(&self) -> usize {
        self.prose.iter().filter(|(t, _)| !t.is_empty()).count()
    }

    /// Range from the first to the last prose character.
    fn prose_range(&self) -> Range<usize> {
        let mut it = self
            .prose
            .iter()
            .filter(|(t, _)| !t.is_empty())
            .map(|(_, r)| r.clone());
        match it.next() {
            Some(first) => {
                let last = it.next_back().unwrap_or_else(|| first.clone());
                first.start..last.end
            }
            None => self.block.range.clone(),
        }
    }

    fn has_meaningful_tag(&self) -> bool {
        !self.line_like
            && self
                .tags
                .iter()
                .any(|t| MEANINGFUL_DOC_TAGS.contains(&t.as_str()))
    }
}

fn is_license(v: &View, first: bool) -> bool {
    (first || v.block.start_line <= 2) && LICENSE_RE.is_match(&v.raw.join(" "))
        || v.raw.iter().any(|l| l.contains("SPDX-License-Identifier"))
}

fn is_suppress_directive(v: &View) -> bool {
    v.raw.iter().any(|l| SUPPRESS_RE.is_match(l))
}

static GO_DECL_NAME_RE: LazyLock<Regex> =
    LazyLock::new(|| re(r"^(?:func|type|var|const)\s+(?:\([^)]*\)\s*)?(\w+)"));
static GO_FIELD_LEAD_RE: LazyLock<Regex> = LazyLock::new(|| re(r"^(\w+)\s+"));
const GO_KEYWORDS: &[&str] = &[
    "return", "if", "for", "switch", "case", "default", "go", "select", "defer", "else", "break",
    "continue", "goto", "package", "import", "map", "chan", "range",
];

/// Go doc comments start with the name of the declaration or field they document.
fn is_go_doc(v: &View, lang: Option<Lang>) -> bool {
    if lang != Some(Lang::Go) || !v.line_like {
        return false;
    }
    let Some(next) = v.block.next_code_line.as_deref() else {
        return false;
    };
    let next = next.trim();
    let first_word = v
        .prose_text()
        .find(|l| !l.is_empty())
        .and_then(|l| l.split_whitespace().next())
        .unwrap_or("");
    if first_word.is_empty() {
        return false;
    }
    if GO_DECL_NAME_RE
        .captures(next)
        .is_some_and(|c| &c[1] == first_word)
    {
        return true;
    }
    GO_FIELD_LEAD_RE
        .captures(next)
        .is_some_and(|c| !GO_KEYWORDS.contains(&&c[1]) && &c[1] == first_word)
}

static RUBY_DOC_INDICATORS: LazyLock<Regex> =
    LazyLock::new(|| re(r"^\s*#\s*(?:#|@\w+|:[\w-]+:|=begin|=end)"));

fn is_ruby_doc(v: &View, lang: Option<Lang>) -> bool {
    lang == Some(Lang::Ruby) && v.line_like && v.raw.iter().any(|l| RUBY_DOC_INDICATORS.is_match(l))
}

static DOC_INDICATOR_RE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i)`[^`]+`|\|\s*[-:]+\s*\||```|\b(?:note|warning|warn|caveat|example|caution|see|todo|fixme|hack|reason|deprecated|deprecation|migration|legacy|historical|context):|\(e\.g\.[^)]+\)|\(i\.e\.[^)]+\)|\b\w+\.\w+(?:\.\w+)+\b|\[[\w/.-]+\]",
    )
});

fn has_doc_indicator(v: &View) -> bool {
    DOC_INDICATOR_RE.is_match(&v.joined) || v.prose_text().any(|l| l.starts_with("- "))
}

fn has_preamble_signal(v: &View) -> bool {
    v.prose_text().any(|l| {
        EXPLANATORY_OPENERS.is_match(l) || JUSTIFICATION_OPENERS.iter().any(|r| r.is_match(l))
    })
}

pub fn check(ctx: &FileCtx, out: &mut Out) {
    let src = ctx.src();
    let lang = match ctx.a.file.kind {
        FileKind::Code(l) => Some(l),
        FileKind::Markdown | FileKind::Gettext => None,
    };
    let (plan, history, narrative, banner) = (
        ctx.enabled("slop/comment-plan"),
        ctx.enabled("slop/comment-history"),
        ctx.enabled("slop/comment-narrative"),
        ctx.enabled("slop/comment-banner"),
    );
    if !(plan || history || narrative || banner) {
        return;
    }
    // The license header may sit after a shebang, which some extractors report as a comment.
    let first_idx = ctx
        .a
        .comments
        .iter()
        .position(|b| !src[b.range.clone()].starts_with("#!"));
    for (idx, block) in ctx.a.comments.iter().enumerate() {
        let v = View::new(src, block, lang);
        if v.joined.trim().is_empty() && v.raw.iter().all(|l| l.trim().is_empty()) {
            continue;
        }
        // Exemptions shared by every comment rule.
        if is_license(&v, Some(idx) == first_idx)
            || is_suppress_directive(&v)
            || v.has_meaningful_tag()
            || WHY_OR_TODO_RE.is_match(&v.joined)
        {
            continue;
        }

        let mut section_header = false;
        if banner
            && !v.rust_doc
            && !block.trailing
            && let Some(f) = check_banner(&v)
        {
            section_header = f.message.contains("header");
            out.push(f);
        }
        if plan {
            check_plan(&v, section_header, out);
        }
        if history {
            check_history(&v, out);
        }
        if narrative
            && !v.rust_doc
            && !block.trailing
            && !is_go_doc(&v, lang)
            && !is_ruby_doc(&v, lang)
            && let Some(reason) = narrative_reason(&v, lang)
        {
            out.push(
                Finding::new(
                    "slop/comment-narrative",
                    sev("slop/comment-narrative"),
                    v.prose_range(),
                    format!("Narrative comment ({reason})"),
                )
                .help("Delete comments that restate what the code does; keep only why, invariants and gotchas the code cannot show."),
            );
        }
    }
}

fn check_banner(v: &View) -> Option<Finding> {
    if !v.line_like {
        return None;
    }
    for (t, r) in &v.prose {
        let decorative = DECORATIVE_SEPARATOR.is_match(t) || DECORATIVE_SECTION_HEADER.is_match(t);
        let header = SECTION_HEADER.is_match(t);
        if decorative || header {
            let (msg, help) = if decorative {
                (
                    "Decorative comment banner",
                    "Remove the separator; group code with modules or functions, or use a plain one-line comment.",
                )
            } else {
                (
                    "Step/phase section header in comment",
                    "Remove the numbered header; name the step with a function, or describe why the order matters.",
                )
            };
            // Separator lines may have no content range; fall back to the block.
            let range = if r.is_empty() {
                v.block.range.clone()
            } else {
                r.clone()
            };
            return Some(
                Finding::new(
                    "slop/comment-banner",
                    sev("slop/comment-banner"),
                    range,
                    msg,
                )
                .help(help),
            );
        }
    }
    // Separator lines made only of comment markers (`////////`, `#########`) have empty content.
    for (l, raw) in v.block.lines.iter().zip(&v.raw) {
        if DECORATIVE_SEPARATOR.is_match(raw.trim()) {
            return Some(
                Finding::new("slop/comment-banner", sev("slop/comment-banner"), l.raw.clone(), "Decorative comment banner")
                    .help("Remove the separator; group code with modules or functions, or use a plain one-line comment."),
            );
        }
    }
    None
}

fn check_plan(v: &View, section_header: bool, out: &mut Out) {
    let hit = if section_header {
        None
    } else {
        PLAN_STEP_RE.find(&v.joined).map(|m| m.range())
    }
    .or_else(|| {
        PLAN_REFERENCE_RES
            .iter()
            .find_map(|r| r.find(&v.joined))
            .map(|m| m.range())
    });
    if let Some(r) = hit {
        let phrase = v.joined[r.clone()].to_string();
        out.push(
            Finding::new(
                "slop/comment-plan",
                sev("slop/comment-plan"),
                v.abs(r),
                format!("Comment refers to the plan, task or spec (\"{}\")", phrase.trim()),
            )
            .help("State the actual constraint the code must satisfy; references to the build plan or ticket belong in the PR description."),
        );
    }
}

fn check_history(v: &View, out: &mut Out) {
    if let Some(m) = BEFORE_AFTER_RES.iter().find_map(|r| r.find(&v.joined)) {
        let phrase: String = m.as_str().chars().take(50).collect();
        out.push(
            Finding::new(
                "slop/comment-history",
                sev("slop/comment-history"),
                v.abs(m.range()),
                format!("Comment narrates how the code changed (\"{}\")", phrase.trim()),
            )
            .help("Describe what the code does now and why; change history belongs in commit messages."),
        );
    }
}

fn narrative_reason(v: &View, lang: Option<Lang>) -> Option<&'static str> {
    if EXPLANATORY_WHY_MARKERS.is_match(&v.joined) || has_doc_indicator(v) {
        return None;
    }
    let next = v.block.next_code_line.as_deref();
    let above_decl = looks_like_declaration(next, lang);
    let signal = has_preamble_signal(v);
    let n = v.non_empty();
    if v.line_like && v.prose.len() >= 3 && above_decl && signal {
        return Some("multi-line preamble before declaration");
    }
    if !v.line_like && v.prose.len() >= 3 && above_decl && signal {
        return Some("doc-comment preamble that restates the code");
    }
    if v.line_like
        && v.prose_text()
            .any(|l| JUSTIFICATION_OPENERS.iter().any(|r| r.is_match(l)))
    {
        return Some("narrates what the code does");
    }
    if v.line_like
        && v.raw.len() == 1
        && v.prose
            .first()
            .is_some_and(|(t, _)| EXPLANATORY_OPENERS.is_match(t))
        && above_decl
    {
        return Some("explanatory preamble that restates the declaration");
    }
    if n >= 5 && !above_decl && signal {
        return Some("long narrative block");
    }
    if n >= 3 && v.line_like && !above_decl && signal {
        return Some("multi-line narrative prose");
    }
    None
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::config::Config;
    use crate::extract::comments::CommentLine;
    use crate::rules::Analyzed;
    use crate::source::SourceFile;

    /// Minimal comment extractor for tests (`//`, `#`, `///`, `//!`, `/** */`), so these
    /// tests do not depend on the real extractor.
    fn blocks(src: &str, lang: Lang) -> Vec<CommentBlock> {
        let hash = matches!(
            lang,
            Lang::Python | Lang::Ruby | Lang::Shell | Lang::Nix | Lang::Elixir
        );
        let mut out: Vec<CommentBlock> = Vec::new();
        let mut starts = vec![0];
        starts.extend(src.match_indices('\n').map(|(i, _)| i + 1));
        let lines: Vec<(usize, &str)> = src
            .split('\n')
            .enumerate()
            .map(|(i, l)| (starts[i], l))
            .collect();
        let next_code = |from: usize| {
            lines[from..]
                .iter()
                .map(|(_, l)| l.trim())
                .find(|l| !l.is_empty())
                .map(String::from)
        };
        let mut i = 0;
        while i < lines.len() {
            let (off, line) = lines[i];
            let t = line.trim_start();
            let indent = line.len() - t.len();
            if t.starts_with("/**") {
                let start = i;
                let mut cls = Vec::new();
                loop {
                    let (o, l) = lines[i];
                    let lt = l.trim_start();
                    let ind = l.len() - lt.len();
                    let mut c = lt;
                    let mut cstart = o + ind;
                    for p in ["/**", "*/", "*"] {
                        if let Some(rest) = c.strip_prefix(p) {
                            cstart += p.len();
                            c = rest;
                        }
                    }
                    let c = c.trim_end().trim_end_matches("*/");
                    cls.push(CommentLine {
                        raw: o..o + l.len(),
                        content: cstart..cstart + c.len(),
                    });
                    i += 1;
                    if lt.ends_with("*/") || i >= lines.len() {
                        break;
                    }
                }
                let end = cls.last().expect("comment block has a line").raw.end;
                out.push(CommentBlock {
                    range: cls[0].raw.start..end,
                    kind: CommentKind::Doc,
                    lines: cls,
                    start_line: start + 1,
                    next_code_line: next_code(i),
                    trailing: false,
                });
                continue;
            }
            let marker = |t: &str| -> Option<(usize, CommentKind)> {
                if hash {
                    (t.starts_with('#') && !t.starts_with("#!")).then_some((1, CommentKind::Line))
                } else if t.starts_with("///") || t.starts_with("//!") {
                    Some((3, CommentKind::Doc))
                } else if t.starts_with("//") {
                    Some((2, CommentKind::Line))
                } else {
                    None
                }
            };
            if let Some((_, kind)) = marker(t) {
                let start = i;
                let mut cls = Vec::new();
                while i < lines.len() {
                    let (o, l) = lines[i];
                    let lt = l.trim_start();
                    let Some((mlen, k)) = marker(lt) else { break };
                    if k != kind {
                        break;
                    }
                    let ind = l.len() - lt.len();
                    let body = &lt[mlen..];
                    let sp = usize::from(body.starts_with(' '));
                    let cs = o + ind + mlen + sp;
                    cls.push(CommentLine {
                        raw: o + ind..o + l.len(),
                        content: cs..o + l.len(),
                    });
                    i += 1;
                }
                out.push(CommentBlock {
                    range: cls[0].raw.start..cls.last().expect("comment block has a line").raw.end,
                    kind,
                    lines: cls,
                    start_line: start + 1,
                    next_code_line: next_code(i),
                    trailing: false,
                });
                let _ = (off, indent);
                continue;
            }
            i += 1;
        }
        out
    }

    fn run(src: &str, lang: Lang) -> Vec<Finding> {
        let file = SourceFile::new(
            PathBuf::from("/x"),
            PathBuf::from("x"),
            FileKind::Code(lang),
            src.to_string(),
        );
        let comments = blocks(src, lang);
        let segments = crate::extract::comments::segments(src, &comments);
        let a = Analyzed {
            file,
            md: None,
            po: None,
            comments,
            segments,
        };
        let config = Config::default();
        let ctx = FileCtx {
            a: &a,
            config: &config,
        };
        let mut out = Vec::new();
        check(&ctx, &mut out);
        out
    }

    fn rules(src: &str, lang: Lang) -> Vec<String> {
        run(src, lang).into_iter().map(|f| f.rule).collect()
    }

    const TS: Lang = Lang::TypeScript;

    #[test]
    fn plan_references() {
        for src in [
            "// Stage 3: wire up the cache layer\nexport const x = 1;\n",
            "// Validate the token per the spec before use\nexport const y = 2;\n",
            "// Sort ascending as per the requirements doc\nexport const z = 3;\n",
            "// Step 1 - validate the incoming payload\nexport const y = 2;\n",
        ] {
            assert!(
                rules(src, TS).contains(&"slop/comment-plan".to_string()),
                "{src}"
            );
        }
        assert_eq!(
            rules(
                "# Implement this from the task description\ndef run():\n    pass\n",
                Lang::Python
            ),
            ["slop/comment-plan"]
        );
    }

    #[test]
    fn plan_range_is_precise() {
        let src = "// Validate the token per the spec before use\nexport const y = 2;\n";
        let f = run(src, TS)
            .into_iter()
            .find(|f| f.rule == "slop/comment-plan")
            .unwrap();
        assert_eq!(&src[f.range], "per the spec");
    }

    #[test]
    fn history_narration() {
        for src in [
            "// Previously this used a Map; switched to a plain object\nexport const m = {};\n",
            "// This used to return null on miss\nexport const g = () => undefined;\n",
            "// The shim is no longer needed after the upgrade\nexport const h = 1;\n",
            "// Changed the default from 10 to 50 here\nexport const limit = 50;\n",
        ] {
            assert!(
                rules(src, TS).contains(&"slop/comment-history".to_string()),
                "{src}"
            );
        }
    }

    #[test]
    fn history_spanning_lines() {
        let src =
            "function f() {\n  // The old parser was\n  // renamed to legacy\n  return 1;\n}\n";
        let f = run(src, TS)
            .into_iter()
            .find(|f| f.rule == "slop/comment-history")
            .unwrap();
        assert_eq!(&src[f.range], "was\n  // renamed to");
    }

    #[test]
    fn exemptions() {
        for src in [
            "// Round up because the API rejects fractional cents\nexport const cents = 1;\n",
            "// TODO: handle the previously-unsupported locale\nexport const l = 1;\n",
            "/**\n * @param n the count\n * @returns doubled value\n */\nexport const dbl = (n: number): number => n * 2;\n",
            "// Copyright (c) 2026 Kenny\n// SPDX-License-Identifier: MIT\nexport const v = 1;\n",
            "// Maps the raw provider payload to our internal shape\nexport const map = () => ({});\n",
            "// Reads phase 2 bytes from the framed message header\nexport const read = () => 0;\n",
            "// Step 2 polling + auto-sync when the user returns\n// Make sure step 3 has projects loaded\n// always start at step 1 — that submit call stamps onboardedAt\nexport const x = 1;\n",
            "// biome-ignore lint/style/useConst: intentional\nlet z = 1;\n",
            "// wcwidth returns -1 for unmapped codepoints; treat as width 1.\nconst width = 1;\n",
            "// eslint-disable-next-line no-console -- previously this was noisy\nconsole.log(1);\n",
        ] {
            assert!(rules(src, TS).is_empty(), "{src}: {:?}", rules(src, TS));
        }
        assert!(rules("# noqa: F401\nimport os\n", Lang::Python).is_empty());
        assert!(
            rules(
                "# rubocop:disable Layout/LineLength\nclass Service\nend\n",
                Lang::Ruby
            )
            .is_empty()
        );
        assert!(
            rules(
                "# shellcheck disable=SC2086\n# ----------\necho $x\n",
                Lang::Shell
            )
            .is_empty()
        );
    }

    #[test]
    fn license_after_shebang() {
        let src =
            "#!/usr/bin/env bash\n# Licensed under MIT.\n# Previously this used curl.\necho hi\n";
        assert!(rules(src, Lang::Shell).is_empty());
    }

    #[test]
    fn banners() {
        assert_eq!(
            rules(
                "// ────────────────────────────────────────────\nexport const x = 1;\n",
                TS
            ),
            ["slop/comment-banner"]
        );
        assert_eq!(
            rules(
                "// ─── Classification ─────────────────\nexport const classify = (): void => { return; };\n",
                TS
            ),
            ["slop/comment-banner"]
        );
        assert_eq!(
            rules(
                "# -------------------------\nclass Service\nend\n",
                Lang::Ruby
            ),
            ["slop/comment-banner"]
        );
        assert_eq!(
            rules("##########\nfoo=1\n", Lang::Shell),
            ["slop/comment-banner"]
        );
        // A "Phase 1:" header is reported once, as a banner, not also as a plan reference.
        assert_eq!(
            rules(
                "// Phase 1: Code changes (imports, lint, dependencies)\nexport const y = 2;\n",
                TS
            ),
            ["slop/comment-banner"]
        );
        assert_eq!(
            rules(
                "# Phase 1: Build payload\ndef build_payload() -> int:\n    return 1\n",
                Lang::Python
            ),
            ["slop/comment-banner"]
        );
    }

    #[test]
    fn narrative_positive() {
        let cases: &[(&str, Lang)] = &[
            (
                "// This function does N things in order.\n// First it parses the input.\n// Then it validates it.\n// Finally it emits the output.\nexport const run = () => 0;\n",
                TS,
            ),
            (
                "/**\n * Detects side-effect expressions.\n * Walks an expression subtree and flags\n * anything that could invoke code when the declaration initializes.\n */\nexport const hasSideEffect = (): boolean => false;\n",
                TS,
            ),
            (
                "// Matches \"// ─── Title ───\" — separator runs flanking some text.\nexport const DECORATIVE_SECTION_HEADER = /^$/;\n",
                TS,
            ),
            (
                "export interface Options {\n\t/**\n\t * Describes when to use mode.\n\t * It does X in mode A and Y in mode B.\n\t * Defaults to mode A.\n\t */\n\tmode?: \"a\" | \"b\";\n}\n",
                TS,
            ),
            (
                "function run() {\n\t// Represents the parsed configuration after merging defaults.\n\t// Holds the resolved values for each configuration key.\n\t// Tracks which keys came from the environment overrides.\n\t// Manages the lifecycle of the cached lookups.\n\t// Controls how often the resolved values refresh.\n\treturn 1;\n}\n",
                TS,
            ),
            (
                "export const run = () => {\n\tinitState();\n\t// Represents the next-step list built from the counts.\n\t// Holds the elided count and the regression status.\n\t// Tracks each actionable sentence separately.\n\tbuildSteps();\n\treturn;\n};\n",
                TS,
            ),
            (
                "// This function takes a request and returns a response.\n// It does this by walking the routing table and matching the\n// path. The match is then dispatched to the handler.\npub fn handle() {}\n",
                Lang::Rust,
            ),
            ("# Holds the per-host overrides\nhosts = { };\n", Lang::Nix),
            (
                "# Wraps the HTTP client\ndefmodule Client do\nend\n",
                Lang::Elixir,
            ),
            (
                "// Stores the allocator state\npub const State = struct {};\n",
                Lang::Zig,
            ),
            ("# Handles argument parsing\nparse_args() {\n", Lang::Shell),
            ("// Represents a parsed token\nstruct token {\n", Lang::C),
        ];
        for (src, lang) in cases {
            assert_eq!(rules(src, *lang), ["slop/comment-narrative"], "{src}");
        }
    }

    #[test]
    fn narrative_negative() {
        let cases: &[(&str, Lang)] = &[
            (
                "function run() {\n\t// Werkzeug doesn't implement subdomain matching yet, so we fall back to\n\t// host matching here. Until that lands upstream we keep this shim, which\n\t// the routing layer relies on to resolve the correct endpoint. The order\n\t// of these checks also matters for the development server's reloader.\n\t// Downstream callers depend on the host being normalised first.\n\treturn 1;\n}\n",
                TS,
            ),
            (
                "function run() {\n\t// Matches the user input\n\treturn 1;\n}\n",
                TS,
            ),
            (
                "/**\n * Middleware to track daily usage counts.\n * Limit enforcement is handled client-side via RevenueCat subscription checks.\n * Server-side enforcement requires a trusted subscription verification mechanism\n * (e.g. RevenueCat webhook writing subscription status to the profiles table).\n */\nexport async function checkUsageLimit() {}\n",
                TS,
            ),
            (
                "/**\n * @swagger\n * /users:\n *   get:\n *     summary: List users\n */\nrouter.get('/users', handler);\n",
                Lang::JavaScript,
            ),
            (
                "/// Returns the canonical, absolute form of a path with all intermediate\n/// components normalized and symbolic links resolved.\n///\n/// This function is an async version of [`std::fs::canonicalize`].\npub async fn canonicalize() {}\n",
                Lang::Rust,
            ),
            (
                "//! This module holds the types documented locally.\n//! It handles re-exports.\n\npub mod foo {}\n",
                Lang::Rust,
            ),
            (
                "/**\n * A small data structure used throughout the runtime. Provides a\n * predictable lifecycle for inspection and update operations during\n * test runs and production paths alike.\n */\npublic class Box {}\n",
                Lang::Java,
            ),
            (
                "# Authenticates a user against the configured backend.\n#\n# @param username [String] the login\n# @return [Boolean] whether authentication succeeded\ndef login(username, password); end\n",
                Lang::Ruby,
            ),
            (
                "##\n# Runs a task by name with the given arguments and returns\n# the captured output. Raises if the task is not registered.\n# Used by the CLI front-end and the test harness.\ndef run_task(name, *args); end\n",
                Lang::Ruby,
            ),
            (
                "package x\n\ntype Key struct {\n\t// Text contains the actual characters received. This usually the same as\n\t// the key code. When the text is non-empty, it indicates that the key\n\t// pressed represents printable character(s).\n\tText string\n}\n",
                Lang::Go,
            ),
            (
                "package x\n\nfunc render() {\n\t// Render top\n\tdoStuff()\n\n\t// Render sides\n\tdoMore()\n}\n",
                Lang::Go,
            ),
            (
                "// buildFixRender will then be used by the header code\nconst y = 1;\n",
                TS,
            ),
        ];
        for (src, lang) in cases {
            assert!(
                rules(src, *lang).is_empty(),
                "{src}: {:?}",
                rules(src, *lang)
            );
        }
    }
}
