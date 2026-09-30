# Payments API

Payments are processed through ⟪product|Stripe⟫ or ⟪product|Adyen⟫ depending on the merchant's
region. The ⟪acronym|API⟫ hides the difference; you never talk to the processor directly.

## Create a payment intent

```http
POST /payments
Idempotency-Key: 6f1c0b2e-...
```

The ⟪code|`Idempotency-Key`⟫ header is required. Two requests with the same key and body
return the same payment; the same key with a different body returns
⟪code|`422`⟫. Keys expire after ⟪unit|24 hours⟫. This is the only ⟦spelling|mechansim|mechanism⟧ that
protects you from charging a customer twice, so do not generate the key per retry.

```json
{
  "amount": 1999,
  "currency": "EUR",
  "customer": "cus_8f2",
  "capture": "manual"
}
```

Amounts are integers in the currency's minor unit. ⟦punctuation|EUR 19.99 is 1999, JPY 1999 is 1999.|EUR 19.99 is 1999; JPY 1999 is 1999.⟧
Sending a float ⟦agreement|are|is⟧ rejected.

## Capture

```http
POST /payments/{id}/capture
```

Manual capture must happen within ⟪unit|7 days⟫ or the authorization
⟦spelling|expiers|expires⟧. Partial capture is supported; the remainder is released. You cannot
capture more ⟪correct|than⟫ was authorized.

## Refunds

```http
POST /payments/{id}/refunds
```

Refunds are asynchronous. The response has ⟪code|`status: "pending"`⟫ and a webhook
⟪code|`refund.succeeded`⟫ or ⟪code|`refund.failed`⟫ follows, usually within minutes but
⟦spelling|ocassionally|occasionally⟧ up to ⟪unit|5 business days⟫ for bank transfers. This depends on the customer's bank.

A refund larger than the captured amount ⟦agreement|fail|fails⟧ with ⟪code|`refund_exceeds_capture`⟫.
Multiple partial refunds are fine as long as ⟪correct|their⟫ sum stays within the capture.

## Statuses

| Status | Meaning |
|---|---|
| ⟪code|`requires_action`⟫ | ⟪table|3DS or similar needed⟫ |
| ⟪code|`authorized`⟫ | ⟪table|Funds held, not captured⟫ |
| ⟪code|`captured`⟫ | ⟪table|Done⟫ |
| ⟪code|`canceled`⟫ | ⟪table|Released before capture⟫ |

Note the American spelling ⟪code|`canceled`⟫ in status values; it is not
⟪correct|affected⟫ by the account's locale setting.

## Testing

Use the test keys and the card number ⟪code|`4242 4242 4242 4242`⟫. To simulate ⟦a_an|an decline|a decline⟧,
use ⟪code|`4000 0000 0000 0002`⟫. Test payments never ⟦spelling|recieve|receive⟧ real webhooks from
the processor; we synthesize them, which means timing differs from production.
