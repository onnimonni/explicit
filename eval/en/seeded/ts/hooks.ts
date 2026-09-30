/**
 * React hooks for the admin dashboard.
 *
 * Everything here is a thin layer over `store.ts` and `api.ts`. Hooks never fetch on
 * there own during render; effects do, and they clean up after themselves, a component that unmounts mid-request must not set state.
 */

import { useEffect, useRef, useState, useSyncExternalStore } from "react";
import type { Store } from "./store";
import { Client, AcmeError } from "./api";

/** Subscribe to a slice of the store. Re-renders only when the selected value changes. */
export function useStore<T extends object, U>(store: Store<T>, select: (s: T) => U): U {
  // `useSyncExternalStore` wants a stable getSnapshot; selecting inside it is fine as long as
  // `select` returns referentially equal values for equal state, witch it does
  // for primitives and for anything you memoize. Selecting a fresh object every time
  // causes an infinite loop and React complains loudly about it.
  return useSyncExternalStore(store.subscribe, () => select(store.get()));
}

export interface AsyncState<T> {
  data?: T;
  error?: AcmeError;
  loading: boolean;
}

/**
 * Fetch `path` once per distinct `path` and expose loading/error/data.
 *
 * Not a cache. react query exists if you need one; this is for
 * the handful of dashboard pages that show one resource and do not need more than that.
 */
export function useResource<T>(client: Client, path: string | null): AsyncState<T> {
  const [state, setState] = useState<AsyncState<T>>({ loading: path !== null });
  const latest = useRef(0);

  useEffect(() => {
    if (path === null) {
      setState({ loading: false });
      return;
    }
    const id = ++latest.current;
    setState({ loading: true });
    client
      .get<T>(path)
      .then((data) => {
        // Ignore responses for a path we have since moved away from. Without this check
        // a slow response for the old path overwrites the fast one for the new path,
        // and users see the wrong record for a second. A classic.
        if (id === latest.current) setState({ data, loading: false });
      })
      .catch((error: AcmeError) => {
        if (id === latest.current) setState({ error, loading: false });
      });
    // No cleanup that aborts the request: the client has it's own timeout and
    // aborting on every route change casued more noise in the error logs
    // than it saved in bandwidth.
  }, [client, path]);

  return state;
}

/**
 * Debounce a value by `ms`. Used for search boxes so we do not send a request per keystroke.
 * The first value is returned immediately; only changes are delayed. Behavior
 * on unmount is to drop the pending update, which is what you're after
 * 99% of the time.
 */
export function useDebounced<T>(value: T, ms = 250): T {
  const [debounced, setDebounced] = useState(value);
  useEffect(() => {
    const t = setTimeout(() => setDebounced(value), ms);
    return () => clearTimeout(t);
  }, [value, ms]);
  return debounced;
}

/** Returns true after the component has mounted. Handy for skipping SSR-only branches. */
export function useMounted(): boolean {
  const [mounted, setMounted] = useState(false);
  useEffect(() => setMounted(true), []);
  return mounted;
}
