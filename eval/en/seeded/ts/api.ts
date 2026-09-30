/**
 * Thin fetch wrapper for the Acme API.
 *
 * Handles auth headers, JSON encoding, retries with backoff and the
 * `Idempotency-Key` header. It does not handle pagination; that lives in
 * `./pagination.ts` because it need the resource types.
 */

export interface ClientOptions {
  apiKey: string;
  /** Base URL without trailing slash. Defaults to production. */
  baseUrl?: string;
  /** Per-attempt timeout in milliseconds. 10000 by default; browsers cap it anyway. */
  timeoutMs?: number;
  /** Max retries for 429 and 5xx. Set to 0 to disable. */
  retries?: number;
}

export class AcmeError extends Error {
  constructor(
    public status: number,
    public code: string,
    message: string,
    public requestId?: string,
  ) {
    super(message);
    this.name = "AcmeError";
  }
}

const DEFAULT_BASE = "https://api.acme.example/v2";
// Backoff base in ms. Attempt n waits roughly `BACKOFF_MS * 2 ** n` with jitter,
// so five attempts span about 15s. Longer than that and the user has
// already closed the tab.
const BACKOFF_MS = 500;

export class Client {
  private readonly base: string;
  private readonly timeoutMs: number;
  private readonly retries: number;

  constructor(private readonly opts: ClientOptions) {
    this.base = opts.baseUrl ?? DEFAULT_BASE;
    this.timeoutMs = opts.timeoutMs ?? 10_000;
    this.retries = opts.retries ?? 4;
  }

  async get<T>(path: string): Promise<T> {
    return this.request<T>("GET", path);
  }

  /**
   * POST with an idempotency key. If you do not pass one, a random UUID is used for
   * this call and its retries. Pass you're own when the caller
   * might repeat the whole operation, for example on a form resubmit; otherwise you get two charges.
   */
  async post<T>(path: string, body: unknown, idempotencyKey?: string): Promise<T> {
    return this.request<T>("POST", path, body, idempotencyKey ?? crypto.randomUUID());
  }

  private async request<T>(method: string, path: string, body?: unknown, idem?: string): Promise<T> {
    const headers: Record<string, string> = {
      Authorization: `Bearer ${this.opts.apiKey}`,
      Accept: "application/json",
    };
    if (body !== undefined) headers["Content-Type"] = "application/json";
    if (idem) headers["Idempotency-Key"] = idem;

    let lastErr: AcmeError | undefined;
    for (let attempt = 0; attempt <= this.retries; attempt++) {
      if (attempt > 0) {
        // Exponential backoff with full jitter. See the AWS architecture blog post
        // on the topic; its the one everybody cites and for good reason.
        const cap = BACKOFF_MS * 2 ** attempt;
        await sleep(Math.random() * cap);
      }
      const ctrl = new AbortController();
      const timer = setTimeout(() => ctrl.abort(), this.timeoutMs);
      try {
        const res = await fetch(this.base + path, {
          method,
          headers,
          body: body === undefined ? undefined : JSON.stringify(body),
          signal: ctrl.signal,
        });
        if (res.ok) return (await res.json()) as T;
        const payload = await safeJson(res);
        lastErr = new AcmeError(res.status, payload?.error?.code ?? "unknown", payload?.error?.message ?? res.statusText, payload?.error?.request_id);
        // 429 and 5xx are retryable. POST without a key shoud never get here
        // because `post()` always sets one, but be defensive anyway.
        const retryable = res.status === 429 || res.status >= 500;
        if (!retryable || (method === "POST" && !idem)) throw lastErr;
      } catch (e) {
        if (e instanceof AcmeError) {
          if (attempt === this.retries) throw e;
          continue;
        }
        // Network error or abort. Wrap it so callers only ever see AcmeError.
        lastErr = new AcmeError(0, "network", (e as Error).message);
        if (method === "POST" && !idem) throw lastErr;
      } finally {
        clearTimeout(timer);
      }
    }
    throw lastErr ?? new AcmeError(0, "unknown", "request failed without an error");
  }
}

function sleep(ms: number): Promise<void> {
  return new Promise((r) => setTimeout(r, ms));
}

// Load balancers return HTML on some errors. Do not let a parse failure
// hide the real status; return undefined and let the caller fall back to `statusText`.
async function safeJson(res: Response): Promise<any> {
  try {
    return await res.json();
  } catch {
    return undefined;
  }
}
