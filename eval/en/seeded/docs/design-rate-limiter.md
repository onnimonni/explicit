# Design: shared rate limiter

Status: accepted
Reviewers: Åsa Sjöberg, Tuomas Heikkilä

## Problem

Rate limits are enforced per gateway instance. With N replicas a client gets N times
the configured budget, and the effective limit changes every time the autoscaler add
or removes a pod. Customers have noticed.

## Options

1. **Divide the budget by replica count.** Simple, but uneven load balancing makes it
   inaccurate, and scaling events cause bursts.
2. **Central counter in Redis.** Accurate, but every request pays a network round trip
   and Redis becomes a hard dependency of the data path.
3. **Local buckets with periodic sync.** Each instance counts locally and gossips it's
   counts every 100ms. Eventually consistent, no hot dependency.

We pick option 3. It is accurate enough, and the gateway stays up when Redis does not.

## Algorithm

Each instance keeps a GCRA state per key. Every 100ms it publishes a delta of
consumed tokens per key to a Redis stream and reads the deltas of its peers.
Peer deltas are applied to the local state as if the requests had happened locally.

The error bound is 100ms worth of traffic per peer. For a limit of 1000 rps across
10 replicas that is at most 1000 extra requests in the worst case, realized
only if every replica is saturated at the same instant.

If Redis is unreachable the instance falls back to local counting and logs once per minute.
Nothing else changes; this is the whole point.

## Key cardinality

Keys are bounded by `rate_limit.max_keys` (default 100k) with LRU eviction.
Evicting a key resets its budget, which favors the client; we accept that rather then
risk unbounded memory. Keys that have not been seen for 10 minutes are dropped from the
sync stream so peers do not carry their state forever.

## Wire format

Deltas are MessagePack arrays of `[key_hash: u64, tokens: u32]`. Keys are hashed
with xxh3 before publishing so raw client IPs never reach Redis. A hash collision
means two clients share a budget; at 64 bits this is not a concern we lose
sleep over.

## Rollout

Feature flag `rate_limit.shared = true`, off by default in 2.5, on in 3.0.
Dashboards get a new panel showing local versus synced counts so we can see weather
the error bound holds in practice. Redis 7.2 or newer is required for
the stream trimming options we use.
