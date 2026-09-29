//! Semantic checks over the parsed D2 AST, modelled on `d2compiler/compile.go`.
//!
//! D2 treats any non-reserved key as a new object, so typos outside of reserved
//! positions are legal. Those are only reported (as warnings) when the value
//! makes the intended keyword obvious, e.g. `shpe: circle`.

use std::collections::HashSet;

use super::keywords::{self as kw, closest};
use super::parser::{Diag, Key, Map, Node, Scalar, ScalarKind, Str, Subst, Value};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Label {
    Text,
    Icon,
    Tooltip,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Ctx {
    /// Object map (root, containers, boards).
    Object,
    Style,
    Classes,
    /// Keys must be reserved: edge maps and class bodies.
    EdgeMap,
    ClassBody,
    Arrowhead,
    Boards,
    Label(Label),
}

/// The key segment being inspected by `Validator::step`.
#[derive(Clone, Copy)]
struct At<'a> {
    seg: &'a Str,
    /// Last segment of the key path.
    last: bool,
    k: &'a Key,
}

#[derive(Default)]
pub struct Validator {
    pub diags: Vec<Diag>,
    classes: HashSet<String>,
    vars: HashSet<String>,
    has_classes: bool,
    has_vars: bool,
    has_imports: bool,
}

pub fn validate(root: &Map) -> Vec<Diag> {
    let mut v = Validator::default();
    v.collect(root, &mut Vec::new());
    v.walk_map(root, Ctx::Object);
    v.diags
}

/// Unquoted, lowercased keyword text, or None for quoted strings.
fn word(s: &Str) -> Option<String> {
    s.unquoted().then(|| s.text.to_ascii_lowercase())
}

fn usable(s: &Scalar) -> bool {
    s.s.substs.is_empty() && !matches!(s.kind, ScalarKind::Null | ScalarKind::Suspension)
}

impl Validator {
    // Pre-pass: collect declared classes, vars and imports.

    fn collect(&mut self, m: &Map, path: &mut Vec<(String, bool)>) {
        for n in &m.nodes {
            let k = match n {
                Node::Key(k) => k,
                Node::SpreadImport => {
                    self.has_imports = true;
                    continue;
                }
                Node::SpreadSubst(_) => continue,
            };
            if matches!(k.value, Some(Value::Import(_))) {
                self.has_imports = true;
            }
            let depth = path.len();
            if let (Some(kp), true) = (&k.key, k.edges == 0) {
                path.extend(kp.parts.iter().map(|p| (p.text.clone(), p.unquoted())));
                self.note_path(path);
            } else {
                path.push((String::new(), false));
            }
            match &k.value {
                Some(Value::Map(sub)) => self.collect(sub, path),
                Some(Value::Array(a)) if a.items.iter().any(|i| matches!(i, Value::Import(_))) => {
                    self.has_imports = true;
                }
                _ => {}
            }
            path.truncate(depth);
        }
    }

    fn note_path(&mut self, path: &[(String, bool)]) {
        fn is(kw: &'static str) -> impl Fn(&(String, bool)) -> bool {
            move |p| p.1 && p.0 == kw
        }
        if let Some(i) = path.iter().position(is("vars")) {
            self.has_vars = true;
            let rest: Vec<&str> = path[i + 1..].iter().map(|p| p.0.as_str()).collect();
            for n in 1..=rest.len() {
                self.vars.insert(rest[..n].join("."));
            }
        }
        if let Some(i) = path.iter().position(is("classes")) {
            self.has_classes = true;
            if let Some((name, _)) = path.get(i + 1) {
                self.classes.insert(name.clone());
            }
        }
    }

    fn walk_map(&mut self, m: &Map, ctx: Ctx) {
        for n in &m.nodes {
            match n {
                Node::Key(k) => self.walk_key(k, ctx),
                Node::SpreadSubst(s) => self.check_subst(s),
                Node::SpreadImport => {}
            }
        }
    }

    fn walk_key(&mut self, k: &Key, ctx: Ctx) {
        self.check_value_substs(k);
        if k.filter {
            return;
        }
        if k.edges > 0 {
            if let Some(ek) = &k.edge_key {
                self.walk_path(&ek.parts, Ctx::EdgeMap, k);
            } else if let Some(m) = k.map() {
                self.walk_map(m, Ctx::EdgeMap);
            }
            return;
        }
        if let Some(kp) = &k.key {
            self.walk_path(&kp.parts, ctx, k);
        }
    }

    fn walk_path(&mut self, parts: &[Str], mut ctx: Ctx, k: &Key) {
        for (i, seg) in parts.iter().enumerate() {
            let last = i + 1 == parts.len();
            match self.step(ctx, seg, last, k) {
                Some(next) => ctx = next,
                None => return,
            }
        }
    }

    /// Handle one key segment. Returns the context for the next segment, or
    /// None when the rest of the key should not be inspected.
    fn step(&mut self, ctx: Ctx, seg: &Str, last: bool, k: &Key) -> Option<Ctx> {
        let w = word(seg);
        let name = w.as_deref();
        let at = At { seg, last, k };
        match ctx {
            Ctx::Object => self.step_object(name, at),
            Ctx::Style => self.step_style(name, at),
            Ctx::Classes => self.enter(Ctx::ClassBody, at),
            Ctx::Boards => self.enter(Ctx::Object, at),
            Ctx::EdgeMap | Ctx::ClassBody => self.step_reserved_map(ctx, name, at),
            Ctx::Arrowhead => self.step_arrowhead(name, at),
            Ctx::Label(kind) => self.step_label(kind, name, at),
        }
    }

    /// Descend into `next`: walk the key's map when this is the last segment,
    /// otherwise continue the path in `next`.
    fn enter(&mut self, next: Ctx, at: At) -> Option<Ctx> {
        if !at.last {
            return Some(next);
        }
        if let Some(m) = at.k.map() {
            self.walk_map(m, next);
        }
        None
    }

    fn step_object(&mut self, name: Option<&str>, at: At) -> Option<Ctx> {
        let At { seg, last, k } = at;
        match name {
            None => self.enter(Ctx::Object, at),
            Some(_) if seg.is_glob() => self.enter(Ctx::Object, at),
            Some("style") => {
                if last {
                    self.check_style_holder(seg, k);
                }
                self.enter(Ctx::Style, at)
            }
            Some("classes") => self.enter(Ctx::Classes, at),
            Some("vars") => None,
            Some(b) if kw::BOARDS.contains(&b) => self.enter(Ctx::Boards, at),
            Some(a @ ("source-arrowhead" | "target-arrowhead")) => {
                self.diags.push(Diag::error(
                    seg.range.clone(),
                    format!("`{a}` can only be used on connections"),
                ));
                None
            }
            Some(s) if kw::is_style(s) => {
                self.misplaced_style(seg, s);
                None
            }
            Some(r) if kw::is_simple_reserved(r) => self.reserved(r, seg, last, k, false),
            Some(n) => {
                if last {
                    self.near_miss(n, seg, k);
                }
                self.enter(Ctx::Object, at)
            }
        }
    }

    fn step_style(&mut self, name: Option<&str>, at: At) -> Option<Ctx> {
        let At { seg, last, k } = at;
        if seg.is_glob() {
            return None;
        }
        if let Some(s) = name.filter(|s| kw::is_style(s)) {
            if last {
                self.check_style_value(s, k);
            }
            return None;
        }
        let sug = closest(&seg.text, kw::STYLE.iter().copied(), kw::budget(&seg.text));
        self.diags.push(
            Diag::error(
                seg.range.clone(),
                format!("unknown style keyword `{}`", seg.text),
            )
            .help(format!("valid style keywords: {}", kw::STYLE.join(", ")))
            .suggest(sug),
        );
        None
    }

    /// Edge maps and class bodies, where every key must be reserved.
    fn step_reserved_map(&mut self, ctx: Ctx, name: Option<&str>, at: At) -> Option<Ctx> {
        let At { seg, last, k } = at;
        if seg.is_glob() {
            return None;
        }
        match name {
            Some("style") => self.enter(Ctx::Style, at),
            Some("source-arrowhead" | "target-arrowhead") => self.enter(Ctx::Arrowhead, at),
            Some("class") if ctx == Ctx::ClassBody => {
                self.diags.push(Diag::error(
                    seg.range.clone(),
                    "`class` cannot appear within `classes`",
                ));
                None
            }
            Some(s) if kw::is_style(s) => {
                self.misplaced_style(seg, s);
                None
            }
            Some(r) if kw::is_simple_reserved(r) => self.reserved(r, seg, last, k, false),
            Some(r) if kw::is_reserved(r) => None,
            _ => {
                self.unreserved_key(ctx, seg);
                None
            }
        }
    }

    fn unreserved_key(&mut self, ctx: Ctx, seg: &Str) {
        let sug = closest(&seg.text, kw::all_reserved(), kw::budget(&seg.text));
        let msg = if ctx == Ctx::EdgeMap {
            format!(
                "connection map keys must be reserved keywords, got `{}`",
                seg.text
            )
        } else {
            format!(
                "`{}` is an invalid class field, must be a reserved keyword",
                seg.text
            )
        };
        let mut d = Diag::error(seg.range.clone(), msg).suggest(sug);
        if let Some(s) = sug {
            d = d.help(format!("did you mean `{s}`?"));
        }
        self.diags.push(d);
    }

    fn step_arrowhead(&mut self, name: Option<&str>, at: At) -> Option<Ctx> {
        let At { seg, last, k } = at;
        match name {
            Some("style") => return self.enter(Ctx::Style, at),
            Some(r) if kw::is_simple_reserved(r) => return self.reserved(r, seg, last, k, true),
            _ => {}
        }
        let sug = closest(
            &seg.text,
            kw::SIMPLE_RESERVED.iter().copied().chain(["style"]),
            kw::budget(&seg.text),
        );
        self.diags.push(
            Diag::error(
                seg.range.clone(),
                format!(
                    "source-arrowhead/target-arrowhead map keys must be reserved keywords, got `{}`",
                    seg.text
                ),
            )
            .suggest(sug),
        );
        None
    }

    fn step_label(&mut self, kind: Label, name: Option<&str>, at: At) -> Option<Ctx> {
        let At { seg, last, k } = at;
        match name {
            Some("near") => {
                if last {
                    self.check_label_near(kind, k);
                }
                return None;
            }
            Some("style") if kind == Label::Icon => return self.enter(Ctx::Style, at),
            _ => {}
        }
        if last && k.scalar().is_some() {
            let sug = closest(&seg.text, ["near"], 2);
            self.diags.push(
                Diag::error(
                    seg.range.clone(),
                    format!("unexpected field `{}`", seg.text),
                )
                .help("only `near` can be set here")
                .suggest(sug),
            );
        }
        None
    }

    fn misplaced_style(&mut self, seg: &Str, s: &str) {
        let mut d = Diag::error(seg.range.clone(), format!("`{s}` must be `style.{s}`"))
            .help("style attributes are only valid inside `style`")
            .suggest(Some(&format!("style.{s}")));
        d.fix = Some(format!("style.{}", seg.text));
        self.diags.push(d);
    }

    fn check_style_holder(&mut self, seg: &Str, k: &Key) {
        let bad = match (&k.primary, &k.value) {
            (Some(_), _) => true,
            (None, Some(Value::Map(m))) => m.nodes.is_empty(),
            (None, Some(Value::Scalar(s))) => s.kind != ScalarKind::Null && s.s.substs.is_empty(),
            (None, Some(Value::Array(_))) => true,
            (None, Some(Value::Import(_))) | (None, None) => false,
        };
        if bad {
            self.diags.push(
                Diag::error(
                    seg.range.clone(),
                    "`style` expects a map of key-values, e.g. `style: {opacity: 0.4}` or `style.opacity: 0.4`",
                ),
            );
        }
    }

    /// Simple reserved keyword `r` reached.
    fn reserved(
        &mut self,
        r: &str,
        seg: &Str,
        last: bool,
        k: &Key,
        arrowhead: bool,
    ) -> Option<Ctx> {
        let label = match r {
            "label" => Some(Label::Text),
            "icon" => Some(Label::Icon),
            "tooltip" => Some(Label::Tooltip),
            _ => None,
        };
        if !last {
            return label.map(Ctx::Label);
        }
        match (&k.primary, &k.value) {
            (None, None) => {
                self.diags.push(Diag::error(
                    seg.range.clone(),
                    format!("reserved field `{r}` must have a value"),
                ));
                return None;
            }
            (None, Some(Value::Map(_))) if label.is_none() => {
                self.diags.push(Diag::error(
                    seg.range.clone(),
                    format!("reserved field `{r}` does not accept a map"),
                ));
                return None;
            }
            (None, Some(Value::Array(a))) => {
                match r {
                    "class" => {
                        for item in &a.items {
                            if let Value::Scalar(s) = item {
                                self.check_class_ref(s);
                            }
                        }
                    }
                    "constraint" => {}
                    _ => self.diags.push(Diag::error(
                        seg.range.clone(),
                        format!("reserved field `{r}` does not accept an array"),
                    )),
                }
                return None;
            }
            _ => {}
        }
        if let (Some(l), Some(m)) = (label, k.map()) {
            self.walk_map(m, Ctx::Label(l));
        }
        let s = k.scalar()?;
        if !usable(s) {
            return None;
        }
        self.check_reserved_value(r, s, arrowhead);
        None
    }

    fn check_reserved_value(&mut self, r: &str, s: &Scalar, arrowhead: bool) {
        let v = s.s.text.as_str();
        let lower = v.to_ascii_lowercase();
        let range = s.s.range.clone();
        let int = kw::atoi(v);
        let int_err =
            |this: &mut Self, msg: String| this.diags.push(Diag::error(range.clone(), msg));
        match r {
            "shape" if arrowhead => {
                if !kw::is_arrowhead(v) {
                    let sug = closest(v, kw::ARROWHEADS.iter().copied(), kw::budget(v));
                    self.diags.push(
                        Diag::error(range, format!("unknown arrowhead shape `{v}`"))
                            .help(format!(
                                "valid arrowhead shapes: {}",
                                kw::ARROWHEADS.join(", ")
                            ))
                            .suggest(sug),
                    );
                }
            }
            "shape" => {
                if kw::is_shape(v) {
                    return;
                }
                if kw::is_arrowhead(v) {
                    self.diags.push(
                        Diag::error(
                            range,
                            format!("invalid shape, `{v}` can only be set for arrowheads"),
                        )
                        .help(format!("valid shapes: {}", kw::SHAPES.join(", "))),
                    );
                    return;
                }
                let sug = closest(v, kw::SHAPES.iter().copied(), kw::budget(v));
                self.diags.push(
                    Diag::error(range, format!("unknown shape `{v}`"))
                        .help(format!("valid shapes: {}", kw::SHAPES.join(", ")))
                        .suggest(sug),
                );
            }
            "direction" => {
                if !kw::DIRECTIONS.contains(&lower.as_str()) {
                    let sug = closest(v, kw::DIRECTIONS.iter().copied(), 2);
                    self.diags.push(
                        Diag::error(range, format!("invalid direction `{v}`"))
                            .help("direction must be one of up, down, right, left")
                            .suggest(sug),
                    );
                }
            }
            "near" => {
                if kw::NEAR_CONSTANTS.contains(&v) || v.contains('.') {
                    return;
                }
                if kw::LABEL_POSITIONS.contains(&v) {
                    self.diags.push(
                        Diag::error(range, format!("`{v}` is not a valid `near` constant for shapes"))
                            .help(format!(
                                "label positions are only valid in `label.near` / `icon.near`; shapes accept: {}",
                                kw::NEAR_CONSTANTS.join(", ")
                            )),
                    );
                    return;
                }
                if let Some(sug) = closest(v, kw::NEAR_CONSTANTS.iter().copied(), 2) {
                    self.diags.push(
                        Diag::error(range, format!("invalid `near` constant `{v}`"))
                            .help(format!("did you mean `{sug}`?"))
                            .suggest(Some(sug)),
                    );
                }
            }
            "width" | "height" => {
                if int.is_none() {
                    int_err(self, format!("non-integer {r} `{v}`"));
                }
            }
            "top" | "left" => match int {
                None => int_err(self, format!("non-integer {r} `{v}`")),
                Some(n) if n < 0 => int_err(
                    self,
                    format!("{r} must be a non-negative integer, got `{v}`"),
                ),
                _ => {}
            },
            "grid-rows" | "grid-columns" => match int {
                None => int_err(self, format!("non-integer {r} `{v}`")),
                Some(n) if n <= 0 => {
                    int_err(self, format!("{r} must be a positive integer, got `{v}`"))
                }
                Some(n) if n > kw::MAX_GRID_DIMENSION => int_err(
                    self,
                    format!("{r} {n} exceeds the maximum of {}", kw::MAX_GRID_DIMENSION),
                ),
                _ => {}
            },
            "grid-gap" | "vertical-gap" | "horizontal-gap" => match int {
                None => int_err(self, format!("non-integer {r} `{v}`")),
                Some(n) if n < 0 => int_err(
                    self,
                    format!("{r} must be a non-negative integer, got `{v}`"),
                ),
                _ => {}
            },
            "class" => self.check_class_ref(s),
            _ => {}
        }
    }

    fn check_label_near(&mut self, kind: Label, k: &Key) {
        let Some(s) = k.scalar() else {
            if let Some(v) = &k.value {
                self.diags
                    .push(Diag::error(v.range(), "invalid `near` field"));
            }
            return;
        };
        if !usable(s) {
            return;
        }
        let list = match kind {
            Label::Tooltip => kw::TOOLTIP_POSITIONS,
            _ => kw::LABEL_POSITIONS,
        };
        let v = s.s.text.as_str();
        if !list.contains(&v) {
            let sug = closest(v, list.iter().copied(), 3);
            self.diags.push(
                Diag::error(s.s.range.clone(), format!("invalid `near` position `{v}`"))
                    .help(match sug {
                        Some(x) => format!("did you mean `{x}`?"),
                        None => format!("valid positions: {}", list.join(", ")),
                    })
                    .suggest(sug),
            );
        }
    }

    fn check_style_value(&mut self, key: &str, k: &Key) {
        let Some(s) = k.scalar() else { return };
        if !usable(s) {
            return;
        }
        if let Some((msg, sug)) = style_error(key, &s.s.text) {
            self.diags
                .push(Diag::error(s.s.range.clone(), msg).suggest(sug.as_deref()));
        }
    }

    fn check_class_ref(&mut self, s: &Scalar) {
        if !self.has_classes || self.has_imports || !usable(s) {
            return;
        }
        let name = s.s.text.as_str();
        if self.classes.contains(name) {
            return;
        }
        let sug = closest(
            name,
            self.classes.iter().map(String::as_str),
            kw::budget(name),
        )
        .map(str::to_string);
        let mut d = Diag::warning(s.s.range.clone(), format!("class `{name}` not found"));
        d = if name.contains(',') {
            d.help("separate array items with `;`, e.g. `class: [a; b]`")
        } else if let Some(x) = &sug {
            d.help(format!("did you mean `{x}`?"))
        } else {
            d.help("classes must be declared under `classes`")
        };
        self.diags.push(d.suggest(sug.as_deref()));
    }

    fn check_value_substs(&mut self, k: &Key) {
        let mut substs: Vec<&Subst> = Vec::new();
        if let Some(p) = &k.primary {
            substs.extend(&p.s.substs);
        }
        match &k.value {
            Some(Value::Scalar(s)) => substs.extend(&s.s.substs),
            Some(Value::Array(a)) => {
                substs.extend(&a.substs);
                for i in &a.items {
                    if let Value::Scalar(s) = i {
                        substs.extend(&s.s.substs);
                    }
                }
            }
            _ => {}
        }
        for s in substs {
            self.check_subst(s);
        }
    }

    fn check_subst(&mut self, s: &Subst) {
        if !self.has_vars || self.has_imports || s.path.is_empty() {
            return;
        }
        let name = s.path.join(".");
        if !self.vars.contains(&name) {
            let sug = closest(
                &name,
                self.vars.iter().map(String::as_str),
                kw::budget(&name),
            )
            .map(str::to_string);
            self.diags.push(
                Diag::warning(s.range.clone(), format!("undefined variable `{name}`"))
                    .help("variables must be declared under `vars`")
                    .suggest(sug.as_deref()),
            );
        }
    }

    /// Unknown key in an object map whose value strongly suggests a misspelled keyword.
    fn near_miss(&mut self, name: &str, seg: &Str, k: &Key) {
        if name.chars().count() < 3 || k.edges > 0 {
            return;
        }
        let candidates = kw::SIMPLE_RESERVED
            .iter()
            .chain(kw::STYLE)
            .copied()
            .chain(["style"]);
        let Some(cand) = closest(name, candidates, kw::budget(name)) else {
            return;
        };
        let dist = kw::levenshtein(name, cand);
        let scalar = k.scalar().filter(|s| usable(s));
        let text = scalar.map(|s| s.s.text.as_str());
        let plausible = match (cand, text) {
            ("style", _) => {
                k.primary.is_none()
                    && k.map().is_some_and(|m| {
                        m.nodes.iter().any(|n| match n {
                            Node::Key(k) => k
                                .key
                                .as_ref()
                                .and_then(|kp| word(&kp.parts[0]))
                                .is_some_and(|w| kw::is_style(&w)),
                            _ => false,
                        })
                    })
            }
            ("shape", Some(v)) => kw::is_shape(v) || kw::is_arrowhead(v),
            ("direction", Some(v)) => kw::DIRECTIONS.contains(&v.to_ascii_lowercase().as_str()),
            ("near", Some(v)) => kw::NEAR_CONSTANTS.contains(&v),
            (
                "width" | "height" | "top" | "left" | "grid-rows" | "grid-columns" | "grid-gap"
                | "vertical-gap" | "horizontal-gap",
                Some(v),
            ) => kw::atoi(v).is_some() && dist <= 2 && name.len() >= 4,
            ("constraint", Some(v)) => {
                matches!(v, "primary_key" | "foreign_key" | "unique")
            }
            ("label" | "tooltip", Some(_)) => {
                dist == 1 && name.len() >= 5 && name != format!("{cand}s")
            }
            ("icon", Some(v)) => v.starts_with("http://") || v.starts_with("https://"),
            ("class", Some(v)) => self.classes.contains(v),
            (s, Some(v)) if kw::is_style(s) => {
                let is_color = matches!(s, "fill" | "stroke" | "font-color");
                // Named colors are ambiguous (`fil: red` may be an object labelled red).
                s != "font" && (!is_color || v.starts_with('#')) && style_error(s, v).is_none()
            }
            _ => false,
        };
        if !plausible {
            return;
        }
        let target = if kw::is_style(cand) {
            format!("style.{cand}")
        } else {
            cand.to_string()
        };
        self.diags.push(
            Diag::warning(
                seg.range.clone(),
                format!(
                    "`{}` is not a D2 keyword; did you mean `{target}`?",
                    seg.text
                ),
            )
            .help(format!(
                "unknown keys create a new shape named `{}` instead of setting an attribute",
                seg.text
            ))
            .suggest(Some(&target)),
        );
    }
}

/// Validate a style value like `d2graph.Style.Apply`. Returns (message, suggestion).
fn style_error(key: &str, v: &str) -> Option<(String, Option<String>)> {
    let int_range = |lo: i64, hi: Option<i64>| -> Option<(String, Option<String>)> {
        let ok = kw::atoi(v).is_some_and(|n| n >= lo && hi.is_none_or(|h| n <= h));
        (!ok).then(|| {
            let msg = match hi {
                Some(h) => {
                    format!("expected `{key}` to be a number between {lo} and {h}, got `{v}`")
                }
                None => {
                    format!("expected `{key}` to be a number greater or equal to {lo}, got `{v}`")
                }
            };
            (msg, None)
        })
    };
    let one_of = |list: &[&'static str]| -> Option<(String, Option<String>)> {
        (!list.contains(&v.to_ascii_lowercase().as_str())).then(|| {
            (
                format!(
                    "expected `{key}` to be one of {}, got `{v}`",
                    list.join(", ")
                ),
                closest(v, list.iter().copied(), 2).map(str::to_string),
            )
        })
    };
    match key {
        "opacity" => {
            let ok = v
                .parse::<f64>()
                .is_ok_and(|f| f.is_finite() && (0.0..=1.0).contains(&f));
            (!ok).then(|| {
                (
                    format!("expected `opacity` to be a number between 0.0 and 1.0, got `{v}`"),
                    None,
                )
            })
        }
        "stroke" | "fill" | "font-color" => (!kw::valid_color(v)).then(|| {
            let sug = if v.starts_with('#') {
                None
            } else {
                closest(v, kw::NAMED_COLORS.iter().copied(), 2).map(str::to_string)
            };
            (
                format!(
                    "expected `{key}` to be a named color (\"orange\"), a hex code (\"#f0ff3a\") or a gradient, got `{v}`"
                ),
                sug,
            )
        }),
        "fill-pattern" => one_of(kw::FILL_PATTERNS),
        "text-transform" => one_of(kw::TEXT_TRANSFORMS),
        "stroke-width" => int_range(0, Some(15)),
        "stroke-dash" => int_range(0, Some(10)),
        "border-radius" => int_range(0, None),
        "font-size" => int_range(8, Some(100)),
        "shadow" | "3d" | "multiple" | "double-border" | "animated" | "bold" | "italic"
        | "underline" | "filled" => kw::parse_bool(v).is_none().then(|| {
            (
                format!("expected `{key}` to be true or false, got `{v}`"),
                closest(v, ["true", "false"], 2).map(str::to_string),
            )
        }),
        _ => None,
    }
}
