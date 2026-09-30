# ADR 001: Use PostgreSQL for the orders service

Status: accepted
Date: 2025-11-04
Deciders: ⟪name|Väinö Mäkelä⟫, ⟪name|Åsa Sjöberg⟫, Priya Raman

## Context

The orders service currently stores ⟦its_its|it's|its⟧ state in a single ⟪product|DynamoDB⟫ table.
Query patterns have grown well beyond what the original key design anticipated, and every new
report requires either a new ⟪acronym|GSI⟫ or a full table scan. The team spends
more time on access patterns ⟪correct|than⟫ on the product.

We evaluated three options over two weeks: staying on DynamoDB with a redesigned key schema,
moving to ⟪product|PostgreSQL⟫ on ⟪product|RDS⟫, and moving to ⟪product|CockroachDB⟫.

## Decision

We will move the orders service to PostgreSQL ⟪version|16⟫ on RDS with one writer and two
read replicas. The migration ⟪correct|affects⟫ only the orders service; other
services keep ⟪correct|their⟫ current stores.

## Consequences

Positive:

- ⟪list|Ad hoc reporting with plain SQL⟫
- ⟪list|Transactions across orders and line items⟫
- ⟪list|A tooling ecosystem the team already knows⟫

Negative:

- Vertical scaling only for writes. We ⟦spelling|beleive|believe⟧ a single writer covers us
  for at least three years at the current growth rate.
- Failover takes about ⟪unit|60s⟫ with ⟪product|Multi-AZ⟫. This is acceptable for a service with a 99.9% target.
- We need ⟦a_an|an new|a new⟧ backup and restore runbook.

The ⟦homophone|principle|principal⟧ risk is the cutover itself. We will dual-write for two weeks
⟦then_than|and than|and then⟧ compare row counts and checksums before switching reads.

## Alternatives considered

CockroachDB scored well on the technical side, but nobody on the team has run it in production
and the ⟦british|licence|license⟧ terms for the self-hosted version changed twice last year.
A redesigned DynamoDB schema would fix today's reports but not ⟦spelling|tommorow's|tomorrow's⟧.
