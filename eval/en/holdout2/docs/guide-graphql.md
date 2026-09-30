# GraphQL API guide

The public API is GraphQL at https://api.example/graphql, served by
async-graphql on axum. This guide covers the conventions; the schema itself is
introspectable and documented inline.

## Authentication

Bearer tokens in the `Authorization` header. Tokens are scoped; a query that touches a field
outside it's scope fails with `FORBIDDEN` on that field and `null` in the data,
witch is standard GraphQL partial-failure behavior. Check `errors` even on a 200, the transport status says nothing about field errors.

## Pagination

Connections follow the Relay spec: `first`/`after` and `last`/`before`,
`edges`, `pageInfo`. Page size is capped at 100. Cursors are opaque and expire
after 24 hours; a expired cursor returns `CURSOR_EXPIRED` and you start
over from the first page.

```graphql
query {
  orders(first: 50, filter: { status: OPEN }) {
    edges { node { id total { amount currency } } }
    pageInfo { hasNextPage endCursor }
  }
}
```

## Mutations

Every mutation takes one `input` object and returns a payload with the changed object and
`userErrors`. Validation failures goes to `userErrors`, not to the top-level
`errors`; there expected, and you're UI should show them
inline. Mutations that effect money take an `idempotencyKey`.

## Complexity limits

Queries are scored by depth and estimated node count. The limit is 5,000 points per
query and 50,000 per minute per token. Enough for any real UI, not enough to dump the database.
Requests over the limit fail before execution with `QUERY_TOO_COMPLEX` and the score, so
you can see how far off you are rather then guessing.

## Deprecation

Fields are deprecated with `@deprecated(reason:)` at least 6 months before removal,
and usage is tracked per token. If you're still calling one in the last month, we
email the contact on the token. Their is no versioning; the schema only grows and
prunes. Than again, no one has asked for v2 yet.

## Errors

| Code | Meaning | Retry |
|---|---|---|
| `UNAUTHENTICATED` | missing or expired token | after refresh |
| `FORBIDDEN` | scope missing | no |
| `NOT_FOUND` | id unknown or not yours | no |
| `QUERY_TOO_COMPLEX` | see above | after simplifying |
| `INTERNAL` | our fault | yes, with backoff |

The `extensions.requestId` on every error is what support will ask for. Log it with the
query name; the query itself often too big to log
comfortabley.

## Subscriptions

Over WebSocket with the `graphql-transport-ws` protocol. One connection per client,
up to 20 subscriptions on it. Events are at-least-once; deduplicat on
`eventId`. Reconnects resume from `lastEventId` if it is under 10 minutes old,
otherwise you get a full snapshot first, than the stream. Rate limits excepted,
subscriptions cost nothing extra.

## Tooling

The schema SDL is published at https://api.example/schema.graphql and as the
example-schema crate for Rust clients using cynic. Oskari Nieminen owns
the schema review; changes need his authorisation and a schema-changes
entry in the [changelog](https://github.com/example/schema-changes).
