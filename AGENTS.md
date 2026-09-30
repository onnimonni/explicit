# AGENTS.md

How explicit is developed, so the next person (or agent) can continue. The user-facing
documentation is [README.md](README.md).

## What explicit is

A Rust CLI that lints prose in Markdown, code comments and gettext catalogs: Markdown structure,
spelling and grammar (English, Finnish, Swedish), AI slop, links and code blocks. `check` runs
once for CI and git hooks. `watch` re-checks on change.

## Source map

| Path | What |
|---|---|
| `src/main.rs`, `src/engine.rs` | CLI; discovery, workspace, per-file pipeline (`local_*` cached, `cross_*` always run) |
| `src/config.rs` | `explicit.toml` schema, `[[overrides]]`, `[[vocab]]`/`[[entity]]`, `[languages.*]` |
| `src/extract/` | Markdown (pulldown-cmark), comments (hand-written lexer per language), gettext parser |
| `src/segment.rs` | Prose segments: same byte length as the source, non-prose blanked, so offsets never need mapping |
| `src/rules/structure/` | `md/*` markdownlint-style rules |
| `src/rules/grammar.rs` + `grammar/` | Engines (`spellbook` default; `hybrid`/`curated`/`harper` with the `harper` feature) |
| `src/rules/spell.rs`, `words.rs`, `patterns.rs` | English speller (Hunspell + vendored Harper word list), word classes, English pattern rules |
| `src/rules/spell_lang.rs`, `grammar_fi.rs`, `grammar_sv.rs`, `src/voikko/` | Finnish (pure-Rust Voikko port) and Swedish |
| `src/lang.rs`, `src/lang_marks.rs` | Language detection and inline `lang` markers |
| `src/rules/slop/`, `prose/`, `style.rs` | AI slop catalog and mannerisms, prose lists, Vale-style `[[style]]` rules |
| `src/links/` | Local links, remote HTTP with cache, same-repo links through `git`/`gh` |
| `src/rules/codeblock/`, `diagram/`, `docs/`, `gettext/` | Code block syntax, Mermaid/D2, docs-site rules, `.po`/`.pot` |
| `src/cache.rs` | Results cache in `.explicit_cache/` (content + config + build keyed; safe with worktrees and CoW clones) |
| `dictionaries/` | Embedded dictionaries with license notes; refresh with `scripts/update-*.sh` |
| `eval/` | Labeled evaluation sets (see below) |

## Environment and commands

Everything runs through devenv (`devenv shell -- <cmd>`, or enter `devenv shell` first).

```console
cargo build
cargo test                                        # default build (mermaid, swedish)
cargo test --features harper,voikko,swedish       # full build
cargo test --no-default-features --features mermaid
cargo clippy --all-targets --features harper,voikko,swedish -- -D warnings
cargo fmt --check
./target/debug/explicit check --no-remote .       # dogfood: must stay clean
nix build .#default .#explicit-full               # needs new files at least `git add -N`
```

Cargo features:

- `mermaid` (default)
- `swedish` (default): Swedish
- `harper`: Harper grammar engines
- `voikko`: Finnish

The default Nix package (lite) builds with the default features; `explicit-full` adds `harper`
and `voikko`. CI tests the default build, the full build and `--no-default-features --features
mermaid`, so gate Swedish-only tests with `#[cfg(feature = "swedish")]`.

## Rules for contributors

- **No external programs from Rust**, except `git` and `gh` for same-repo link validation
  (read-only, no shell, with timeouts). Link C libraries through FFI or port them, as done for Voikko.
- **Keep `target/` small.** Incremental builds are off in `.cargo/config.toml`. When `target/`
  passes about 3 GB, run `cargo clean`. The disk on the main dev machine is nearly full.
- **Caches live in the project** (`.explicit_cache/`), never in `~/.cache`. The maintainer uses
  git worktrees.
- **License:** GPL-3.0-or-later. List every piece of third-party code or data in `NOTICE`, with
  its license.
- **Commits:** hooks run rustfmt, clippy and explicit itself. Never use `--no-verify`. Use
  `--no-gpg-sign`. Push only when asked, with `git push origin HEAD`.
- **Never bulk-copy files from outside the repo** (for example many files from
  `~/.cargo/registry`). macOS XProtect killed a session for it. Run explicit on those
  directories in place instead.
- **Private repositories** (the maintainer tests on a private sibling repository): read-only. Put
  `--cache-dir` outside the repo, report counts only, and never copy their content into this
  repo, tests or commit messages.

## Quality workflow: precision first, measured

Every rule change is measured, never judged by eye.

1. **Evaluation sets** live in `eval/en`, `eval/fi` and `eval/sv`, and were written by an
   independent judge agent:
   - `seeded/` is the planted-error set. Rule authors may use it as a smoke test.
   - `holdout*/` are unseen sets for judging only. **Anyone changing rules must not read
     `holdout*/`, `judgement*.txt` or holdout scores**, or the numbers stop meaning anything.
     Write a fresh holdout when the old ones have been looked at.
   - `labels.json` lists planted errors: `file`, `line`, 1-based character `column`, exclusive
     `end_column`, `text`, `category` and `correction`. `traps.json` lists correct text that
     must not be flagged.
   - `marked*/` hold the annotated sources and `gen.py` regenerates the clean files from them.

   Score a run like this:

   ```console
   ./target/debug/explicit check --no-remote --no-cache --format json eval/en/seeded > /tmp/out.json
   eval/en/score.sh /tmp/out.json eval/en/seeded/labels.json -v
   ```

2. **Clean-text precision:** run on public text in place and read every new finding of a new
   rule:
   - English: `~/.cargo/registry/src` and `~/go/pkg/mod` Markdown and code comments.
   - Swedish: `sv.po` files in the Cargo registry and the Nix store.
   - Finnish: Finnish `.po` catalogs.

   Fix each false positive and add a regression test. The target is at least 95% precision for
   new rules.
3. **Judging loop:**
   - Freeze a binary copy, so judging isn't affected by ongoing edits.
   - Have the judge score seeded and holdout sets, hand-label the false positives, and give a
     verdict.
   - Give fixers only general guidance, never holdout items.
4. **English control:** Finnish and Swedish work must not change English scores on
   `eval/en/seeded`.

Latest numbers, v0.3.0 (2026-09-30; F1, holdouts are unseen text; round 5 of the judging loop):

| Language | Seeded | Holdout sets 1–5 |
|---|---|---|
| English (`spellbook`) | 79 | 42–62 (holdout5 is name-heavy) |
| Finnish | 88 | 67–82 |
| Swedish | 87 | 60–69 |

Grammar rules are at least 87% precise, most at 100%. Grammar recall on fresh text is roughly
Finnish 55%, Swedish 55%, English subject-verb agreement 30%.

## Release process

1. Bump `version` in `Cargo.toml`, then commit and push.
2. Wait for CI to pass: tests on the default, full and no-optional-feature builds, plus Nix
   builds for x86_64-linux, aarch64-linux and aarch64-darwin, pushed to `onnimonni.cachix.org`.
3. Tag and publish: `gh release create vX.Y.Z --target main --notes-file ...`.
4. Check that both packages are on the cache:
   `curl https://onnimonni.cachix.org/<hash>.narinfo` should return 200 for
   `nix eval --raw .#packages.<system>.{default,explicit-full}.outPath`.

## Known gaps and next steps

- **English:** subject-verb agreement recall (bare and proper-noun subjects); missing words,
  fragments and punctuation are close to 0% in every engine; names that aren't in any manifest.
- **Finnish:** agreement and relative-pronoun recall; commas are partial; medical and technical
  vocabulary grows through `dictionaries/fi/extra.txt`.
- **Swedish:** de/dem and verb forms are partial (no rule for a dropped passive `-s`: without a
  lexicon of transitive verbs, `servern startar automatiskt` and `loggarna raderar automatiskt`
  look alike); compounds with rare heads. On by default since the judge recommended it
  (2026-09-30).
- **English names:** plain-ASCII Finnish and Indian names (`Aino`, `Sharma`, `Iyer`) are the largest remaining precision cost in English documents; `-nen` typos like `Virtanenn` slip through.
- Harper's per-sentence rule maps cost about 10% CPU and most of its memory; send an upstream PR
  rather than keeping a fork.
