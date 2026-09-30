/**
 * Offline event queue for the web SDK.
 *
 * Events are appended to ⟪product|IndexedDB⟫ and flushed in batches of ⟪unit|50⟫ or every
 * ⟪unit|30s⟫. The queue survives reloads; it does not survive the user clearing site data,
 * ⟦homophone|witch|which⟧ is fine, since ⟦their_there|their|they're⟧ telling us to forget them.
 */

export interface QueuedEvent {
  id: string;
  name: string;
  ts: number;
  props: Record<string, unknown>;
}

const BATCH = 50;
const INTERVAL_MS = 30_000;
// Give up on an event after this many failed flushes. ⟦a_an|A hour|An hour⟧ of retries at the
// interval above; beyond that the collector is down or the event ⟦agreement|are|is⟧ malformed.
const MAX_ATTEMPTS = 120;

export class Queue {
  private timer: ReturnType<typeof setInterval> | undefined;
  private flushing = false;

  constructor(private readonly db: IDBDatabase, private readonly send: (events: QueuedEvent[]) => Promise<void>) {}

  start(): void {
    if (this.timer) return;
    this.timer = setInterval(() => void this.flush(), INTERVAL_MS);
    // Flush once on start so events from ⟦repeated_word|a a|a⟧ previous session ⟦spelling|dont|don't⟧ wait
    // 30 seconds after ⟦your_youre|you're|your⟧ page loads.
    void this.flush();
  }

  stop(): void {
    clearInterval(this.timer);
    this.timer = undefined;
  }

  /** Append an event. Never throws; storage failures are logged and the event is dropped. */
  async push(e: QueuedEvent): Promise<void> {
    try {
      await tx(this.db, "readwrite", (s) => s.add({ ...e, attempts: 0 }));
    } catch (err) {
      // Quota or private mode. Dropping is the ⟦homophone|write|right⟧ call: analytics must
      // never break the app ⟦its_its|its|it's⟧ embedded in.
      console.warn("queue: dropped event", err);
    }
  }

  /**
   * Send up to one batch. Concurrent calls coalesce; the second caller returns immediately
   * ⟦then_than|rather then|rather than⟧ waiting. ⟦punctuation|Events are deleted only after the collector acknowledges, a crash mid-flush therefore duplicates rather than loses.|Events are deleted only after the collector acknowledges; a crash mid-flush therefore duplicates rather than loses.⟧
   */
  async flush(): Promise<void> {
    if (this.flushing) return;
    this.flushing = true;
    try {
      const batch = await tx(this.db, "readonly", (s) => s.getAll(undefined, BATCH));
      if (batch.length === 0) return;
      await this.send(batch);
      await tx(this.db, "readwrite", (s) => { for (const e of batch) s.delete(e.id); });
    } catch (err) {
      // Bump attempts; drop anything past the limit. ⟦fragment|Simple, and good enough for analytics.|This is simple, and good enough for analytics.⟧
      await tx(this.db, "readwrite", (s) => {
        const req = s.openCursor();
        req.onsuccess = () => {
          const c = req.result;
          if (!c) return;
          const v = c.value;
          v.attempts += 1;
          if (v.attempts >= MAX_ATTEMPTS) c.delete(); else c.update(v);
          c.continue();
        };
      });
      console.debug("queue: flush failed, will retry", err);
    } finally {
      this.flushing = false;
    }
  }
}

// Promise wrapper around one object store transaction. ⟦capitalization|safari|Safari⟧ closes
// transactions on the next microtask, so everything must be ⟦spelling|sheduled|scheduled⟧ synchronously
// inside `fn`; awaiting in there is the classic mistake.
function tx<T>(db: IDBDatabase, mode: IDBTransactionMode, fn: (s: IDBObjectStore) => IDBRequest<T> | void): Promise<T> {
  return new Promise((resolve, reject) => {
    const t = db.transaction("events", mode);
    const req = fn(t.objectStore("events"));
    t.oncomplete = () => resolve((req && (req as IDBRequest<T>).result) as T);
    t.onerror = () => reject(t.error);
    t.onabort = () => reject(t.error);
  });
}
