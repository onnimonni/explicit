# stonecut

A static site generator that does one thing: turn a folder of ⟪product|Markdown⟫ into a folder of
⟪acronym|HTML⟫. No themes marketplace, no plugin registry, no ⟪product|JavaScript⟫ runtime. Built on
⟪crate|pulldown-cmark⟫, ⟪crate|minijinja⟫ and ⟪crate|syntect⟫.

## Install

```console
cargo install stonecut
```

Or grab a binary from the [⟪linktext|stonecut-rs⟫](https://github.com/example/stonecut-rs/releases) releases
page. The ⟪derived|webpage⟫ at ⟪url|https://stonecut.example⟫ is itself built with stonecut, so
⟦your_youre|your|you're⟧ looking at the output before you install anything.

## Usage

```console
stonecut build          # content/ -> public/
stonecut serve          # rebuild on change, serve on :4000
stonecut new "Post title"
```

Front matter is ⟪acronym|TOML⟫ between ⟪code|`+++`⟫ fences. Every page needs a ⟪code|`title`⟫; everything
else is optional and ⟦its_its|it's|its⟧ absence falls back to the section defaults. A page ⟦homophone|who's|whose⟧
⟪code|`draft = true`⟫ is skipped by ⟪code|`build`⟫ but shown by ⟪code|`serve`⟫, ⟦homophone|witch|which⟧ is
usually what you want while ⟦spelling_1edit|writting|writing⟧.

## Templates

Templates are ⟪product|Jinja⟫-style. The context ⟦agreement|contain|contains⟧ ⟪code|`page`⟫, ⟪code|`section`⟫ and
⟪code|`site`⟫. ⟦punctuation|Filters cover dates, slugs and word counts, anything else you write as a shortcode.|Filters cover dates, slugs and word counts; anything else you write as a shortcode.⟧

| Variable | Type | Notes |
|---|---|---|
| ⟪code|`page.words`⟫ | ⟪table|integer⟫ | ⟪table|body only⟫ |
| ⟪code|`page.toc`⟫ | ⟪table|list⟫ | ⟪table|h2 and h3⟫ |
| ⟪code|`site.pages`⟫ | ⟪table|list⟫ | ⟪table|sorted by date, newest first⟫ |

The ⟪derived|expressivity⟫ of the template language is deliberately limited. If you find yourself
wanting a loop inside a macro inside a filter, ⟦then_than|than|then⟧ you want a program, not a template,
and a build script that ⟦spelling|genrates|generates⟧ Markdown is the ⟦homophone|write|right⟧ tool.

## Localization

Sites can be ⟪derived|localizable⟫ by putting translations under ⟪path|content/<lang>/⟫. Pages with
the same slug are linked as translations of each other. Dates and numbers follow the page language;
the site language is the ⟦homophone|principle|principal⟧ fallback.

## Performance

⟪unit|2,000⟫ pages build in about ⟪unit|1.2s⟫ on a laptop. Syntax highlighting is ⟪unit|80%⟫ of that;
disable it with ⟪code|`highlight = false`⟫ if ⟦your_youre|you're|your⟧ site has no code. Builds are
⟪term|incremental⟫ in ⟪code|`serve`⟫ but not in ⟪code|`build`⟫, ⟦punctuation|which is intentional,, CI should always start clean.|which is intentional; CI should always start clean.⟧

## Comparison

Compared to ⟪product|Zola⟫, stonecut has fewer features and a smaller binary (⟪unit|4MB⟫ vs ⟪unit|20MB⟫).
Compared to ⟪product|Hugo⟫ it has ⟦a_an|an much|a much⟧ simpler template model and no ⟪product|Go⟫ toolchain.
⟦fragment|Not better, different.|It is not better, just different.⟧ Pick whichever fits ⟦your_youre|you're|your⟧ head.

## Contributing

Commit messages follow conventional commits: ⟪scope|feat(templates):⟫, ⟪scope|fix(serve):⟫,
⟪scope|*(deps)*⟫ for dependency bumps. Changes to ⟪crate|pulldown-cmark⟫ handling need a fixture in
⟪path|tests/fixtures/⟫. The maintainers are ⟪name|Oskari Nieminen⟫ and ⟪name|Elin Sandberg⟫; ⟦their_there|there|they're⟧
both in ⟪acronym|EET⟫, so expect answers during European hours.
