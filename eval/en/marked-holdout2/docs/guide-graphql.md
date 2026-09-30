# GraphQL API guide

The public ⟪acronym|API⟫ is ⟪product|GraphQL⟫ at ⟪url|https://api.example/graphql⟫, served by
⟪crate|async-graphql⟫ on ⟪crate|axum⟫. This guide covers the conventions; the schema itself is
introspectable and documented inline.

## Authentication

Bearer tokens in the ⟪code|`Authorization`⟫ header. Tokens are scoped; a query that touches a field
outside ⟦its_its|it's|its⟧ scope fails with ⟪code|`FORBIDDEN`⟫ on that field and ⟪code|`null`⟫ in the data,
⟦homophone|witch|which⟧ is standard ⟪product|GraphQL⟫ partial-failure behavior. ⟦punctuation|Check `errors` even on a 200, the transport status says nothing about field errors.|Check `errors` even on a 200; the transport status says nothing about field errors.⟧

## Pagination

Connections follow the ⟪product|Relay⟫ spec: ⟪code|`first`⟫/⟪code|`after`⟫ and ⟪code|`last`⟫/⟪code|`before`⟫,
⟪code|`edges`⟫, ⟪code|`pageInfo`⟫. Page size is capped at ⟪unit|100⟫. Cursors are opaque and expire
after ⟪unit|24 hours⟫; ⟦a_an|a expired|an expired⟧ cursor returns ⟪code|`CURSOR_EXPIRED`⟫ and you start
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

Every mutation takes one ⟪code|`input`⟫ object and returns a payload with the changed object and
⟪code|`userErrors`⟫. Validation failures ⟦agreement|goes|go⟧ to ⟪code|`userErrors`⟫, not to the top-level
⟪code|`errors`⟫; ⟦their_there|there|they're⟧ expected, and ⟦your_youre|you're|your⟧ ⟪acronym|UI⟫ should show them
inline. Mutations that ⟦homophone|effect|affect⟧ money take an ⟪code|`idempotencyKey`⟫.

## Complexity limits

Queries are scored by depth and estimated node count. The limit is ⟪unit|5,000⟫ points per
query and ⟪unit|50,000⟫ per minute per token. ⟦fragment|Enough for any real UI, not enough to dump the database.|That is enough for any real UI and not enough to dump the database.⟧
Requests over the limit fail before execution with ⟪code|`QUERY_TOO_COMPLEX`⟫ and the score, so
you can see how far off you are ⟦then_than|rather then|rather than⟧ guessing.

## Deprecation

Fields are deprecated with ⟪code|`@deprecated(reason:)`⟫ at least ⟪unit|6 months⟫ before removal,
and usage is tracked per token. If ⟦your_youre|you're|your⟧ still calling one in the last month, we
email the contact on the token. ⟦their_there|Their|There⟧ is no versioning; the schema only grows and
prunes. ⟦then_than|Than|Then⟧ again, no one has asked for ⟪version|v2⟫ yet.

## Errors

| Code | Meaning | Retry |
|---|---|---|
| ⟪code|`UNAUTHENTICATED`⟫ | ⟪table|missing or expired token⟫ | ⟪table|after refresh⟫ |
| ⟪code|`FORBIDDEN`⟫ | ⟪table|scope missing⟫ | ⟪table|no⟫ |
| ⟪code|`NOT_FOUND`⟫ | ⟪table|id unknown or not yours⟫ | ⟪table|no⟫ |
| ⟪code|`QUERY_TOO_COMPLEX`⟫ | ⟪table|see above⟫ | ⟪table|after simplifying⟫ |
| ⟪code|`INTERNAL`⟫ | ⟪table|our fault⟫ | ⟪table|yes, with backoff⟫ |

The ⟪code|`extensions.requestId`⟫ on every error is what support will ask for. Log it with the
query name; ⟦missing_extra_word|the query itself often too big|the query itself is often too big⟧ to log
⟦spelling_1edit|comfortabley|comfortably⟧.

## Subscriptions

Over ⟪acronym|WebSocket⟫ with the ⟪code|`graphql-transport-ws`⟫ protocol. One connection per client,
up to ⟪unit|20⟫ subscriptions on it. Events are at-least-once; ⟦spelling|deduplicat|deduplicate⟧ on
⟪code|`eventId`⟫. Reconnects resume from ⟪code|`lastEventId`⟫ if it is under ⟪unit|10 minutes⟫ old,
otherwise you get a full snapshot first, ⟦then_than|than|then⟧ the stream. Rate limits ⟪correct|excepted⟫,
subscriptions cost nothing extra.

## Tooling

The schema ⟪acronym|SDL⟫ is published at ⟪url|https://api.example/schema.graphql⟫ and as the
⟪crate|example-schema⟫ crate for ⟪product|Rust⟫ clients using ⟪crate|cynic⟫. ⟪name|Oskari Nieminen⟫ owns
the schema review; changes need his ⟦british|authorisation|authorization⟧ and a ⟪linktext|schema-changes⟫
entry in the [changelog](https://github.com/example/schema-changes).
