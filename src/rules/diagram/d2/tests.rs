use super::check;
use crate::diagnostic::{Finding, Severity};

fn run(src: &str) -> Vec<Finding> {
    let mut out = Vec::new();
    check(src, 0, &mut out);
    out
}

/// Range of the `nth` (0-based) occurrence of `needle` in `src`.
fn at(src: &str, needle: &str, nth: usize) -> std::ops::Range<usize> {
    let start = src
        .match_indices(needle)
        .nth(nth)
        .unwrap_or_else(|| panic!("{needle:?} not in source"))
        .0;
    start..start + needle.len()
}

#[track_caller]
fn expect_one(src: &str, msg: &str, range: std::ops::Range<usize>, sev: Severity) -> Finding {
    let f = run(src);
    assert_eq!(f.len(), 1, "expected one finding for {src:?}, got {f:#?}");
    let f = f.into_iter().next().unwrap();
    assert!(
        f.message.contains(msg),
        "{:?} does not contain {msg:?}",
        f.message
    );
    assert_eq!(f.range, range, "range for {:?}", f.message);
    assert_eq!(f.severity, sev);
    assert_eq!(f.rule, "diagram/d2");
    f
}

// Valid corpus, taken from the D2 tour at https://d2lang.com/tour.

const VALID: &[&str] = &[
    // shapes
    r##"imAShape
im_a_shape
im a shape
i'm a shape
a-shape
pg: PostgreSQL
Cloud: my cloud
Cloud.shape: cloud
SQLite; Cassandra
"##,
    // connections
    r##"Write Replica Canada <-> Read Replica
Write Replica Australia <-> Read Replica
Read Replica <- Master
Write Replica Canada -- Master
a -> b: To err is human, to moo bovine {
  source-arrowhead: 1
  target-arrowhead: * {
    shape: diamond
    style.filled: true
  }
}
High Mem Instance -> EC2 <- High CPU Instance: Hosted By
Stage One -> Stage Two -> Stage Three -> Stage Four
(a -> b)[0].style.stroke: red
x -> y: hi
x -> y: hi
(x -> y)[1]: { style.stroke-dash: 3 }
"##,
    // containers, near object ref, _ parent
    r##"server
server.process
im a parent.im a child
apartment.Bedroom.Bathroom -> office.Spare Room.Cat Rest Area: A mystery
clouds: {
  aws: {
    load_balancer -> api
    api -> db
  }
  gcloud: {
    auth -> db
  }
  gcloud -> aws
}
christmas: {
  presents
}
birthdays: {
  presents
  _.christmas.presents -> presents: regift
}
explanation: |md
  # I can do headers
  - lists
  - lists
|
explanation.near: clouds.aws
"##,
    // styles
    r##"x: {
  style: {
    opacity: 0.4
    stroke: "#53C0D8"
    fill: honeydew
    fill-pattern: dots
    stroke-width: 5
    stroke-dash: 3
    border-radius: 4
    shadow: true
    3d: true
    multiple: true
    double-border: true
    font-size: 20
    font-color: red
    bold: true
    italic: false
    underline: true
    text-transform: uppercase
  }
}
y.style.fill: linear-gradient(#f69d3c, #3f87a6)
x -> y: { style.animated: true }
"##,
    // text & code blocks
    r##"md: |md
  # Hi `code` and **bold**
|
a: |||ts
  declare function getSmallPet(): Fish | Bird;
|||
b: |`go
  x := "|"
`|
latex: |latex
  \lim_{h \rightarrow 0 } \frac{f(x+h)-f(x)}{h}
|
latex.shape: text
title: A title {
  near: top-center
  shape: text
  style.font-size: 55
}
"##,
    // classes
    r##"classes: {
  load balancer: {
    label: load\nbalancer
    width: 100
    height: 200
    style: {
      stroke-width: 0
      fill: "#44C7B1"
      shadow: true
      border-radius: 5
    }
  }
  unhealthy: {
    style: {
      fill: "#FE7070"
      stroke: "#F69E03"
    }
  }
}
web traffic -> web lb
web lb.class: load balancer
web lb -> api1
api1.class: [load balancer; unhealthy]
"##,
    // vars
    r##"vars: {
  server-name: Cat
  primaryColors: {
    button: {
      active: "#4baae5"
    }
  }
  d2-config: {
    layout-engine: elk
    theme-id: 300
  }
}
server1: ${server-name}-1
server2: "${server-name}-2"
button: {
  style.fill: ${primaryColors.button.active}
}
"##,
    // globs & filters
    r##"iphone 10
iphone 11 mini
*.shape: circle
*iphone*.style.fill: red
**.style.stroke: blue
(* -> *)[*].style.stroke-width: 2
(** -> **)[*]: {
  &src: a
  style.stroke: green
}
x: {
  &shape: circle
  !&label: hi
}
"##,
    // layers / scenarios / steps
    r##"a -> b
layers: {
  x: {
    y -> z
  }
}
scenarios: {
  s1: {
    a.style.fill: red
  }
}
steps: {
  1: {
    Approach road
  }
  2: {
    Approach road -> Cross road
  }
}
"##,
    // grid
    r##"grid-rows: 5
grid-columns: 3
grid-gap: 0
vertical-gap: 10
horizontal-gap: 20
Executive Services
Legislative Services
"##,
    // sql_table
    r##"users: {
  shape: sql_table
  id: int {constraint: primary_key}
  last_login: datetime
  email: varchar(255) {constraint: [unique; foreign_key]}
  "left": int
}
users.id <-> orders.user_id
"##,
    // class shape
    r##"MyClass: {
  shape: class

  field: "[]string"
  method(a uint64): (x, y int)
  +public: bool
  -private: int
  \#protected: string
  +getInfo(): string
}
"##,
    // icons, links, tooltips, label near
    r##"github: {
  icon: https://icons.terrastruct.com/dev/github.svg
  icon.near: outside-top-left
  link: https://github.com/terrastruct/d2
  tooltip: Unicode ✓ tooltip — ok
  label.near: border-bottom-center
  shape: image
}
x: { tooltip: hello { near: top-left } }
y: 世界 {
  label: "Hello.world: quoted"
  width: 140
  height: 60
  top: 10
  left: 20
}
direction: right
z.direction: up
"##,
    // comments, block comments, semicolons, nulls, imports
    r##"# a comment
"""
block comment { unbalanced [ stuff
"""
a; b; c -> d # trailing
a.style.fill: null
x: @x.d2
...@y
z: suspend
'single ''quoted'' key': ok
"with \"escapes\"": "and escaped \${x}"
"##,
    // arrowheads and sequence diagrams
    r##"shape: sequence_diagram
alice -> bob: hello {
  source-arrowhead.shape: cf-one
  target-arrowhead: {shape: cf-many-required; style.filled: false}
}
bob."note to self"
alice.t1 -> bob: {
  target-arrowhead.shape: arrow
  style: {stroke: "#f00"; stroke-width: 3}
}
"##,
];

#[test]
fn valid_corpus_has_no_findings() {
    for (i, src) in VALID.iter().enumerate() {
        let f = run(src);
        assert!(
            f.is_empty(),
            "valid snippet #{i} produced findings: {f:#?}\n---\n{src}"
        );
    }
}

// Syntax errors.

#[test]
fn unclosed_brace_points_at_opener() {
    let src = "a: {\n  b\n";
    expect_one(src, "unclosed `{`", 3..4, Severity::Error);
}

#[test]
fn stray_close_brace() {
    let src = "a\n}\n";
    expect_one(src, "unexpected `}`", 2..3, Severity::Error);
}

#[test]
fn unclosed_array_points_at_opener() {
    let src = "x.class: [a; b\n";
    expect_one(src, "unclosed `[`", 9..10, Severity::Error);
}

#[test]
fn stray_close_bracket() {
    let src = "a: b\n]\n";
    expect_one(src, "unexpected `]`", 5..6, Severity::Error);
}

#[test]
fn unterminated_double_string() {
    let src = "a: \"hello\nb\n";
    expect_one(src, "unterminated string", 3..9, Severity::Error);
}

#[test]
fn unterminated_single_string() {
    let src = "'abc: x\n";
    expect_one(src, "unterminated string", 0..7, Severity::Error);
}

#[test]
fn unterminated_block_string() {
    let src = "a: |||md\n  # hi\n||\n";
    let f = expect_one(src, "unterminated block string", 3..8, Severity::Error);
    assert!(f.help.unwrap().contains("|||"));
}

#[test]
fn unterminated_block_comment() {
    let src = "a\n\"\"\"\nnever closed\n";
    expect_one(src, "unterminated block comment", 2..5, Severity::Error);
}

#[test]
fn connection_missing_destination() {
    let src = "a -> b\nx ->\n";
    expect_one(
        src,
        "connection missing destination",
        at(src, "x ->", 0),
        Severity::Error,
    );
}

#[test]
fn connection_missing_source() {
    let src = "-> b\n";
    expect_one(src, "connection missing source", 0..1, Severity::Error);
}

#[test]
fn unclosed_edge_group() {
    let src = "(a -> b.style.stroke: red\n";
    expect_one(src, "unclosed `(`", 0..1, Severity::Error);
}

#[test]
fn bad_edge_index() {
    let src = "(a -> b)[x]: hi\n";
    expect_one(
        src,
        "unexpected character in edge index",
        9..10,
        Severity::Error,
    );
}

#[test]
fn hex_color_unquoted_is_comment() {
    let src = "a.style.fill: #ff0000\n";
    let f = expect_one(src, "missing value after colon", 12..13, Severity::Error);
    assert!(f.help.unwrap().contains("\"#ff0000\""));
}

#[test]
fn text_after_quoted_value() {
    let src = "a: \"x\" y\n";
    expect_one(
        src,
        "unexpected text after double quoted string",
        7..8,
        Severity::Error,
    );
}

// Keyword and value errors.

#[test]
fn typo_shape_keyword_warns_with_suggestion() {
    let src = "x: {\n  shpe: circle\n}\n";
    let f = expect_one(
        src,
        "did you mean `shape`",
        at(src, "shpe", 0),
        Severity::Warning,
    );
    assert_eq!(f.suggestions, vec!["shape"]);
}

#[test]
fn unknown_style_keyword_suggests() {
    let src = "x.style.fil: red\n";
    let f = expect_one(src, "unknown style keyword `fil`", 8..11, Severity::Error);
    assert_eq!(f.suggestions, vec!["fill"]);
}

#[test]
fn unknown_style_keyword_in_map() {
    let src = "x: {style: {opacty: 0.5}}\n";
    let f = expect_one(
        src,
        "unknown style keyword",
        at(src, "opacty", 0),
        Severity::Error,
    );
    assert_eq!(f.suggestions, vec!["opacity"]);
}

#[test]
fn invalid_shape_value() {
    let src = "x.shape: circel\n";
    let f = expect_one(src, "unknown shape `circel`", 9..15, Severity::Error);
    assert_eq!(f.suggestions, vec!["circle"]);
}

#[test]
fn arrowhead_only_shape_on_object() {
    let src = "x.shape: cf-many\n";
    expect_one(
        src,
        "can only be set for arrowheads",
        9..16,
        Severity::Error,
    );
}

#[test]
fn invalid_arrowhead_shape() {
    let src = "a -> b: {target-arrowhead.shape: hexagon}\n";
    expect_one(
        src,
        "unknown arrowhead shape",
        at(src, "hexagon", 0),
        Severity::Error,
    );
}

#[test]
fn invalid_direction() {
    let src = "direction: rigth\n";
    let f = expect_one(src, "invalid direction", 11..16, Severity::Error);
    assert_eq!(f.suggestions, vec!["right"]);
}

#[test]
fn invalid_near_constant() {
    let src = "x.near: top-centre\n";
    let f = expect_one(src, "invalid `near` constant", 8..18, Severity::Error);
    assert_eq!(f.suggestions, vec!["top-center"]);
}

#[test]
fn invalid_label_near() {
    let src = "x.label.near: outside-top\n";
    expect_one(src, "invalid `near` position", 14..25, Severity::Error);
}

#[test]
fn opacity_out_of_range() {
    let src = "x.style.opacity: 1.5\n";
    expect_one(src, "between 0.0 and 1.0", 17..20, Severity::Error);
}

#[test]
fn stroke_width_out_of_range() {
    let src = "x: {style.stroke-width: 20}\n";
    expect_one(src, "between 0 and 15", at(src, "20", 0), Severity::Error);
}

#[test]
fn font_size_out_of_range() {
    let src = "x.style.font-size: 4\n";
    expect_one(src, "between 8 and 100", 19..20, Severity::Error);
}

#[test]
fn boolean_style_wrong_type() {
    let src = "x.style.shadow: yes\n";
    expect_one(src, "true or false", 16..19, Severity::Error);
}

#[test]
fn invalid_color() {
    let src = "x.style.fill: \"#12345\"\n";
    expect_one(src, "named color", 14..22, Severity::Error);
}

#[test]
fn invalid_fill_pattern() {
    let src = "x.style.fill-pattern: dot\n";
    let f = expect_one(src, "fill-pattern", 22..25, Severity::Error);
    assert_eq!(f.suggestions, vec!["dots"]);
}

#[test]
fn style_keyword_outside_style() {
    let src = "x: {\n  fill: red\n}\n";
    let f = expect_one(
        src,
        "`fill` must be `style.fill`",
        at(src, "fill", 0),
        Severity::Error,
    );
    assert_eq!(f.fix.unwrap().replacement, "style.fill");
}

#[test]
fn style_with_scalar() {
    let src = "x.style: red\n";
    expect_one(src, "`style` expects a map", 2..7, Severity::Error);
}

#[test]
fn edge_map_non_reserved_key() {
    let src = "a -> b: {\n  lable: hi\n}\n";
    let f = expect_one(
        src,
        "must be reserved keywords",
        at(src, "lable", 0),
        Severity::Error,
    );
    assert_eq!(f.suggestions, vec!["label"]);
}

#[test]
fn class_field_not_reserved() {
    let src = "classes: {\n  c: {\n    colour: red\n  }\n}\n";
    expect_one(
        src,
        "invalid class field",
        at(src, "colour", 0),
        Severity::Error,
    );
}

#[test]
fn grid_rows_non_integer() {
    let src = "grid-rows: two\n";
    expect_one(src, "non-integer grid-rows", 11..14, Severity::Error);
}

#[test]
fn arrowhead_on_object() {
    let src = "x.source-arrowhead: 1\n";
    expect_one(
        src,
        "can only be used on connections",
        2..18,
        Severity::Error,
    );
}

#[test]
fn undefined_var_warns() {
    let src = "vars: {\n  color: red\n}\nx.style.fill: ${colr}\n";
    let f = expect_one(
        src,
        "undefined variable `colr`",
        at(src, "${colr}", 0),
        Severity::Warning,
    );
    assert_eq!(f.suggestions, vec!["color"]);
}

#[test]
fn unknown_class_warns() {
    let src = "classes: {\n  big: {style.font-size: 30}\n}\nx.class: bgi\ny.class: [big; nope]\n";
    let f = run(src);
    assert_eq!(f.len(), 2, "{f:#?}");
    assert!(f[0].message.contains("class `bgi` not found"));
    assert_eq!(f[0].range, at(src, "bgi", 0));
    assert_eq!(f[0].suggestions, vec!["big"]);
    assert_eq!(f[0].severity, Severity::Warning);
    assert_eq!(f[1].range, at(src, "nope", 0));
}

#[test]
fn offset_is_applied() {
    let mut out = Vec::new();
    check("x.shape: blob\n", 100, &mut out);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].range, 109..113);
}

#[test]
fn no_class_or_var_checks_without_declarations() {
    assert!(run("x.class: whatever\ny: ${undeclared}\n").is_empty());
}

#[test]
fn legal_lookalikes_are_not_flagged() {
    // Non-reserved keys are just object names; no evidence of a typo here.
    let src = "labels: hello\nshapes -> styles\nfil: red\ncolor: blue\nnear: x.y\n";
    let f = run(src);
    assert!(f.is_empty(), "{f:#?}");
}

#[test]
fn truncated_inputs_never_panic() {
    for src in VALID {
        for (i, _) in src.char_indices() {
            run(&src[..i]);
        }
    }
    for src in [
        "|", "|||", "\"", "'", "(", "(a -> ", "${", "a: ${x", "[", "{", "...@", "a.", "\\", "a: \\",
    ] {
        run(src);
    }
}
