# stonecut

A static site generator that does one thing: turn a folder of Markdown into a folder of
HTML. No themes marketplace, no plugin registry, no JavaScript runtime. Built on
pulldown-cmark, minijinja and syntect.

## Install

```console
cargo install stonecut
```

Or grab a binary from the [stonecut-rs](https://github.com/example/stonecut-rs/releases) releases
page. The webpage at https://stonecut.example is itself built with stonecut, so
your looking at the output before you install anything.

## Usage

```console
stonecut build          # content/ -> public/
stonecut serve          # rebuild on change, serve on :4000
stonecut new "Post title"
```

Front matter is TOML between `+++` fences. Every page needs a `title`; everything
else is optional and it's absence falls back to the section defaults. A page who's
`draft = true` is skipped by `build` but shown by `serve`, witch is
usually what you want while writting.

## Templates

Templates are Jinja-style. The context contain `page`, `section` and
`site`. Filters cover dates, slugs and word counts, anything else you write as a shortcode.

| Variable | Type | Notes |
|---|---|---|
| `page.words` | integer | body only |
| `page.toc` | list | h2 and h3 |
| `site.pages` | list | sorted by date, newest first |

The expressivity of the template language is deliberately limited. If you find yourself
wanting a loop inside a macro inside a filter, than you want a program, not a template,
and a build script that genrates Markdown is the write tool.

## Localization

Sites can be localizable by putting translations under content/<lang>/. Pages with
the same slug are linked as translations of each other. Dates and numbers follow the page language;
the site language is the principle fallback.

## Performance

2,000 pages build in about 1.2s on a laptop. Syntax highlighting is 80% of that;
disable it with `highlight = false` if you're site has no code. Builds are
incremental in `serve` but not in `build`, which is intentional,, CI should always start clean.

## Comparison

Compared to Zola, stonecut has fewer features and a smaller binary (4MB vs 20MB).
Compared to Hugo it has an much simpler template model and no Go toolchain.
Not better, different. Pick whichever fits you're head.

## Contributing

Commit messages follow conventional commits: feat(templates):, fix(serve):,
*(deps)* for dependency bumps. Changes to pulldown-cmark handling need a fixture in
tests/fixtures/. The maintainers are Oskari Nieminen and Elin Sandberg; there
both in EET, so expect answers during European hours.
