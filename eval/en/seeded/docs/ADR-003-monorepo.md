# ADR 003: Move backend services into one repository

Status: proposed
Date: 2026-02-02

## Context

We have 14 backend repositories, each with it's own CI pipeline, lint
configuration and release process. Cross-cutting changes such as bumping the tokio
version or fixing a shared middleware take a week pull
requests and rarely land everywhere. The shared crates live in a 15th repository, acme-common,
who's versioning nobody enjoys.

## Decision

Merge the 14 service repositories and acme-common into one repository, backend, using a
Cargo workspace. History is preserved with `git subtree`. Each service keeps
its own Dockerfile and deploys independently.

## Consequences

- CI runs only the crates effected by a change, using cargo-hakari
  and a change detection script.
- One rust-toolchain.toml, one clippy.toml, one deny.toml.
- The repository will be big. A fresh clone is about 1.2GB today; we expect
  3GB within two years. Partial clone and sparse checkout are documented in the
  onboarding guide.
- Code owners replace per-repository permissions. Reviews are still required.
  the CODEOWNERS file decides who.

## Risks

Merge conflicts on Cargo.lock will be more common then today. We accept
this; Cargo resolves most of them and the rest are a two-minute fix, honestly.

The build cache is an shared resource now. If sccache goes down,
every team feels it at once rather than one team at a time.

Teams that prefered full autonomy over there tooling will lose
some of it. This is especially true for the payments team, which pinned an old formatter for years.
