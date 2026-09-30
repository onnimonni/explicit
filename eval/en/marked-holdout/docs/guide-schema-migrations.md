# Schema migrations without downtime

Every migration in ⟪path|db/migrations/⟫ must be safe to run while the previous release is
still serving traffic. This page is the checklist reviewers apply. It assumes
⟪product|PostgreSQL⟫ ⟪version|15+⟫ and ⟪product|sqlx⟫ migrations.

## The expand and contract pattern

1. **Expand.** Add the new column, table or index. Old code ignores it.
2. **Migrate data.** Backfill in batches of ⟪unit|10,000⟫ rows with a sleep between batches.
3. **Switch.** Deploy code that reads the new shape and writes both.
4. **Contract.** After a full release cycle, drop the old column.

Skipping step 4 is how ⟦your_youre|you're|your⟧ schema ends up with ⟪unit|14⟫ columns named
⟪code|`legacy_*`⟫. ⟦punctuation|Put the contract migration in the same PR as the expand one, it is easy to forget otherwise.|Put the contract migration in the same PR as the expand one; it is easy to forget otherwise.⟧

## Locks

⟪code|`ALTER TABLE ... ADD COLUMN`⟫ with a constant default is instant since ⟪version|11⟫. Adding a
⟪code|`NOT NULL`⟫ constraint to an existing column ⟦agreement|scan|scans⟧ the table under an
⟪code|`ACCESS EXCLUSIVE`⟫ lock; add it as ⟪code|`NOT VALID`⟫ first, ⟦then_than|than|then⟧ ⟪code|`VALIDATE`⟫
in a second statement, which takes only a ⟪code|`SHARE UPDATE EXCLUSIVE`⟫ lock.

Indexes are always ⟪code|`CREATE INDEX CONCURRENTLY`⟫, which cannot run inside a transaction.
Mark the migration with ⟪code|`-- sqlx:no-transaction`⟫ or it will fail with ⟦a_an|an confusing|a confusing⟧
error about transaction blocks.

Set ⟪code|`lock_timeout = '5s'`⟫ at the top of every migration. A migration that waits behind a
long ⟦spelling|anaytics|analytics⟧ query holds up every other connection behind it, and
⟦its_its|its|it's⟧ better to fail and retry than to take the site down.

## Renames

Never rename a column in one step. Add the new one, dual-write, backfill, switch reads, drop.
⟦fragment|Five migrations for a rename, and worth it.|That is five migrations for a rename, and it is worth it.⟧ Renaming a table is the same
dance with a view as the compatibility layer.

## Enums

⟪code|`ALTER TYPE ... ADD VALUE`⟫ cannot run in a transaction before ⟪version|12⟫ and the new value
cannot be used in the same transaction after. Prefer a lookup table or a ⟪code|`text`⟫ column
with a ⟪code|`CHECK`⟫ constraint; ⟦their_there|there|they're⟧ easier to change and just as fast for
our sizes.

## Review checklist

- ⟪list|`lock_timeout` set⟫
- ⟪list|No `ACCESS EXCLUSIVE` lock on a table above 1M rows⟫
- ⟪list|Backfill batched and resumable⟫
- ⟪list|Contract migration written, dated, and reversible⟫
- ⟪list|Tested against a copy of production with `pg_dump --schema-only` plus sampled data⟫

A migration that fails any item ⟦spelling|recieves|receives⟧ a request for changes, no exceptions.
The one time we made ⟦a_an|a exception|an exception⟧, in ⟪unit|2024⟫, cost ⟪unit|40 minutes⟫ of
checkout downtime; ⟦capitalization|postgres|Postgres⟧ did exactly what it was asked to do.
