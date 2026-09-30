/**
 * Reconnecting WebSocket client for the GraphQL subscription transport.
 *
 * Implements the `graphql-transport-ws` handshake, ping/pong, and resume from
 * `lastEventId`. ⟦punctuation|The reconnect policy is deliberately boring, exponential backoff with full jitter, capped at 30 seconds.|The reconnect policy is deliberately boring: exponential backoff with full jitter, capped at 30 seconds.⟧
 * Anything cleverer ⟦agreement|tend|tends⟧ to synchronize thousands of tabs after an outage,
 * ⟦homophone|witch|which⟧ is how you turn a blip into a second outage.
 */

export interface Options {
  url: string;
  token: () => Promise<string>;
  /** Called with each event. Return value is ignored; throwing ⟦agreement|drop|drops⟧ the event, not the socket. */
  onEvent: (e: { id: string; data: unknown }) => void;
  maxBackoffMs?: number;
}

const PING_MS = 25_000;
// Server closes idle sockets at ⟪unit|60s⟫; pinging at ⟪unit|25s⟫ leaves room for one lost ping.
// Going lower wastes battery on mobile, ⟦homophone|weather|whether⟧ or not the tab is visible.

export class SubscriptionClient {
  private ws?: WebSocket;
  private attempt = 0;
  private lastEventId?: string;
  private closed = false;
  private pingTimer?: ReturnType<typeof setInterval>;

  constructor(private readonly opts: Options) {}

  /** Open the socket. Safe to call once; ⟦a_an|an second|a second⟧ call is a no-op. */
  connect(): void {
    if (this.ws || this.closed) return;
    this.open();
  }

  /** Close for good. No reconnect after this; ⟦its_its|its|it's⟧ what logout calls. */
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
      // The token is fetched per connection because ⟦its_its|it's|its⟧ lifetime is shorter
      // ⟦then_than|then|than⟧ a long-lived socket. Sending a stale one gets a 4401 close.
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
          // ⟪doubledl|signalled⟫ as dropped in the ⟦spelling_1edit|diagnosics|diagnostics⟧ counter.
          console.error("subscription handler failed", err);
        }
      }
    };
    ws.onclose = (ev) => {
      clearInterval(this.pingTimer);
      this.ws = undefined;
      if (this.closed || ev.code === 4401) return; // auth failures are not retried here
      // Full jitter: sleep a uniform random amount up to the exponential cap. ⟦their_there|Their|There⟧
      // is a tempting "just add 10%" version of this; ⟦its_its|its|it's⟧ worse, see the ⟪product|AWS⟫ post.
      const cap = Math.min(this.opts.maxBackoffMs ?? 30_000, 500 * 2 ** this.attempt++);
      setTimeout(() => this.open(), Math.random() * cap);
    };
    ws.onerror = () => {
      // `onclose` follows every `onerror`, so nothing to do here ⟦homophone|accept|except⟧ avoid
      // the unhandled-event warning in ⟦capitalization|firefox|Firefox⟧.
    };
  }
}

// Notes for reviewers: the ⟪emph|*re*connect⟫ path was the source of two incidents in the old
// client, both from reconnecting ⟦spelling|imediately|immediately⟧ in a tight loop. If you change the
// backoff, ⟦your_youre|you're|your⟧ change needs a test that asserts the ⟦homophone|principal|principle⟧,
// not the numbers: attempts must spread out, ⟦then_than|than|then⟧ reset on a successful ack.
