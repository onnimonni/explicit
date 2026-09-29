//! Shell sessions without output (MD014) and required front matter keys.

use std::sync::LazyLock;

use regex::Regex;

use super::Ctx;
use crate::rules::Out;

static FM_KEY_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?m)^["']?(\w[\w-]*)["']?[ \t]*:"#).expect("hardcoded regex is valid")
});

pub fn check(c: &mut Ctx, out: &mut Out) {
    if c.on("md/commands-show-output") {
        commands(c, out);
    }
    if c.on("md/front-matter-required") {
        front_matter(c, out);
    }
}

fn commands(c: &Ctx, out: &mut Out) {
    for cb in &c.md.code_blocks {
        if cb.range.is_empty() {
            continue;
        }
        let first = c.lines.index_of(cb.range.start);
        let mut last = c.lines.index_of(cb.range.end - 1);
        let (a, b) = if cb.fenced {
            let fence = |i: usize| {
                let l = &c.lines.v[i];
                let t = c.src[l.body..l.end].trim_start();
                t.starts_with("```") || t.starts_with("~~~")
            };
            if last > first && fence(last) {
                last -= 1;
            }
            (first + 1, last)
        } else {
            (first, last)
        };
        if a > b {
            continue;
        }
        let texts: Vec<&str> = (a..=b)
            .map(|i| {
                let l = &c.lines.v[i];
                c.src[l.body..l.end].trim()
            })
            .filter(|t| !t.is_empty())
            .collect();
        if !texts.is_empty() && texts.iter().all(|t| t.starts_with("$ ")) {
            let l = &c.lines.v[a];
            out.push(
                c.finding(
                    "md/commands-show-output",
                    l.body..c.lines.v[b].end,
                    "Every command starts with '$' but no output is shown",
                )
                .help("Drop the '$' prompts, or include the command output"),
            );
        }
    }
}

fn front_matter(c: &Ctx, out: &mut Out) {
    let required = &c.fc.config.markdown.front_matter_required;
    if required.is_empty() {
        return;
    }
    let (keys, range): (Vec<&str>, _) = match &c.md.front_matter {
        Some(r) => {
            let text = &c.src[r.clone()];
            let keys = FM_KEY_RE
                .captures_iter(text)
                .map(|m| m.get(1).expect("regex group 1 is not optional").as_str())
                .collect();
            let first = &c.lines.v[c.lines.index_of(r.start)];
            (keys, first.start..first.end)
        }
        None => (Vec::new(), 0..0),
    };
    for k in required {
        if !keys.contains(&k.as_str()) {
            let msg = if c.md.front_matter.is_some() {
                format!("Front matter is missing required key '{k}'")
            } else {
                format!("Missing front matter with required key '{k}'")
            };
            out.push(c.finding("md/front-matter-required", range.clone(), msg));
        }
    }
}
