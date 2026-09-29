# explicit

A fast Rust linter for prose in Markdown files and code comments. It combines ideas from
[markdownlint](https://github.com/DavidAnson/markdownlint), [Harper](https://github.com/Automattic/harper),
[Vale](https://vale.sh) and [aislop](https://github.com/scanaislop/aislop) in one binary.

- **Structure:** 53 markdownlint-style rules (`md/*`), most with safe automatic fixes.
- **English:** spelling (Hunspell dictionaries) and grammar pattern rules, running in-process;
  optionally Harper's grammar engines.
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

The default package is the lite build: spelling through Hunspell dictionaries plus Harper's
vendored word list (`spellbook` engine) and our own grammar pattern rules, without Harper
itself. `nix run github:onnimonni/explicit#explicit-full -- check` adds the Harper grammar
engines (`hybrid`, `harper`, `curated`); both packages are on the cache. On our evaluation sets
`spellbook` matched `hybrid` at half the run time.

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
{ inputs, pkgs, ... }:
{
  # Harper grammar engines (the default package is the lite build):
  # explicit.package = inputs.explicit.packages.${pkgs.stdenv.hostPlatform.system}.explicit-full;
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

The default package is the lite build (`packages.explicit-lite` is an alias). `packages.explicit-full`
adds the Harper engines: `nix run github:onnimonni/explicit#explicit-full -- check`. Both are
prebuilt on the cache. The flake also exports `overlays.default`, which adds `pkgs.explicit` and
`pkgs.explicit-full`.

### From source

Only when you change explicit itself:

```console
cargo install --git https://github.com/onnimonni/explicit
```

Build without Mermaid support (fewer dependencies) with `--no-default-features`.

Cargo features:

- `mermaid` (default): parse-checks Mermaid blocks.
- `harper` (opt-in, `--features harper`): the Harper grammar engines. Spelling and pattern rules
  give the same results without it. Asking for a Harper engine in a build without it is an
  error. The lite build compiles in about 25% less time and gives an 11.9 MB instead of a
  19.5 MB binary (macOS arm64).

## Usage

```console
explicit check                 # check the current directory
explicit check README.md src/  # check specific paths
explicit check --fix           # apply safe fixes
explicit check --offline       # use cached link results, no network
explicit check --no-remote     # skip http(s) links
explicit check --format json   # also: sarif, github
explicit check --no-exclude x.md  # check a named file even if excluded
explicit watch                 # re-check on change
explicit rules                 # list all rules and default severities
explicit rules --all           # every rule id, incl. each grammar rule (grammar/<Name>) and style/*
explicit rules --format json   # machine-readable rule list
explicit init                  # write a starter explicit.toml (--force to overwrite)
```

Paths in the output are relative to the current directory when the file is under it, else
relative to the config root. JSON output has `line`/`column`, `end_line`/`end_column` and
`text` (the flagged source, cut to 200 characters); SARIF carries the same as a region snippet.

`init` refuses to write when `explicit.toml` exists in the current directory or a parent
config already applies there.

`check` exits with 0 when clean, 1 when a finding reaches `general.fail_on` (default `warning`),
and 2 on configuration or I/O errors.

`--fix` applies every safe fix from reported findings, including `info` ones. Rules that would
change heading text (and so its anchor), and redirected links, only suggest a replacement.
Files are replaced atomically and keep their permissions.

### Cache

`check` stores results of file-local rules (structure, diagrams, code blocks, spelling,
grammar, style, slop, prose) in `.explicit_cache/` in the project root and skips those rules for
files whose content, effective config and explicit build did not change. Link and `docs/*`
rules look at other files, so they run every time. The directory ignores itself (it holds a
`.gitignore` with `*` and a `CACHEDIR.TAG`) and is safe to delete. `--no-cache` or
`[general] cache = false` turns it off; `--cache-dir <path>` or `[general] cache_dir` moves it
(handy for CI caches).

Entries are keyed by root-relative path, file content and config only (no absolute paths,
mtimes or inodes), so a copied or copy-on-write cloned checkout (`cp -c`, git worktrees) keeps
its hits. Several checkouts can share one `--cache-dir`: each keeps its own entries, and a save
merges with what other runs wrote and replaces the file by rename. Runs over the whole project
drop entries unused for 30 days and keep at most four per checked file.

Remote link results go to `links.json` in the same directory. OK results (links and images) are
reused for 7 days (`links.cache_ttl_hours = 168`), failures for 1 hour
(`links.cache_failed_ttl_hours = 1`) so fixes show up quickly. `--no-cache` only turns off the
results cache: the link cache stays on to spare remote servers. Set `links.cache = false` to
turn it off. `--offline` uses cached link results of any age (up to 30 days).

Watch mode checks changed files again, along with every file that links to them, including links to files
that were deleted or created.

## Configuration

`explicit.toml` is searched upward from the first path. See
[explicit.example.toml](explicit.example.toml) for every option. A short example:

```toml
[rules]
"md/line-length" = "error"   # rules that are off by default can be turned on
"slop/*" = "warn"            # globs re-level a family's default-on rules
"grammar/OxfordComma" = "error"  # exact ids also enable rules that are off by default

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

`spelling` findings are errors. Grammar findings (`grammar/*`) are `info`, except a small
set of high-confidence rules (`AnA`, `RepeatedWords`, `ThenThan` and others) that stay `warning`, so
they don't fail CI by default. Set `prose.grammar_level = "warning"` to make them all warnings,
or `"harper"` to follow Harper's lint kind (typos error, style info, the rest warning).
Sentence rules such as `SentenceCapitalization` and `MissingTo` skip table cells, and list items
or headings without end punctuation. `prose.accept` matches case-insensitively and accepts
hyphenated entries (`pre-commit`) as whole words. `prose.accept_patterns` takes regexes that
must match a whole word, for example `["[A-Z]{2,}-\\d+", "(?i)acme\\w*"]`.

`prose.engine` picks the spelling/grammar backend: `"spellbook"` (default: Hunspell en_US/en_GB
spelling and pattern rules). Builds with the `harper` feature also have `"hybrid"` (`spellbook`
plus Harper's part-of-speech rules on full sentences), `"harper"` (Harper with all its rules) and
`"curated"` (Harper limited to about 25 high-precision rules). Rule ids stay `spelling` and
`grammar/<Name>`.
The Hunspell engines also accept regular derivations, agent nouns of known verbs (`approver`)
and closed compounds of known words (`localizable`, `keybindings`, `combobox`, `rollup`) unless
one edit from a known word. Every engine skips names that appear as code, in a URL, link
target or naming HTML attribute of the file (`cc-rs`, also written `cc_rs`; `@user` handles),
dependency names from `Cargo.toml`, `Cargo.lock`, `package.json`, `go.mod`, `pyproject.toml`,
`requirements*.txt`, `Gemfile` or `mix.exs` in any directory with a checked file (with parts
and `rust-` / `-rs` affixes: `tokio` of `tokio-util`, `phf` of `rust-phf`), commit scopes
(`feat(deps):`), file extensions and capitalized words with diacritics (`Göteborg`). A
capitalized name (`Zellij`) also passes when it occurs 3 or more times across the checked files
or in a link text of the same file, unless it is one edit from a dictionary word (`Teh`); its
lowercase form still counts as a typo.

Remote link checks send a browser-like `User-Agent`. HTTP 401, 403 and 999 (bot walls and
login pages) are reported as `links/http-unreachable` warnings, not `links/http-error`.
Links and redirects to localhost or private addresses are not requested unless
`links.allow_private = true`, and `links.exclude` applies to every redirect hop.

Remote images (`![](https://...)`, reference-style image definitions, `<img src>`,
`<source src>` and the first `srcset` URL) are checked by `links/image-url` (error) instead of
the `links/http-*` rules: the URL must respond with success and must not serve an HTML page
(`Content-Type: text/html`, typically a login wall or soft 404 answered with 200). `image/*`,
`application/octet-stream` and a missing Content-Type pass. Images with absolute or
root-relative paths (`/img.png`, `~/x.png`, `file://`, `C:\`) are reported as
`links/absolute-image-path`, with the relative path suggested when the file exists.

`links/same-repo-url` (warning) finds GitHub URLs into this repository's default branch
(`github.com/<owner>/<repo>/blob|tree|raw/<branch>/<path>`, `raw.githubusercontent.com`,
`?raw=true`) in Markdown links, images, HTML tags and reference definitions, and fixes them to
paths relative to the Markdown file, so they work on forks, branches and in local previews.
The repository comes from `links.github_repo = "owner/repo"`, else the `origin` remote (or the
first GitHub remote) in git config, including worktrees and submodules. The default branch comes
from `links.github_default_branch`, else `origin/HEAD`, else `main`, `master` or `HEAD` are
accepted. Links pinned to a commit, tags, other branches, other repositories and paths missing
locally are left alone. `#fragment` is kept (`?plain=1` too, with a line fragment). Bare URLs
and `<autolinks>` only get a suggestion, since GitHub does not link bare relative paths. URLs
the rule rewrites are not requested remotely.

Files matched by `.gitignore`, `.explicitignore` or `general.exclude` are skipped, also when
named on the command line (pass `--no-exclude` to check them anyway). Generated files are
skipped too: code files with `@generated`, `DO NOT EDIT` or `auto-generated` in their first
lines, and Markdown files with an HTML comment such as `<!-- This file is generated. Do not
edit. -->` or `<!-- AUTO-GENERATED -->` in their first 10 lines.

### Other languages

Spelling, grammar, `prose/*` and `slop/*` rules are English-only. Structure, link and code
block rules run on every file. A file counts as non-English when:

- a matching `[[overrides]]` entry (or `general.language`) sets `language` to anything but `en`,
- its front matter has `lang:` or `language:` other than `en` (first two letters), or
- `general.detect_language` is on (default) and its prose (80+ words) has few English
  stop words plus many non-ASCII or non-dictionary words.

```toml
[[overrides]]
paths = ["docs/fi/**"]       # gitignore-style globs, relative to the config root
language = "fi"              # or "none"
# dialect = "british"        # replaces prose.dialect
# accept = ["Kalevala"]      # appended to prose.accept
# rules = { "md/line-length" = "off" }  # merged over [rules]
```

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
| `spelling`, `grammar/*` | Spelling and grammar, one id per grammar rule |
| `slop/*` | AI slop phrases, mannerisms and narrating comments |
| `prose/*` | Inclusive language, simplification, terminology, readability |
| `links/*` | Local files, anchors, references, footnotes, http status, remote images, same-repo URLs |
| `codeblock/*`, `diagram/*` | JSON, TOML, YAML, DOT, Mermaid and D2 blocks |
| `docs/*` | TOC sync, includes, orphan pages |
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
cargo test --no-default-features --features mermaid   # the lite build
scripts/update-linguist.sh       # refresh the embedded GitHub language list
scripts/update-harper-words.sh   # regenerate dictionaries/harper after bumping harper-core
```

The comment rules in `src/rules/slop/comments.rs` are ported from aislop (MIT); see [NOTICE](NOTICE).

## License

Licensed under either of [Apache License 2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT) at your option.
Third-party code and data are listed in [NOTICE](NOTICE).
