//! D2 keyword and value tables.
//!
//! Copied from github.com/terrastruct/d2 at commit
//! 01bc7ecdbdd04c13d6fe5df1967d2d9aa14ae579:
//! - `d2ast/keywords.go` (reserved / style / board keywords, near constants,
//!   label positions, fill patterns, text transforms)
//! - `d2target/d2target.go` (`Shapes`, `Arrowheads`)
//! - `d2graph/d2graph.go` `Style.Apply` (style value validation)
//! - `d2compiler/compile.go` `compileReserved` (reserved value validation)
//! - `lib/color/color.go` (`NamedColors`, `ColorHexRegex`, theme colors)

/// `SimpleReservedKeywords`: non style / holder keywords.
pub const SIMPLE_RESERVED: &[&str] = &[
    "label",
    "shape",
    "icon",
    "constraint",
    "tooltip",
    "link",
    "near",
    "width",
    "height",
    "direction",
    "top",
    "left",
    "grid-rows",
    "grid-columns",
    "grid-gap",
    "vertical-gap",
    "horizontal-gap",
    "class",
    "vars",
];

/// `StyleKeywords`: only valid under `style`.
pub const STYLE: &[&str] = &[
    "opacity",
    "stroke",
    "fill",
    "fill-pattern",
    "stroke-width",
    "stroke-dash",
    "border-radius",
    "font",
    "font-size",
    "font-color",
    "bold",
    "italic",
    "underline",
    "text-transform",
    "shadow",
    "multiple",
    "double-border",
    "3d",
    "animated",
    "filled",
];

/// `CompositeReservedKeywords` + `ReservedKeywordHolders` + `BoardKeywords`.
pub const COMPOSITE: &[&str] = &[
    "source-arrowhead",
    "target-arrowhead",
    "classes",
    "constraint",
    "label",
    "icon",
    "tooltip",
    "style",
    "layers",
    "scenarios",
    "steps",
];

pub const BOARDS: &[&str] = &["layers", "scenarios", "steps"];

pub const NEAR_CONSTANTS: &[&str] = &[
    "top-left",
    "top-center",
    "top-right",
    "center-left",
    "center-right",
    "bottom-left",
    "bottom-center",
    "bottom-right",
];

/// `LabelPositionsArray`: valid `label.near` / `icon.near` values.
pub const LABEL_POSITIONS: &[&str] = &[
    "top-left",
    "top-center",
    "top-right",
    "center-left",
    "center-center",
    "center-right",
    "bottom-left",
    "bottom-center",
    "bottom-right",
    "outside-top-left",
    "outside-top-center",
    "outside-top-right",
    "outside-left-top",
    "outside-left-center",
    "outside-left-bottom",
    "outside-right-top",
    "outside-right-center",
    "outside-right-bottom",
    "outside-bottom-left",
    "outside-bottom-center",
    "outside-bottom-right",
    "border-top-left",
    "border-top-center",
    "border-top-right",
    "border-left-top",
    "border-left-center",
    "border-left-bottom",
    "border-right-top",
    "border-right-center",
    "border-right-bottom",
    "border-bottom-left",
    "border-bottom-center",
    "border-bottom-right",
];

/// `TooltipPositionsArray` (same as near constants).
pub const TOOLTIP_POSITIONS: &[&str] = NEAR_CONSTANTS;

pub const FILL_PATTERNS: &[&str] = &["none", "dots", "lines", "grain", "paper"];
pub const TEXT_TRANSFORMS: &[&str] = &["none", "uppercase", "lowercase", "capitalize"];
pub const DIRECTIONS: &[&str] = &["up", "down", "right", "left"];

/// `d2target.Shapes`.
pub const SHAPES: &[&str] = &[
    "rectangle",
    "square",
    "page",
    "parallelogram",
    "document",
    "cylinder",
    "queue",
    "package",
    "step",
    "callout",
    "stored_data",
    "person",
    "c4-person",
    "diamond",
    "oval",
    "circle",
    "hexagon",
    "cloud",
    "text",
    "code",
    "class",
    "sql_table",
    "image",
    "sequence_diagram",
    "hierarchy",
];

/// `d2target.Arrowheads`: valid values for `source-arrowhead.shape`.
pub const ARROWHEADS: &[&str] = &[
    "none",
    "arrow",
    "triangle",
    "diamond",
    "circle",
    "box",
    "cf-one",
    "cf-many",
    "cf-one-required",
    "cf-many-required",
    "cross",
];

/// `d2graph.MaxGridDimension` (d2graph/grid_diagram.go).
pub const MAX_GRID_DIMENSION: i64 = 10_000;

/// `color.NamedColors`.
pub const NAMED_COLORS: &[&str] = &[
    "currentcolor",
    "transparent",
    "aliceblue",
    "antiquewhite",
    "aqua",
    "aquamarine",
    "azure",
    "beige",
    "bisque",
    "black",
    "blanchedalmond",
    "blue",
    "blueviolet",
    "brown",
    "burlywood",
    "cadetblue",
    "chartreuse",
    "chocolate",
    "coral",
    "cornflowerblue",
    "cornsilk",
    "crimson",
    "cyan",
    "darkblue",
    "darkcyan",
    "darkgoldenrod",
    "darkgray",
    "darkgrey",
    "darkgreen",
    "darkkhaki",
    "darkmagenta",
    "darkolivegreen",
    "darkorange",
    "darkorchid",
    "darkred",
    "darksalmon",
    "darkseagreen",
    "darkslateblue",
    "darkslategray",
    "darkslategrey",
    "darkturquoise",
    "darkviolet",
    "deeppink",
    "deepskyblue",
    "dimgray",
    "dimgrey",
    "dodgerblue",
    "firebrick",
    "floralwhite",
    "forestgreen",
    "fuchsia",
    "gainsboro",
    "ghostwhite",
    "gold",
    "goldenrod",
    "gray",
    "grey",
    "green",
    "greenyellow",
    "honeydew",
    "hotpink",
    "indianred",
    "indigo",
    "ivory",
    "khaki",
    "lavender",
    "lavenderblush",
    "lawngreen",
    "lemonchiffon",
    "lightblue",
    "lightcoral",
    "lightcyan",
    "lightgoldenrodyellow",
    "lightgray",
    "lightgrey",
    "lightgreen",
    "lightpink",
    "lightsalmon",
    "lightseagreen",
    "lightskyblue",
    "lightslategray",
    "lightslategrey",
    "lightsteelblue",
    "lightyellow",
    "lime",
    "limegreen",
    "linen",
    "magenta",
    "maroon",
    "mediumaquamarine",
    "mediumblue",
    "mediumorchid",
    "mediumpurple",
    "mediumseagreen",
    "mediumslateblue",
    "mediumspringgreen",
    "mediumturquoise",
    "mediumvioletred",
    "midnightblue",
    "mintcream",
    "mistyrose",
    "moccasin",
    "navajowhite",
    "navy",
    "oldlace",
    "olive",
    "olivedrab",
    "orange",
    "orangered",
    "orchid",
    "palegoldenrod",
    "palegreen",
    "paleturquoise",
    "palevioletred",
    "papayawhip",
    "peachpuff",
    "peru",
    "pink",
    "plum",
    "powderblue",
    "purple",
    "rebeccapurple",
    "red",
    "rosybrown",
    "royalblue",
    "saddlebrown",
    "salmon",
    "sandybrown",
    "seagreen",
    "seashell",
    "sienna",
    "silver",
    "skyblue",
    "slateblue",
    "slategray",
    "slategrey",
    "snow",
    "springgreen",
    "steelblue",
    "tan",
    "teal",
    "thistle",
    "tomato",
    "turquoise",
    "violet",
    "wheat",
    "white",
    "whitesmoke",
    "yellow",
    "yellowgreen",
];

pub fn is_simple_reserved(s: &str) -> bool {
    SIMPLE_RESERVED.contains(&s)
}
pub fn is_style(s: &str) -> bool {
    STYLE.contains(&s)
}
/// `ReservedKeywords` = simple + style + composite (incl. holders and boards).
pub fn is_reserved(s: &str) -> bool {
    is_simple_reserved(s) || is_style(s) || COMPOSITE.contains(&s)
}
pub fn all_reserved() -> impl Iterator<Item = &'static str> {
    SIMPLE_RESERVED
        .iter()
        .chain(STYLE)
        .chain(COMPOSITE)
        .copied()
}
pub fn is_shape(s: &str) -> bool {
    SHAPES.iter().any(|x| x.eq_ignore_ascii_case(s))
}
pub fn is_arrowhead(s: &str) -> bool {
    ARROWHEADS.contains(&s.to_ascii_lowercase().as_str())
}

/// `color.ValidColor`, plus theme color codes (`N1`, `B2`, ...) to stay permissive.
pub fn valid_color(s: &str) -> bool {
    let lower = s.to_ascii_lowercase();
    if lower.starts_with("linear-gradient(") || lower.starts_with("radial-gradient(") {
        return true;
    }
    if NAMED_COLORS.contains(&lower.as_str()) || is_theme_color(s) {
        return true;
    }
    if let Some(hex) = s.strip_prefix('#') {
        return (hex.len() == 3 || hex.len() == 6) && hex.bytes().all(|b| b.is_ascii_hexdigit());
    }
    false
}

fn is_theme_color(s: &str) -> bool {
    let b = s.as_bytes();
    match b {
        [b'N', d] => (b'1'..=b'7').contains(d),
        [b'B', d] => (b'1'..=b'6').contains(d),
        [b'A', b'A', d] => matches!(d, b'2' | b'4' | b'5'),
        [b'A', b'B', d] => matches!(d, b'4' | b'5'),
        _ => false,
    }
}

/// Go `strconv.ParseBool`.
pub fn parse_bool(s: &str) -> Option<bool> {
    match s {
        "1" | "t" | "T" | "TRUE" | "true" | "True" => Some(true),
        "0" | "f" | "F" | "FALSE" | "false" | "False" => Some(false),
        _ => None,
    }
}

/// Go `strconv.Atoi` (optional sign, decimal digits).
pub fn atoi(s: &str) -> Option<i64> {
    let digits = s.strip_prefix(['+', '-']).unwrap_or(s);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    s.parse().ok()
}

/// Optimal string alignment distance (Levenshtein + adjacent transpositions).
pub fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let w = b.len() + 1;
    let mut d = vec![0usize; (a.len() + 1) * w];
    for i in 0..=a.len() {
        d[i * w] = i;
    }
    for (j, cell) in d.iter_mut().enumerate().take(w) {
        *cell = j;
    }
    for i in 1..=a.len() {
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            let mut v = (d[(i - 1) * w + j] + 1)
                .min(d[i * w + j - 1] + 1)
                .min(d[(i - 1) * w + j - 1] + cost);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                v = v.min(d[(i - 2) * w + j - 2] + 1);
            }
            d[i * w + j] = v;
        }
    }
    d[a.len() * w + b.len()]
}

/// Closest candidate within `max` edits (ties: first wins). Exact matches excluded.
pub fn closest<'a>(
    word: &str,
    candidates: impl IntoIterator<Item = &'a str>,
    max: usize,
) -> Option<&'a str> {
    let w = word.to_ascii_lowercase();
    let mut best: Option<(usize, &str)> = None;
    for c in candidates {
        if c == w {
            return None;
        }
        let d = levenshtein(&w, c);
        if d <= max && best.is_none_or(|(bd, _)| d < bd) {
            best = Some((d, c));
        }
    }
    best.map(|(_, c)| c)
}

/// Edit budget scaled by word length, so short words don't match everything.
pub fn budget(word: &str) -> usize {
    match word.chars().count() {
        0..=3 => 1,
        4..=6 => 2,
        _ => 3,
    }
}
