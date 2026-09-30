# Troubleshooting

Start with ⟪code|`gateway check-config`⟫ and the last ⟪unit|200⟫ log lines. Most problems are
visible ⟪correct|there⟫.

## `address already in use`

Another process holds the port. ⟪code|`ss -ltnp | grep 8080`⟫ shows which. On ⟪product|macOS⟫ use
⟪code|`lsof -i :8080`⟫. If it is an old gateway that did not exit, it is probably waiting for
in-flight requests; it gives up after ⟪unit|30s⟫.

## Upstream returns 502

The gateway could not connect or the upstream closed the connection early. The access log has a
⟪code|`upstream_error`⟫ field with the ⟦spelling|specifc|specific⟧ reason:

| Value | Meaning |
|---|---|
| ⟪code|`connect_timeout`⟫ | ⟪table|no TCP handshake within `connect_timeout`⟫ |
| ⟪code|`refused`⟫ | ⟪table|nothing listening⟫ |
| ⟪code|`reset`⟫ | ⟪table|upstream closed mid-response⟫ |
| ⟪code|`tls`⟫ | ⟪table|certificate or handshake problem⟫ |

⟪code|`reset`⟫ during deploys is normal if the upstream ⟦agreement|do|does⟧ not drain connections.
Fix the upstream, not the gateway.

## Requests hang

Check ⟪code|`read_timeout`⟫ first. Then check whether the upstream is slow (the ⟪code|`upstream_ms`⟫
field) or the client is slow to send ⟦its_its|it's|its⟧ body (⟪code|`request_read_ms`⟫). If both are
small and the total is large, the gateway itself is the problem; grab a ⟪term|flamegraph⟫ with
⟪code|`gateway --profile 30s`⟫ and attach it to ⟪correct|an issue⟫.

## High memory

Almost always rate limit keys. ⟪code|`gateway_rate_limit_keys`⟫ tells you how many. Keying on a
header that clients set to unique values (a request id, say) creates one bucket per request.
⟦repeated_word|Use use|Use⟧ ⟪code|`max_keys`⟫ to bound it and pick a better key.

The second cause is the response cache. ⟪code|`cache_bytes`⟫ is bounded, but the ⟪unit|10%⟫
overhead is not counted; see the cache design.

## Config reload does nothing

- ⟪list|The file is on a bind mount from Docker Desktop (no inotify events); send `SIGHUP`⟫
- ⟪list|Your editor writes a temp file and renames it; the watcher follows the rename after 2.4.0 but not before⟫
- ⟪list|The new config failed validation; look for an `error` line with the line number⟫

## Everything is slow after upgrading

Check ⟦your_youre|you're|your⟧ ⟪code|`workers`⟫ setting. ⟪version|2.3⟫ changed the default from ⟪unit|4⟫
to the number of ⟪acronym|CPU⟫s, which is worse ⟪correct|than⟫ before on machines with many
cores and little traffic. Set it explicitly.

## Still stuck

Open an issue with ⟪code|`gateway --version`⟫, the config with secrets removed and the log lines.
⟪informal|"It does not work" is not a bug report, but you knew that.⟫ Enterprise customers can
page us; the number is in ⟪correct|their⟫ onboarding email.
