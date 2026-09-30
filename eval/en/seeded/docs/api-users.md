# Users API

Base URL: https://api.acme.example/v2. All endpoints require a bearer token. Responses
are JSON with `Content-Type: application/json; charset=utf-8`.

## Create a user

```http
POST /users
```

| Field | Type | Required | Notes |
|---|---|---|---|
| `email` | string | yes | Lowercased on save |
| `name` | string | no | Max 200 chars |
| `locale` | string | no | BCP 47, default `en-US` |

Email addresses is unique per workspace. Creating a user with a email
that already exists returns `409 Conflict` and the exisiting user's id in
the `Location` header, so the call is safe to retry.

```json
{
  "id": "usr_01J9X4K2",
  "email": "vaino.makela@example.fi",
  "name": "Väinö Mäkelä",
  "created_at": "2026-03-01T09:12:44Z"
}
```

## Get a user

```http
GET /users/{id}
```

Returns `404` when the user does not exist or belongs to another workspace. We do not
distinguish the two cases on purpose; doing so would leak weather an id is valid.

## Update a user

```http
PATCH /users/{id}
```

Only the fields you send are changed. Sending `null` clears a field. The
`email` field can be changed but new address must be verified
before its used for login. Until then the response carries both
`email` and `pending_email`.

## Delete a user

```http
DELETE /users/{id}
```

Deletion is soft for 30 days, then permanent. During the soft period the user
can be restored with `POST /users/{id}/restore`. The user's sessions are revoked immediately, their API keys are not.
Revoke keys separately through the keys endpoint if needed.

## List users

```http
GET /users?limit=50&cursor=...
```

Cursor pagination. The `limit` maximum is 200. Results are ordered by `created_at`
decending. Filtering by email uses an exact match after
normalisation; there is no prefix search.

## Rate limits

600 requests/minute per token. The `Retry-After` header tells you how long to wait. Bulk
imports should use the Import API instead, it's built for it.
