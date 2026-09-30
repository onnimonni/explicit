// explicit-disable-file spelling grammar/* prose/* slop/* -- Finnish and Swedish samples
//! Language detection for prose: foreign-language stretches inside English files (whole
//! segments such as a table cell, list item or paragraph, and phrases between sentence
//! punctuation, quotes, brackets, table pipes and line breaks), Finnish vs Swedish, English
//! stretches inside Finnish or Swedish files, and whole Finnish or Swedish documents.
//! English-only findings (spelling, grammar, prose, slop) inside foreign stretches are dropped;
//! the engine checks Finnish and Swedish stretches with their own spellers.
//!
//! Two cheap detectors: a generic one (few English stopwords, many non-ASCII words), and one
//! for Finnish and Swedish, which are common in Nordic projects' requirement matrices: mostly
//! words unknown to the English dictionary plus Finnish/Swedish function words or Finnish
//! morphology. A single foreign word in an English sentence is left to spelling.

use std::ops::Range;

use crate::rules::Out;
use crate::segment::{Segment, SegmentKind};

/// Common English function words; they make up roughly a quarter of English prose.
pub const STOPWORDS: &[&str] = &[
    "the", "and", "of", "to", "is", "that", "for", "with", "it", "as", "are", "be", "this", "was",
    "by", "an", "or", "from", "at", "not", "you", "we", "can", "will", "have", "has", "if",
    "which", "your", "they", "their", "these", "when", "than", "but", "into", "its", "been",
    "were", "a",
];

/// Finnish and Swedish function words (lowercase): Finnish up to `yms`, then Swedish.
const NORDIC_FUNCTION_WORDS: &[&str] = &[
    // Finnish
    "ja", "tai", "ei", "on", "ovat", "jos", "että", "kun", "sekä", "myös", "onko", "voi", "ole",
    "oli", "olla", "eikä", "mutta", "kuin", "joka", "jotka", "jossa", "joissa", "tämä", "nämä",
    "se", "sen", "ne", "niiden", "vain", "eli", "tulee", "mukaan", "kanssa", "ennen", "jälkeen",
    "osin", "heti", "vai", "niin", "siitä", "voidaan", "esim", "mm", "yms", // Swedish
    "och", "eller", "inte", "är", "som", "för", "med", "att", "det", "ett", "av", "på", "till",
    "har", "ska", "skall", "vid", "från", "också", "när", "utan", "enligt", "samt",
];

/// Index of the first Swedish word in [`NORDIC_FUNCTION_WORDS`].
fn swedish_start() -> usize {
    NORDIC_FUNCTION_WORDS
        .iter()
        .position(|w| *w == "och")
        .unwrap_or(NORDIC_FUNCTION_WORDS.len())
}

/// More Swedish function words for telling Swedish from Finnish.
const SWEDISH_WORDS: &[&str] = &[
    "den", "de", "en", "om", "kan", "vi", "jag", "men", "så", "sig", "sin", "sina", "denna",
    "detta", "dessa", "vara", "blir", "bli", "var", "där", "hur", "vad", "alla", "eller", "även",
    "måste", "får", "finns", "under", "efter", "mellan", "genom", "hos", "mot", "inom", "utan",
];

/// Swedish endings (`-tion`, `-het`, definite plurals).
const SWEDISH_ENDINGS: &[&str] = &[
    "tion", "het", "arna", "erna", "orna", "ande", "ende", "ning", "ingen", "ligt", "iska",
];

/// `fi` or `sv` for text that [`looks_nordic`]: function words, `å`, Finnish case endings and
/// double vowels against Swedish endings.
pub fn nordic_language(text: &str) -> &'static str {
    let (fi, sv) = nordic_scores(text);
    if sv > fi { "sv" } else { "fi" }
}

/// Finnish and Swedish evidence in `text`: two points per function word or word with `å`,
/// one per Finnish case ending, double vowel or Swedish ending.
pub fn nordic_scores(text: &str) -> (usize, usize) {
    let sv_from = swedish_start();
    let (mut fi, mut sv) = (0usize, 0usize);
    for w in words(text) {
        let l = w.to_lowercase();
        // `on`, `se`, `man`: English words too, no evidence.
        if english_known(w) || english_known(&l) {
            continue;
        }
        if let Some(i) = NORDIC_FUNCTION_WORDS.iter().position(|f| *f == l) {
            if i < sv_from {
                fi += 2;
            } else {
                sv += 2;
            }
        }
        if SWEDISH_WORDS.contains(&l.as_str()) {
            sv += 2;
        }
        if l.contains('å') {
            sv += 2;
        }
        if l.chars().count() >= 6 && FINNISH_ENDINGS.iter().any(|e| l.ends_with(e)) {
            fi += 1;
        }
        if ["aa", "ii", "uu", "yy", "ää", "öö", "ee", "oo"]
            .iter()
            .any(|d| l.contains(d))
        {
            fi += 1;
        }
        if l.chars().count() >= 6 && SWEDISH_ENDINGS.iter().any(|e| l.ends_with(e)) {
            sv += 1;
        }
    }
    (fi, sv)
}

/// The language of `text` when its evidence clearly points one way: at least four points and
/// twice the other language's.
pub fn decisive_nordic(text: &str) -> Option<&'static str> {
    let (fi, sv) = nordic_scores(text);
    if fi >= 4 && fi >= 2 * sv {
        Some("fi")
    } else if sv >= 4 && sv >= 2 * fi {
        Some("sv")
    } else {
        None
    }
}

/// Stretches whose evidence clearly points to another supported language.
/// Nordic crossover keeps its existing threshold; other files require longer Nordic evidence.
pub fn other_language_ranges(
    segments: &[Segment],
    lang: &str,
) -> Vec<(Range<usize>, &'static str)> {
    let detect = |text: &str| {
        language_hint(text, 6).or_else(|| {
            (matches!(lang, "fi" | "sv") || looks_nordic(text, 8, true))
                .then(|| decisive_nordic(text))
                .flatten()
        })
    };
    let mut out = Vec::new();
    for s in segments {
        match detect(&s.text) {
            Some(l) if l != lang => {
                // A mixed segment (a Finnish quote next to a Swedish one) splits by phrase.
                for r in phrases(&s.text) {
                    let own = detect(&s.text[r.clone()]).unwrap_or(l);
                    if own != lang {
                        out.push((s.abs(r), own));
                    }
                }
            }
            _ => out.extend(phrases(&s.text).filter_map(|r| {
                detect(&s.text[r.clone()])
                    .filter(|l| *l != lang)
                    .map(|l| (s.abs(r), l))
            })),
        }
    }
    out
}

/// Finnish case and verb endings, checked on words unknown to English.
const FINNISH_ENDINGS: &[&str] = &[
    "ssa", "ssä", "sta", "stä", "lla", "llä", "lta", "ltä", "ksi", "nen", "isen", "uus", "yys",
    "iin", "aan", "ään", "een", "jen", "ien", "tta", "ttä", "vat", "vät", "taan", "tään",
];

fn words(text: &str) -> impl Iterator<Item = &str> {
    text.split(|c: char| !c.is_alphabetic() && c != '\'')
        .map(|w| w.trim_matches('\''))
        .filter(|w| !w.is_empty())
}

fn eq_lower(word: &str, lower: &str) -> bool {
    word.chars().flat_map(char::to_lowercase).eq(lower.chars())
}

fn is_stopword(w: &str) -> bool {
    STOPWORDS.iter().any(|s| s.eq_ignore_ascii_case(w))
}

fn is_nordic_function_word(w: &str) -> bool {
    w.chars().count() <= 6 && NORDIC_FUNCTION_WORDS.iter().any(|f| eq_lower(w, f))
}

/// Function words shared with another supported language cannot establish English crossover.
fn shared_function_word(w: &str) -> bool {
    [
        "a", "an", "as", "or", "be", "by", "all", "do", "no", "me", "so", "per", "via",
    ]
    .iter()
    .any(|word| word.eq_ignore_ascii_case(w))
}

/// Strong Finnish/Swedish spelling: `ä ö å`, or a Finnish ending on a longer word.
fn nordic_letters_or_ending(w: &str) -> bool {
    w.contains(['ä', 'ö', 'å', 'Ä', 'Ö', 'Å'])
        || (w.len() >= 6 && FINNISH_ENDINGS.iter().any(|e| w.ends_with(e)))
}

/// Finnish/Swedish-looking spelling: strong markers or a double vowel English rarely has.
fn nordic_morphology(w: &str) -> bool {
    nordic_letters_or_ending(w) || ["aa", "ii", "uu", "yy"].iter().any(|d| w.contains(d))
}

/// Single letters and acronyms (`A`, `B`, `THL`): class names and codes in either language.
fn neutral(w: &str) -> bool {
    w.chars().nth(1).is_none() || (w.is_ascii() && !w.bytes().any(|b| b.is_ascii_lowercase()))
}

fn english_known(w: &str) -> bool {
    crate::rules::words::words().contains(w)
}

/// `w` or its American spelling is an English word: `programme`, `colour`, `centre`,
/// `organisation`, `analyse`, `licence`, `travelled`.
fn english_word(w: &str) -> bool {
    if english_known(w) || english_known(&w.to_lowercase()) {
        return true;
    }
    let l = w.to_lowercase();
    if !l.is_ascii() || l.len() < 5 {
        return false;
    }
    let swaps: &[(&str, &str)] = &[
        ("programme", "program"),
        ("our", "or"),
        ("tre", "ter"),
        ("tres", "ters"),
        ("isation", "ization"),
        ("isations", "izations"),
        ("ise", "ize"),
        ("ised", "ized"),
        ("ises", "izes"),
        ("ising", "izing"),
        ("yse", "yze"),
        ("ysed", "yzed"),
        ("ence", "ense"),
        ("ogue", "og"),
        ("lled", "led"),
        ("lling", "ling"),
        ("ours", "ors"),
        ("oured", "ored"),
    ];
    swaps.iter().any(|(gb, us)| {
        l.strip_suffix(gb)
            .is_some_and(|stem| english_known(&format!("{stem}{us}")))
    }) || l.contains("our") && english_known(&l.replacen("our", "or", 1))
        || l.contains("programme") && english_known(&l.replacen("programme", "program", 1))
}

/// English function words beyond [`STOPWORDS`] that no Finnish or Swedish word spells:
/// evidence for a short English sentence (`Please ask your manager.`).
const ENGLISH_FUNCTION_WORDS: &[&str] = &[
    "please", "our", "all", "do", "does", "did", "should", "would", "could", "must", "every",
    "each", "there", "here", "how", "what", "who", "why", "where", "about", "after", "before",
    "because", "them", "he", "she", "his", "her", "us", "any", "some", "no", "only", "just",
    "also", "then", "more", "most", "such", "other", "my", "me", "him", "via", "per", "may",
    "might", "shall", "so", "out", "up", "over", "under", "through", "without", "within",
];

/// At least `min_words` words, under 5% English stopwords and at least one in `ascii_div`
/// words with non-ASCII letters.
pub fn looks_foreign(text: &str, min_words: usize, ascii_div: usize) -> bool {
    let mut n = 0;
    let mut stop = 0;
    let mut non_ascii = 0;
    for w in words(text) {
        n += 1;
        if !w.is_ascii() {
            non_ascii += 1;
        } else if is_stopword(w) {
            stop += 1;
        }
    }
    n >= min_words && stop * 20 < n && non_ascii * ascii_div >= n
}

/// Finnish or Swedish text: at least `min_words` words (single letters and acronyms aside),
/// mostly unknown to English, few English stopwords, and a Finnish/Swedish function word that
/// is not also English, two words with Finnish morphology, or one in a longer, 75% foreign
/// span. A one-word span needs `ä ö å` or a Finnish ending. `whole` (a whole segment, which may
/// mix an English sentence with a Finnish one) needs 80% foreign words instead of a majority.
pub fn looks_nordic(text: &str, min_words: usize, whole: bool) -> bool {
    // Cheap pass without dictionary lookups: bail out unless there is some marker.
    if !words(text).any(|w| is_nordic_function_word(w) || nordic_morphology(w)) {
        return false;
    }
    let (mut n, mut stop, mut foreign, mut function, mut morph) = (0, 0, 0, 0, 0);
    let mut single_strong = false;
    for w in words(text).filter(|w| !neutral(w)) {
        n += 1;
        if w.is_ascii() && is_stopword(w) {
            stop += 1;
        } else if english_known(w) {
            // "on", "se", "det": Finnish or Swedish too, but no evidence on their own.
            foreign += usize::from(is_nordic_function_word(w));
        } else {
            foreign += 1;
            if is_nordic_function_word(w) {
                function += 1;
            } else if nordic_morphology(w) {
                morph += 1;
                single_strong = nordic_letters_or_ending(w);
            }
        }
    }
    if n < min_words.max(1) || stop * 4 >= n {
        return false;
    }
    let mostly = if whole {
        foreign * 5 >= n * 4
    } else {
        foreign * 2 > n
    };
    let evidence = function > 0
        || morph >= 2
        || (n == 1 && single_strong)
        || (morph == 1 && n >= 3 && foreign * 4 >= n * 3);
    mostly && evidence
}

/// Words of `text` with their byte offsets.
fn words_at(text: &str) -> impl Iterator<Item = (usize, &str)> {
    let mut rest = text.char_indices().peekable();
    std::iter::from_fn(move || {
        let word = |c: char| c.is_alphabetic() || c == '\'';
        while rest.next_if(|&(_, c)| !word(c)).is_some() {}
        let (start, _) = *rest.peek()?;
        let mut end = start;
        while let Some((i, c)) = rest.next_if(|&(_, c)| word(c)) {
            end = i + c.len_utf8();
        }
        let w = &text[start..end];
        let t = w.trim_matches('\'');
        let lead = w.len() - w.trim_start_matches('\'').len();
        Some((start + lead, t))
    })
    .filter(|(_, w)| !w.is_empty())
}

/// Runs of Finnish/Swedish words inside English text, e.g. a quoted title: consecutive words
/// unknown to English (single letters and acronyms in between are neutral) with a function word,
/// two words with Finnish morphology, or three or more words with one. Runs end at sentence
/// punctuation, table pipes, brackets and quotes. Local byte ranges.
fn nordic_runs(text: &str) -> Vec<Range<usize>> {
    // Cheap pre-check: no function word and no morphology marker, no run.
    if !words(text).any(|w| is_nordic_function_word(w) || nordic_morphology(w)) {
        return Vec::new();
    }
    #[derive(Default)]
    struct Run {
        range: Option<Range<usize>>,
        words: usize,
        function: usize,
        morph: usize,
    }
    impl Run {
        fn qualifies(&self) -> bool {
            self.words >= 2
                && (self.function > 0 || self.morph >= 2 || (self.words >= 3 && self.morph > 0))
        }
    }
    let mut out = Vec::new();
    let mut run = Run::default();
    let mut prev_end = 0;
    let flush = |run: &mut Run, out: &mut Vec<Range<usize>>| {
        let done = std::mem::take(run);
        if done.qualifies() {
            out.extend(done.range);
        }
    };
    for (at, w) in words_at(text) {
        if text[prev_end..at].contains(|c: char| ".!?|()[]\"“”«»".contains(c)) {
            flush(&mut run, &mut out);
        }
        prev_end = at + w.len();
        if neutral(w) {
            continue;
        }
        if english_known(w) {
            flush(&mut run, &mut out);
            continue;
        }
        run.words += 1;
        if is_nordic_function_word(w) {
            run.function += 1;
        } else if nordic_morphology(w) {
            run.morph += 1;
        }
        let start = run.range.as_ref().map_or(at, |r| r.start);
        run.range = Some(start..at + w.len());
    }
    flush(&mut run, &mut out);
    out
}

/// Byte ranges (file offsets) of foreign-language stretches in `segments` of file text `src`.
pub fn foreign_ranges(segments: &[Segment], src: &str) -> Vec<Range<usize>> {
    foreign_stretches(segments, src)
        .into_iter()
        .map(|(r, _)| r)
        .collect()
}

/// Phrases of `text` between sentence punctuation, quotes, brackets, pipes and line breaks,
/// as local byte ranges.
fn phrases(text: &str) -> impl Iterator<Item = Range<usize>> + '_ {
    let mut start = 0;
    text.match_indices(|c: char| ".!?;:|\"“”«»()[]\n".contains(c))
        .map(|(i, m)| (i, i + m.len()))
        .chain([(text.len(), text.len())])
        .map(move |(end, next)| {
            let r = start..end;
            start = next;
            r
        })
}

/// `w` (five letters or more) is one swap, deletion or insertion away from an English word
/// (`recieved`): an English typo rather than a foreign word. Shorter words have English
/// neighbours by chance (`tila`, `til`).
fn english_typo(w: &str) -> bool {
    let c: Vec<char> = w.to_lowercase().chars().collect();
    if c.len() < 5 {
        return false;
    }
    let known = |v: &[char]| english_known(&v.iter().collect::<String>());
    for i in 0..c.len() {
        let mut v = c.clone();
        if i + 1 < c.len() {
            v.swap(i, i + 1);
            if known(&v) {
                return true;
            }
        }
        let mut v = c.clone();
        v.remove(i);
        if c.len() >= 4 && known(&v) {
            return true;
        }
    }
    for i in 0..=c.len() {
        for l in 'a'..='z' {
            let mut v = c.clone();
            v.insert(i, l);
            if known(&v) {
                return true;
            }
        }
    }
    false
}

/// Short table cells (one or two words, all unknown to English) that no detector marks on
/// their own but whose row or column has Finnish or Swedish cells: a `Valmis` or `Kesken`
/// status next to Finnish requirement text. They take the language of those cells (the
/// majority, column before row). File offsets; `src` is the file text the segments index.
pub fn table_cell_stretches(segments: &[Segment], src: &str) -> Vec<(Range<usize>, &'static str)> {
    struct Cell {
        at: usize,
        table: usize,
        line: usize,
        column: usize,
        lang: Option<&'static str>,
        short: bool,
    }
    let mut cells: Vec<Cell> = Vec::new();
    let (mut line, mut scanned) = (0, 0);
    let mut table = 0;
    let mut prev: Option<(usize, usize)> = None; // (segment index, offset) of the previous cell
    for (i, s) in segments.iter().enumerate() {
        if s.kind != SegmentKind::TableCell || s.range.start > src.len() {
            continue;
        }
        let Some(before) = src.get(scanned..s.range.start) else {
            continue;
        };
        line += before.matches('\n').count();
        scanned = s.range.start;
        // A new table after another segment or a blank line.
        let same_table = prev.is_some_and(|(pi, at)| {
            let between: Vec<&str> = src[at..s.range.start].split('\n').collect();
            pi + 1 == i
                && between.len() >= 2
                && between[1..between.len() - 1]
                    .iter()
                    .all(|l| !l.trim().is_empty())
                || pi + 1 == i && between.len() == 1
        });
        if !same_table {
            table += 1;
        }
        prev = Some((i, s.range.start));
        let line_start = src[..s.range.start].rfind('\n').map_or(0, |n| n + 1);
        let head = &src[line_start..s.range.start];
        let column = head
            .char_indices()
            .filter(|&(j, c)| c == '|' && !head[..j].ends_with('\\'))
            .count();
        let lang = looks_nordic(&s.text, 1, true).then(|| nordic_language(&s.text));
        let ws: Vec<&str> = words(&s.text).filter(|w| !neutral(w)).collect();
        let short = lang.is_none()
            && (1..=2).contains(&ws.len())
            && words(&s.text).count() == ws.len()
            && ws.iter().all(|w| {
                w.chars().count() >= 3
                    && !english_known(w)
                    && !english_known(&w.to_lowercase())
                    && !english_typo(w)
            });
        cells.push(Cell {
            at: i,
            table,
            line,
            column,
            lang,
            short,
        });
    }
    // Two rounds: a cell settled in the first (by its row) is evidence for its column in the
    // second (the `Tila` header over `Valmis` and `Kesken`).
    for _ in 0..2 {
        let settled: Vec<(usize, &'static str)> = cells
            .iter()
            .enumerate()
            .filter(|(_, c)| c.short && c.lang.is_none())
            .filter_map(|(i, c)| {
                let vote = |same: &dyn Fn(&Cell) -> bool| -> Option<&'static str> {
                    let (mut fi, mut sv) = (0, 0);
                    for o in cells
                        .iter()
                        .filter(|o| o.table == c.table && o.at != c.at && same(o))
                    {
                        match o.lang {
                            Some("fi") => fi += 1,
                            Some("sv") => sv += 1,
                            _ => {}
                        }
                    }
                    match (fi, sv) {
                        (0, 0) => None,
                        _ if sv > fi => Some("sv"),
                        _ => Some("fi"),
                    }
                };
                vote(&|o| o.column == c.column)
                    .or_else(|| vote(&|o| o.line == c.line))
                    .map(|l| (i, l))
            })
            .collect();
        for (i, l) in settled {
            cells[i].lang = Some(l);
        }
    }
    cells
        .iter()
        .filter(|c| c.short)
        .filter_map(|c| c.lang.map(|l| (segments[c.at].range.clone(), l)))
        .collect()
}

/// Foreign-language stretches of `segments` (file offsets into `src`), tagged `fi` / `sv` when
/// they look Finnish or Swedish and `None` for other languages.
pub fn foreign_stretches(
    segments: &[Segment],
    src: &str,
) -> Vec<(Range<usize>, Option<&'static str>)> {
    let mut foreign: Vec<(Range<usize>, Option<&'static str>)> =
        table_cell_stretches(segments, src)
            .into_iter()
            .map(|(r, l)| (r, Some(l)))
            .collect();
    for s in segments {
        // A table cell is one unit: a one-word Finnish cell counts.
        let min_words = if s.kind == SegmentKind::TableCell {
            1
        } else {
            2
        };
        if let Some(language) = language_hint(&s.text, min_words) {
            foreign.push((s.range.clone(), Some(language)));
            continue;
        }
        if looks_nordic(&s.text, min_words, true) {
            // Phrases take the segment's language unless their own evidence says otherwise
            // (a Swedish quote in a Finnish block quote).
            let whole = nordic_language(&s.text);
            let mut any = false;
            for r in phrases(&s.text) {
                if s.text[r.clone()].trim().is_empty() {
                    continue;
                }
                let l = decisive_nordic(&s.text[r.clone()]).unwrap_or(whole);
                foreign.push((s.abs(r), Some(l)));
                any = true;
            }
            if !any {
                foreign.push((s.range.clone(), Some(whole)));
            }
            continue;
        }
        if looks_foreign(&s.text, 6, 6) {
            foreign.push((s.range.clone(), None));
            continue;
        }
        foreign.extend(
            nordic_runs(&s.text)
                .into_iter()
                .map(|r| (s.abs(r.clone()), Some(nordic_language(&s.text[r])))),
        );
        for r in phrases(&s.text) {
            let phrase = &s.text[r.clone()];
            if let Some(language) = language_hint(phrase, 4) {
                foreign.push((s.abs(r), Some(language)));
            } else if looks_nordic(phrase, 2, false) {
                foreign.push((s.abs(r), Some(nordic_language(phrase))));
            } else if looks_foreign(phrase, 3, 4) {
                foreign.push((s.abs(r), None));
            }
        }
    }
    foreign
}

/// English text inside a Finnish or Swedish file: at least `min_words` words (single letters
/// and acronyms aside), 80% known to English, one in eight an English stopword, and nothing
/// Finnish or Swedish about it.
///
/// A short sentence (up to eight words) also counts with another English function word
/// (`Please ask your manager.`) or, of three words or more, with every word English
/// (`Programme updates arrive weekly.`). British spellings count as English.
pub fn looks_english(text: &str, min_words: usize) -> bool {
    let (mut n, mut stop, mut function, mut known, mut shared) = (0, 0, 0, 0, 0);
    for w in words(text).filter(|w| !neutral(w)) {
        n += 1;
        let ambiguous = shared_function_word(w);
        shared += usize::from(ambiguous);
        if w.is_ascii() && is_stopword(w) {
            stop += usize::from(!ambiguous);
            known += 1;
        } else if english_word(w) && !is_nordic_function_word(w) {
            known += 1;
            function += usize::from(
                !ambiguous && w.is_ascii() && ENGLISH_FUNCTION_WORDS.iter().any(|f| eq_lower(w, f)),
            );
        }
    }
    // One sentence: `Contact support via email. Kiitos.` is two.
    let one_sentence = !text
        .trim_end()
        .trim_end_matches(['.', '!', '?'])
        .contains(['.', '!', '?', '\n']);
    let short = one_sentence && (min_words.max(3)..=8).contains(&n);
    let evidence =
        stop * 8 >= n || short && (function > 0 && known * 5 >= n * 4 || known == n && shared == 0);
    n >= min_words.max(1)
        && known * 5 >= n * 4
        && evidence
        // Dictionary homographs alone do not outweigh a native function-word profile.
        && (stop > 0 || function > 0 || language_hint(text, 4).is_none())
        && !words(text).any(|w| nordic_letters_or_ending(w) && !english_word(w))
}

/// Runs of English words inside a Finnish or Swedish phrase, between commas (`..., että the
/// deployment pipeline is broken again, joten ...`): three or more ASCII words known to English
/// and not Finnish or Swedish function words, one of them an English stopword and two of them
/// longer words. Acronyms and single letters continue a run. Local byte ranges.
fn english_runs(text: &str) -> Vec<Range<usize>> {
    #[derive(Default)]
    struct Run {
        range: Option<Range<usize>>,
        words: usize,
        stop: usize,
        long: usize,
    }
    let mut out = Vec::new();
    let mut run = Run::default();
    let mut prev_end = 0;
    let flush = |run: &mut Run, out: &mut Vec<Range<usize>>| {
        let done = std::mem::take(run);
        if done.words >= 3 && done.stop > 0 && done.long >= 2 {
            out.extend(done.range);
        }
    };
    for (at, w) in words_at(text) {
        if text[prev_end..at].contains(|c: char| ",.!?;:|()[]\"“”«»".contains(c)) {
            flush(&mut run, &mut out);
        }
        prev_end = at + w.len();
        if neutral(w) {
            continue;
        }
        let stop = w.is_ascii() && is_stopword(w) && !shared_function_word(w);
        let english = w.is_ascii() && !is_nordic_function_word(w) && (stop || english_word(w));
        if !english {
            flush(&mut run, &mut out);
            continue;
        }
        run.words += 1;
        run.stop += usize::from(stop);
        run.long += usize::from(!stop && w.len() >= 4);
        let start = run.range.as_ref().map_or(at, |r| r.start);
        run.range = Some(start..at + w.len());
    }
    flush(&mut run, &mut out);
    out
}

/// English stretches of `segments` (file offsets): whole segments or phrases that
/// [`looks_english`], and runs of English words inside other phrases ([`english_runs`]).
pub fn english_ranges(segments: &[Segment]) -> Vec<Range<usize>> {
    let mut out = Vec::new();
    for s in segments {
        let min_words = if s.kind == SegmentKind::TableCell {
            2
        } else {
            3
        };
        if looks_english(&s.text, min_words) {
            out.push(s.range.clone());
            continue;
        }
        for r in phrases(&s.text) {
            if looks_english(&s.text[r.clone()], 3) {
                out.push(s.abs(r));
            } else {
                out.extend(
                    english_runs(&s.text[r.clone()])
                        .into_iter()
                        .map(|x| s.abs(r.start + x.start..r.start + x.end)),
                );
            }
        }
    }
    out
}

/// `fi` / `sv` when the prose of `segments` as a whole is Finnish or Swedish.
pub fn document_nordic(segments: &[Segment]) -> Option<&'static str> {
    let text: String = segments
        .iter()
        .map(|s| s.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    looks_nordic(&text, 20, false).then(|| nordic_language(&text))
}

/// A cheap German, French, Spanish or Portuguese hint from distinctive function words.
/// At least two distinct markers, enough prose, little English evidence and a clear winner
/// are required. Ambiguous and short stretches remain unclassified; no dictionary is loaded.
pub fn language_hint(text: &str, min_words: usize) -> Option<&'static str> {
    language_hint_words(words(text), min_words)
}

pub fn document_hint(segments: &[Segment]) -> Option<&'static str> {
    language_hint_words(segments.iter().flat_map(|s| words(&s.text)), 20)
}

fn language_hint_words<'a>(
    prose: impl Iterator<Item = &'a str>,
    min_words: usize,
) -> Option<&'static str> {
    const PROFILES: &[(&str, &[&str])] = &[
        (
            "de",
            &[
                "der", "die", "das", "und", "nicht", "ist", "eine", "einen", "einer", "wird",
                "werden", "mit", "auch", "sich", "auf", "für", "zum", "zur",
            ],
        ),
        (
            "fr",
            &[
                "et", "le", "les", "une", "des", "dans", "avec", "pour", "est", "sont", "vous",
                "nous", "cette", "cet", "aux", "du", "ne", "il", "elle", "ils", "elles",
            ],
        ),
        (
            "es",
            &[
                "con", "el", "los", "las", "una", "unos", "unas", "del", "hay", "muy", "pero",
                "porque", "también", "puede", "debe", "son", "sus",
            ],
        ),
        (
            "pt",
            &[
                "com",
                "uma",
                "um",
                "não",
                "são",
                "dos",
                "das",
                "pelo",
                "pela",
                "os",
                "ao",
                "aos",
                "você",
                "também",
                "ficheiro",
                "utilizador",
                "tem",
            ],
        ),
    ];
    let mut counts = [0usize; 4];
    let mut seen = [0u32; 4];
    let (mut total, mut english) = (0usize, 0usize);
    for word in prose.filter(|w| !neutral(w)) {
        total += 1;
        english += usize::from(is_stopword(word));
        for (index, (_, markers)) in PROFILES.iter().enumerate() {
            if let Some(marker) = markers.iter().position(|m| eq_lower(word, m)) {
                counts[index] += 1;
                seen[index] |= 1 << marker;
            }
        }
    }
    if total < min_words.max(4) || english * 4 >= total {
        return None;
    }
    let winner = (0..counts.len()).max_by_key(|&i| counts[i])?;
    let runner_up = (0..counts.len())
        .filter(|&i| i != winner)
        .map(|i| counts[i])
        .max()
        .unwrap_or(0);
    (counts[winner] >= 2
        && seen[winner].count_ones() >= 2
        && counts[winner] * 8 >= total
        && counts[winner] >= 2 * runner_up.max(1))
    .then_some(PROFILES[winner].0)
}

/// English-only rule families.
pub fn is_english_rule(rule: &str) -> bool {
    rule == "spelling"
        || ["grammar/", "prose/", "slop/"]
            .iter()
            .any(|p| rule.starts_with(p))
}

/// Drop English-only findings that start inside foreign-language stretches of `segments`
/// (of file text `src`).
pub fn drop_foreign_findings(segments: &[Segment], src: &str, out: &mut Out) {
    if !out.iter().any(|f| is_english_rule(&f.rule)) {
        return;
    }
    let foreign = foreign_ranges(segments, src);
    if !foreign.is_empty() {
        out.retain(|f| {
            !is_english_rule(&f.rule) || !foreign.iter().any(|r| r.contains(&f.range.start))
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::{Finding, Severity};
    use crate::rules::Analyzed;
    use crate::source::{FileKind, SourceFile};
    use std::path::PathBuf;

    #[test]
    fn function_word_hints_leave_english_and_ambiguous_text_alone() {
        for (language, sentence) in [
            (
                "de",
                "Der Bericht wird auf dem Server gespeichert und nicht gelöscht.",
            ),
            (
                "fr",
                "Le rapport est dans le dossier et les données sont disponibles.",
            ),
            (
                "es",
                "El informe contiene los datos pero las imágenes son nuevas.",
            ),
            ("pt", "O ficheiro tem uma imagem e os dados não são novos."),
        ] {
            assert_eq!(language_hint(sentence, 4), Some(language), "{sentence}");
        }
        assert_eq!(
            language_hint("The die is cast and the report is ready.", 4),
            None
        );
        assert_eq!(language_hint("Le de la que para da do.", 4), None);
        assert_eq!(language_hint("Le fichier.", 4), None);
        assert_eq!(language_hint("Der der der der.", 4), None);
    }

    fn md(text: &str) -> Analyzed {
        Analyzed::new(SourceFile::new(
            PathBuf::from("/x/a.md"),
            PathBuf::from("a.md"),
            FileKind::Markdown,
            text.to_string(),
        ))
    }

    /// Words of `text` that keep a spelling finding after foreign stretches are dropped.
    fn kept(text: &str, flagged: &[&str]) -> Vec<String> {
        let a = md(text);
        let mut out: Out = flagged
            .iter()
            .map(|w| {
                let at = text.find(w).unwrap_or_else(|| panic!("{w} not in text"));
                Finding::new("spelling", Severity::Error, at..at + w.len(), "x")
            })
            .collect();
        drop_foreign_findings(&a.segments, text, &mut out);
        out.iter()
            .map(|f| text[f.range.clone()].to_string())
            .collect()
    }

    #[test]
    fn foreign_phrases() {
        assert!(looks_foreign("Tämä on hyvä käyttäjälle", 3, 4));
        assert!(!looks_foreign("The user sees the café menu", 3, 4));
        assert!(!looks_foreign("Hyvä päivä", 3, 4));
    }

    #[test]
    fn quoted_finnish_in_english_paragraph() {
        let en = "This program is a simple tool that helps you manage your files and folders \
            on your computer. You can use it to copy, move and delete files.";
        let text = format!("# T\n\n{en} The label says \"Tämä älä käytä väärin\" and teh end.\n");
        assert_eq!(kept(&text, &["älä", "teh"]), ["teh"]);
        // Other rule families are kept.
        let a = md(&text);
        let at = text.find("älä").unwrap();
        let mut out: Out = vec![Finding::new("md/x", Severity::Error, at..at + 4, "x")];
        drop_foreign_findings(&a.segments, &a.file.text, &mut out);
        assert_eq!(out.len(), 1);
    }

    #[test]
    fn nordic_detection() {
        assert!(looks_nordic(
            "Potilasasiakirja-merkintöjen tai asiakasasiakirjojen tekeminen ja muokkaaminen",
            2,
            false
        ));
        assert!(looks_nordic("voimassa , jos tuottaa asiakirjoja", 2, false));
        assert!(looks_nordic(
            "Uppgifterna ska sparas och inte delas",
            2,
            false
        ));
        assert!(looks_nordic("Voimassa", 1, false));
        assert!(!looks_nordic("Voimassa", 2, false));
        assert!(!looks_nordic("The record is stored in Kanta", 2, false));
        assert!(!looks_nordic("Contact Mäkinen", 2, false));
        assert!(!looks_nordic("Kanta", 1, false));
        assert!(!looks_nordic("The screen shows fifteen results", 2, false));
        assert!(!looks_nordic("on the other hand", 2, false));
    }

    #[test]
    fn english_run_inside_finnish_sentence() {
        let text = "Kokouksessa todettiin, että the deployment pipeline is broken again, joten \
            korjaus siirtyy.";
        let runs = english_runs(text);
        assert_eq!(runs.len(), 1);
        assert_eq!(
            &text[runs[0].clone()],
            "the deployment pipeline is broken again"
        );
        assert!(english_runs("Palvelu on auki ja sauna on lämmin.").is_empty());
    }

    /// Short English sentences and British spellings inside Finnish text are English; Finnish
    /// sentences made of words English also has are not.
    #[test]
    fn short_english_sentences() {
        for en in [
            "Programme updates arrive weekly.",
            "Please ask your manager.",
            "Ask questions early.",
            "The organisation behind the programme is based in Helsinki.",
            "Our colour scheme follows the brand guidelines.",
            "If you have questions, ask the programme manager.",
        ] {
            assert!(looks_english(en, 3), "{en}");
        }
        for fi in [
            "Tarkista aikataulu huolellisesti.",
            "Data on tallessa.",
            "Auto on pihalla.",
            "Se on hyvä.",
            "Kiitos.",
            "Lue ohje ennen asennusta.",
            "Sauna on lämmin ja kala on tuore.",
            "Ota yhteyttä ylläpitoon.",
            "Contact support via email. Kiitos.",
        ] {
            assert!(!looks_english(fi, 3), "{fi}");
        }
        assert!(english_word("programme") && english_word("centre") && english_word("colours"));
        assert!(!english_word("aikataulu"));
    }

    #[test]
    fn table_with_finnish_cells() {
        let text = "# Matrix\n\n\
            | Id | Requirement | Status | Source text |\n\
            |----|-------------|--------|-------------|\n\
            | R1 | Recieve records | done | Tietojen luovutus tai käsittely ennen voimassaoloa |\n\
            | R2 | Store the Kanta referal | open | Voimassa |\n\
            | R3 | Recieve the records safely. Onko palvelujen vaatimukset voimassa jos luokka muuttuu | open | Uppgifterna ska sparas och inte delas |\n";
        let got = kept(
            text,
            &[
                "Recieve",
                "Tietojen",
                "voimassaoloa",
                "Kanta",
                "referal",
                "Voimassa |",
                "Onko",
                "luokka",
                "Uppgifterna",
                "delas",
            ],
        );
        assert_eq!(got, ["Recieve", "Kanta", "referal"]);
        let mixed = text.rfind("Recieve").unwrap();
        let a = md(text);
        let mut out: Out = vec![Finding::new(
            "spelling",
            Severity::Error,
            mixed..mixed + 7,
            "x",
        )];
        drop_foreign_findings(&a.segments, &a.file.text, &mut out);
        assert_eq!(
            out.len(),
            1,
            "English sentence of a mixed cell is still checked"
        );
    }

    #[test]
    fn english_list_item_with_one_finnish_term_and_name() {
        let text = "# T\n\n- The toimikortti reader is requierd for login.\n\
            - Ask Jyrki Mäkinen about the deploy.\n\n\
            Paragraph where Kela and Mäkinen are named with a speling error.\n";
        assert_eq!(
            kept(
                text,
                &["toimikortti", "requierd", "Mäkinen", "Kela", "speling"]
            ),
            ["toimikortti", "requierd", "Mäkinen", "Kela", "speling"]
        );
    }

    /// One- and two-word cells unknown to English take the language of the Finnish or Swedish
    /// cells in their column (or row); English words, other tables and lone cells stay English.
    #[test]
    fn short_cells_next_to_finnish_cells() {
        let text = "# Matrix\n\n\
            | Id | Vaatimus | Tila |\n\
            |----|----------|------|\n\
            | R1 | Potilastiedot tallennetaan salattuna | Valmis |\n\
            | R2 | Lokitiedot säilytetään viisi vuotta | Kesken |\n\
            | R3 | Käyttäjä tunnistetaan vahvasti | done |\n\n\
            | Key | Value |\n\
            |-----|-------|\n\
            | Owner | Valmis |\n\n\
            | Tila | Kuvaus |\n\
            |------|--------|\n\
            | Hyvaksytty | Muutos on hyväksytty ja otetaan käyttöön |\n";
        let a = md(text);
        let got: Vec<(&str, &str)> = table_cell_stretches(&a.segments, text)
            .into_iter()
            .map(|(r, l)| (text[r].trim(), l))
            .collect();
        // `Valmis`, `Kesken` and the header `Vaatimus` by column, `Hyvaksytty` by row, the
        // header `Tila` by the settled cells below it; `done` is English; the `Owner | Valmis`
        // table has no Finnish cells.
        assert_eq!(
            got,
            [
                ("Vaatimus", "fi"),
                ("Tila", "fi"),
                ("Valmis", "fi"),
                ("Kesken", "fi"),
                ("Tila", "fi"),
                ("Kuvaus", "fi"),
                ("Hyvaksytty", "fi")
            ]
        );
        assert_eq!(
            kept(
                text,
                &[
                    "Valmis |\n| R2",
                    "Kesken",
                    "done",
                    "Valmis |\n\n",
                    "Hyvaksytty"
                ]
            ),
            ["done", "Valmis |\n\n"]
        );
    }

    #[test]
    fn shared_function_words_do_not_establish_english_crossover() {
        for text in [
            "Un an de creative coding.",
            "an private cloud",
            "as private clouds",
            "do private cloud",
        ] {
            assert!(!looks_english(text, 3), "{text}");
            assert!(english_runs(text).is_empty(), "{text}");
        }
        for text in [
            "The private cloud has an error.",
            "You should do more.",
            "Our colour scheme follows the brand guidelines.",
        ] {
            assert!(looks_english(text, 3), "{text}");
        }
    }

    #[test]
    fn basic_native_conjunctions_and_prepositions_complete_language_evidence() {
        for (text, language) in [
            (
                "Le service fonctionne et affiche plusieurs informations.",
                "fr",
            ),
            ("El sistema trabaja con datos.", "es"),
            ("Uma instalação funciona com dados.", "pt"),
        ] {
            assert_eq!(language_hint(text, 4), Some(language), "{text}");
        }
        assert_eq!(
            language_hint("The con artist builds a network with tools.", 4),
            None
        );
    }
}
