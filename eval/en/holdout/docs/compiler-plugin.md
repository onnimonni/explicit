# Writing a lint plugin for tinycc-rs

Lint plugins run after type checking and before codegen. A plugin sees the typed AST,
can report diagnostics, and cannot mutate anything. Plugins are Rust crates loaded as
`cdylib`; the ABI is versioned and checked at load time.

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

Build with `cargo build --release` and point `lints.plugins` in tinycc.toml at the
resulting .so or .dylib. The compiler verify the ABI hash
and refuses plugins built against a older `tinycc_lint`.

## Visiting

`visit` is called once per node in pre-order. Its the only required method.
`enter_fn` and `leave_fn` exist for lints that need function scope, such as tracking
weather a return value was checked. Keep per-node work under a microsecond;
a plugin that alocates on every node makes the whole compile noticeably slower
on the 400k line kernel tree we use as a benchmark.

## Reporting

Diagnostics carry a level, a span and a message. Optional suggestions are applied by
`tinycc fix` only when marked `Applicability::Safe`. Be conservative here, a wrong safe fix is the fastest way to lose users.
Messages are lowercase, no period, and name there subject:
`unused variable x` rather than "There is an unused variable."

## Configuration

Plugins receive they're section of tinycc.toml as a TOML table.
Unknown keys should be reported as warnings, not ignored; than a typo in the config
shows up immediately instead of silently doing nothing months.

## Testing

`tinycc_lint::test::check(source, lint)` compiles a snippet with only you're
lint enabled and returns the diagnostics. insta snapshots of the rendered output are the
usual pattern. Cheap, fast, and readable in review.

## Distribution

Publish the crate; users build it themselves. We do not ship prebuilt plugins because the ABI
changes with every minor rust release and a mismatched plugin effects
every compile, not just the lint. We tried prebuilt in 0.4 and regretted it within a week.
