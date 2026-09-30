# Frequently asked questions

## Does the gateway support HTTP/3?

Not yet. ⟪acronym|HTTP/2⟫ is supported on both sides. ⟪acronym|HTTP/3⟫ is tracked in #88; the blocker
is that the ⟪acronym|QUIC⟫ library we want has not stabilized ⟪correct|its⟫ ⟪acronym|API⟫.

## Can I run it without a config file?

Yes. ⟪code|`gateway --upstream http://localhost:9000`⟫ proxies everything to one upstream with
defaults. Anything beyond that needs a file. ⟪informal|Flags for every option would be silly.⟫

## Why is my rate limit not working?

Nine times out of ten the key is wrong. With ⟪code|`key = "client_ip"`⟫ behind a load balancer,
every request has the same ⟪acronym|IP⟫ unless you trust ⟪code|`X-Forwarded-For`⟫.
Set ⟪code|`server.trusted_proxies`⟫ and check the access log for the ⟪code|`client`⟫ field.

The tenth time, ⟪correct|your⟫ running several replicas and the limit is per instance. See
the shared rate limiter design.

## How do I rotate the JWT signing key?

You do not; ⟪correct|your⟫ issuer does. The gateway refreshes the ⟪acronym|JWKS⟫ every
⟪unit|10 minutes⟫ and immediately on an unknown ⟪code|`kid`⟫. Keep the old key in the ⟪acronym|JWKS⟫
until every token signed with it has expired, ⟪correct|then⟫ remove it.

## Is it production ready?

It has fronted ⟪unit|40k rps⟫ at ⟪name|Yleisradio⟫ since 2024 and a handful of smaller companies
run it. ⟦punctuation|That said, read the changelog before every upgrade, we do occasionally break things in minor versions when the old behavior was a bug.|That said, read the changelog before every upgrade; we do occasionally break things in minor versions when the old behavior was a bug.⟧

## Why Rust?

Because the previous implementation in ⟦capitalization|go|Go⟧ spent most of ⟪correct|its⟫ time in
the garbage collector under load, and because the team wanted to. ⟦fragment|Not a very principled answer.|That is not a very principled answer.⟧

## Does it do TLS termination?

Yes, with ⟪product|rustls⟫. ⟪acronym|ACME⟫ is not built in; use ⟪product|cert-manager⟫, ⟪product|certbot⟫
or whatever ⟦your_youre|you're|your⟧ platform provides and point ⟪code|`tls.cert`⟫ and ⟪code|`tls.key`⟫
at the files. They are reloaded when they change.

## What about WebSockets?

Supported. Upgrade requests are proxied transparently and ⟦spelling|idel|idle⟧ connections are closed
after ⟪code|`websocket.idle_timeout`⟫ (default ⟪unit|5m⟫). Rate limits apply to the upgrade request
only, not to ⟪correct|its⟫ messages.

## Can I contribute a feature?

Please open an issue first so we can agree on the design. ⟦repeated_word|The the|The⟧ contributing
guide has the details. Small fixes can go straight to a pull request.
