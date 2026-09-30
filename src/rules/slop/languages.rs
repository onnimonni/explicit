//! Conservative non-English hype: empty promises and stacked promotional claims, not words
//! such as “innovative” or ordinary connective phrases in isolation.
use std::ops::Range;
use std::sync::LazyLock;

use regex::Regex;

use super::{FileCtx, Out};
use crate::diagnostic::Finding;
use crate::segment::SegmentKind;

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
    matcher!(DE, r"(?i)\b(?:(?:revolutionär\w*|bahnbrechend\w*|einzigartig\w*)\b[ \p{L},-]{0,80}\b(?:bahnbrechend\w*|beispiellos\w*|alles[ ]+verändernd\w*|unbegrenzte[ ]+möglichkeiten)|welt[ ]+(?:voller|unbegrenzter|endloser)[ ]+möglichkeiten)\b");
    matcher!(FR, r"(?i)\b(?:(?:révolutionnaire|miraculeu(?:x|se)|inégalé\w*)\b[ \p{L},-]{0,80}\b(?:sans[ ]+(?:égal|précédent)|possibilités[ ]+(?:infinies|illimitées))|monde[ ]+de[ ]+possibilités[ ]+(?:infinies|illimitées))\b");
    matcher!(ES, r"(?i)\b(?:(?:revolucionari[oa]s?|revolución|incomparables?|definitiv[oa]s?)\b[ \p{L},-]{0,80}\b(?:incomparable|transforma[ ]+todo|sin[ ]+(?:igual|precedentes|límites))|posibilidades[ ]+(?:ilimitadas|infinitas)[ ]+y[ ]+(?:excelencia|perfección)[ ]+sin[ ]+límites)\b");
    matcher!(PT, r"(?i)\b(?:(?:revolucionári[oa]s?|milagros[oa]s?|incomparáveis)\b[ \p{L},-]{0,80}\b(?:redefine[ ]+tudo|sem[ ]+(?:precedentes|limites))|possibilidades[ ]+(?:infinitas|ilimitadas)[ ]+e[ ]+(?:excelência|perfeição)[ ]+sem[ ]+limites)\b");
    matcher!(FI, r"(?i)\b(?:mullistaa[ ]+kaiken|vallankumouksellinen[ ]+ja[ ]+ennennäkemätön)\b");
    matcher!(SV, r"(?i)\b(?:revolutionerar[ ]+allt|revolutionerande[ ]+och[ ]+utan[ ]+motstycke)\b");
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
            let Some(text) = segment.text.get(start - segment.range.start..end - segment.range.start) else { return };
            for matched in re.find_iter(text) {
                let mut finding = Finding::new(
                    "slop/phrase",
                    super::seg_sev("slop/phrase", segment),
                    start + matched.start()..start + matched.end(),
                    "Promotional claim without concrete evidence",
                ).help("Replace the promise with a specific capability, result or number.");
                if segment.kind == SegmentKind::BlockQuote {
                    finding.severity = crate::diagnostic::Severity::Info;
                }
                out.push(finding);
            }
        };
        if let Some(ranges) = ranges {
            for range in ranges { check(range.clone(), out); }
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
            ("fr", "Une révolution politique peut modifier la constitution."),
            ("es", "El futuro de esta tarea depende del resultado."),
            ("pt", "As possibilidades são limitadas pela memória disponível."),
            ("fi", "Mahdollisuuksien määrä on rajattu kahteen."),
            ("sv", "Det finns fyra möjligheter i den här menyn."),
        ] { assert!(!pattern(language).unwrap().is_match(text), "{text}"); }
    }

    #[test]
    fn empty_promises_match_case_insensitively_but_not_across_sentences() {
        assert!(pattern("fr").unwrap().is_match("Un monde de possibilités illimitées."));
        assert!(pattern("pt").unwrap().is_match("REVOLUCIONÁRIA E SEM PRECEDENTES."));
        assert!(!pattern("de").unwrap().is_match("Revolutionär. Bahnbrechend."));
        assert!(!pattern("es").unwrap().is_match("Posibilidades ilimitadas. Excelencia sin límites."));
    }
}
