# Frequently asked questions

## Does the gateway support HTTP/3?

Not yet. HTTP/2 is supported on both sides. HTTP/3 is tracked in #88; the blocker
is that the QUIC library we want has not stabilized its API.

## Can I run it without a config file?

Yes. `gateway --upstream http://localhost:9000` proxies everything to one upstream with
defaults. Anything beyond that needs a file. Flags for every option would be silly.

## Why is my rate limit not working?

Nine times out of ten the key is wrong. With `key = "client_ip"` behind a load balancer,
every request has the same IP unless you trust `X-Forwarded-For`.
Set `server.trusted_proxies` and check the access log for the `client` field.

The tenth time, your running several replicas and the limit is per instance. See
the shared rate limiter design.

## How do I rotate the JWT signing key?

You do not; your issuer does. The gateway refreshes the JWKS every
10 minutes and immediately on an unknown `kid`. Keep the old key in the JWKS
until every token signed with it has expired, then remove it.

## Is it production ready?

It has fronted 40k rps at Yleisradio since 2024 and a handful of smaller companies
run it. That said, read the changelog before every upgrade, we do occasionally break things in minor versions when the old behavior was a bug.

## Why Rust?

Because the previous implementation in go spent most of its time in
the garbage collector under load, and because the team wanted to. Not a very principled answer.

## Does it do TLS termination?

Yes, with rustls. ACME is not built in; use cert-manager, certbot
or whatever you're platform provides and point `tls.cert` and `tls.key`
at the files. They are reloaded when they change.

## What about WebSockets?

Supported. Upgrade requests are proxied transparently and idel connections are closed
after `websocket.idle_timeout` (default 5m). Rate limits apply to the upgrade request
only, not to its messages.

## Can I contribute a feature?

Please open an issue first so we can agree on the design. The the contributing
guide has the details. Small fixes can go straight to a pull request.
