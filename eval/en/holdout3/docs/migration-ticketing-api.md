# Migrating the museum ticketing API to v3

Version 3 replaces per-visit tickets with time-slot reservations. The SOAP endpoints
from v1 are gone; v2 REST stays until 2027-03-31. This page is for
partners who integrate the box office, the Shopify plugin or there own kiosks.

## What changes

| v2 | v3 | Notes |
|---|---|---|
| `POST /tickets` | `POST /reservations` | slot required |
| `GET /tickets/{id}` | `GET /reservations/{id}` | same shape plus `slot` |
| `DELETE /tickets/{id}` | `POST /reservations/{id}/cancel` | returns refund status |
| `GET /prices` | `GET /slots?date=` | prices are per slot |

A reservation hold up to 20 visitors. Group bookings above that goes
through the groups desk, who's API is unchanged. Its the same
key for both; you're existing credentials keep working.

## Slots

Slots are 30 minutes long and open 60 days ahead. An slot has a capacity
and a price list; when capacity reach zero the slot still appears with
`available: 0` so kiosks can show it as sold out rather then missing.
Do not cache slot availability for more than 60 seconds, popular exhibitions sell out in minutes.

## Idempotency

`POST /reservations` requires an `Idempotency-Key`. Retrying with the same key returns
the original reservation, even if its since been cancelled; the response carries
it's current `status`. Keys expire after 48 hours. Kiosks that generate
they're keys from the terminal id and a counter have had no duplicates in the pilot.

## Errors

Error bodies are RFC 9457 problem details. The `type` URI is stable;
`detail` is for humans and changes. Same rule as v2. New in v3:
`slot-full`, `slot-closed` and `visitor-limit`. An 429 carries
`Retry-After`; kiosks should back off then retry with same key.

## Webhooks

`reservation.cancelled` and `slot.closed` are new. Payloads are signed with the
same HMAC secret as v2. Delivery is at least once; you're handler
must be idempotent on `event.id`, witch was true for v2 as well but
rarly implemented.

## Timeline

- 2026-09-01: v3 generally available; v2 continues.
- 2026-12-01: new partner keys are v3 only.
- 2027-03-31: v2 shut down. There will be no extension; the v1
  shutdown taught us that an deadline with an extension is not a deadline.

Questions to Aada Virtanen (partner integrations) or the `#ticketing-api` channel.
We answer faster than the old ticket queue did, which is a low bar.
