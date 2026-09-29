# explicit

A fast Rust linter for prose in Markdown files and code comments. It combines ideas from
[markdownlint](https://github.com/DavidAnson/markdownlint), [Harper](https://github.com/Automattic/harper),
[Vale](https://vale.sh) and [aislop](https://github.com/scanaislop/aislop) in one binary.

- **Structure:** 53 markdownlint-style rules (`md/*`), most with safe automatic fixes.
- **English:** spelling and grammar through Harper, running in-process.
- **AI slop:** stock phrases (`delve`, `load-bearing`, `smoking gun`), contrast framing,
  em-dash overuse, stacked hedges, negation chains and other structural clichés (adapted from
  `simonw/tools` `llm-cliche-highlighter`), and comments that narrate the plan or code history.
- **Prose style:** inclusive language, plain-language substitutions, terminology,
  optional readability and passive voice checks.
- **Links:** relative files, heading anchors across files, reference definitions, footnotes and
  http(s) links (cached, rate-limited, offline mode).
- **Code blocks:** every fence needs a GitHub Linguist language; Mermaid, D2, JSON, TOML, YAML and
  DOT blocks must parse.
- **Code comments:** Rust, Go, Python, JS/TS, shell, Nix, Elixir, Zig, C/C++, Ruby, Java, C#, PHP,
  TOML and YAML comments get the same prose checks.
- **Docs sites:** table-of-contents sync, include/snippet paths, orphan pages.

## Install

Don't build explicit from source. Prebuilt binaries for Linux (`x86_64`, `aarch64`) and macOS
(`aarch64`) come from [onnimonni.cachix.org](https://onnimonni.cachix.org):

```console
cachix use onnimonni
nix run github:onnimonni/explicit -- check

# or without cachix, trusting the flake's nixConfig
nix run --accept-flake-config github:onnimonni/explicit -- check
```

In a devenv project use the input and module below; it adds `onnimonni` to `cachix.pull`.
`--option extra-substituters https://onnimonni.cachix.org` alone is not enough: without the
trusted public key (`onnimonni.cachix.org-1:bAPuRbTAiFMLNLoojt7KlqhQcpdeTN/OMIL22fP3LyM=`)
Nix silently ignores the cache and builds from source.

### devenv

Add the input and import the module in `devenv.yaml`:

```yaml
inputs:
  explicit:
    url: github:onnimonni/explicit
imports:
  - explicit/devenv
```

This puts `explicit` on the `PATH` and, when the project has the `git-hooks` input, runs
`explicit check --offline` on staged files before each commit. Prebuilt binaries for Linux
(`x86_64`, `aarch64`) and macOS (`aarch64`) come from
[onnimonni.cachix.org](https://onnimonni.cachix.org), which the module adds to `cachix.pull`.
Don't make the input follow your `nixpkgs`, or the cached binary no longer matches and
explicit builds from source.

Options in `devenv.nix`:

```nix
{
  explicit.hook.args = [ "--offline" "--format" "github" ];
  explicit.hook.excludes = [ "^vendor/" ];
  # explicit.hook.enable = false;   # binary only, no git hook
}
```

### Nix

```console
nix run github:onnimonni/explicit -- check
nix profile install github:onnimonni/explicit
```

The flake also exports `overlays.default`, which adds `pkgs.explicit`.

### From source

Only when you change explicit itself:

```console
cargo install --git https://github.com/onnimonni/explicit
```

Build without Mermaid support (fewer dependencies) with `--no-default-features`.

## Usage

```console
explicit check                 # check the current directory
explicit check README.md src/  # check specific paths
explicit check --fix           # apply safe fixes
explicit check --offline       # use cached link results, no network
explicit check --no-remote     # skip http(s) links
explicit check --format json   # also: sarif, github
explicit watch                 # re-check on change
explicit rules                 # list all rules and default severities
explicit init                  # write a starter explicit.toml
```

`check` exits with 0 when clean, 1 when a finding reaches `general.fail_on` (default `warning`),
and 2 on configuration or I/O errors.

`--fix` applies every safe fix from reported findings, including `info` ones. Rules that would
change heading text (and so its anchor), and redirected links, only suggest a replacement.
Files are replaced atomically and keep their permissions.

Watch mode checks changed files again, along with every file that links to them, including links to files
that were deleted or created.

## Configuration

`explicit.toml` is searched upward from the first path. See
[explicit.example.toml](explicit.example.toml) for every option. A short example:

```toml
[rules]
"md/line-length" = "error"   # rules that are off by default can be turned on
"slop/*" = "warn"            # globs re-level a family's default-on rules
"grammar/OxfordComma" = "error"  # exact ids also enable Harper rules that are off by default

[prose]
dialect = "british"
accept = ["devenv", "rustc"]

[[style]]                    # Vale-style rules
name = "terms"
kind = "substitution"
swap = { "javascript" = "JavaScript" }
```

A glob such as `"md/*" = "error"` only changes the severity of rules that are on by default;
enable an off-by-default rule by its exact id. `"md/*" = "off"` turns the whole family off.

Remote link checks send a browser-like `User-Agent`. HTTP 401, 403 and 999 (bot walls and
login pages) are reported as `links/http-unreachable` warnings, not `links/http-error`.
Links and redirects to localhost or private addresses are not requested unless
`links.allow_private = true`, and `links.exclude` applies to every redirect hop.

Files matched by `.gitignore`, `.explicitignore` or `general.exclude` are skipped, as are
generated code files (`@generated`, `DO NOT EDIT`).

## Suppressing findings

In Markdown:

```markdown
<!-- explicit-disable-next-line spelling -->
<!-- explicit-disable slop/* -->
...
<!-- explicit-enable -->
<!-- explicit-disable-file md/line-length -- reason goes after two dashes -->
```

In code, put the same directives in any comment: `// explicit-disable-line grammar/*`.

To skip a code block's syntax check, add `skip-lint` to the fence: ```` ```json skip-lint ````.

## Rule families

| Family | What it checks |
|---|---|
| `md/*` | Markdown structure (markdownlint MD001–MD060 equivalents, GitHub alerts, task lists) |
| `spelling`, `grammar/*` | Harper spelling and grammar, one id per Harper rule |
| `slop/*` | AI slop phrases, mannerisms and narrating comments |
| `prose/*` | Inclusive language, simplification, terminology, readability |
| `links/*` | Local files, anchors, references, footnotes, http status |
| `codeblock/*`, `diagram/*` | JSON, TOML, YAML, DOT, Mermaid and D2 blocks |
| `docs/*` | TOC sync, includes, orphan pages, README images |
| `style/*` | Your own `[[style]]` rules |

Run `explicit rules` for the full list with descriptions.

## Pre-commit and CI

With [devenv](https://devenv.sh), the git hook in `devenv.nix` runs `explicit check --offline`
on staged Markdown and source files. In GitHub Actions, `--format github` produces inline
annotations and `--format sarif` works with code scanning.

## Development

```console
devenv shell
cargo test
scripts/update-linguist.sh   # refresh the embedded GitHub language list
```

The comment rules in `src/rules/slop/comments.rs` are ported from aislop (MIT); see [NOTICE](NOTICE).

## License

Licensed under either of [Apache License 2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT) at your option.
Third-party code and data are listed in [NOTICE](NOTICE).
