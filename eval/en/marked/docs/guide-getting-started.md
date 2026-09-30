# Getting started

This guide takes you from nothing to a running gateway in front of one upstream. It takes about
⟪unit|10 minutes⟫.

## Prerequisites

- ⟪product|Linux⟫ or ⟪product|macOS⟫. ⟪product|Windows⟫ works under ⟪acronym|WSL⟫ but is not tested in CI.
- An upstream to proxy. If you have nothing handy, ⟪code|`python3 -m http.server 9000`⟫ will do.
- ⟪unit|50MB⟫ of disk and ⟪unit|64MB⟫ of ⟪acronym|RAM⟫. The gateway is small.

## Step 1: install

```console
curl -fsSL https://get.gateway.example | sh
```

The script downloads the binary for ⟦your_youre|you're|your⟧ platform into ⟪path|~/.local/bin⟫ and
verifies ⟦its_its|it's|its⟧ checksum. Nothing else is touched.
⟪informal|If you would rather read the script first, good instinct; it is 40 lines.⟫

## Step 2: write a config

Create ⟪path|gateway.toml⟫:

```toml
[server]
listen = "127.0.0.1:8080"

[[route]]
path = "/"
upstream = "http://127.0.0.1:9000"
```

⟦capitalization|toml|TOML⟧ keys are case sensitive. ⟪code|`Listen`⟫ is not ⟪code|`listen`⟫ and the
gateway will tell you so rather ⟪correct|than⟫ guess.

## Step 3: run

```console
gateway --config gateway.toml
```

Open ⟪url|http://127.0.0.1:8080⟫ in a browser. You should see the upstream's response,
and the terminal should show an access log line for ⟦missing_extra_word|request|the request⟧.

## Step 4: add a rate limit

```toml
[[route]]
path = "/"
upstream = "http://127.0.0.1:9000"
rate_limit = { requests = 10, per = "1m" }
```

Send the config a ⟪code|`SIGHUP`⟫ or just save the file; the gateway watches it. The eleventh request
within a minute gets ⟪code|`429`⟫ with a ⟪code|`Retry-After`⟫ header.

## Where next

- ⟪list|The configuration guide for every option⟫
- ⟪list|The deployment guide for systemd, Docker and Kubernetes⟫
- ⟪list|The observability page for metrics and tracing⟫

If something did not work, check the troubleshooting page; it covers the usual suspects.
