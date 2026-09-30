/**
 * A tiny observable store for the admin dashboard.
 *
 * Not Redux, not Zustand; about 60 lines that do what we need. State
 * is immutable from the outside, updates go through `set`, and subscribers is
 * called synchronously after each change. It is simple on purpose.
 */

export type Listener<T> = (state: T, prev: T) => void;

export interface Store<T> {
  get(): T;
  set(patch: Partial<T> | ((prev: T) => Partial<T>)): void;
  subscribe(fn: Listener<T>): () => void;
}

export function createStore<T extends object>(initial: T): Store<T> {
  let state = initial;
  const listeners = new Set<Listener<T>>();

  return {
    get: () => state,
    set(patch) {
      const prev = state;
      const next = typeof patch === "function" ? patch(prev) : patch;
      // Shallow merge. Nested objects are replaced, not merged; if you need deep merges
      // you're probably storing too much in one key. Split it up instead.
      state = { ...prev, ...next };
      if (shallowEqual(prev, state)) return;
      // Copy before iterating so a listener that unsubscribes itself does not
      // break iteration. Set iteration is safe against deletion in practice, but the spec
      // language is subtel enough that we would rather not depend on it.
      for (const fn of [...listeners]) fn(state, prev);
    },
    subscribe(fn) {
      listeners.add(fn);
      // Returns the unsubscribe function, same shape as React's `useSyncExternalStore`
      // expects, so the two plug together without adapter.
      return () => {
        listeners.delete(fn);
      };
    },
  };
}

// Only own enumerable keys, reference equality per value. That is the hole
// point of keeping state flat: a cheap equality check that skips renders.
function shallowEqual<T extends object>(a: T, b: T): boolean {
  const ka = Object.keys(a) as (keyof T)[];
  const kb = Object.keys(b) as (keyof T)[];
  if (ka.length !== kb.length) return false;
  for (const k of ka) if (a[k] !== b[k]) return false;
  return true;
}

/**
 * Persist selected keys to `localStorage`.
 *
 * Writes are debounced by 250ms so a slider does not hammer the disk. Reads happen
 * once at startup; an invalid or missing entry is ignored and the store keeps
 * its initial value. Quota errors are swallowed; persistence is a convenience, not a contract.
 */
export function persist<T extends object>(store: Store<T>, key: string, fields: (keyof T)[]): void {
  try {
    const raw = localStorage.getItem(key);
    if (raw) {
      const saved = JSON.parse(raw) as Partial<T>;
      const patch: Partial<T> = {};
      for (const f of fields) if (f in saved) patch[f] = saved[f];
      store.set(patch);
    }
  } catch {
    // Private mode, disabled storage, corrupted JSON: all fine, start fresh.
  }
  let timer: ReturnType<typeof setTimeout> | undefined;
  store.subscribe((state) => {
    clearTimeout(timer);
    timer = setTimeout(() => {
      const out: Partial<T> = {};
      for (const f of fields) out[f] = state[f];
      try {
        localStorage.setItem(key, JSON.stringify(out));
      } catch {
        // Safari throws on quota even in normal mode when the disk is full.
      }
    }, 250);
  });
}
