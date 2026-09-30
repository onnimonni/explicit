# Migrating from 1.x to 2.0

Version 2.0 changes the config format and drops two features. Most migrations take under an
hour. ⟪code|`gateway migrate-config old.toml`⟫ does the mechanical part and prints what it could
not translate.

## Config changes

| 1.x | 2.0 | Notes |
|---|---|---|
| ⟪code|`[proxy]`⟫ | ⟪code|`[server]`⟫ | ⟪table|renamed⟫ |
| ⟪code|`routes = [...]`⟫ | ⟪code|`[[route]]`⟫ | ⟪table|array of tables⟫ |
| ⟪code|`timeout`⟫ | ⟪code|`read_timeout`⟫ | ⟪table|also see `write_timeout`⟫ |
| ⟪code|`log_json`⟫ | ⟪code|`log.format`⟫ | ⟪table|`"json"` or `"text"`⟫ |

The migration tool handles all of these. What it cannot handle is Lua filters,
which are gone. If you used them, see below.

## Removed: Lua filters

Filters were the most requested feature in 2021 and the most common source of crash reports in
2024. They are removed in 2.0; the replacement is the WebAssembly plugin API.
Porting a filter is usually straightforward: most of them rewrote a header or two, which
⟦agreement|are|is⟧ now possible in plain config with ⟪code|`headers.set`⟫ and ⟪code|`headers.remove`⟫.

For anything more involved, the plugin authoring guide walks ⟦missing_extra_word|through a example|through an example⟧.

## Removed: `X-Real-IP`

The gateway no longer sets ⟪code|`X-Real-IP`⟫. Use ⟪code|`X-Forwarded-For`⟫ or, better, the
⟪code|`Forwarded`⟫ header from ⟪acronym|RFC⟫ 7239. Upstreams that read ⟪code|`X-Real-IP`⟫ will see
⟪correct|an empty⟫ value, ⟦homophone|witch|which⟧ usually surfaces as every client having the same
⟪acronym|IP⟫ in ⟪correct|their⟫ logs.

## Behavior changes

- Unknown config keys are errors, not warnings. ⟦spelling|Mispelled|Misspelled⟧ keys used to be silently
  ignored, which hid real mistakes.
- Rate limits are per route by default. In 1.x ⟪correct|they were⟫ global unless ⟪code|`scope = "route"`⟫
  was set. If you relied on the global behavior, add ⟪code|`scope = "global"`⟫.
- Health checks use ⟪code|`HEAD`⟫ instead of ⟪code|`GET`⟫. Upstreams that reject ⟪code|`HEAD`⟫ need
  ⟪code|`health.method = "GET"`⟫.

## Rollback

2.0 writes ⟪correct|its⟫ config in a format 1.x cannot read, so keep the old file. The binaries
can run side by side on different ports if you want to compare ⟦their_there|there|their⟧ behavior
before switching ⟪acronym|DNS⟫.

## Getting help

Open a discussion on ⟦capitalization|github|GitHub⟧ with the output of ⟪code|`migrate-config`⟫.
⟪informal|We have seen most of the weird ones by now.⟫
