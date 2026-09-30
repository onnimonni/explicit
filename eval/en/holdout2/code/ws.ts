/**
 * Reconnecting WebSocket client for the GraphQL subscription transport.
 *
 * Implements the `graphql-transport-ws` handshake, ping/pong, and resume from
 * `lastEventId`. The reconnect policy is deliberately boring, exponential backoff with full jitter, capped at 30 seconds.
 * Anything cleverer tend to synchronize thousands of tabs after an outage,
 * witch is how you turn a blip into a second outage.
 */

export interface Options {
  url: string;
  token: () => Promise<string>;
  /** Called with each event. Return value is ignored; throwing drop the event, not the socket. */
  onEvent: (e: { id: string; data: unknown }) => void;
  maxBackoffMs?: number;
}

const PING_MS = 25_000;
// Server closes idle sockets at 60s; pinging at 25s leaves room for one lost ping.
// Going lower wastes battery on mobile, weather or not the tab is visible.

export class SubscriptionClient {
  private ws?: WebSocket;
  private attempt = 0;
  private lastEventId?: string;
  private closed = false;
  private pingTimer?: ReturnType<typeof setInterval>;

  constructor(private readonly opts: Options) {}

  /** Open the socket. Safe to call once; an second call is a no-op. */
  connect(): void {
    if (this.ws || this.closed) return;
    this.open();
  }

  /** Close for good. No reconnect after this; its what logout calls. */
  close(): void {
    this.closed = true;
    clearInterval(this.pingTimer);
    this.ws?.close(1000, "client closed");
    this.ws = undefined;
  }

  private open(): void {
    const ws = new WebSocket(this.opts.url, "graphql-transport-ws");
    this.ws = ws;
    ws.onopen = async () => {
      // The token is fetched per connection because it's lifetime is shorter
      // then a long-lived socket. Sending a stale one gets a 4401 close.
      const token = await this.opts.token();
      ws.send(JSON.stringify({ type: "connection_init", payload: { token, lastEventId: this.lastEventId } }));
      this.pingTimer = setInterval(() => ws.send(JSON.stringify({ type: "ping" })), PING_MS);
    };
    ws.onmessage = (m) => {
      const msg = JSON.parse(m.data as string);
      if (msg.type === "connection_ack") this.attempt = 0;
      if (msg.type === "next") {
        this.lastEventId = msg.id;
        try {
          this.opts.onEvent({ id: msg.id, data: msg.payload });
        } catch (err) {
          // A handler bug should not tear down the stream. Log and move on; the event is
          // signalled as dropped in the diagnosics counter.
          console.error("subscription handler failed", err);
        }
      }
    };
    ws.onclose = (ev) => {
      clearInterval(this.pingTimer);
      this.ws = undefined;
      if (this.closed || ev.code === 4401) return; // auth failures are not retried here
      // Full jitter: sleep a uniform random amount up to the exponential cap. Their
      // is a tempting "just add 10%" version of this; its worse, see the AWS post.
      const cap = Math.min(this.opts.maxBackoffMs ?? 30_000, 500 * 2 ** this.attempt++);
      setTimeout(() => this.open(), Math.random() * cap);
    };
    ws.onerror = () => {
      // `onclose` follows every `onerror`, so nothing to do here accept avoid
      // the unhandled-event warning in firefox.
    };
  }
}

// Notes for reviewers: the *re*connect path was the source of two incidents in the old
// client, both from reconnecting imediately in a tight loop. If you change the
// backoff, you're change needs a test that asserts the principal,
// not the numbers: attempts must spread out, than reset on a successful ack.
