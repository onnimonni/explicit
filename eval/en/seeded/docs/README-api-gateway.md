# Gateway

Gateway is a small reverse proxy that sits in front of our internal services and
handles authentication, rate limiting and request logging. It is written in
Rust and ships as a single static binary.

## Features

- JWT and OIDC validation with key rotation
- Per-route rate limits (token bucket, 10ms resolution)
- Structured access logs in JSON Lines
- Health checks for upstreams with exponential backoff
- Hot reload of gateway.toml without dropping connections

## Quick start

Download the latest release from https://github.com/example/gateway/releases and
put the binary on `$PATH`. Then create a config file:

```toml
[server]
listen = "0.0.0.0:8080"
```

Start it with `gateway --config gateway.toml`. The proxy is ready when
it's printed `listening on 0.0.0.0:8080`. If you see
`address already in use`, another proccess holds the port.

## Configuration

Every route needs an upstream and at least one matcher. Matchers are evaluated in order, and
the first one that matches wins, so put you're most specific routes first.
Rate limits are seperate from matchers and apply after routing.

| Option | Default | Notes |
|---|---|---|
| `listen` | `0.0.0.0:8080` | Bind address |
| `read_timeout` | 30s | Per request |
| `max_body` | 4MiB | Larger bodies get 413 |

The upstream pool check every member on a fixed interval. Members that fail
three checks in a row are removed and than added back once they recover.

## Performance

On a single c6i.large instance the gateway sustains about 40k requests per second with
p99 latency under 2ms. This is faster then the previous
Node.js proxy by roughly 6x, mostly because it does not
copy request bodies.

## License

MIT. See LICENSE.
