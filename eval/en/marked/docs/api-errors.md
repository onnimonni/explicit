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

Always log the ⟪code|`request_id`⟫. Support cannot help without it, and
⟪informal|"it failed yesterday around lunch" is not a request id⟫.

## Status codes

| Code | Meaning | Retry? |
|---|---|---|
| ⟪table|400⟫ | ⟪table|Malformed request⟫ | ⟪table|No⟫ |
| ⟪table|401⟫ | ⟪table|Missing or invalid token⟫ | ⟪table|No⟫ |
| ⟪table|403⟫ | ⟪table|Token lacks permission⟫ | ⟪table|No⟫ |
| ⟪table|404⟫ | ⟪table|Not found or not yours⟫ | ⟪table|No⟫ |
| ⟪table|409⟫ | ⟪table|Conflict, see body⟫ | ⟪table|Sometimes⟫ |
| ⟪table|422⟫ | ⟪table|Validation failed⟫ | ⟪table|No⟫ |
| ⟪table|429⟫ | ⟪table|Rate limited⟫ | ⟪table|After `Retry-After`⟫ |
| ⟪table|5xx⟫ | ⟪table|Our fault⟫ | ⟪table|Yes, with backoff⟫ |

## Error codes

The ⟪code|`code`⟫ field is stable and safe to switch on. The ⟪code|`message`⟫ is for humans and
⟦agreement|change|changes⟧ without notice. Do not match on message text; we have broken integrations that did.

Common codes:

- ⟪code|`invalid_request`⟫: ⟪list|a field is missing or has the wrong type⟫
- ⟪code|`idempotency_mismatch`⟫: ⟪list|same key, different body⟫
- ⟪code|`insufficient_funds`⟫: ⟪list|the customer's card declined⟫
- ⟪code|`resource_locked`⟫: ⟪list|another request is modifying the resource⟫

⟪code|`resource_locked`⟫ is ⟦a_an|an transient|a transient⟧ condition. Wait ⟪unit|100ms⟫ and retry, up
to five times. If ⟦its_its|its|it's⟧ still locked after that, something upstream is stuck and
⟪correct|your⟫ best option is to give up and alert.

## Validation errors

⟪code|`422`⟫ responses include an ⟪code|`errors`⟫ array with one entry per
failing field. Fields are named with ⟪acronym|JSON⟫ pointer syntax, for example
⟪code|`/items/2/quantity`⟫.

## Retrying safely

Retry ⟪table|5xx⟫ and ⟪table|429⟫ with ⟪term|exponential backoff⟫ and jitter. Never retry a
⟪code|`POST`⟫ without an ⟪code|`Idempotency-Key`⟫; you will ⟦spelling|definately|definitely⟧ create
duplicates. ⟦missing_extra_word|The both|Both⟧ ⟪acronym|SDK⟫s do this for you.

Timeouts are ambiguous: the request may or may not have been ⟦spelling|proccessed|processed⟧. Treat a
timeout like a ⟪table|5xx⟫ and retry with the same key. ⟪correct|There⟫ is no way to know
otherwise, which is exactly why the key exists.
