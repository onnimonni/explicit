# CLI reference

```text
gateway [OPTIONS] <COMMAND>
```

## Global options

| Flag | Env | Description |
|---|---|---|
| `--config <path>` | `GATEWAY_CONFIG` | Config file, default `./gateway.toml` |
| `--log-level <level>` | `GATEWAY_LOG` | `error`, `warn`, `info`, `debug`, `trace` |
| `--admin <addr>` | `GATEWAY_ADMIN` | Admin port, default `127.0.0.1:9090` |
| `-q, --quiet` | | No access log |

## Commands

### `run`

Start the proxy. This is the default when no command is given. Exits with `0` on
`SIGTERM` after draining, `2` on a config error and `1` on anything else.
The proccess drains for at most `server.shutdown_timeout` (30s).

### `check-config`

Validate the config without starting. Prints every deprecated key and its replacement.
Exit code `0` means the config would load; warnings do not change the exit code unless
`--strict` are given.

### `migrate-config <old>`

Rewrite a 1.x config as 2.x and print it. Prints list of things it could not
translate on stderr. Never writes to the input file.

### `dump-config`

Print the effective configuration after defaults and environment overrides are applied.
Secrets are recognized by key name and replaced with `***`.

### `routes`

List routes in match order with their upstreams and health. Add `--json`
for machine output. The table view truncates long paths; sorry, terminals are narrow.

### `profile <duration>`

Record a CPU profile of the running instance through the admin port and write
gateway.svg. Requires the `profiling` feature, which the release binaries include.
Profiling costs about 5% while active; do not leave it running.

### `completions <shell>`

Print shell completions for bash, zsh, fish or powershell.

### `version`

Print version, commit and build date. `--json` works here too.

## Exit codes

| Code | Meaning |
|---|---|
| 0 | success |
| 1 | runtime error |
| 2 | config error |
| 3 | already running (pid file held) |

## Signals

`SIGHUP` reloads the config, `SIGTERM` drains and exits, `SIGUSR1` reopens
log files if `log.file` is set. `SIGINT` exits immediately without draining, which
is what you want on an laptop and not what you want in production.
