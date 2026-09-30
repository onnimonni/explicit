# ADR 002: Do not adopt event sourcing for billing

Status: accepted
Date: 2026-01-19

## Context

A proposal from the platform group suggested rebuilding billing around an append-only event
log, with projections for every read model. The argument was that the audit trail would be free
and that replaying events makes bugs easy to fix after the fact.

Billing has about 40 aggregate types and ⟪unit|2TB⟫ of historical data. Invoices are legally
binding documents, and ⟦their_there|their|there⟧ is no undo once they are sent.

## Decision

We keep the current relational model and add an outbox table for integration events. We do not
adopt event sourcing for billing.

## Reasoning

Event sourcing solves problems we do not have and introduces ones we would
rather avoid:

1. Schema evolution of stored events is hard. Every change to an event needs an upcaster, and
   after ⟦a_an|an year|a year⟧ ⟪correct|your⟫ replay path runs code nobody remembers.
2. Projections lag. Support staff would see an invoice ⟪correct|than⟫ fail to find it a second
   later in another view. This is a confusing experience that generates tickets.
3. ⟦capitalization|gdpr|GDPR⟧ erasure is awkward when history is immutable. Crypto-shredding works but
   adds ⟪correct|a key⟫ management problem on top of the billing problem.

The audit requirement is real. We meet it with ⟪product|PostgreSQL⟫ ⟪term|temporal tables⟫, which
give us row history without changing how the application writes data. ⟦punctuation|The team already knows this pattern, it is used in the accounts service.|The team already knows this pattern; it is used in the accounts service.⟧

## Consequences

- The outbox table needs a relay process. We reuse the one from notifications.
- Replay of integration events is possible for the last ⟪unit|90 days⟫ only.
- Anyone proposing event sourcing again should ⟦spelling|adress|address⟧ points 1 to 3 above first.

## Notes

⟪name|Mikko Järvinen⟫ raised a valid point about analytics: ⟪correct|it's⟫ easier to build a
funnel from events ⟦then_than|then|than⟧ from row history. We will export outbox events to the
warehouse, which covers that use case without turning billing into an event store.
