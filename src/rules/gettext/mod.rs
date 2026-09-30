//! GNU gettext PO/POT catalog checks (`gettext/*`).
//!
//! Structural rules (syntax, duplicates, header) run on POT templates too; translation rules
//! (placeholders, plural counts, markup, whitespace, untranslated, ...) only on PO files.
//! The msgids are English source text, so [`prose`] turns them into prose segments for the
//! spelling, grammar, slop and style rules. Translations are checked in the catalog's language
//! (header Language, else the path's locale): English with the English rules, Finnish and
//! Swedish with their spellers; other languages are skipped.

mod placeholders;
#[cfg(test)]
mod tests;

use std::collections::{BTreeSet, HashMap};
use std::ops::Range;
use std::path::Path;
use std::sync::LazyLock;

use regex::Regex;

use super::{FileCtx, Out};
use crate::diagnostic::{Finding, Severity};
use crate::extract::comments::{CommentBlock, CommentKind, CommentLine};
use crate::extract::gettext::{Catalog, Entry, Field, PoString, is_key, parse_plural_forms};
use crate::segment::{Segment, SegmentKind};
use crate::source::SourceFile;
use placeholders::{Styles, is_count_name, keys};

/// POT templates carry no translations.
fn is_template(file: &SourceFile) -> bool {
    file.path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("pot"))
}

fn is_english(lang: &str) -> bool {
    let l = lang.to_ascii_lowercase();
    l == "en" || l.starts_with("en_") || l.starts_with("en-") || l.starts_with("en@")
}

/// Header Language, if set.
fn language(po: &Catalog) -> Option<&str> {
    po.header
        .as_ref()?
        .get("Language")
        .map(|f| f.value.as_str())
        .filter(|v| !v.is_empty())
}

/// Translator and extracted comment lines used as metadata (`origin: machine`), not prose.
static META_COMMENT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[A-Za-z][\w.-]*:\s*\S*$").expect("hardcoded regex is valid"));

/// Comment blocks (for suppressions and comment rules) and prose segments of a catalog.
pub fn prose(file: &SourceFile, po: &Catalog) -> (Vec<CommentBlock>, Vec<Segment>) {
    let src = file.text.as_str();
    // With a POT template next to it, the template is where msgids and extracted comments are
    // checked; its PO files would only repeat the same findings once per language.
    let from_template =
        !is_template(file) && template_path(&file.path).is_some_and(|p| p.is_file());
    let checked_target =
        target_language(file, po).is_some_and(|l| PROSE_TARGETS.contains(&l.as_str()));
    let header = po.header.as_ref().map(|h| h.entry);
    let mut blocks = Vec::new();
    let mut segments = Vec::new();
    for (idx, e) in po.entries.iter().enumerate() {
        let is_header = Some(idx) == header;
        let extracted: &[crate::extract::gettext::CommentLine] =
            if from_template { &[] } else { &e.extracted };
        for lines in [e.translator.as_slice(), extracted] {
            let mut cur: Vec<CommentLine> = Vec::new();
            let mut flush = |cur: &mut Vec<CommentLine>| {
                if cur.is_empty() {
                    return;
                }
                let lines = std::mem::take(cur);
                let start = lines[0].raw.start;
                let end = lines.last().map_or(start, |l| l.raw.end);
                blocks.push(CommentBlock {
                    range: start..end,
                    kind: CommentKind::Line,
                    start_line: file.line_col(start).0,
                    lines,
                    next_code_line: None,
                    trailing: false,
                });
            };
            for l in lines {
                let content = src[l.content.clone()].trim();
                // Header boilerplate and `key: value` metadata are not prose; keep directives.
                let directive = content.contains("explicit-");
                if !directive && (is_header || META_COMMENT.is_match(content)) {
                    flush(&mut cur);
                    continue;
                }
                // Consecutive lines only.
                if cur
                    .last()
                    .is_some_and(|p| !src[p.raw.end..l.raw.start].trim().is_empty())
                {
                    flush(&mut cur);
                }
                cur.push(CommentLine {
                    raw: l.raw.clone(),
                    content: l.content.clone(),
                });
            }
            flush(&mut cur);
        }
        if e.obsolete || is_header {
            continue;
        }
        let key = is_key(e.msgid_str());
        let st = placeholders::styles(e);
        if !key && !from_template {
            for f in e.msgid.iter().chain(&e.msgid_plural) {
                segments.extend(field_segment(src, &f.s, st));
            }
        }
        if checked_target {
            for f in checked_translations(e, key) {
                segments.extend(field_segment(src, &f.s, st));
            }
        }
    }
    segments.extend(crate::extract::comments::segments(src, &blocks));
    segments.sort_by_key(|s| s.range.start);
    blocks.sort_by_key(|b| b.range.start);
    (blocks, segments)
}

/// Translation languages whose msgstrs are prose segments: English with the English rules,
/// Finnish and Swedish as marked regions ([`translation_regions`]) with their own spellers.
const PROSE_TARGETS: &[&str] = &["en", "fi", "sv"];

/// Primary language subtag of a PO file's translations: the header Language, else the locale
/// the path names (`fi/LC_MESSAGES/x.po`); `None` for templates.
pub fn target_language(file: &SourceFile, po: &Catalog) -> Option<String> {
    if is_template(file) {
        return None;
    }
    let tag = language(po)
        .map(String::from)
        .or_else(|| path_locale(&file.path))?;
    let tag = tag.split('@').next().unwrap_or("");
    (!tag.is_empty()).then(|| crate::rules::spell_lang::primary(tag))
}

/// Translations of `e` checked as prose: non-empty and, for real messages (not keys), not a
/// copy of the source text.
fn checked_translations(e: &Entry, key: bool) -> impl Iterator<Item = &Field> {
    let sources = [
        e.msgid_str(),
        e.msgid_plural.as_ref().map_or("", |f| f.value()),
    ];
    e.translations()
        .into_iter()
        .filter(move |f| !f.value().is_empty() && (key || !sources.contains(&f.value())))
}

/// Non-English translations checked as prose, as regions in the catalog's language (see
/// [`crate::lang_marks`]).
pub fn translation_regions(file: &SourceFile, po: &Catalog) -> Vec<crate::lang_marks::Region> {
    let Some(lang) =
        target_language(file, po).filter(|l| l != "en" && PROSE_TARGETS.contains(&l.as_str()))
    else {
        return Vec::new();
    };
    let header = po.header.as_ref().map(|h| h.entry);
    po.entries
        .iter()
        .enumerate()
        .filter(|(i, e)| !e.obsolete && Some(*i) != header)
        .flat_map(|(_, e)| checked_translations(e, is_key(e.msgid_str())))
        .map(|f| (f.s.span(), lang.clone()))
        .collect()
}

/// The POT template a PO file is merged from: `<domain>.pot` above `<lang>/LC_MESSAGES/` or
/// `<lang>/`, or a single `.pot` in the same directory (`po/fi.po` + `po/app.pot`).
fn template_path(path: &Path) -> Option<std::path::PathBuf> {
    let stem = path.file_stem()?.to_str()?;
    let dir = path.parent()?;
    if dir.file_name().is_some_and(|n| n == "LC_MESSAGES") {
        return Some(dir.parent()?.parent()?.join(format!("{stem}.pot")));
    }
    if dir
        .file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| LOCALE_RE.is_match(n))
        && let Some(up) = dir.parent()
        && up.join(format!("{stem}.pot")).is_file()
    {
        return Some(up.join(format!("{stem}.pot")));
    }
    let mut pots = std::fs::read_dir(dir)
        .ok()?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "pot"));
    let first = pots.next()?;
    pots.next().is_none().then_some(first)
}

static TAG_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"</?([A-Za-z0-9][\w:.-]*)(?:\s+[^<>]*?)?\s*/?>"#)
        .expect("hardcoded regex is valid")
});
static MD_LINK_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[[^\]\n]+\]\([^)\s]+\)").expect("hardcoded regex is valid"));
static CODE_SPAN_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"`[^`\n]+`").expect("hardcoded regex is valid"));

/// One prose segment for a string: its pieces, with escapes, placeholders and markup blanked.
fn field_segment(src: &str, s: &PoString, st: Styles) -> Option<Segment> {
    if s.value.trim().is_empty() {
        return None;
    }
    let span = s.span();
    let mut seg = Segment::from_ranges(src, span, &s.pieces, SegmentKind::Paragraph);
    let v = &s.value;
    let mut blanks = s.escape_ranges().to_vec();
    blanks.extend(
        placeholders::extract(v, st)
            .iter()
            .map(|p| s.src_range(p.range.clone())),
    );
    for re in [&*TAG_RE, &*CODE_SPAN_RE] {
        blanks.extend(re.find_iter(v).map(|m| s.src_range(m.range())));
    }
    // A Markdown link keeps its text: blank `](url)`.
    for m in MD_LINK_RE.find_iter(v) {
        if let Some(i) = m.as_str().find("](") {
            blanks.push(s.src_range(m.start()..m.start() + 1));
            blanks.push(s.src_range(m.start() + i..m.end()));
        }
    }
    seg.blank_all(&blanks);
    let mut more = crate::extract::markdown::non_prose_ranges(&seg);
    more.extend(crate::segment::noise_ranges(&seg));
    seg.blank_all(&more);
    (!seg.is_blank()).then_some(seg)
}

fn sev(rule: &str) -> Severity {
    super::default_severity(rule).unwrap_or(Severity::Warning)
}

pub fn check(ctx: &FileCtx, out: &mut Out) {
    let Some(po) = &ctx.a.po else {
        return;
    };
    if !ctx.family_enabled("gettext/") {
        return;
    }
    let file = &ctx.a.file;
    let template = is_template(file);

    for e in &po.errors {
        out.push(Finding::new(
            "gettext/syntax",
            sev("gettext/syntax"),
            e.range.clone(),
            e.message.clone(),
        ));
    }
    duplicates(ctx, po, out);
    header(ctx, po, template, out);

    let header_idx = po.header.as_ref().map(|h| h.entry);
    let nplurals = po
        .header
        .as_ref()
        .and_then(|h| h.get("Plural-Forms"))
        .and_then(|f| parse_plural_forms(&f.value).ok())
        .map(|p| p.nplurals);
    let english = language(po).is_some_and(is_english);
    let accel = accelerators(po);

    for (idx, e) in po.entries.iter().enumerate() {
        if e.msgid.is_none() {
            continue;
        }
        if e.obsolete {
            out.push(
                Finding::new(
                    "gettext/obsolete",
                    sev("gettext/obsolete"),
                    e.anchor(),
                    "Obsolete entry",
                )
                .help(
                    "Remove it (msgattrib --no-obsolete) or restore it if the source still uses it",
                ),
            );
            continue;
        }
        if !template && let Some((_, r)) = e.flags.iter().find(|(f, _)| f == "fuzzy") {
            let what = if Some(idx) == header_idx {
                "Header is marked fuzzy, so msgfmt ignores it (including Plural-Forms)"
            } else {
                "Fuzzy translation is not used at runtime"
            };
            out.push(
                Finding::new("gettext/fuzzy", sev("gettext/fuzzy"), r.clone(), what)
                    .help("Review the translation and remove the fuzzy flag"),
            );
        }
        if template || Some(idx) == header_idx {
            continue;
        }
        translation(ctx, e, nplurals, english, accel, out);
    }
}

/// Same (msgctxt, msgid) twice among live entries.
fn duplicates(ctx: &FileCtx, po: &Catalog, out: &mut Out) {
    let mut seen: HashMap<(Option<&str>, &str), usize> = HashMap::new();
    for e in po.entries.iter().filter(|e| !e.obsolete) {
        let Some(m) = &e.msgid else {
            continue;
        };
        let key = (e.msgctxt.as_ref().map(Field::value), m.value());
        if let Some(&first) = seen.get(&key) {
            let (line, _) = ctx.a.file.line_col(first);
            out.push(
                Finding::new(
                    "gettext/duplicate",
                    sev("gettext/duplicate"),
                    m.range(),
                    "Duplicate message definition",
                )
                .help(format!(
                    "First defined on line {line}; msgfmt rejects duplicates"
                )),
            );
        } else {
            seen.insert(key, m.keyword.start);
        }
    }
}

fn header(ctx: &FileCtx, po: &Catalog, template: bool, out: &mut Out) {
    let rule = "gettext/header";
    let has_entries = po.entries.iter().any(|e| e.msgid.is_some() && !e.obsolete);
    let Some(h) = &po.header else {
        if has_entries {
            let first = po.entries.iter().find(|e| e.msgid.is_some());
            let r = first.map_or(0..0, Entry::anchor);
            // Templates without a header (mix gettext.extract) still merge fine.
            let severity = if template {
                Severity::Warning
            } else {
                Severity::Error
            };
            out.push(
                Finding::new(
                    rule,
                    severity,
                    r,
                    "Catalog has no header entry (msgid \"\")",
                )
                .help("Add a header with Content-Type charset, Language and Plural-Forms"),
            );
        }
        return;
    };
    let entry = &po.entries[h.entry];
    let anchor = entry.anchor();
    let header_text = entry.translations().first().map_or("", |f| f.value());
    if h.fields.is_empty() && !header_text.trim().is_empty() {
        out.push(
            Finding::new(rule, Severity::Error, anchor.clone(), "Header is not `Name: value` lines")
                .help("Write fields like \"Language: fi\\n\" \"Content-Type: text/plain; charset=UTF-8\\n\""),
        );
    }
    let at = |name: &str| h.get(name).map_or(anchor.clone(), |f| f.range.clone());
    match (h.get("Content-Type"), h.charset()) {
        // Without a charset GNU tools assume ASCII, which only matters for non-ASCII text.
        (None, _) if ctx.src().is_ascii() => {}
        (None, _) => out.push(
            Finding::new(
                rule,
                Severity::Warning,
                anchor.clone(),
                "Header has no Content-Type",
            )
            .help("Add \"Content-Type: text/plain; charset=UTF-8\\n\"; msgfmt assumes ASCII"),
        ),
        (Some(_), None) => out.push(Finding::new(
            rule,
            Severity::Error,
            at("Content-Type"),
            "Content-Type has no charset",
        )),
        (Some(_), Some(cs)) => {
            let lc = cs.to_ascii_lowercase();
            if lc == "charset" {
                if !template {
                    out.push(
                        Finding::new(
                            rule,
                            Severity::Error,
                            at("Content-Type"),
                            "charset is still the template placeholder CHARSET",
                        )
                        .suggest("UTF-8"),
                    );
                }
            } else if lc != "utf-8" && lc != "utf8" {
                out.push(
                    Finding::new(
                        rule,
                        Severity::Warning,
                        at("Content-Type"),
                        format!("charset {cs} is not UTF-8"),
                    )
                    .suggest("UTF-8"),
                );
            }
        }
    }
    let live = po.entries.iter().filter(|e| !e.obsolete);
    let has_plurals = live.clone().any(|e| e.msgid_plural.is_some());
    match h.get("Plural-Forms") {
        Some(f) if template && f.value.contains("INTEGER") => {}
        Some(f) => match parse_plural_forms(&f.value) {
            Err(msg) => out.push(Finding::new(
                rule,
                Severity::Error,
                f.range.clone(),
                format!("Invalid Plural-Forms: {msg}"),
            )),
            Ok(p) => {
                let bad = (0..=1000u64).find_map(|n| match p.index(n) {
                    None => Some(format!("plural expression divides by zero for n = {n}")),
                    Some(i) if i >= p.nplurals as u64 => Some(format!(
                        "plural expression gives form {i} for n = {n}, but nplurals = {}",
                        p.nplurals
                    )),
                    _ => None,
                });
                if let Some(msg) = bad {
                    out.push(Finding::new(rule, Severity::Error, f.range.clone(), msg));
                }
            }
        },
        None if has_plurals && !template => out.push(
            Finding::new(
                rule,
                Severity::Warning,
                anchor.clone(),
                "Catalog has plural entries but no Plural-Forms header",
            )
            .help("Add e.g. \"Plural-Forms: nplurals=2; plural=(n != 1);\\n\""),
        ),
        None => {}
    }
    if template {
        return;
    }
    match language(po) {
        None => out.push(
            Finding::new(
                rule,
                Severity::Warning,
                at("Language"),
                "Header has no Language",
            )
            .help("Add e.g. \"Language: fi\\n\""),
        ),
        Some(lang) => {
            if let Some(p) = path_locale(&ctx.a.file.rel).or_else(|| path_locale(&ctx.a.file.path))
                && !same_locale(lang, &p)
            {
                out.push(Finding::new(
                    rule,
                    Severity::Warning,
                    at("Language"),
                    format!("Language {lang} does not match the path locale {p}"),
                ));
            }
        }
    }
}

static LOCALE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[a-z]{2,3}(?:[_-](?:[A-Za-z]{4}|[A-Z]{2}|\d{3}))*(?:@\w+)?$")
        .expect("hardcoded regex is valid")
});

/// Locale named by the path: `fi/LC_MESSAGES/x.po`, `fi.po`, `fi_FI.po`, or `fi/messages.po`.
fn path_locale(p: &Path) -> Option<String> {
    let comps: Vec<&str> = p.iter().filter_map(|c| c.to_str()).collect();
    if let Some(i) = comps.iter().position(|c| *c == "LC_MESSAGES")
        && i > 0
    {
        return Some(comps[i - 1].to_string());
    }
    let stem = p.file_stem()?.to_str()?;
    if LOCALE_RE.is_match(stem) {
        return Some(stem.to_string());
    }
    let parent = p.parent()?.file_name()?.to_str()?;
    LOCALE_RE.is_match(parent).then(|| parent.to_string())
}

/// `fi` matches `fi_FI`; regions must agree when both have one.
fn same_locale(a: &str, b: &str) -> bool {
    let parts = |s: &str| -> Vec<String> {
        s.split('@')
            .next()
            .unwrap_or("")
            .split(['_', '-'])
            .map(str::to_ascii_lowercase)
            .collect()
    };
    let (a, b) = (parts(a), parts(b));
    a[0] == b[0] && a.iter().skip(1).zip(b.iter().skip(1)).all(|(x, y)| x == y)
}

#[derive(Clone, Copy, Default)]
struct Accel {
    amp: bool,
    underscore: bool,
}

static AMP_ACCEL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:^|[^&])&([\p{L}\p{N}][\p{L}\p{N}]*;?)").expect("hardcoded regex is valid")
});
static UNDERSCORE_ACCEL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?:^|[\s(])_[\p{L}\p{N}]").expect("hardcoded regex is valid"));

/// `&F` accelerators (not `&amp;` entities or `&&`).
fn amp_count(s: &str) -> usize {
    AMP_ACCEL
        .captures_iter(s)
        .filter(|c| !c[1].ends_with(';'))
        .count()
}

fn underscore_count(s: &str) -> usize {
    UNDERSCORE_ACCEL.find_iter(s).count()
}

/// Whether the catalog uses `&` or `_` accelerators (in at least 3 msgids).
fn accelerators(po: &Catalog) -> Accel {
    let msgids = || {
        po.entries
            .iter()
            .filter(|e| !e.obsolete && !e.is_header())
            .map(Entry::msgid_str)
    };
    Accel {
        amp: msgids().filter(|m| amp_count(m) == 1).count() >= 3,
        underscore: msgids().filter(|m| underscore_count(m) == 1).count() >= 3,
    }
}

/// Checks comparing an entry's translations with its msgid(s).
fn translation(
    ctx: &FileCtx,
    e: &Entry,
    nplurals: Option<usize>,
    english: bool,
    accel: Accel,
    out: &mut Out,
) {
    let msgid = e.msgid_str();
    let plural = e.msgid_plural.as_ref().map(Field::value);
    let translations = e.translations();
    if e.is_untranslated() {
        // An English catalog with source-text msgids falls back to the msgid.
        if english && !is_key(msgid) {
            return;
        }
        out.push(Finding::new(
            "gettext/untranslated",
            sev("gettext/untranslated"),
            e.anchor(),
            "Untranslated message",
        ));
        return;
    }
    if plural.is_some() {
        for (i, f) in &e.msgstr_n {
            if f.value().is_empty() {
                out.push(Finding::new(
                    "gettext/untranslated",
                    sev("gettext/untranslated"),
                    f.range(),
                    format!("Plural form msgstr[{i}] is empty"),
                ));
            }
        }
        if let Some(n) = nplurals
            && e.msgstr_n.len() != n
        {
            let first = e
                .msgstr_n
                .first()
                .map_or(e.anchor(), |(_, f)| f.keyword.clone());
            out.push(
                Finding::new(
                    "gettext/plural-count",
                    sev("gettext/plural-count"),
                    first,
                    format!(
                        "{} plural forms, but the header's Plural-Forms has nplurals={n}",
                        e.msgstr_n.len()
                    ),
                )
                .help(format!(
                    "Provide msgstr[0] to msgstr[{}]",
                    n.saturating_sub(1)
                )),
            );
        }
    }
    // Key-based catalogs (`auth.title`) have nothing to compare the translation with; fuzzy
    // entries are already reported and unused at runtime.
    if is_key(msgid) || e.is_fuzzy() {
        return;
    }
    let st = placeholders::styles(e);
    if st.any() && ctx.enabled("gettext/placeholders") || ctx.enabled("gettext/plural-placeholder")
    {
        placeholder_check(e, st, nplurals, out);
    }
    for (i, f) in translations.iter().enumerate() {
        let v = f.value();
        if v.is_empty() {
            continue;
        }
        let source = if i > 0 {
            plural.unwrap_or(msgid)
        } else {
            msgid
        };
        markup(f, source, out);
        whitespace(f, source, out);
        punctuation(f, source, out);
        accelerator(f, source, accel, out);
        if !english && (v == msgid || Some(v) == plural) && long_prose(v, st) {
            out.push(
                Finding::new(
                    "gettext/same-as-source",
                    sev("gettext/same-as-source"),
                    f.range(),
                    "Translation is identical to the source text",
                )
                .help("Translate it, or leave msgstr empty so the source is used as a fallback"),
            );
        }
    }
}

fn placeholder_check(e: &Entry, st: Styles, nplurals: Option<usize>, out: &mut Out) {
    let msgid_ph = placeholders::extract(e.msgid_str(), st);
    let plural_ph = e
        .msgid_plural
        .as_ref()
        .map(|f| placeholders::extract(f.value(), st))
        .unwrap_or_default();
    let singular: BTreeSet<String> = keys(&msgid_ph);
    let plural_keys: BTreeSet<String> = keys(&plural_ph);
    let all: BTreeSet<String> = singular.union(&plural_keys).cloned().collect();
    // Count variables: in msgid_plural only, or named like a count.
    let count: BTreeSet<String> = all
        .iter()
        .filter(|k| {
            (plural_keys.contains(*k) && !singular.contains(*k))
                || is_count_name(k)
                || (plural_keys.len() == 1 && k.ends_with('d') && k.starts_with("%1$"))
        })
        .cloned()
        .collect();
    let is_plural = e.msgid_plural.is_some();
    for f in e.translations() {
        if f.value().is_empty() {
            continue;
        }
        let ph = placeholders::extract(f.value(), st);
        let got = keys(&ph);
        let expected = if is_plural { &all } else { &singular };
        let missing: Vec<&String> = expected.difference(&got).collect();
        for p in &ph {
            if !all.contains(&p.key) {
                out.push(
                    Finding::new(
                        "gettext/placeholders",
                        sev("gettext/placeholders"),
                        f.s.src_range(p.range.clone()),
                        format!("Placeholder {} is not in the source text", show(&p.key)),
                    )
                    .help(source_list(&all)),
                );
            }
        }
        if missing.is_empty() {
            continue;
        }
        let only_count =
            is_plural && nplurals.unwrap_or(2) > 1 && missing.iter().all(|m| count.contains(*m));
        let rule = if only_count {
            "gettext/plural-placeholder"
        } else {
            "gettext/placeholders"
        };
        let names: Vec<String> = missing.iter().map(|m| show(m)).collect();
        out.push(
            Finding::new(
                rule,
                sev(rule),
                f.range(),
                format!("Translation is missing placeholder {}", names.join(", ")),
            )
            .help(source_list(&all)),
        );
    }
}

/// A key as the user wrote it: printf argument keys (`%2$d`) read as `%d` (argument 2).
fn show(key: &str) -> String {
    if let Some(rest) = key.strip_prefix('%')
        && let Some((n, conv)) = rest.split_once('$')
        && n.chars().all(|c| c.is_ascii_digit())
    {
        return format!("%{conv} (argument {n})");
    }
    key.to_string()
}

fn source_list(all: &BTreeSet<String>) -> String {
    if all.is_empty() {
        return "The source text has no placeholders".into();
    }
    let v: Vec<String> = all.iter().map(|k| show(k)).collect();
    format!("Source placeholders: {}", v.join(", "))
}

fn tags(s: &str) -> Vec<String> {
    let mut v: Vec<String> = TAG_RE
        .captures_iter(s)
        .map(|c| {
            let close = c[0].starts_with("</");
            format!(
                "{}{}",
                if close { "/" } else { "" },
                c[1].to_ascii_lowercase()
            )
        })
        .collect();
    v.sort();
    v
}

fn markup(f: &Field, source: &str, out: &mut Out) {
    let v = f.value();
    let (a, b) = (tags(source), tags(v));
    let mut problems = Vec::new();
    if a != b {
        let fmt = |t: &[String]| {
            if t.is_empty() {
                "none".to_string()
            } else {
                t.iter()
                    .map(|t| format!("<{t}>"))
                    .collect::<Vec<_>>()
                    .join(" ")
            }
        };
        problems.push(format!(
            "tags differ (source: {}, translation: {})",
            fmt(&a),
            fmt(&b)
        ));
    }
    let count = |re: &Regex, s: &str| re.find_iter(s).count();
    let (la, lb) = (count(&MD_LINK_RE, source), count(&MD_LINK_RE, v));
    if la != lb {
        problems.push(format!(
            "{la} Markdown link(s) in the source, {lb} in the translation"
        ));
    }
    let (ca, cb) = (count(&CODE_SPAN_RE, source), count(&CODE_SPAN_RE, v));
    if ca != cb {
        problems.push(format!(
            "{ca} code span(s) in the source, {cb} in the translation"
        ));
    }
    if !problems.is_empty() {
        out.push(Finding::new(
            "gettext/markup",
            sev("gettext/markup"),
            f.range(),
            format!("Markup mismatch: {}", problems.join("; ")),
        ));
    }
}

fn whitespace(f: &Field, source: &str, out: &mut Out) {
    let v = f.value();
    type Test = fn(&str) -> bool;
    let checks: [(&str, Test); 4] = [
        ("start with a newline", |s| s.starts_with('\n')),
        ("end with a newline", |s| s.ends_with('\n')),
        ("start with a space", |s| s.starts_with([' ', '\t'])),
        ("end with a space", |s| s.ends_with([' ', '\t'])),
    ];
    for (what, test) in checks {
        let (a, b) = (test(source), test(v));
        if a != b {
            let msg = if a {
                format!("Source text and translation do not both {what}: the translation does not")
            } else {
                format!("Source text and translation do not both {what}: only the translation does")
            };
            out.push(Finding::new(
                "gettext/whitespace",
                sev("gettext/whitespace"),
                f.range(),
                msg,
            ));
        }
    }
}

/// Final punctuation class: `.`, `:`, `?`, `!` or `…`, mapping full-width and script forms.
fn end_punct(s: &str) -> Option<char> {
    let t = s.trim_end();
    if t.ends_with("...") || t.ends_with('…') {
        return Some('…');
    }
    match t.chars().last()? {
        '.' | '。' | '．' | '।' | '։' | '።' => Some('.'),
        ':' | '：' => Some(':'),
        '?' | '？' | '؟' | ';' | '\u{37e}' => Some('?'),
        '!' | '！' => Some('!'),
        _ => None,
    }
}

fn punctuation(f: &Field, source: &str, out: &mut Out) {
    let (a, b) = (end_punct(source), end_punct(f.value()));
    // Greek uses `;` as a question mark; elsewhere `;` is not end punctuation.
    let a = a.filter(|_| !source.trim_end().ends_with(';'));
    let b = if f.value().trim_end().ends_with(';') && a != Some('?') {
        None
    } else {
        b
    };
    if a == b {
        return;
    }
    let show = |c: Option<char>| c.map_or("no punctuation".to_string(), |c| format!("`{c}`"));
    out.push(Finding::new(
        "gettext/punctuation",
        sev("gettext/punctuation"),
        f.range(),
        format!(
            "Source text ends with {}, the translation with {}",
            show(a),
            show(b)
        ),
    ));
}

fn accelerator(f: &Field, source: &str, accel: Accel, out: &mut Out) {
    let v = f.value();
    let mut check = |on: bool, count: fn(&str) -> usize, mark: &str| {
        if !on {
            return;
        }
        let (a, b) = (count(source), count(v));
        if a != b {
            out.push(Finding::new(
                "gettext/accelerator",
                sev("gettext/accelerator"),
                f.range(),
                format!("{a} `{mark}` accelerator(s) in the source text, {b} in the translation"),
            ));
        }
    };
    check(accel.amp, amp_count, "&");
    check(accel.underscore, underscore_count, "_");
}

/// Enough prose to expect a translation: 3+ words, 15+ letters, not all capitalized words
/// (brand or product names), once placeholders and markup are removed.
fn long_prose(s: &str, st: Styles) -> bool {
    let mut t = s.to_string();
    let mut ranges: Vec<Range<usize>> = placeholders::extract(s, st)
        .into_iter()
        .map(|p| p.range)
        .collect();
    ranges.extend(TAG_RE.find_iter(s).map(|m| m.range()));
    ranges.sort_by_key(|r| std::cmp::Reverse(r.start));
    for r in ranges {
        if t.is_char_boundary(r.start) && t.is_char_boundary(r.end) && r.end <= t.len() {
            t.replace_range(r, " ");
        }
    }
    let words: Vec<&str> = t
        .split_whitespace()
        .filter(|w| w.chars().any(char::is_alphabetic))
        .collect();
    let letters = t.chars().filter(|c| c.is_alphabetic()).count();
    words.len() >= 3
        && letters >= 15
        && !t.contains("://")
        && !words.iter().all(|w| {
            w.chars()
                .find(|c| c.is_alphabetic())
                .is_some_and(char::is_uppercase)
        })
}
