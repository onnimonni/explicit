# Gateway

Gateway is a small reverse proxy that sits in front of our internal services and
handles authentication, rate limiting and request logging. It is written in
Rust and ships as a single static binary.

## Features

- ⟪acronym|JWT⟫ and ⟪acronym|OIDC⟫ validation with key rotation
- Per-route rate limits (token bucket, ⟪unit|10ms⟫ resolution)
- Structured access logs in ⟪acronym|JSON⟫ Lines
- Health checks for upstreams with ⟪term|exponential backoff⟫
- Hot reload of ⟪path|gateway.toml⟫ without dropping connections

## Quick start

Download the latest release from ⟪url|https://github.com/example/gateway/releases⟫ and
put the binary on ⟪code|`$PATH`⟫. Then create ⟪correct|a config⟫ file:

```toml
[server]
listen = "0.0.0.0:8080"
```

Start it with ⟪code|`gateway --config gateway.toml`⟫. The proxy is ready when
⟪correct|it's⟫ printed ⟪code|`listening on 0.0.0.0:8080`⟫. If you see
⟪code|`address already in use`⟫, another ⟦spelling|proccess|process⟧ holds the port.

## Configuration

Every route needs an upstream and at least one matcher. Matchers are evaluated in order, and
the first one that matches wins, so put ⟦your_youre|you're|your⟧ most specific routes first.
Rate limits are ⟦spelling|seperate|separate⟧ from matchers and apply after routing.

| Option | Default | Notes |
|---|---|---|
| ⟪code|`listen`⟫ | ⟪code|`0.0.0.0:8080`⟫ | ⟪table|Bind address⟫ |
| ⟪code|`read_timeout`⟫ | ⟪unit|30s⟫ | ⟪table|Per request⟫ |
| ⟪code|`max_body`⟫ | ⟪unit|4MiB⟫ | ⟪table|Larger bodies get 413⟫ |

The upstream pool ⟦agreement|check|checks⟧ every member on a fixed interval. Members that fail
three checks in a row are removed ⟦then_than|and than|and then⟧ added back once they recover.

## Performance

On a single ⟪product|c6i.large⟫ instance the gateway sustains about 40k requests per second with
⟪unit|p99⟫ latency under ⟪unit|2ms⟫. This is faster ⟦then_than|then|than⟧ the previous
Node.js proxy by roughly ⟪unit|6x⟫, mostly because it does not
copy request bodies.

## License

⟪acronym|MIT⟫. See ⟪path|LICENSE⟫.
