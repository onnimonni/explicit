# Runbook: PostgreSQL failover

Applies to the `orders-db` and `billing-db` clusters. Both run Patroni
on three nodes with etcd for leader election.

## When to use this

- The primary is unreachable for more than 60 seconds
- Replication lag above 30s and climbing
- Planned maintenance on the primary's host

Automatic failover usually handles the first case. If it did not, something is wrong with etcd; check that first.

## Manual switchover

Switchover is the planned, graceful behaviour. It waits for replicas to catch
up and then promotes one.

```console
patronictl -c /etc/patroni.yml switchover orders-db --candidate db-2
```

The command asks for confirmation. Dobule check the cluster name; its
easy to run this against billing when you meant orders. Connections are dropped for
about 5s while PgBouncer reconnects.

## Manual failover

Failover is the forceful version for when the primary is already gone. Data written to the old
primary after the last replicated WAL segment is lost. Before running it,
note the current LSN on each replica so you can pick the most recent one:

```console
patronictl -c /etc/patroni.yml list
```

Then:

```console
patronictl -c /etc/patroni.yml failover orders-db --candidate db-3 --force
```

## After the failover

1. Confirm the application reconnected. PgBouncer follow the DNS record,
   which Patroni updates within 10s.
2. Check that the old primary rejoined as a replica. If it didnt, it probably has
   diverged WAL and needs `patronictl reinit`.
3. Watch replication lag for 15 minutes. Watch billing-db especially, since it has heavier writes.
4. Update the incident channel. Mention which node is primary now and weather any
   data was lost.

## Common mistakes

- Running `failover` when `switchover` would do. You loose writes for nothing.
- Forgetting the `-c` flag and getting error about a missing config.
- Promoting the replica with the most lag because it's the first in the list.

## Owners

Database team: Jyri Lehtinen, Ingrid Nordström. Page `db-oncall` outside office hours.
