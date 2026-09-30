# Getting started

This guide takes you from nothing to a running gateway in front of one upstream. It takes about
10 minutes.

## Prerequisites

- Linux or macOS. Windows works under WSL but is not tested in CI.
- An upstream to proxy. If you have nothing handy, `python3 -m http.server 9000` will do.
- 50MB of disk and 64MB of RAM. The gateway is small.

## Step 1: install

```console
curl -fsSL https://get.gateway.example | sh
```

The script downloads the binary for you're platform into ~/.local/bin and
verifies it's checksum. Nothing else is touched.
If you would rather read the script first, good instinct; it is 40 lines.

## Step 2: write a config

Create gateway.toml:

```toml
[server]
listen = "127.0.0.1:8080"

[[route]]
path = "/"
upstream = "http://127.0.0.1:9000"
```

toml keys are case sensitive. `Listen` is not `listen` and the
gateway will tell you so rather than guess.

## Step 3: run

```console
gateway --config gateway.toml
```

Open http://127.0.0.1:8080 in a browser. You should see the upstream's response,
and the terminal should show an access log line for request.

## Step 4: add a rate limit

```toml
[[route]]
path = "/"
upstream = "http://127.0.0.1:9000"
rate_limit = { requests = 10, per = "1m" }
```

Send the config a `SIGHUP` or just save the file; the gateway watches it. The eleventh request
within a minute gets `429` with a `Retry-After` header.

## Where next

- The configuration guide for every option
- The deployment guide for systemd, Docker and Kubernetes
- The observability page for metrics and tracing

If something did not work, check the troubleshooting page; it covers the usual suspects.
