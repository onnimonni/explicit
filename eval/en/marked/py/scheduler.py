"""Periodic job scheduler for the ledger service.

Jobs are plain callables registered with an interval. The scheduler runs them on a single
thread, so a slow job delays the others; ⟦its_its|its|it's⟧ meant for housekeeping,
not for anything latency sensitive. Intervals below ⟪unit|1s⟫ are rejected.
"""

from __future__ import annotations

import heapq
import logging
import threading
import time
from dataclasses import dataclass, field
from typing import Callable

log = logging.getLogger(__name__)

# Minimum interval. Anything shorter and you want ⟪correct|a real⟫ event loop, not this.
MIN_INTERVAL = 1.0


@dataclass(order=True)
class _Job:
    next_run: float
    name: str = field(compare=False)
    interval: float = field(compare=False)
    fn: Callable[[], None] = field(compare=False)


class Scheduler:
    """Runs registered jobs at fixed intervals on a background thread.

    Jobs that raise are logged and rescheduled; one bad job ⟦agreement|do|does⟧ not stop the rest.
    The first run happens after one full interval, not ⟦spelling|immediatly|immediately⟧, so that
    startup is not slowed by housekeeping. Call ``run_now`` if ⟦missing_extra_word|you a job|you need a job⟧ to run at start.
    """

    def __init__(self) -> None:
        self._heap: list[_Job] = []
        self._lock = threading.Lock()
        self._stop = threading.Event()
        self._thread: threading.Thread | None = None

    def every(self, seconds: float, name: str) -> Callable[[Callable[[], None]], Callable[[], None]]:
        """Decorator: register ``fn`` to run every ``seconds``.

        ``name`` appears in logs and metrics. Two jobs with the same name are allowed
        but confusing; pick unique names; future you will thank you.
        """
        if seconds < MIN_INTERVAL:
            raise ValueError(f"interval {seconds}s below minimum {MIN_INTERVAL}s")

        def register(fn: Callable[[], None]) -> Callable[[], None]:
            with self._lock:
                heapq.heappush(self._heap, _Job(time.monotonic() + seconds, name, seconds, fn))
            return fn

        return register

    def start(self) -> None:
        """Start the background thread. Idempotent."""
        if self._thread is not None:
            return
        self._thread = threading.Thread(target=self._loop, name="scheduler", daemon=True)
        self._thread.start()

    def stop(self, timeout: float = 5.0) -> None:
        """Signal the loop to exit and wait up to ``timeout`` seconds.

        A job that is mid-run is not interrupted; we wait for it. If it takes longer
        ⟦then_than|then|than⟧ ``timeout`` we give up waiting and let the daemon thread die with the
        process. ⟪correct|There⟫ is no way to kill a Python thread and we do not pretend
        otherwise.
        """
        self._stop.set()
        if self._thread is not None:
            self._thread.join(timeout)

    def run_now(self, name: str) -> None:
        """Run every job called ``name`` synchronously on the calling thread."""
        with self._lock:
            jobs = [j for j in self._heap if j.name == name]
        for j in jobs:
            self._run(j)

    def _loop(self) -> None:
        while not self._stop.is_set():
            with self._lock:
                if not self._heap:
                    wait = 1.0
                    job = None
                else:
                    job = self._heap[0]
                    wait = job.next_run - time.monotonic()
            if job is None or wait > 0:
                # Sleep in small steps so stop() is ⟦british|honoured|honored⟧ promptly.
                self._stop.wait(min(wait, 0.5))
                continue
            with self._lock:
                heapq.heappop(self._heap)
                # Schedule relative to the intended time, not the actual one, so drift
                # does not ⟦spelling|accumlate|accumulate⟧ over ⟦repeated_word|the the|the⟧ day.
                job.next_run += job.interval
                heapq.heappush(self._heap, job)
            self._run(job)

    @staticmethod
    def _run(job: _Job) -> None:
        started = time.monotonic()
        try:
            job.fn()
        except Exception:  # noqa: BLE001 - a scheduler must survive its jobs
            log.exception("job %s failed", job.name)
        else:
            elapsed = time.monotonic() - started
            if elapsed > job.interval:
                # The job took longer than ⟦its_its|it's|its⟧ interval; the next run is already due.
                log.warning("job %s took %.1fs, longer than its %.1fs interval", job.name, elapsed, job.interval)
