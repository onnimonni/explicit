# Design: shared rate limiter

Status: accepted
Reviewers: ⟪name|Åsa Sjöberg⟫, ⟪name|Tuomas Heikkilä⟫

## Problem

Rate limits are enforced per gateway instance. With ⟪unit|N⟫ replicas a client gets ⟪unit|N⟫ times
the configured budget, and the effective limit changes every time the autoscaler ⟦agreement|add|adds⟧
or removes a pod. Customers have noticed.

## Options

1. **Divide the budget by replica count.** Simple, but uneven load balancing makes it
   inaccurate, and scaling events cause bursts.
2. **Central counter in ⟪product|Redis⟫.** Accurate, but every request pays a network round trip
   and Redis becomes a hard dependency of the data path.
3. **Local buckets with periodic sync.** Each instance counts locally and gossips ⟦its_its|it's|its⟧
   counts every ⟪unit|100ms⟫. ⟪term|Eventually consistent⟫, no hot dependency.

We pick option 3. It is accurate enough, and the gateway stays up when Redis does not.

## Algorithm

Each instance keeps a ⟪acronym|GCRA⟫ state per key. Every ⟪unit|100ms⟫ it publishes a delta of
consumed tokens per key to a ⟪product|Redis⟫ stream and reads the deltas of ⟪correct|its⟫ peers.
Peer deltas are applied to the local state as if the requests had happened locally.

The error bound is ⟪unit|100ms⟫ worth of traffic per peer. For a limit of ⟪unit|1000 rps⟫ across
10 replicas that is at most ⟪unit|1000⟫ extra requests in the worst case, realized
only if every replica is saturated at the same instant.

If Redis is unreachable the instance falls back to local counting and logs once per minute.
Nothing else changes; this is the whole point.

## Key cardinality

Keys are bounded by ⟪code|`rate_limit.max_keys`⟫ (default ⟪unit|100k⟫) with ⟪acronym|LRU⟫ eviction.
Evicting a key resets ⟪correct|its⟫ budget, which favors the client; we accept that ⟦then_than|rather then|rather than⟧
risk unbounded memory. Keys that have not been seen for ⟪unit|10 minutes⟫ are dropped from the
sync stream so peers do not carry ⟪correct|their⟫ state forever.

## Wire format

Deltas are ⟪acronym|MessagePack⟫ arrays of ⟪code|`[key_hash: u64, tokens: u32]`⟫. Keys are hashed
with ⟪term|xxh3⟫ before publishing so raw client ⟪acronym|IP⟫s never reach Redis. A hash collision
means two clients share a budget; at ⟪unit|64 bits⟫ this is not ⟪correct|a concern⟫ we lose
sleep over.

## Rollout

Feature flag ⟪code|`rate_limit.shared = true`⟫, off by default in ⟪version|2.5⟫, on in ⟪version|3.0⟫.
Dashboards get a new panel showing local versus synced counts so we can see ⟦homophone|weather|whether⟧
the error bound holds in practice. Redis ⟪version|7.2⟫ or newer is required for
the stream trimming options we use.
