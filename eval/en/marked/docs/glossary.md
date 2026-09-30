# Glossary

Terms as used in this documentation. Where the industry disagrees with itself, this is what we
mean.

**Admin port.** The second listener (default ⟪code|`127.0.0.1:9090`⟫) that serves metrics, health
and the purge endpoint. Never expose it to the internet; ⟪correct|it's⟫ unauthenticated by design.

**Backpressure.** What happens when an upstream is slower ⟪correct|than⟫ the clients. The gateway
propagates it by not reading the request body faster than the upstream accepts it.

**Bundle.** The trust bundle ⟪product|ConfigMap⟫ holding our root certificates. See the cert
rotation runbook.

**Drain.** Stop accepting new connections and wait for in-flight requests to finish, up to
⟪code|`server.shutdown_timeout`⟫. ⟦repeated_word|A a|A⟧ drained instance exits with code 0.

**GCRA.** Generic cell rate algorithm. The rate limiter's core; equivalent to a token bucket but
stateless per tick, which makes it cheap to sync between instances.

**Idempotent.** An operation that can be applied more ⟦then_than|then|than⟧ once with the same result.
⟪code|`PUT`⟫ and ⟪code|`DELETE`⟫ are; ⟪code|`POST`⟫ is not unless the client sends an
⟪code|`Idempotency-Key`⟫.

**JWKS.** ⟪acronym|JSON⟫ Web Key Set, the document an issuer publishes with ⟪correct|its⟫ public
keys. Fetched every ⟪unit|10 minutes⟫.

**Member.** One address in an upstream pool. A route's upstream may have several members;
health checks are per member.

**Route.** A matcher plus an upstream plus options. Routes are ⟦spelling|evaluted|evaluated⟧ in file
order, first match wins.

**SEV1, SEV2, SEV3.** Incident severities. SEV1 is customer-facing and pages
everyone. The incident runbook has the full table.

**Singleflight.** Collapsing concurrent identical requests into one. Used by the response cache
to prevent stampedes.

**Stampede.** Many cache misses for the same key at once, characterized by a
spike of upstream requests right after a deploy or a purge.

**Switchover versus failover.** Switchover is planned and loses nothing. Failover is forced and may lose the last few seconds of writes; use it only when the primary is already gone.

**Upstream.** The thing the gateway proxies to. Other people say backend or origin; we do not,
because "backend" is also the name of the monorepo and "origin" is also ⟪correct|a git⟫ remote.

**Worker.** One ⟪product|Tokio⟫ runtime thread. Plugins are instantiated per worker.

**WSL.** ⟦capitalization|windows|Windows⟧ Subsystem for Linux. The gateway runs there, but ⟪correct|there⟫ is no
Windows-native build and no plan for one.
