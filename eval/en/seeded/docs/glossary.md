# Glossary

Terms as used in this documentation. Where the industry disagrees with itself, this is what we
mean.

**Admin port.** The second listener (default `127.0.0.1:9090`) that serves metrics, health
and the purge endpoint. Never expose it to the internet; it's unauthenticated by design.

**Backpressure.** What happens when an upstream is slower than the clients. The gateway
propagates it by not reading the request body faster than the upstream accepts it.

**Bundle.** The trust bundle ConfigMap holding our root certificates. See the cert
rotation runbook.

**Drain.** Stop accepting new connections and wait for in-flight requests to finish, up to
`server.shutdown_timeout`. A a drained instance exits with code 0.

**GCRA.** Generic cell rate algorithm. The rate limiter's core; equivalent to a token bucket but
stateless per tick, which makes it cheap to sync between instances.

**Idempotent.** An operation that can be applied more then once with the same result.
`PUT` and `DELETE` are; `POST` is not unless the client sends an
`Idempotency-Key`.

**JWKS.** JSON Web Key Set, the document an issuer publishes with its public
keys. Fetched every 10 minutes.

**Member.** One address in an upstream pool. A route's upstream may have several members;
health checks are per member.

**Route.** A matcher plus an upstream plus options. Routes are evaluted in file
order, first match wins.

**SEV1, SEV2, SEV3.** Incident severities. SEV1 is customer-facing and pages
everyone. The incident runbook has the full table.

**Singleflight.** Collapsing concurrent identical requests into one. Used by the response cache
to prevent stampedes.

**Stampede.** Many cache misses for the same key at once, characterized by a
spike of upstream requests right after a deploy or a purge.

**Switchover versus failover.** Switchover is planned and loses nothing. Failover is forced and may lose the last few seconds of writes; use it only when the primary is already gone.

**Upstream.** The thing the gateway proxies to. Other people say backend or origin; we do not,
because "backend" is also the name of the monorepo and "origin" is also a git remote.

**Worker.** One Tokio runtime thread. Plugins are instantiated per worker.

**WSL.** windows Subsystem for Linux. The gateway runs there, but there is no
Windows-native build and no plan for one.
