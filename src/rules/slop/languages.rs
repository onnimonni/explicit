//! Conservative non-English hype: empty promises and stacked promotional claims, not words
//! such as “innovative” or ordinary connective phrases in isolation.
use std::ops::Range;
use std::sync::LazyLock;

use regex::Regex;

use super::{FileCtx, Out};
use crate::diagnostic::Finding;

fn pattern(code: &str) -> Option<&'static Regex> {
    macro_rules! matcher {
        ($name:ident, $pattern:literal) => {
            static $name: LazyLock<Regex> = LazyLock::new(|| {
                Regex::new($pattern).expect("hardcoded multilingual hype regex is valid")
            });
        };
    }
    // Require a promotional superlative plus a second absolute claim nearby. A lone
    // descriptive adjective or an ordinary connective is not evidence of slop.
    matcher!(
        DE,
        r"(?i)\b(?:(?:revolutionär|bahnbrechend|einzigartig)(?:e[nmrs]?)?\b[^.!?]{0,80}\b(?:bahnbrechend(?:e[nmrs]?)?|beispiellos(?:e[nmrs]?)?|alles[ \t\r\n]+verändernd(?:e[nmrs]?)?|unbegrenzte[ \t\r\n]+möglichkeiten)|welt[ \t\r\n]+(?:voller|unbegrenzter|endloser)[ \t\r\n]+möglichkeiten)\b"
    );
    matcher!(
        FR,
        r"(?i)\b(?:(?:révolutionnaire|miraculeu(?:x|se)|inégalée?s?)\b[^.!?]{0,80}\b(?:sans[ \t\r\n]+(?:égal|précédent)|possibilités[ \t\r\n]+(?:infinies|illimitées))|monde[ \t\r\n]+de[ \t\r\n]+possibilités[ \t\r\n]+(?:infinies|illimitées))\b"
    );
    matcher!(
        ES,
        r"(?i)\b(?:(?:revolucionari[oa]s?|revolución|incomparables?|definitiv[oa]s?)\b[^.!?]{0,80}\b(?:incomparable|transforma[ \t\r\n]+todo|sin[ \t\r\n]+(?:igual|precedentes|límites))|posibilidades[ \t\r\n]+(?:ilimitadas|infinitas)[ \t\r\n]+y[ \t\r\n]+(?:excelencia|perfección)[ \t\r\n]+sin[ \t\r\n]+límites)\b"
    );
    matcher!(
        PT,
        r"(?i)\b(?:(?:revolucionári[oa]s?|milagros[oa]s?|incomparáveis)\b[^.!?]{0,80}\b(?:redefine[ \t\r\n]+tudo|sem[ \t\r\n]+(?:precedentes|limites))|possibilidades[ \t\r\n]+(?:infinitas|ilimitadas)[ \t\r\n]+e[ \t\r\n]+(?:excelência|perfeição)[ \t\r\n]+sem[ \t\r\n]+limites)\b"
    );
    matcher!(
        FI,
        r"(?i)\b(?:mullistaa[ \t\r\n]+kaiken|vallankumouksellinen[ \t\r\n]+ja[ \t\r\n]+ennennäkemätön)\b"
    );
    matcher!(
        SV,
        r"(?i)\b(?:revolutionerar[ \t\r\n]+allt|revolutionerande[ \t\r\n]+och[ \t\r\n]+utan[ \t\r\n]+motstycke)\b"
    );
    match code {
        "de" => Some(&DE),
        "fr" => Some(&FR),
        "es" => Some(&ES),
        "pt" => Some(&PT),
        "fi" => Some(&FI),
        "sv" => Some(&SV),
        _ => None,
    }
}

/// Run only within this language's prose ranges; code and other languages stay masked.
pub fn check(ctx: &FileCtx, code: &str, ranges: Option<&[Range<usize>]>, out: &mut Out) {
    if !ctx.enabled("slop/phrase") {
        return;
    }
    let Some(re) = pattern(code) else { return };
    for segment in &ctx.a.segments {
        let check = |range: Range<usize>, out: &mut Out| {
            let start = range.start.max(segment.range.start);
            let end = range.end.min(segment.range.end);
            if start >= end {
                return;
            }
            let Some(text) = segment
                .text
                .get(start - segment.range.start..end - segment.range.start)
            else {
                return;
            };
            let emit = |text: &str, origin: usize, out: &mut Out| {
                for matched in re.find_iter(text) {
                    let finding = Finding::new(
                        "slop/phrase",
                        super::seg_sev("slop/phrase", segment),
                        origin + matched.start()..origin + matched.end(),
                        "Promotional claim without concrete evidence",
                    )
                    .help("Replace the promise with a specific capability, result or number.");
                    out.push(finding);
                }
            };
            // Soft wraps belong to a paragraph; blank lines separate claims even in comments.
            let mut paragraph = 0;
            let mut offset = 0;
            for line in text.split_inclusive('\n') {
                if line.trim().is_empty() {
                    if paragraph < offset {
                        emit(&text[paragraph..offset], start + paragraph, out);
                    }
                    paragraph = offset + line.len();
                }
                offset += line.len();
            }
            if paragraph < text.len() {
                emit(&text[paragraph..], start + paragraph, out);
            }
        };
        if let Some(ranges) = ranges {
            for range in ranges {
                check(range.clone(), out);
            }
        } else {
            check(segment.range.clone(), out);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn isolated_descriptive_words_and_literal_quantities_are_not_hype() {
        for (language, text) in [
            ("de", "Die revolutionäre Bewegung entstand vor dem Krieg."),
            (
                "fr",
                "Une révolution politique peut modifier la constitution.",
            ),
            ("es", "El futuro de esta tarea depende del resultado."),
            (
                "pt",
                "As possibilidades são limitadas pela memória disponível.",
            ),
            ("fi", "Mahdollisuuksien määrä on rajattu kahteen."),
            ("sv", "Det finns fyra möjligheter i den här menyn."),
        ] {
            assert!(!pattern(language).unwrap().is_match(text), "{text}");
        }
    }

    #[test]
    fn empty_promises_match_case_insensitively_but_not_across_sentences() {
        assert!(
            pattern("fr")
                .unwrap()
                .is_match("Un monde de possibilités illimitées.")
        );
        assert!(
            pattern("pt")
                .unwrap()
                .is_match("REVOLUCIONÁRIA E SEM PRECEDENTES.")
        );
        assert!(
            !pattern("de")
                .unwrap()
                .is_match("Revolutionär. Bahnbrechend.")
        );
        assert!(
            !pattern("es")
                .unwrap()
                .is_match("Posibilidades ilimitadas. Excelencia sin límites.")
        );
    }
}
