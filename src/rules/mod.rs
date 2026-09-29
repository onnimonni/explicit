//! Rule families and the shared per-file context.

pub mod codeblock;
pub mod diagram;
pub mod docs;
pub mod grammar;
pub mod prose;
pub mod slop;
pub mod structure;
pub mod style;

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
        let default = RULES.iter().find(|r| r.id == rule).map(|r| r.default);
        self.config
            .severity(rule, default.unwrap_or(Some(Severity::Warning)))
            .is_some()
    }
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
    "md/task-list-style" => I, "Task list checkboxes are [ ] or [x]";

    // English.
    "spelling" => E, "Spelling (Harper)";
    "grammar/*" => W, "Harper grammar and style rules, one id per Harper rule; severity follows Harper's lint kind";

    // Links.
    "links/missing-file" => E, "Relative link or image target exists";
    "links/missing-anchor" => E, "Link #fragment matches a heading or anchor";
    "links/undefined-ref" => E, "Reference link label is defined";
    "links/unused-ref" => W, "Reference definition is used";
    "links/http-error" => E, "Remote link responds with success";
    "links/http-unreachable" => W, "Remote link host is reachable";
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
    "docs/readme-absolute-image" => I, "README image uses a local path instead of a GitHub URL to this repo";

    // Prose style (write-good, alex, retext-simplify, textlint terminology, Vale metrics).
    "prose/inclusive" => W, "Insensitive or exclusionary wording (alex/retext-equality)";
    "prose/simplify" => I, "Wordy or complex phrasing with a plain alternative (retext-simplify)";
    "prose/terminology" => W, "Canonical spelling of product and tech names (JavaScript, GitHub)";
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

    // Vale-style rules from explicit.toml.
    "style/*" => W, "User-defined existence/substitution/repetition/occurrence/capitalization rules";
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
    fn family_keys() {
        assert!(key_may_match_family("grammar/OxfordComma", "grammar/"));
        assert!(key_may_match_family("*OxfordComma", "grammar/"));
        assert!(key_may_match_family("gr*", "grammar/"));
        assert!(key_may_match_family("grammar/*", "grammar/"));
        assert!(!key_may_match_family("md/*", "grammar/"));
        assert!(!key_may_match_family("spelling", "grammar/"));
    }
}
