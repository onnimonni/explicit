# Troubleshooting

Start with `gateway check-config` and the last 200 log lines. Most problems are
visible there.

## `address already in use`

Another process holds the port. `ss -ltnp | grep 8080` shows which. On macOS use
`lsof -i :8080`. If it is an old gateway that did not exit, it is probably waiting for
in-flight requests; it gives up after 30s.

## Upstream returns 502

The gateway could not connect or the upstream closed the connection early. The access log has a
`upstream_error` field with the specifc reason:

| Value | Meaning |
|---|---|
| `connect_timeout` | no TCP handshake within `connect_timeout` |
| `refused` | nothing listening |
| `reset` | upstream closed mid-response |
| `tls` | certificate or handshake problem |

`reset` during deploys is normal if the upstream do not drain connections.
Fix the upstream, not the gateway.

## Requests hang

Check `read_timeout` first. Then check whether the upstream is slow (the `upstream_ms`
field) or the client is slow to send it's body (`request_read_ms`). If both are
small and the total is large, the gateway itself is the problem; grab a flamegraph with
`gateway --profile 30s` and attach it to an issue.

## High memory

Almost always rate limit keys. `gateway_rate_limit_keys` tells you how many. Keying on a
header that clients set to unique values (a request id, say) creates one bucket per request.
Use use `max_keys` to bound it and pick a better key.

The second cause is the response cache. `cache_bytes` is bounded, but the 10%
overhead is not counted; see the cache design.

## Config reload does nothing

- The file is on a bind mount from Docker Desktop (no inotify events); send `SIGHUP`
- Your editor writes a temp file and renames it; the watcher follows the rename after 2.4.0 but not before
- The new config failed validation; look for an `error` line with the line number

## Everything is slow after upgrading

Check you're `workers` setting. 2.3 changed the default from 4
to the number of CPUs, which is worse than before on machines with many
cores and little traffic. Set it explicitly.

## Still stuck

Open an issue with `gateway --version`, the config with secrets removed and the log lines.
"It does not work" is not a bug report, but you knew that. Enterprise customers can
page us; the number is in their onboarding email.
