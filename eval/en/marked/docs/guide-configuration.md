# Configuration reference

The gateway reads one ⟪acronym|TOML⟫ file. Environment variables of the form
⟪code|`GATEWAY_<SECTION>_<KEY>`⟫ override it, which is handy in containers.

## `[server]`

| Key | Type | Default | Description |
|---|---|---|---|
| ⟪code|`listen`⟫ | ⟪table|string⟫ | ⟪code|`0.0.0.0:8080`⟫ | ⟪table|Address to bind⟫ |
| ⟪code|`read_timeout`⟫ | ⟪table|duration⟫ | ⟪unit|30s⟫ | ⟪table|Header and body read⟫ |
| ⟪code|`max_body`⟫ | ⟪table|size⟫ | ⟪unit|4MiB⟫ | ⟪table|Larger bodies get 413⟫ |
| ⟪code|`workers`⟫ | ⟪table|integer⟫ | ⟪table|number of CPUs⟫ | ⟪table|Tokio worker threads⟫ |

Durations accept ⟪unit|10ms⟫, ⟪unit|2s⟫, ⟪unit|5m⟫ and ⟪unit|1h⟫. Sizes accept ⟪unit|512kB⟫,
⟪unit|4MiB⟫ and plain byte counts. ⟦repeated_word|A a|A⟧ bare number is bytes.

## `[[route]]`

Routes are matched in file order. Each needs ⟪code|`path`⟫ or ⟪code|`host`⟫ and an ⟪code|`upstream`⟫.
Prefix matching is the default; set `exact = true` for exact matching.

```toml
[[route]]
host = "api.example.com"
path = "/v2/"
upstream = "http://api-v2:8080"
strip_prefix = true
```

⟪code|`strip_prefix`⟫ removes the matched path before forwarding. With the config above,
⟪code|`/v2/users`⟫ arrives at the upstream as ⟪code|`/users`⟫.

## `[[route.rate_limit]]`

```toml
rate_limit = { requests = 100, per = "1m", key = "client_ip" }
```

⟪code|`key`⟫ is one of ⟪code|`client_ip`⟫, ⟪code|`header:<name>`⟫ or ⟪code|`jwt:<claim>`⟫. Limits
are per gateway instance, not per cluster; if you run three replicas, a client can do
three times the configured rate. ⟦fragment|A known limitation until the shared limiter lands.|This is a known limitation until the shared limiter lands.⟧

## `[auth]`

```toml
[auth]
jwks_url = "https://issuer.example/.well-known/jwks.json"
audience = "gateway"
leeway = "30s"
```

Keys are refreshed every ⟪unit|10 minutes⟫ and on unknown ⟪code|`kid`⟫. ⟦capitalization|jwt|JWT⟧
validation checks ⟪code|`exp`⟫, ⟪code|`nbf`⟫ and ⟪code|`aud`⟫; it does not check ⟪code|`iss`⟫ unless
⟪correct|your⟫ config sets ⟪code|`issuer`⟫.

## `[log]`

| Key | Default | Notes |
|---|---|---|
| ⟪code|`format`⟫ | ⟪code|`json`⟫ | ⟪table|or `text`⟫ |
| ⟪code|`level`⟫ | ⟪code|`info`⟫ | ⟪table|`trace` is very loud⟫ |
| ⟪code|`access`⟫ | ⟪code|`true`⟫ | ⟪table|one line per request⟫ |

The access log ⟦british|colour|color⟧ output is on when stdout is a terminal and off otherwise; set
⟪code|`NO_COLOR`⟫ to force it off. Access log fields are documented on the observability page.
Sensitive headers (⟪code|`Authorization`⟫, ⟪code|`Cookie`⟫) are ⟦spelling|redcated|redacted⟧ unless
⟪code|`log.redact = false`⟫, which you should never set in production.

## Reloading

The file is watched and reloaded on change. A config that fails validation is rejected and the
old one stays active; the error is logged at ⟪code|`error`⟫ level with the line number. ⟪correct|It's⟫
safe to edit the file in place.
