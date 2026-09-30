# tinyq

[![crates.io](https://img.shields.io/crates/v/tinyq.svg)](https://crates.io/crates/tinyq)

A bounded, lock-free MPSC queue for Rust. `no_std` compatible, no
allocations after construction, and about 300 lines of code.

## Why

Most queue crates are either huge or unnecessarily generic. tinyq does one
thing: it moves `T: Send` values from many producers to one consumer without locks. The
capacity is fixed at construction time and must be a power of two.

## Example

```rust
use tinyq::Queue;

let q: Queue<u32, 1024> = Queue::new();
let (tx, rx) = q.split();
tx.push(7).unwrap();
assert_eq!(rx.pop(), Some(7));
```

`push` returns `Err(value)` when the queue is full, so callers never
loose data silently. `pop` returns `None` when empty.

## Memory ordering

Producers reserve a slot with `fetch_add(1, AcqRel)` on the tail
counter and than write the value. The consumer reads the head with
`Acquire`. Idempotent retries are safe because a failed reservation is rolled
back with a CAS loop. The implementation is similiar to the classic
Vyukov queue but keeps a per-slot sequence number to avoid the ABA problem.

## Performance

| Benchmark | tinyq | crossbeam |
|---|---|---|
| push/pop, 1 producer | 9ns | 12ns |
| push/pop, 8 producers | 41ns | 38ns |

Numbers from a Ryzen 7 5800X, rustc 1.85.0, `--release`. As
usual, benchmark you're own workload before choosing.

## Minimum supported Rust version

1.70. Bumping the MSRV is a minor version change, not an major one.

## Safety

The crate uses `unsafe` in four places, each with a comment explaining the invariant.
Miri runs in CI on every push.
If you find a bug, please open an issue. Do not email it to the maintainers.
