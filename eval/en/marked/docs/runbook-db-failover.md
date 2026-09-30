# Runbook: PostgreSQL failover

Applies to the ⟪code|`orders-db`⟫ and ⟪code|`billing-db`⟫ clusters. Both run ⟪product|Patroni⟫
on three nodes with ⟪product|etcd⟫ for leader election.

## When to use this

- ⟪list|The primary is unreachable for more than 60 seconds⟫
- ⟪list|Replication lag above 30s and climbing⟫
- ⟪list|Planned maintenance on the primary's host⟫

Automatic failover usually handles the first case. If it did not, something is wrong with etcd; check that first.

## Manual switchover

Switchover is the planned, graceful ⟦british|behaviour|behavior⟧. It waits for replicas to catch
up ⟪correct|and then⟫ promotes one.

```console
patronictl -c /etc/patroni.yml switchover orders-db --candidate db-2
```

The command asks for confirmation. ⟦spelling|Dobule|Double⟧ check the cluster name; ⟦its_its|its|it's⟧
easy to run this against billing when you meant orders. Connections are dropped for
about ⟪unit|5s⟫ while ⟪product|PgBouncer⟫ reconnects.

## Manual failover

Failover is the forceful version for when the primary is already gone. Data written to the old
primary after the last replicated ⟪acronym|WAL⟫ segment is lost. Before running it,
note the current ⟪acronym|LSN⟫ on each replica so you can pick the most recent one:

```console
patronictl -c /etc/patroni.yml list
```

Then:

```console
patronictl -c /etc/patroni.yml failover orders-db --candidate db-3 --force
```

## After the failover

1. Confirm the application reconnected. ⟪product|PgBouncer⟫ ⟦agreement|follow|follows⟧ the ⟪acronym|DNS⟫ record,
   which ⟪product|Patroni⟫ updates within ⟪unit|10s⟫.
2. Check that the old primary rejoined as a replica. If it ⟦spelling|didnt|didn't⟧, it probably has
   diverged ⟪acronym|WAL⟫ and needs ⟪code|`patronictl reinit`⟫.
3. Watch replication lag for 15 minutes. Watch billing-db especially, since it has heavier writes.
4. Update the incident channel. Mention which node is primary now and ⟦homophone|weather|whether⟧ any
   data was lost.

## Common mistakes

- Running ⟪code|`failover`⟫ when ⟪code|`switchover`⟫ would do. You ⟦homophone|loose|lose⟧ writes for nothing.
- Forgetting the ⟪code|`-c`⟫ flag ⟦missing_extra_word|and getting error|and getting an error⟧ about a missing config.
- Promoting the replica with the most lag because ⟪correct|it's⟫ the first in the list.

## Owners

Database team: ⟪name|Jyri Lehtinen⟫, ⟪name|Ingrid Nordström⟫. Page ⟪code|`db-oncall`⟫ outside office hours.
