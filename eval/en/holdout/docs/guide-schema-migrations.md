# Schema migrations without downtime

Every migration in db/migrations/ must be safe to run while the previous release is
still serving traffic. This page is the checklist reviewers apply. It assumes
PostgreSQL 15+ and sqlx migrations.

## The expand and contract pattern

1. **Expand.** Add the new column, table or index. Old code ignores it.
2. **Migrate data.** Backfill in batches of 10,000 rows with a sleep between batches.
3. **Switch.** Deploy code that reads the new shape and writes both.
4. **Contract.** After a full release cycle, drop the old column.

Skipping step 4 is how you're schema ends up with 14 columns named
`legacy_*`. Put the contract migration in the same PR as the expand one, it is easy to forget otherwise.

## Locks

`ALTER TABLE ... ADD COLUMN` with a constant default is instant since 11. Adding a
`NOT NULL` constraint to an existing column scan the table under an
`ACCESS EXCLUSIVE` lock; add it as `NOT VALID` first, than `VALIDATE`
in a second statement, which takes only a `SHARE UPDATE EXCLUSIVE` lock.

Indexes are always `CREATE INDEX CONCURRENTLY`, which cannot run inside a transaction.
Mark the migration with `-- sqlx:no-transaction` or it will fail with an confusing
error about transaction blocks.

Set `lock_timeout = '5s'` at the top of every migration. A migration that waits behind a
long anaytics query holds up every other connection behind it, and
its better to fail and retry than to take the site down.

## Renames

Never rename a column in one step. Add the new one, dual-write, backfill, switch reads, drop.
Five migrations for a rename, and worth it. Renaming a table is the same
dance with a view as the compatibility layer.

## Enums

`ALTER TYPE ... ADD VALUE` cannot run in a transaction before 12 and the new value
cannot be used in the same transaction after. Prefer a lookup table or a `text` column
with a `CHECK` constraint; there easier to change and just as fast for
our sizes.

## Review checklist

- `lock_timeout` set
- No `ACCESS EXCLUSIVE` lock on a table above 1M rows
- Backfill batched and resumable
- Contract migration written, dated, and reversible
- Tested against a copy of production with `pg_dump --schema-only` plus sampled data

A migration that fails any item recieves a request for changes, no exceptions.
The one time we made a exception, in 2024, cost 40 minutes of
checkout downtime; postgres did exactly what it was asked to do.
