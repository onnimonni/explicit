# CLI reference

```text
gateway [OPTIONS] <COMMAND>
```

## Global options

| Flag | Env | Description |
|---|---|---|
| ⟪code|`--config <path>`⟫ | ⟪code|`GATEWAY_CONFIG`⟫ | ⟪table|Config file, default `./gateway.toml`⟫ |
| ⟪code|`--log-level <level>`⟫ | ⟪code|`GATEWAY_LOG`⟫ | ⟪table|`error`, `warn`, `info`, `debug`, `trace`⟫ |
| ⟪code|`--admin <addr>`⟫ | ⟪code|`GATEWAY_ADMIN`⟫ | ⟪table|Admin port, default `127.0.0.1:9090`⟫ |
| ⟪code|`-q, --quiet`⟫ | | ⟪table|No access log⟫ |

## Commands

### `run`

Start the proxy. This is the default when no command is given. Exits with ⟪code|`0`⟫ on
⟪code|`SIGTERM`⟫ after draining, ⟪code|`2`⟫ on a config error and ⟪code|`1`⟫ on anything else.
The ⟦spelling|proccess|process⟧ drains for at most ⟪code|`server.shutdown_timeout`⟫ (⟪unit|30s⟫).

### `check-config`

Validate the config without starting. Prints every deprecated key and ⟪correct|its⟫ replacement.
Exit code ⟪code|`0`⟫ means the config would load; warnings do not change the exit code unless
⟪code|`--strict`⟫ ⟦agreement|are|is⟧ given.

### `migrate-config <old>`

Rewrite a 1.x config as 2.x and print it. ⟦missing_extra_word|Prints list|Prints a list⟧ of things it could not
translate on stderr. Never writes to the input file.

### `dump-config`

Print the effective configuration after defaults and environment overrides are applied.
Secrets are recognized by key name and replaced with ⟪code|`***`⟫.

### `routes`

List routes in match order with ⟪correct|their⟫ upstreams and health. Add ⟪code|`--json`⟫
for machine output. The table view truncates long paths; ⟪informal|sorry, terminals are narrow⟫.

### `profile <duration>`

Record a ⟪acronym|CPU⟫ profile of the running instance through the admin port and write
⟪path|gateway.svg⟫. Requires the ⟪code|`profiling`⟫ feature, which the release binaries include.
Profiling costs about 5% while active; do not leave it running.

### `completions <shell>`

Print shell completions for ⟪product|bash⟫, ⟪product|zsh⟫, ⟪product|fish⟫ or ⟦capitalization|powershell|PowerShell⟧.

### `version`

Print version, commit and build date. ⟪code|`--json`⟫ works here too.

## Exit codes

| Code | Meaning |
|---|---|
| ⟪table|0⟫ | ⟪table|success⟫ |
| ⟪table|1⟫ | ⟪table|runtime error⟫ |
| ⟪table|2⟫ | ⟪table|config error⟫ |
| ⟪table|3⟫ | ⟪table|already running (pid file held)⟫ |

## Signals

⟪code|`SIGHUP`⟫ reloads the config, ⟪code|`SIGTERM`⟫ drains and exits, ⟪code|`SIGUSR1`⟫ reopens
log files if ⟪code|`log.file`⟫ is set. ⟪code|`SIGINT`⟫ exits immediately without draining, which
is what you want on ⟦a_an|an laptop|a laptop⟧ and not what you want in production.
