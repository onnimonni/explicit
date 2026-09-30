# Webhooks

Webhooks notify your server when something happens in Acme. You register an
HTTPS endpoint, pick the events you care about, and we `POST` to it.

## Registering an endpoint

```http
POST /webhooks
{ "url": "https://example.com/hooks/acme", "events": ["payment.captured", "refund.*"] }
```

The response contains a `secret` shown exactly once. Store it; you need it to verify
signatures. If you loose it, rotate it with `POST /webhooks/{id}/rotate`.

## Delivery

Each event is delivered at least once. In in rare cases you will see the same
event twice, so make you're handler idempotent by keying on `event.id`.

We retry failed deliveries with backoff: 1m, 5m, 30m, 2h, 6h,
then once a day for three days. A delivery counts as failed when your endpoint
returns anything other then `2xx` within 10s. Return quickly and do the
work afterwards; a slow handler that eventually succeeds still counts as a failure.

## Verifying signatures

Every request carries `Acme-Signature: t=1712345678,v1=<hex>`. Compute
`HMAC-SHA256(secret, t + "." + body)` over the raw body and compare with a constant-time
comparison. Reject requests who's timestamp is older than 5 minutes.

Do not parse the json before verifying. Most libraries re-serialize with different whitespace, so the signature will not match.

## Event shape

```json
{
  "id": "evt_01J9XA7Q",
  "type": "payment.captured",
  "created_at": "2026-03-01T09:15:02Z",
  "data": { "id": "pay_3k1", "amount": 1999 }
}
```

The `data` object matches the resource as returned by the corresponding
`GET` endpoint at the time of the event. It may be stale by the time you process it; fetch
the resource again if you need it's current state.

## Disabling

Endpoints that fail every delivery for 7 days are disabled automatically and the workspace
owner gets an email. Re-enable with `POST /webhooks/{id}/enable`. Missed events
are not replayed; use the Events API to backfil.

## Local development

Use ngrok or the Acme CLI `acme listen --forward-to localhost:3000`,
which streams events to you're machine over a WebSocket. No tunnels, no fuss.
