# Users API

Base URL: ⟪url|https://api.acme.example/v2⟫. All endpoints require a bearer token. Responses
are ⟪acronym|JSON⟫ with ⟪code|`Content-Type: application/json; charset=utf-8`⟫.

## Create a user

```http
POST /users
```

| Field | Type | Required | Notes |
|---|---|---|---|
| ⟪code|`email`⟫ | ⟪table|string⟫ | ⟪table|yes⟫ | ⟪table|Lowercased on save⟫ |
| ⟪code|`name`⟫ | ⟪table|string⟫ | ⟪table|no⟫ | ⟪table|Max 200 chars⟫ |
| ⟪code|`locale`⟫ | ⟪table|string⟫ | ⟪table|no⟫ | ⟪table|BCP 47, default `en-US`⟫ |

Email addresses ⟦agreement|is|are⟧ unique per workspace. Creating a user with ⟦a_an|a email|an email⟧
that already exists returns ⟪code|`409 Conflict`⟫ and the ⟦spelling|exisiting|existing⟧ user's id in
the ⟪code|`Location`⟫ header, so the call is safe to retry.

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

Returns ⟪code|`404`⟫ when the user does not exist or belongs to another workspace. We do not
distinguish the two cases on purpose; doing so would leak ⟦homophone|weather|whether⟧ an id is valid.

## Update a user

```http
PATCH /users/{id}
```

Only the fields you send are changed. Sending ⟪code|`null`⟫ clears a field. The
⟪code|`email`⟫ field can be changed ⟦missing_extra_word|but new address|but the new address⟧ must be verified
before ⟦its_its|its|it's⟧ used for login. Until then the response carries both
⟪code|`email`⟫ and ⟪code|`pending_email`⟫.

## Delete a user

```http
DELETE /users/{id}
```

Deletion is soft for ⟪unit|30 days⟫, ⟪correct|then⟫ permanent. During the soft period the user
can be restored with ⟪code|`POST /users/{id}/restore`⟫. ⟦punctuation|The user's sessions are revoked immediately, their API keys are not.|The user's sessions are revoked immediately; their API keys are not.⟧
Revoke keys separately through the keys endpoint if needed.

## List users

```http
GET /users?limit=50&cursor=...
```

Cursor pagination. The ⟪code|`limit`⟫ maximum is 200. Results are ordered by ⟪code|`created_at`⟫
⟦spelling|decending|descending⟧. Filtering by email uses ⟪correct|an exact⟫ match after
⟦british|normalisation|normalization⟧; there is no prefix search.

## Rate limits

⟪unit|600 requests/minute⟫ per token. The ⟪code|`Retry-After`⟫ header tells you how long to wait. Bulk
imports should use the ⟪product|Import API⟫ instead, ⟪correct|it's⟫ built for it.
