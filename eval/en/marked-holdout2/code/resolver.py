"""Caching DNS resolver used by the health checker.

Wraps ``dns.resolver`` with a positive and negative cache, a per-name lock so a
⟪term|thundering herd⟫ of checks ⟦agreement|result|results⟧ in one query, and ⟪term|jittered⟫ expiry so
entries created together do not expire together. ⟦punctuation|The negative cache is short, five seconds, NXDOMAIN is often transient during deploys.|The negative cache is short, five seconds; NXDOMAIN is often transient during deploys.⟧
"""

from __future__ import annotations

import random
import threading
import time
from dataclasses import dataclass

# Upper bound on how long we trust an answer, regardless of the record's TTL.
# Some ⟪acronym|CDN⟫s hand out ⟪unit|86400s⟫ TTLs on records they then change within the hour.
MAX_TTL = 300.0
NEGATIVE_TTL = 5.0
# Fraction of TTL added as jitter. A ten percent spread is enough to break up ⟦british|synchronised|synchronized⟧
# expiry; larger values start to matter for correctness on short TTLs.
JITTER = 0.1


@dataclass
class Entry:
    addrs: tuple[str, ...]
    expires: float


class Resolver:
    """Resolve names to IPv4 and IPv6 addresses with caching.

    Thread safe. ⟦its_its|Its|It's⟧ fine to share one instance across the whole checker; ⟦their_there|their|there⟧
    is no benefit to more, and the cache only works if they share it.
    """

    def __init__(self, backend) -> None:
        self._backend = backend
        self._cache: dict[str, Entry] = {}
        self._locks: dict[str, threading.Lock] = {}
        self._guard = threading.Lock()

    def resolve(self, name: str) -> tuple[str, ...]:
        """Return the addresses for ``name``.

        Empty tuple means ⟪code|``NXDOMAIN``⟫ or no records; ⟦homophone|weather|whether⟧ that is an
        error is the caller's decision. Exceptions from the backend (timeouts, ⟪acronym|SERVFAIL⟫)
        propagate; they are not cached, so the next call retries. ⟦fragment|Deliberate, if a little chatty under outage.|This is deliberate, if a little chatty under an outage.⟧
        """
        name = name.rstrip(".").lower()
        now = time.monotonic()
        e = self._cache.get(name)
        if e is not None and e.expires > now:
            return e.addrs
        with self._lock_for(name):
            # Re-check under the lock: another thread may have filled it while we waited.
            e = self._cache.get(name)
            if e is not None and e.expires > now:
                return e.addrs
            addrs, ttl = self._backend.lookup(name)
            ttl = NEGATIVE_TTL if not addrs else min(float(ttl), MAX_TTL)
            ttl *= 1 + random.uniform(-JITTER, JITTER)
            self._cache[name] = Entry(tuple(addrs), now + ttl)
            return self._cache[name].addrs

    def _lock_for(self, name: str) -> threading.Lock:
        # One lock per name. Locks are never freed; the set of names is bounded by the
        # check configuration, so this ⟦spelling|dosent|doesn't⟧ grow without bound in practice.
        with self._guard:
            lock = self._locks.get(name)
            if lock is None:
                lock = self._locks[name] = threading.Lock()
            return lock

    def flush(self, name: str | None = None) -> None:
        """Drop one entry or all of them.

        Called by the config reloader when a target changes. Flushing everything is cheap
        and rarely wrong; flushing one name is ⟦your_youre|you're|your⟧ tool when a single
        ⟦homophone|principle|principal⟧ host was re-pointed and you would ⟦then_than|rather then|rather than⟧
        not wait ⟪unit|5 minutes⟫.
        """
        with self._guard:
            if name is None:
                self._cache.clear()
            else:
                self._cache.pop(name.rstrip(".").lower(), None)


def is_private(addr: str) -> bool:
    # RFC 1918 and link-local. Good enough for the checker's "do not probe internal hosts
    # from the public runner" rule; ⟦its_its|it's|its⟧ scope is intentionally narrow and it
    # does not try to be ⟦spelling|comprehensiv|comprehensive⟧ about IPv6 ⟦homophone|witch|which⟧ is
    # handled by the ⟪crate|ipaddress⟫ layer above.
    return addr.startswith(("10.", "192.168.", "169.254.")) or any(addr.startswith(f"172.{i}.") for i in range(16, 32))
