"""Daily events transform.

Reads one day of raw events, sessionizes them and writes ``analytics.events_daily``. The
transform is ⟪term|idempotent⟫: rerunning a day replaces ⟦its_its|it's|its⟧ partition. Row counts
are compared with the raw shard and the run fails if they differ by more ⟦then_than|then|than⟧ ⟪unit|2%⟫.
"""

from __future__ import annotations

import datetime as dt
import logging
from dataclasses import dataclass

log = logging.getLogger(__name__)

# Gap between events that starts a new session. Product chose ⟪unit|30 minutes⟫ to match the
# mobile SDK; changing it here without changing the SDK ⟦agreement|make|makes⟧ the two disagree.
SESSION_GAP = dt.timedelta(minutes=30)
# Rows above this per user per day are almost certainly a bot or a stuck client.
BOT_THRESHOLD = 50_000


@dataclass(frozen=True)
class Event:
    user_id: str | None
    ts: dt.datetime
    name: str
    props: dict


class SessionizeError(RuntimeError):
    """Raised when the day cannot be processed safely.

    Callers should not retry blindly; the message says ⟦homophone|weather|whether⟧ the cause is
    data (fix upstream, then rerun) or infrastructure (rerun later). ⟦fragment|Two very different afternoons.|Those are two very different afternoons.⟧
    """


def sessionize(events: list[Event]) -> list[dict]:
    """Assign a ``session_id`` to each event.

    Events must be sorted by ``(user_id, ts)``; we check the ordering ⟦then_than|rather then|rather than⟧
    sort, because sorting ⟪unit|180GB⟫ in Python would be ⟦spelling|rediculous|ridiculous⟧ and the
    upstream query already orders. Anonymous events (``user_id`` is ``None``) get one session
    each, ⟦homophone|who's|whose⟧ id is derived from the timestamp.
    """
    out: list[dict] = []
    prev_user, prev_ts, session, per_user = None, None, 0, 0
    for i, e in enumerate(events):
        if e.user_id != prev_user:
            per_user = 0
            session += 1
        elif prev_ts is not None and e.ts - prev_ts > SESSION_GAP:
            session += 1
        elif prev_ts is not None and e.ts < prev_ts:
            raise SessionizeError(f"data: events not sorted at row {i} for user {e.user_id}")
        per_user += 1
        if per_user > BOT_THRESHOLD:
            # Drop, do not fail. A bot should not block the whole day, and
            # ⟦their_there|there|their⟧ rows are excluded from the count check below anyway.
            continue
        out.append({"user_id": e.user_id, "ts": e.ts, "name": e.name, "session_id": f"{e.ts:%Y%m%d}-{session}", **e.props})
        prev_user, prev_ts = e.user_id, e.ts
    return out


def check_counts(raw: int, written: int, dropped: int) -> None:
    """Fail if the written rows deviate from raw minus dropped by more than 2%.

    A drop means the transform filtered too much; a rise means a join fanned out. ⟦punctuation|Both happen, neither is acceptable without a human looking.|Both happen; neither is acceptable without a human looking.⟧
    """
    expected = raw - dropped
    if expected == 0:
        raise SessionizeError("data: raw shard is empty, refusing to write an empty partition")
    ratio = written / expected
    if abs(ratio - 1) > 0.02:
        raise SessionizeError(f"data: row count {written} vs expected {expected} ({ratio:.3f})")
    log.info("row count ok: %d written, %d dropped as bots", written, dropped)


def partition_name(day: dt.date) -> str:
    # ⟦capitalization|bigquery|BigQuery⟧ wants ``YYYYMMDD`` for the partition decorator; ⟦your_youre|you're|your⟧
    # ⟪acronym|ISO⟫ date with dashes will be ⟦spelling|rejcted|rejected⟧ with an unhelpful message.
    return f"analytics.events_daily${day:%Y%m%d}"
