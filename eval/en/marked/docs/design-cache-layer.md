# Design: response cache

Author: ⟪name|Väinö Mäkelä⟫
Status: draft

## Goal

Cache upstream responses in the gateway so that repeated ⟪code|`GET`⟫ requests for the same
resource do not hit the upstream. Target: ⟪unit|80%⟫ hit rate on the catalog service, which
serves the same ⟪unit|2,000⟫ products to everyone.

## Non-goals

- ⟪list|Caching anything but `GET` and `HEAD`⟫
- ⟪list|A distributed cache; each instance keeps its own⟫
- ⟪list|Replacing the CDN⟫

## Design

The cache is an in-memory ⟪acronym|LRU⟫ keyed by method, host, path and the ⟪code|`Vary`⟫ headers.
⟦spelling|Entires|Entries⟧ store the status, headers and body up to ⟪unit|1MiB⟫; larger bodies bypass
the cache. Memory is bounded by ⟪code|`cache.max_bytes`⟫ (default ⟪unit|256MiB⟫).

Freshness follows ⟪acronym|RFC⟫ 9111. ⟪code|`Cache-Control: no-store`⟫ and ⟪code|`private`⟫ are
honored. Responses without explicit freshness get ⟦a_an|an heuristic|a heuristic⟧ lifetime of
⟪unit|10%⟫ of the ⟪code|`Last-Modified`⟫ age, capped at ⟪unit|1 hour⟫. ⟦fragment|Same as browsers.|This is the same as browsers.⟧

Stale entries are revalidated with ⟪code|`If-None-Match`⟫. A ⟪code|`304`⟫ refreshes the entry
without copying the body, so revalidation ⟦agreement|cost|costs⟧ one small round trip.

### Stampede protection

When many requests miss on the same key at once, only the first goes upstream; the rest wait on
a ⟪term|singleflight⟫ group for up to ⟪unit|2s⟫ ⟦then_than|and than|and then⟧ fall through. This
⟦homophone|effects|affects⟧ only cold keys and is the single biggest win for the catalog service,
⟦homophone|who's|whose⟧ product pages get hammered after every deploy.

### Purging

⟪code|`PURGE /path`⟫ on the admin port evicts one key. ⟪code|`PURGE /*`⟫ evicts everything.
Purges are not propagated between instances. If you need coordinated purges, use short TTLs instead; it is simpler and good enough.

## Metrics

| Metric | Type | Labels |
|---|---|---|
| ⟪code|`cache_requests_total`⟫ | ⟪table|counter⟫ | ⟪code|`result` (hit, miss, bypass, stale)⟫ |
| ⟪code|`cache_bytes`⟫ | ⟪table|gauge⟫ | ⟪table|none⟫ |
| ⟪code|`cache_evictions_total`⟫ | ⟪table|counter⟫ | ⟪code|`reason` (lru, purge, expired)⟫ |

## Risks

Caching ⟪code|`Set-Cookie`⟫ responses would leak sessions between users. ⟦repeated_word|We we|We⟧ never
store a response carrying ⟪code|`Set-Cookie`⟫ or ⟪code|`Authorization`⟫ regardless of
⟪code|`Cache-Control`⟫. This is ⟪term|belt and suspenders⟫ but the failure mode ⟦spelling|justifys|justifies⟧ it.

Memory accounting counts body bytes only. Headers and keys add roughly ⟪unit|10%⟫ overhead
that is not visible in ⟪code|`cache_bytes`⟫; ⟦its_its|its|it's⟧ documented, but people will be surprised.

## Alternatives

⟪product|Varnish⟫ in front of the gateway. Rejected because it doubles the moving parts and
⟦british|centralises|centralizes⟧ what is otherwise a stateless tier. We may revisit if hit rates
disappoint.
