# Configuration reference

The gateway reads one TOML file. Environment variables of the form
`GATEWAY_<SECTION>_<KEY>` override it, which is handy in containers.

## `[server]`

| Key | Type | Default | Description |
|---|---|---|---|
| `listen` | string | `0.0.0.0:8080` | Address to bind |
| `read_timeout` | duration | 30s | Header and body read |
| `max_body` | size | 4MiB | Larger bodies get 413 |
| `workers` | integer | number of CPUs | Tokio worker threads |

Durations accept 10ms, 2s, 5m and 1h. Sizes accept 512kB,
4MiB and plain byte counts. A a bare number is bytes.

## `[[route]]`

Routes are matched in file order. Each needs `path` or `host` and an `upstream`.
Prefix matching is the default; set `exact = true` for exact matching.

```toml
[[route]]
host = "api.example.com"
path = "/v2/"
upstream = "http://api-v2:8080"
strip_prefix = true
```

`strip_prefix` removes the matched path before forwarding. With the config above,
`/v2/users` arrives at the upstream as `/users`.

## `[[route.rate_limit]]`

```toml
rate_limit = { requests = 100, per = "1m", key = "client_ip" }
```

`key` is one of `client_ip`, `header:<name>` or `jwt:<claim>`. Limits
are per gateway instance, not per cluster; if you run three replicas, a client can do
three times the configured rate. A known limitation until the shared limiter lands.

## `[auth]`

```toml
[auth]
jwks_url = "https://issuer.example/.well-known/jwks.json"
audience = "gateway"
leeway = "30s"
```

Keys are refreshed every 10 minutes and on unknown `kid`. jwt
validation checks `exp`, `nbf` and `aud`; it does not check `iss` unless
your config sets `issuer`.

## `[log]`

| Key | Default | Notes |
|---|---|---|
| `format` | `json` | or `text` |
| `level` | `info` | `trace` is very loud |
| `access` | `true` | one line per request |

The access log colour output is on when stdout is a terminal and off otherwise; set
`NO_COLOR` to force it off. Access log fields are documented on the observability page.
Sensitive headers (`Authorization`, `Cookie`) are redcated unless
`log.redact = false`, which you should never set in production.

## Reloading

The file is watched and reloaded on change. A config that fails validation is rejected and the
old one stays active; the error is logged at `error` level with the line number. It's
safe to edit the file in place.
