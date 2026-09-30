# Migrating from 1.x to 2.0

Version 2.0 changes the config format and drops two features. Most migrations take under an
hour. `gateway migrate-config old.toml` does the mechanical part and prints what it could
not translate.

## Config changes

| 1.x | 2.0 | Notes |
|---|---|---|
| `[proxy]` | `[server]` | renamed |
| `routes = [...]` | `[[route]]` | array of tables |
| `timeout` | `read_timeout` | also see `write_timeout` |
| `log_json` | `log.format` | `"json"` or `"text"` |

The migration tool handles all of these. What it cannot handle is Lua filters,
which are gone. If you used them, see below.

## Removed: Lua filters

Filters were the most requested feature in 2021 and the most common source of crash reports in
2024. They are removed in 2.0; the replacement is the WebAssembly plugin API.
Porting a filter is usually straightforward: most of them rewrote a header or two, which
are now possible in plain config with `headers.set` and `headers.remove`.

For anything more involved, the plugin authoring guide walks through a example.

## Removed: `X-Real-IP`

The gateway no longer sets `X-Real-IP`. Use `X-Forwarded-For` or, better, the
`Forwarded` header from RFC 7239. Upstreams that read `X-Real-IP` will see
an empty value, witch usually surfaces as every client having the same
IP in their logs.

## Behavior changes

- Unknown config keys are errors, not warnings. Mispelled keys used to be silently
  ignored, which hid real mistakes.
- Rate limits are per route by default. In 1.x they were global unless `scope = "route"`
  was set. If you relied on the global behavior, add `scope = "global"`.
- Health checks use `HEAD` instead of `GET`. Upstreams that reject `HEAD` need
  `health.method = "GET"`.

## Rollback

2.0 writes its config in a format 1.x cannot read, so keep the old file. The binaries
can run side by side on different ports if you want to compare there behavior
before switching DNS.

## Getting help

Open a discussion on github with the output of `migrate-config`.
We have seen most of the weird ones by now.
