# Error handling

Every error response has the same shape, regardless of endpoint:

```json
{
  "error": {
    "code": "invalid_request",
    "message": "amount must be a positive integer",
    "param": "amount",
    "request_id": "req_01J9XB2M"
  }
}
```

Always log the `request_id`. Support cannot help without it, and
"it failed yesterday around lunch" is not a request id.

## Status codes

| Code | Meaning | Retry? |
|---|---|---|
| 400 | Malformed request | No |
| 401 | Missing or invalid token | No |
| 403 | Token lacks permission | No |
| 404 | Not found or not yours | No |
| 409 | Conflict, see body | Sometimes |
| 422 | Validation failed | No |
| 429 | Rate limited | After `Retry-After` |
| 5xx | Our fault | Yes, with backoff |

## Error codes

The `code` field is stable and safe to switch on. The `message` is for humans and
change without notice. Do not match on message text; we have broken integrations that did.

Common codes:

- `invalid_request`: a field is missing or has the wrong type
- `idempotency_mismatch`: same key, different body
- `insufficient_funds`: the customer's card declined
- `resource_locked`: another request is modifying the resource

`resource_locked` is an transient condition. Wait 100ms and retry, up
to five times. If its still locked after that, something upstream is stuck and
your best option is to give up and alert.

## Validation errors

`422` responses include an `errors` array with one entry per
failing field. Fields are named with JSON pointer syntax, for example
`/items/2/quantity`.

## Retrying safely

Retry 5xx and 429 with exponential backoff and jitter. Never retry a
`POST` without an `Idempotency-Key`; you will definately create
duplicates. The both SDKs do this for you.

Timeouts are ambiguous: the request may or may not have been proccessed. Treat a
timeout like a 5xx and retry with the same key. There is no way to know
otherwise, which is exactly why the key exists.
