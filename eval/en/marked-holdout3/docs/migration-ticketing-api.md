# Migrating the museum ticketing API to v3

Version 3 replaces per-visit tickets with time-slot reservations. The ⟪acronym|SOAP⟫ endpoints
from ⟪version|v1⟫ are gone; ⟪version|v2⟫ ⟪acronym|REST⟫ stays until ⟪unit|2027-03-31⟫. This page is for
partners who integrate the box office, the ⟪product|Shopify⟫ plugin or ⟦their_there|there|their⟧ own kiosks.

## What changes

| v2 | v3 | Notes |
|---|---|---|
| ⟪code|`POST /tickets`⟫ | ⟪code|`POST /reservations`⟫ | ⟪table|slot required⟫ |
| ⟪code|`GET /tickets/{id}`⟫ | ⟪code|`GET /reservations/{id}`⟫ | ⟪table|same shape plus `slot`⟫ |
| ⟪code|`DELETE /tickets/{id}`⟫ | ⟪code|`POST /reservations/{id}/cancel`⟫ | ⟪table|returns refund status⟫ |
| ⟪code|`GET /prices`⟫ | ⟪code|`GET /slots?date=`⟫ | ⟪table|prices are per slot⟫ |

A reservation ⟦agreement|hold|holds⟧ up to ⟪unit|20⟫ visitors. Group bookings above that ⟦agreement|goes|go⟧
through the groups desk, ⟦homophone|who's|whose⟧ ⟪acronym|API⟫ is unchanged. ⟦its_its|Its|It's⟧ the same
key for both; ⟦your_youre|you're|your⟧ existing credentials keep working.

## Slots

Slots are ⟪unit|30 minutes⟫ long and open ⟪unit|60 days⟫ ahead. ⟦a_an|An slot|A slot⟧ has a capacity
and a price list; when capacity ⟦agreement|reach|reaches⟧ zero the slot still appears with
⟪code|`available: 0`⟫ so kiosks can show it as sold out ⟦then_than|rather then|rather than⟧ missing.
⟦punctuation|Do not cache slot availability for more than 60 seconds, popular exhibitions sell out in minutes.|Do not cache slot availability for more than 60 seconds; popular exhibitions sell out in minutes.⟧

## Idempotency

⟪code|`POST /reservations`⟫ requires an ⟪code|`Idempotency-Key`⟫. Retrying with the same key returns
the original reservation, even if ⟦its_its|its|it's⟧ since been cancelled; the response carries
⟦its_its|it's|its⟧ current ⟪code|`status`⟫. Keys expire after ⟪unit|48 hours⟫. Kiosks that generate
⟦their_there|they're|their⟧ keys from the terminal id and a counter have had no duplicates in the pilot.

## Errors

Error bodies are ⟪acronym|RFC⟫ 9457 problem details. The ⟪code|`type`⟫ ⟪acronym|URI⟫ is stable;
⟪code|`detail`⟫ is for humans and changes. ⟦fragment|Same rule as v2.|This is the same rule as in v2.⟧ New in v3:
⟪code|`slot-full`⟫, ⟪code|`slot-closed`⟫ and ⟪code|`visitor-limit`⟫. ⟦a_an|An 429|A 429⟧ carries
⟪code|`Retry-After`⟫; kiosks should back off ⟦then_than|then|than⟧ retry ⟦missing_extra_word|with same key|with the same key⟧.

## Webhooks

⟪code|`reservation.cancelled`⟫ and ⟪code|`slot.closed`⟫ are new. Payloads are signed with the
same ⟪acronym|HMAC⟫ secret as ⟪version|v2⟫. Delivery is at least once; ⟦your_youre|you're|your⟧ handler
must be ⟪term|idempotent⟫ on ⟪code|`event.id`⟫, ⟦homophone|witch|which⟧ was true for v2 as well but
⟦spelling|rarly|rarely⟧ implemented.

## Timeline

- ⟪unit|2026-09-01⟫: v3 generally available; v2 continues.
- ⟪unit|2026-12-01⟫: new partner keys are v3 only.
- ⟪unit|2027-03-31⟫: v2 shut down. ⟦their_there|There|Their⟧ will be no extension; the ⟪version|v1⟫
  shutdown taught us that ⟦a_an|an deadline|a deadline⟧ with an extension is not a deadline.

Questions to ⟪name|Aada Virtanen⟫ (partner integrations) or the ⟪code|`#ticketing-api`⟫ channel.
⟪informal|We answer faster than the old ticket queue did, which is a low bar.⟫
