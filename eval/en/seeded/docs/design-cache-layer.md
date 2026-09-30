# Design: response cache

Author: Väinö Mäkelä
Status: draft

## Goal

Cache upstream responses in the gateway so that repeated `GET` requests for the same
resource do not hit the upstream. Target: 80% hit rate on the catalog service, which
serves the same 2,000 products to everyone.

## Non-goals

- Caching anything but `GET` and `HEAD`
- A distributed cache; each instance keeps its own
- Replacing the CDN

## Design

The cache is an in-memory LRU keyed by method, host, path and the `Vary` headers.
Entires store the status, headers and body up to 1MiB; larger bodies bypass
the cache. Memory is bounded by `cache.max_bytes` (default 256MiB).

Freshness follows RFC 9111. `Cache-Control: no-store` and `private` are
honored. Responses without explicit freshness get an heuristic lifetime of
10% of the `Last-Modified` age, capped at 1 hour. Same as browsers.

Stale entries are revalidated with `If-None-Match`. A `304` refreshes the entry
without copying the body, so revalidation cost one small round trip.

### Stampede protection

When many requests miss on the same key at once, only the first goes upstream; the rest wait on
a singleflight group for up to 2s and than fall through. This
effects only cold keys and is the single biggest win for the catalog service,
who's product pages get hammered after every deploy.

### Purging

`PURGE /path` on the admin port evicts one key. `PURGE /*` evicts everything.
Purges are not propagated between instances. If you need coordinated purges, use short TTLs instead; it is simpler and good enough.

## Metrics

| Metric | Type | Labels |
|---|---|---|
| `cache_requests_total` | counter | `result` (hit, miss, bypass, stale) |
| `cache_bytes` | gauge | none |
| `cache_evictions_total` | counter | `reason` (lru, purge, expired) |

## Risks

Caching `Set-Cookie` responses would leak sessions between users. We we never
store a response carrying `Set-Cookie` or `Authorization` regardless of
`Cache-Control`. This is belt and suspenders but the failure mode justifys it.

Memory accounting counts body bytes only. Headers and keys add roughly 10% overhead
that is not visible in `cache_bytes`; its documented, but people will be surprised.

## Alternatives

Varnish in front of the gateway. Rejected because it doubles the moving parts and
centralises what is otherwise a stateless tier. We may revisit if hit rates
disappoint.
