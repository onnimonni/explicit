# Writing a lint plugin for tinycc-rs

Lint plugins run after type checking and before codegen. A plugin sees the typed ⟪acronym|AST⟫,
can report diagnostics, and cannot mutate anything. Plugins are ⟪product|Rust⟫ crates loaded as
⟪code|`cdylib`⟫; the ⟪acronym|ABI⟫ is versioned and checked at load time.

## Skeleton

```rust
use tinycc_lint::{Lint, Ctx, Node, Level};

pub struct NoTodoStrings;

impl Lint for NoTodoStrings {
    fn name(&self) -> &'static str { "no-todo-strings" }
    fn visit(&mut self, cx: &mut Ctx, node: &Node) {
        if let Node::StrLit(s) = node {
            if s.contains("TODO") {
                cx.report(Level::Warn, node.span(), "string literal contains TODO");
            }
        }
    }
}

tinycc_lint::export!(NoTodoStrings);
```

Build with ⟪code|`cargo build --release`⟫ and point ⟪code|`lints.plugins`⟫ in ⟪path|tinycc.toml⟫ at the
resulting ⟪path|.so⟫ or ⟪path|.dylib⟫. The compiler ⟦agreement|verify|verifies⟧ the ⟪acronym|ABI⟫ hash
and refuses plugins built against ⟦a_an|a older|an older⟧ ⟪code|`tinycc_lint`⟫.

## Visiting

⟪code|`visit`⟫ is called once per node in pre-order. ⟦its_its|Its|It's⟧ the only required method.
⟪code|`enter_fn`⟫ and ⟪code|`leave_fn`⟫ exist for lints that need function scope, such as tracking
⟦homophone|weather|whether⟧ a return value was checked. Keep per-node work under a microsecond;
a plugin that ⟦spelling|alocates|allocates⟧ on every node makes the whole compile noticeably slower
on the ⟪unit|400k⟫ line ⟪product|kernel⟫ tree we use as a benchmark.

## Reporting

Diagnostics carry a level, a span and a message. Optional suggestions are applied by
⟪code|`tinycc fix`⟫ only when marked ⟪code|`Applicability::Safe`⟫. ⟦punctuation|Be conservative here, a wrong safe fix is the fastest way to lose users.|Be conservative here; a wrong safe fix is the fastest way to lose users.⟧
Messages are lowercase, no period, and name ⟦their_there|there|their⟧ subject:
⟪code|`unused variable x`⟫ rather than "There is an unused variable."

## Configuration

Plugins receive ⟦their_there|they're|their⟧ section of ⟪path|tinycc.toml⟫ as a ⟪acronym|TOML⟫ table.
Unknown keys should be reported as warnings, not ignored; ⟦then_than|than|then⟧ a typo in the config
shows up immediately instead of ⟦missing_extra_word|silently doing nothing months|silently doing nothing for months⟧.

## Testing

⟪code|`tinycc_lint::test::check(source, lint)`⟫ compiles a snippet with only ⟦your_youre|you're|your⟧
lint enabled and returns the diagnostics. ⟪product|insta⟫ snapshots of the rendered output are the
usual pattern. ⟦fragment|Cheap, fast, and readable in review.|They are cheap, fast, and readable in review.⟧

## Distribution

Publish the crate; users build it themselves. We do not ship prebuilt plugins because the ABI
changes with every minor ⟦capitalization|rust|Rust⟧ release and a mismatched plugin ⟦homophone|effects|affects⟧
every compile, not just the lint. ⟪informal|We tried prebuilt in 0.4 and regretted it within a week.⟫
