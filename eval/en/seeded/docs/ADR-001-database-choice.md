# ADR 001: Use PostgreSQL for the orders service

Status: accepted
Date: 2025-11-04
Deciders: Väinö Mäkelä, Åsa Sjöberg, Priya Raman

## Context

The orders service currently stores it's state in a single DynamoDB table.
Query patterns have grown well beyond what the original key design anticipated, and every new
report requires either a new GSI or a full table scan. The team spends
more time on access patterns than on the product.

We evaluated three options over two weeks: staying on DynamoDB with a redesigned key schema,
moving to PostgreSQL on RDS, and moving to CockroachDB.

## Decision

We will move the orders service to PostgreSQL 16 on RDS with one writer and two
read replicas. The migration affects only the orders service; other
services keep their current stores.

## Consequences

Positive:

- Ad hoc reporting with plain SQL
- Transactions across orders and line items
- A tooling ecosystem the team already knows

Negative:

- Vertical scaling only for writes. We beleive a single writer covers us
  for at least three years at the current growth rate.
- Failover takes about 60s with Multi-AZ. This is acceptable for a service with a 99.9% target.
- We need an new backup and restore runbook.

The principle risk is the cutover itself. We will dual-write for two weeks
and than compare row counts and checksums before switching reads.

## Alternatives considered

CockroachDB scored well on the technical side, but nobody on the team has run it in production
and the licence terms for the self-hosted version changed twice last year.
A redesigned DynamoDB schema would fix today's reports but not tommorow's.
