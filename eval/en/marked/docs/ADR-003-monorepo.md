# ADR 003: Move backend services into one repository

Status: proposed
Date: 2026-02-02

## Context

We have 14 backend repositories, each with ⟦its_its|it's|its⟧ own ⟪acronym|CI⟫ pipeline, lint
configuration and release process. Cross-cutting changes such as bumping the ⟪product|tokio⟫
version or fixing a shared ⟪term|middleware⟫ take a ⟦missing_extra_word|week pull|week of pull⟧
requests and rarely land everywhere. The shared crates live in a 15th repository, ⟪product|acme-common⟫,
⟦homophone|who's|whose⟧ versioning nobody enjoys.

## Decision

Merge the 14 service repositories and acme-common into one repository, ⟪product|backend⟫, using a
⟪product|Cargo⟫ workspace. History is preserved with ⟪code|`git subtree`⟫. Each service keeps
⟪correct|its⟫ own ⟪path|Dockerfile⟫ and deploys independently.

## Consequences

- CI runs only the crates ⟦homophone|effected|affected⟧ by a change, using ⟪product|cargo-hakari⟫
  and a change detection script.
- One ⟪path|rust-toolchain.toml⟫, one ⟪path|clippy.toml⟫, one ⟪path|deny.toml⟫.
- The repository will be big. A fresh clone is about ⟪unit|1.2GB⟫ today; we expect
  ⟪unit|3GB⟫ within two years. ⟪term|Partial clone⟫ and ⟪term|sparse checkout⟫ are documented in the
  onboarding guide.
- Code owners replace per-repository permissions. Reviews are still required.
  the ⟪path|CODEOWNERS⟫ file decides who.

## Risks

Merge conflicts on ⟪path|Cargo.lock⟫ will be more common ⟦then_than|then|than⟧ today. We accept
this; ⟪product|Cargo⟫ resolves most of them ⟪informal|and the rest are a two-minute fix, honestly⟫.

The build cache is ⟦a_an|an shared|a shared⟧ resource now. If ⟪product|sccache⟫ goes down,
every team feels it at once rather ⟪correct|than⟫ one team at a time.

Teams that ⟦spelling|prefered|preferred⟧ full autonomy over ⟦their_there|there|their⟧ tooling will lose
some of it. This is especially true for the payments team, which pinned an old formatter for years.
