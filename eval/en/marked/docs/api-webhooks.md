# Webhooks

Webhooks notify ⟪correct|your⟫ server when something happens in Acme. You register an
⟪acronym|HTTPS⟫ endpoint, pick the events you care about, and we ⟪code|`POST`⟫ to it.

## Registering an endpoint

```http
POST /webhooks
{ "url": "https://example.com/hooks/acme", "events": ["payment.captured", "refund.*"] }
```

The response contains a ⟪code|`secret`⟫ shown exactly once. Store it; you need it to verify
signatures. If you ⟦homophone|loose|lose⟧ it, rotate it with ⟪code|`POST /webhooks/{id}/rotate`⟫.

## Delivery

Each event is delivered at least once. ⟦repeated_word|In in|In⟧ rare cases you will see the same
event twice, so make ⟦your_youre|you're|your⟧ handler ⟪term|idempotent⟫ by keying on ⟪code|`event.id`⟫.

We retry failed deliveries with backoff: ⟪unit|1m⟫, ⟪unit|5m⟫, ⟪unit|30m⟫, ⟪unit|2h⟫, ⟪unit|6h⟫,
then once a day for three days. A delivery counts as failed when ⟪correct|your⟫ endpoint
returns anything other ⟦then_than|then|than⟧ ⟪code|`2xx`⟫ within ⟪unit|10s⟫. Return quickly and do the
work afterwards; a slow handler that eventually succeeds still counts as a failure.

## Verifying signatures

Every request carries ⟪code|`Acme-Signature: t=1712345678,v1=<hex>`⟫. Compute
⟪code|`HMAC-SHA256(secret, t + "." + body)`⟫ over the raw body and compare with a constant-time
comparison. Reject requests ⟦homophone|who's|whose⟧ timestamp is older than ⟪unit|5 minutes⟫.

Do not parse the ⟦capitalization|json|JSON⟧ before verifying. Most libraries re-serialize with different whitespace, so the signature will not match.

## Event shape

```json
{
  "id": "evt_01J9XA7Q",
  "type": "payment.captured",
  "created_at": "2026-03-01T09:15:02Z",
  "data": { "id": "pay_3k1", "amount": 1999 }
}
```

The ⟪code|`data`⟫ object matches the resource as returned by the corresponding
⟪code|`GET`⟫ endpoint at the time of the event. It may be stale by the time you process it; fetch
the resource again if you need ⟦its_its|it's|its⟧ current state.

## Disabling

Endpoints that fail every delivery for ⟪unit|7 days⟫ are disabled automatically and the workspace
owner gets ⟪correct|an email⟫. Re-enable with ⟪code|`POST /webhooks/{id}/enable`⟫. Missed events
are not replayed; use the ⟪product|Events API⟫ to ⟦spelling|backfil|backfill⟧.

## Local development

Use ⟪product|ngrok⟫ or the Acme ⟪acronym|CLI⟫ ⟪code|`acme listen --forward-to localhost:3000`⟫,
which streams events to ⟦your_youre|you're|your⟧ machine over a ⟪acronym|WebSocket⟫. ⟪informal|No tunnels, no fuss.⟫
