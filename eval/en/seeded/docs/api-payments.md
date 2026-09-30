# Payments API

Payments are processed through Stripe or Adyen depending on the merchant's
region. The API hides the difference; you never talk to the processor directly.

## Create a payment intent

```http
POST /payments
Idempotency-Key: 6f1c0b2e-...
```

The `Idempotency-Key` header is required. Two requests with the same key and body
return the same payment; the same key with a different body returns
`422`. Keys expire after 24 hours. This is the only mechansim that
protects you from charging a customer twice, so do not generate the key per retry.

```json
{
  "amount": 1999,
  "currency": "EUR",
  "customer": "cus_8f2",
  "capture": "manual"
}
```

Amounts are integers in the currency's minor unit. EUR 19.99 is 1999, JPY 1999 is 1999.
Sending a float are rejected.

## Capture

```http
POST /payments/{id}/capture
```

Manual capture must happen within 7 days or the authorization
expiers. Partial capture is supported; the remainder is released. You cannot
capture more than was authorized.

## Refunds

```http
POST /payments/{id}/refunds
```

Refunds are asynchronous. The response has `status: "pending"` and a webhook
`refund.succeeded` or `refund.failed` follows, usually within minutes but
ocassionally up to 5 business days for bank transfers. This depends on the customer's bank.

A refund larger than the captured amount fail with `refund_exceeds_capture`.
Multiple partial refunds are fine as long as their sum stays within the capture.

## Statuses

| Status | Meaning |
|---|---|
| `requires_action` | 3DS or similar needed |
| `authorized` | Funds held, not captured |
| `captured` | Done |
| `canceled` | Released before capture |

Note the American spelling `canceled` in status values; it is not
affected by the account's locale setting.

## Testing

Use the test keys and the card number `4242 4242 4242 4242`. To simulate an decline,
use `4000 0000 0000 0002`. Test payments never recieve real webhooks from
the processor; we synthesize them, which means timing differs from production.
