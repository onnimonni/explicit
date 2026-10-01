//! Rule families and the shared per-file context.

pub mod codeblock;
pub mod diagram;
pub mod docs;
pub mod gettext;
pub mod grammar;
pub mod grammar_de;
pub mod grammar_es;
pub mod grammar_fi;
pub mod grammar_fr;
pub mod grammar_pt;
pub mod grammar_sv;
pub mod lint;
pub mod patterns;
pub mod prose;
pub mod slop;
pub mod spell;
pub mod spell_lang;
pub mod structure;
pub mod style;
pub mod words;

use crate::config::Config;
use crate::diagnostic::{Finding, Severity};
use crate::extract::comments::CommentBlock;
use crate::extract::markdown::MdDoc;
use crate::segment::Segment;
use crate::source::SourceFile;

/// Everything extracted from one file.
#[derive(Debug)]
pub struct Analyzed {
    pub file: SourceFile,
    /// Present for Markdown files.
    pub md: Option<MdDoc>,
    /// Present for gettext PO/POT catalogs.
    pub po: Option<crate::extract::gettext::Catalog>,
    /// Comment blocks for code files.
    pub comments: Vec<CommentBlock>,
    /// Prose segments: Markdown blocks or comment blocks.
    pub segments: Vec<Segment>,
}

impl Analyzed {
    pub fn new(file: SourceFile) -> Analyzed {
        use crate::source::FileKind;
        match file.kind {
            FileKind::Markdown => {
                let md = crate::extract::markdown::parse(&file.text);
                let segments = md.segments.clone();
                Analyzed {
                    file,
                    md: Some(md),
                    po: None,
                    comments: Vec::new(),
                    segments,
                }
            }
            FileKind::Code(lang) => {
                let comments = crate::extract::comments::extract(lang, &file.text);
                let segments = crate::extract::comments::segments(&file.text, &comments);
                Analyzed {
                    file,
                    md: None,
                    po: None,
                    comments,
                    segments,
                }
            }
            FileKind::Gettext => {
                let po = crate::extract::gettext::parse(&file.text);
                let (comments, segments) = gettext::prose(&file, &po);
                Analyzed {
                    file,
                    md: None,
                    po: Some(po),
                    comments,
                    segments,
                }
            }
        }
    }
}

pub struct FileCtx<'a> {
    pub a: &'a Analyzed,
    pub config: &'a Config,
}

impl<'a> FileCtx<'a> {
    pub fn src(&self) -> &'a str {
        &self.a.file.text
    }

    /// Whether a rule would produce output (so expensive checks can be skipped).
    pub fn enabled(&self, rule: &str) -> bool {
        rule_enabled(self.config, rule)
    }
}

/// Whether `rule` would produce output under `config`.
pub fn rule_enabled(config: &Config, rule: &str) -> bool {
    let default = RULES.iter().find(|r| r.id == rule).map(|r| r.default);
    config
        .severity(rule, default.unwrap_or(Some(Severity::Warning)))
        .is_some()
}

pub type Out = Vec<Finding>;

/// Static rule metadata used for `explicit rules` and default severities.
pub struct RuleInfo {
    pub id: &'static str,
    /// `None` = off by default.
    pub default: Option<Severity>,
    pub description: &'static str,
}

macro_rules! rules {
    ($($id:literal => $sev:expr, $desc:literal;)*) => {
        pub static RULES: &[RuleInfo] = &[$(RuleInfo { id: $id, default: $sev, description: $desc }),*];
    };
}

const E: Option<Severity> = Some(Severity::Error);
const W: Option<Severity> = Some(Severity::Warning);
const I: Option<Severity> = Some(Severity::Info);
const OFF: Option<Severity> = None;

rules! {
    // Markdown structure (markdownlint equivalents in parentheses).
    "md/heading-increment" => W, "Heading levels increase by one at a time (MD001)";
    "md/heading-style" => W, "Consistent ATX/setext heading style (MD003)";
    "md/list-marker" => W, "Consistent unordered list marker (MD004)";
    "md/list-indent" => W, "Consistent list item indentation (MD005/MD007)";
    "md/no-trailing-spaces" => W, "No trailing whitespace, except 2-space hard breaks (MD009)";
    "md/no-hard-tabs" => W, "No hard tabs outside code blocks (MD010)";
    "md/no-multiple-blanks" => W, "No multiple consecutive blank lines (MD012)";
    "md/line-length" => OFF, "Lines not longer than markdown.line_length (MD013)";
    "md/blanks-around-headings" => W, "Headings surrounded by blank lines (MD022)";
    "md/heading-start-left" => W, "Headings start at the beginning of the line (MD023)";
    "md/no-duplicate-heading" => W, "No sibling headings with the same text (MD024)";
    "md/single-h1" => W, "Only one top-level heading (MD025)";
    "md/no-trailing-punctuation" => W, "No trailing punctuation in headings (MD026)";
    "md/ol-prefix" => W, "Ordered list numbering is sequential or all ones (MD029)";
    "md/blanks-around-fences" => W, "Fenced code blocks surrounded by blank lines (MD031)";
    "md/blanks-around-lists" => W, "Lists surrounded by blank lines (MD032)";
    "md/no-bare-urls" => W, "URLs wrapped in <> or written as links (MD034)";
    "md/no-emphasis-as-heading" => W, "Emphasis-only lines used instead of headings (MD036)";
    "md/no-space-in-code" => W, "No spaces inside code span backticks (MD038)";
    "md/fenced-code-language" => W, "Fenced code blocks declare a language (MD040)";
    "md/first-line-heading" => OFF, "The first line is a top-level heading (MD041)";
    "md/no-empty-links" => E, "Links have a destination (MD042)";
    "md/code-language-known" => W, "Fenced code block languages are known to GitHub Linguist";
    "md/code-block-style" => W, "Code blocks are fenced, not indented (MD046)";
    "md/single-trailing-newline" => W, "File ends with exactly one newline (MD047)";
    "md/heading-case" => OFF, "Heading case follows markdown.heading_case";
    "md/image-alt" => W, "Images have alt text (MD045)";
    "md/no-reversed-links" => E, "Links are not written (text)[url] (MD011)";
    "md/commands-show-output" => I, "Shell blocks where every line starts with $ also show output (MD014)";
    "md/no-missing-space-atx" => W, "Space after the hashes of ATX headings (MD018)";
    "md/no-multiple-space-atx" => W, "Single space around ATX heading text (MD019/MD021)";
    "md/no-missing-space-closed-atx" => W, "Spaces inside closed ATX heading hashes (MD020)";
    "md/no-multiple-space-blockquote" => W, "Single space after block quote marker (MD027)";
    "md/no-blanks-blockquote" => W, "No blank line splitting a block quote (MD028)";
    "md/list-marker-space" => W, "One space after list markers (MD030)";
    "md/hr-style" => W, "Consistent thematic break style (MD035)";
    "md/no-space-in-emphasis" => W, "No spaces inside emphasis markers (MD037)";
    "md/no-space-in-links" => W, "No spaces inside link text brackets (MD039)";
    "md/code-fence-style" => W, "Consistent code fence style (MD048)";
    "md/emphasis-style" => W, "Consistent emphasis marker (MD049)";
    "md/strong-style" => W, "Consistent strong marker (MD050)";
    "md/table-pipe-style" => W, "Consistent leading/trailing table pipes (MD055)";
    "md/table-column-count" => E, "Table rows have as many cells as the header (MD056)";
    "md/blanks-around-tables" => W, "Tables surrounded by blank lines (MD058)";
    "md/descriptive-link-text" => W, "Link text is descriptive, not \"click here\" (MD059)";
    "md/no-duplicate-definition" => E, "Reference definition label defined only once";
    "md/no-empty-section" => W, "Headings have content before the next same-or-higher heading";
    "md/alert-syntax" => E, "GitHub alert type is valid and alone on its line";
    "md/no-heading-like-paragraph" => W, "Lines starting with 7+ hashes (not a heading)";
    "md/max-heading-length" => OFF, "Headings not longer than markdown.max_heading_length";
    "md/no-inline-html" => OFF, "Only markdown.allowed_html elements in raw HTML (MD033)";
    "md/front-matter-required" => OFF, "Front matter has the markdown.front_matter_required keys";
    "md/front-matter-lang" => I, "Finnish or Swedish files declare `lang:` in front matter (fix adds it)";
    "md/task-list-style" => I, "Task list checkboxes are [ ] or [x]";

    // English.
    "spelling" => E, "Spelling (Harper)";
    "grammar/*" => I, "Harper grammar and style rules, one id per Harper rule; info, except a few high-confidence rules at warning (see prose.grammar_level)";

    // Links.
    "links/missing-file" => E, "Relative link or image target exists";
    "links/absolute-image-path" => E, "Images use paths relative to the Markdown file, not absolute or root-relative paths";
    "links/missing-anchor" => E, "Link #fragment matches a heading or anchor";
    "links/undefined-ref" => E, "Reference link label is defined";
    "links/unused-ref" => W, "Reference definition is used";
    "links/http-error" => E, "Remote link responds with success";
    "links/http-unreachable" => W, "Remote link host is reachable";
    "links/same-repo-url" => W, "Links into this GitHub repository's default branch use relative paths";
    "links/same-repo-ref" => E, "Links to this repository's commits, tags, paths, issues and pull requests exist (checked with local git and gh)";
    "links/image-url" => E, "Remote image URL exists and serves an image, not an HTML page";
    "links/http-redirect" => I, "Remote link permanently redirects";
    "links/insecure" => I, "Link uses http:// instead of https://";
    "links/undefined-footnote" => E, "Footnote reference has a definition";
    "links/unused-footnote" => W, "Footnote definition is referenced";
    "links/duplicate-anchor" => W, "Explicit heading ids are unique";

    // AI slop and mannerisms.
    "slop/phrase" => W, "Stock AI phrasing from the slop catalogue";
    "slop/word" => I, "Overused vague or hyped word";
    "slop/density" => W, "High density of slop words in a section";
    "slop/em-dash" => W, "Em dash overuse";
    "slop/not-just-but" => W, "\"Not just X, but Y\" contrast framing";
    "slop/hedging" => W, "Stacked hedges (might possibly, could potentially)";
    "slop/rule-of-three" => I, "Reflexive triads of adjectives";
    "slop/negation-chain" => W, "\"No X, no Y\" / \"did not X, did not Y\" chains";
    "slop/dont-verb-it" => W, "\"Don't call it X. Call it Y.\" reframe";
    "slop/echo-sentences" => I, "Consecutive sentences ending on the same 4+ word skeleton";
    "slop/stacked-questions" => I, "Two or more rhetorical questions in a row";
    "slop/anaphora" => I, "Three or more consecutive sentences opening with the same word";
    "slop/stranded-auxiliary" => I, "Clause ending on a bare auxiliary (\"the data didn't.\")";
    "slop/colon-triple" => OFF, "Colon opening onto three comma-separated items";
    "slop/not-but" => OFF, "Plain \"not X, but Y\" negative parallelism";
    "slop/comment-plan" => W, "Comment references the plan, task or spec";
    "slop/comment-history" => W, "Comment narrates what the code used to do";
    "slop/comment-narrative" => W, "Comment restates what the code does";
    "slop/comment-banner" => I, "Decorative comment banners and separators";

    // Diagrams in fenced code blocks.
    "diagram/mermaid" => E, "Mermaid diagram parses (merman-core)";
    "diagram/d2" => E, "D2 diagram is valid: syntax, keywords, shapes, style values";

    // Data and graph languages in fenced code blocks (opt out with `skip-lint` in the info string).
    "codeblock/json" => E, "```json parses strictly; ```jsonc allows comments and trailing commas";
    "codeblock/toml" => E, "```toml parses";
    "codeblock/yaml" => E, "```yaml / ```yml parses";
    "codeblock/dot" => W, "```dot / ```graphviz has a graph header, balanced delimiters, matching edge ops";
    "codeblock/empty" => W, "Fenced code block is not empty";

    // Documentation sites.
    "docs/toc-sync" => W, "Table of contents matches the headings that follow it";
    "docs/orphan-page" => OFF, "Page under docs.root is linked from another Markdown file";
    "docs/include-missing" => E, "Include directive (snippets, Jekyll, Hugo, markdown-include) target exists";

    // Prose style (write-good, alex, retext-simplify, textlint terminology, Vale metrics).
    "prose/inclusive" => W, "Insensitive or exclusionary wording (alex/retext-equality)";
    "prose/simplify" => I, "Wordy or complex phrasing with a plain alternative (retext-simplify)";
    "prose/terminology" => W, "Canonical spelling of product and tech names (JavaScript, GitHub)";
    "prose/entity-name" => W, "Configured [[entity]] names and case-sensitive [[vocab]] terms keep their casing";
    "prose/ambiguous-person" => W, "A name part several [[person]] entries share, used alone without context";
    "prose/passive" => OFF, "Passive voice (write-good)";
    "prose/weasel" => I, "Weasel words (very, quite, several, some people say)";
    "prose/there-is" => I, "Sentence starts with There is/are/was/were";
    "prose/so-start" => I, "Sentence starts with \"So\"";
    "prose/sentence-length" => OFF, "Sentence longer than prose.max_sentence_words";
    "prose/readability" => OFF, "Section Flesch-Kincaid grade above prose.max_grade (Markdown)";
    "prose/consistency" => I, "Two spellings of the same word in one file (color/colour)";
    "prose/acronym-defined" => OFF, "Acronym used before it is defined (Markdown)";
    "prose/smart-quotes" => OFF, "Mixed straight and curly quotes in one file";
    "prose/sentence-spacing" => OFF, "Two or more spaces after a sentence";

    // GNU gettext PO/POT catalogs (POT templates skip the translation checks).
    "gettext/syntax" => E, "PO syntax: terminated strings, known escapes and keywords, msgstr present";
    "gettext/duplicate" => E, "Each msgctxt + msgid pair appears once";
    "gettext/header" => E, "Header entry exists with a UTF-8 charset, valid Plural-Forms and a Language matching the path";
    "gettext/plural-count" => E, "Translated plural entries have nplurals msgstr[N] forms";
    "gettext/placeholders" => E, "msgstr keeps the msgid's printf, python, brace, ICU, Elixir and Ruby placeholders";
    "gettext/plural-placeholder" => OFF, "A plural msgstr[N] omits the count placeholder (allowed by default)";
    "gettext/markup" => W, "msgstr keeps the msgid's HTML tags, Markdown links and code spans";
    "gettext/whitespace" => W, "msgstr keeps the msgid's leading and trailing newlines and spaces";
    "gettext/punctuation" => I, "msgstr ends with the same punctuation (: . ? ! …) as the msgid";
    "gettext/accelerator" => W, "msgstr keeps the msgid's & or _ keyboard accelerator (when the catalog uses them)";
    "gettext/untranslated" => I, "Entry has no translation";
    "gettext/fuzzy" => W, "Entry is marked fuzzy and is ignored at runtime";
    "gettext/obsolete" => I, "Obsolete #~ entry left in the catalog";
    "gettext/same-as-source" => I, "msgstr identical to a longer msgid (probably untranslated)";

    // explicit.toml itself.
    "config/placeholder" => W, "explicit.toml description, relationship and role values are not placeholders (TODO, TBD, ...)";

    // Vale-style rules from explicit.toml.
    "style/*" => W, "User-defined existence/substitution/repetition/occurrence/capitalization rules";
}

/// One row of `explicit rules`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RuleRow {
    pub id: String,
    /// `None` = off by default.
    pub default: Option<Severity>,
    pub description: String,
}

/// Rule list for `explicit rules`. With `all`, the `grammar/*` and `style/*` families expand to
/// one row per Harper rule and per `[[style]]` rule in `config`.
pub fn rule_rows(config: &Config, all: bool) -> Vec<RuleRow> {
    let mut rows = Vec::new();
    for r in RULES {
        let row = RuleRow {
            id: r.id.to_string(),
            default: r.default,
            description: r.description.to_string(),
        };
        match r.id {
            "grammar/*" if all => rows.extend(harper_rows(config)),
            "style/*" if all => rows.extend(config.style.iter().map(|s| {
                RuleRow {
                    id: format!("style/{}", s.name),
                    default: s.level.severity(),
                    description: s
                        .message
                        .clone()
                        .unwrap_or_else(|| format!("{:?} rule", s.kind).to_lowercase()),
                }
            })),
            _ => rows.push(row),
        }
    }
    rows
}

/// Every Harper rule except spell checking (`spelling`), sorted by name, plus our own pattern
/// rules. Rules Harper leaves off or `prose.disable` turns off are listed as off; the rest with
/// their `prose.grammar_level` severity (under `harper`, the real severity follows each lint's
/// kind; shown as warning). Without the `harper` feature: only our pattern rules.
fn harper_rows(config: &Config) -> Vec<RuleRow> {
    let disabled = &config.prose.disable;
    let severity = |name: &str| {
        grammar::default_severity(name, lint::LintKind::Grammar, config.prose.grammar_level)
    };
    let mut rows: Vec<RuleRow> = Vec::new();
    #[cfg(feature = "harper")]
    {
        use harper_core::linting::LintGroup;
        use harper_core::spell::FstDictionary;
        let group =
            LintGroup::new_curated(FstDictionary::curated(), harper_core::Dialect::American);
        rows.extend(
            group
                .all_descriptions()
                .into_iter()
                .filter(|(name, _)| *name != "SpellCheck")
                .map(|(name, desc)| {
                    let on =
                        group.config.is_rule_enabled(name) && !disabled.iter().any(|d| d == name);
                    RuleRow {
                        id: format!("grammar/{name}"),
                        default: on.then(|| severity(name)),
                        description: desc.split_whitespace().collect::<Vec<_>>().join(" "),
                    }
                }),
        );
    }
    #[cfg(not(feature = "harper"))]
    rows.extend(patterns::HARPER_NAMED.iter().map(|(name, desc)| {
        let on = !disabled.iter().any(|d| d == name);
        RuleRow {
            id: format!("grammar/{name}"),
            default: on.then(|| severity(name)),
            description: desc.to_string(),
        }
    }));
    // Native grammar rules for the bundled languages.
    rows.extend(
        grammar_fi::RULES
            .iter()
            .chain(grammar_sv::RULES)
            .chain(grammar_de::RULES)
            .chain(grammar_fr::RULES)
            .chain(grammar_es::RULES)
            .chain(grammar_pt::RULES)
            .map(|(name, desc)| {
                let on = !disabled.iter().any(|d| d == name);
                RuleRow {
                    id: format!("grammar/{name}"),
                    default: on.then(|| severity(name)),
                    description: desc.to_string(),
                }
            }),
    );
    // Our own word-confusion rules, which every engine runs.
    rows.extend(patterns::OWN_DESCRIPTIONS.iter().map(|(name, desc)| {
        let on = !disabled.iter().any(|d| d == name);
        RuleRow {
            id: format!("grammar/{name}"),
            default: on.then(|| severity(name)),
            description: desc.to_string(),
        }
    }));
    rows.sort_by(|a, b| a.id.cmp(&b.id));
    rows
}

/// Default severity for a rule id; `grammar/X` and `style/X` use their family default.
pub fn default_severity(rule: &str) -> Option<Severity> {
    if let Some(r) = RULES.iter().find(|r| r.id == rule) {
        return r.default;
    }
    let family = rule.split('/').next().unwrap_or("");
    RULES
        .iter()
        .find(|r| r.id.strip_suffix("/*") == Some(family))
        .map_or(Some(Severity::Warning), |r| r.default)
}

impl FileCtx<'_> {
    /// Whether any rule matching the family prefix could be enabled.
    pub fn family_enabled(&self, family: &str) -> bool {
        RULES
            .iter()
            .filter(|r| r.id.starts_with(family))
            .any(|r| self.config.severity(r.id, r.default).is_some())
            || self
                .config
                .rules
                .iter()
                .any(|(k, l)| l.severity().is_some() && key_may_match_family(k, family))
    }
}

/// Whether config key `k` (an id or glob) could match some rule id starting with `family`.
fn key_may_match_family(k: &str, family: &str) -> bool {
    match k.split_once('*') {
        None => k.starts_with(family),
        Some((prefix, _)) => k.starts_with(family) || family.starts_with(prefix),
    }
}

#[cfg(test)]
mod tests {
    use super::key_may_match_family;

    #[test]
    fn all_rules_expand_families() {
        use crate::config::{Config, StyleKind, StyleRule};
        let mut c = Config::default();
        c.style.push(StyleRule {
            name: "no-simply".into(),
            kind: StyleKind::Existence,
            message: None,
            level: crate::config::Level::Error,
            tokens: vec!["simply".into()],
            swap: Default::default(),
            regex: false,
            ignore_case: None,
            max: None,
            case: None,
            exceptions: Vec::new(),
            scope: None,
        });
        let short = super::rule_rows(&c, false);
        assert!(short.iter().any(|r| r.id == "grammar/*"));
        let all = super::rule_rows(&c, true);
        assert!(!all.iter().any(|r| r.id.contains('*')));
        let get = |id: &str| all.iter().find(|r| r.id == id);
        assert!(get("grammar/SpellCheck").is_none() && get("spelling").is_some());
        let grammar = all.iter().filter(|r| r.id.starts_with("grammar/")).count();
        #[cfg(feature = "harper")]
        {
            assert!(get("grammar/OxfordComma").is_some_and(|r| r.default.is_none()));
            assert!(grammar > 100);
        }
        // Without Harper: our pattern rules only.
        #[cfg(not(feature = "harper"))]
        {
            assert!(get("grammar/OxfordComma").is_none());
            assert!(get("grammar/AnA").is_some_and(|r| r.default.is_some()));
            assert!(grammar > 20);
        }
        let s = get("style/no-simply").unwrap();
        assert_eq!(s.default, Some(crate::diagnostic::Severity::Error));
        assert_eq!(s.description, "existence rule");
    }

    #[test]
    fn family_keys() {
        assert!(key_may_match_family("grammar/OxfordComma", "grammar/"));
        assert!(key_may_match_family("*OxfordComma", "grammar/"));
        assert!(key_may_match_family("gr*", "grammar/"));
        assert!(key_may_match_family("grammar/*", "grammar/"));
        assert!(!key_may_match_family("md/*", "grammar/"));
        assert!(!key_may_match_family("spelling", "grammar/"));
    }
}
