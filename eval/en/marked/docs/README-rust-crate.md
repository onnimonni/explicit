# tinyq

[![crates.io](https://img.shields.io/crates/v/tinyq.svg)](https://crates.io/crates/tinyq)

A bounded, lock-free ⟪acronym|MPSC⟫ queue for ⟪product|Rust⟫. ⟪code|`no_std`⟫ compatible, no
allocations after construction, and about 300 lines of code.

## Why

Most queue crates are either huge or unnecessarily generic. tinyq does one
thing: it moves ⟪code|`T: Send`⟫ values from many producers to one consumer without locks. The
capacity is fixed at construction time and must be ⟪correct|a power⟫ of two.

## Example

```rust
use tinyq::Queue;

let q: Queue<u32, 1024> = Queue::new();
let (tx, rx) = q.split();
tx.push(7).unwrap();
assert_eq!(rx.pop(), Some(7));
```

`push` returns ⟪code|`Err(value)`⟫ when the queue is full, so callers never
⟦homophone|loose|lose⟧ data silently. ⟪code|`pop`⟫ returns ⟪code|`None`⟫ when empty.

## Memory ordering

Producers reserve a slot with ⟪code|`fetch_add(1, AcqRel)`⟫ on the tail
counter ⟦then_than|and than|and then⟧ write the value. The consumer reads the head with
⟪code|`Acquire`⟫. ⟪term|Idempotent⟫ retries are safe because a failed reservation is rolled
back with a ⟪acronym|CAS⟫ loop. The implementation is ⟦spelling|similiar|similar⟧ to the classic
Vyukov queue but keeps a per-slot sequence number to avoid the ⟪acronym|ABA⟫ problem.

## Performance

| Benchmark | tinyq | ⟪product|crossbeam⟫ |
|---|---|---|
| ⟪table|push/pop, 1 producer⟫ | ⟪unit|9ns⟫ | ⟪unit|12ns⟫ |
| ⟪table|push/pop, 8 producers⟫ | ⟪unit|41ns⟫ | ⟪unit|38ns⟫ |

Numbers from a ⟪product|Ryzen 7 5800X⟫, ⟪version|rustc 1.85.0⟫, ⟪code|`--release`⟫. As
usual, benchmark ⟦your_youre|you're|your⟧ own workload before choosing.

## Minimum supported Rust version

⟪version|1.70⟫. Bumping the ⟪acronym|MSRV⟫ is a minor version change, not ⟦a_an|an major|a major⟧ one.

## Safety

The crate uses ⟪code|`unsafe`⟫ in four places, each with a comment explaining the invariant.
⟪product|Miri⟫ runs in ⟪acronym|CI⟫ on every push.
If you find a bug, please open an issue. Do not email it to the maintainers.
