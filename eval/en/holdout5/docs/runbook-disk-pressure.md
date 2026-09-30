# Runbook: disk pressure on database nodes

This runbook is written in British English and is used when Grafana fires `disk_free < 20%` on a PostgreSQL node. The behaviour of the alert has changed since the August incident; the threshold is now 20 % and a forecast alert fires 24 h ahead.

## First ten minutes

1. Acknowledge the alert in Opsgenie so that the rest of the team knows you are on it.
2. Check what is using the space: `du -sh /var/lib/postgresql/*` and `SELECT pg_size_pretty(pg_database_size(current_database()));`.
3. Check the WAL archive. The number of archived segments is usually the culprit.

If the archive is the problem, the archiver has probably stalled. The logs of the archiver show why; the usual causes are an expired credential or a full object store. Each of these has its own section below. Prioritise the archive over everything else.

## Stalled archiver

The credentials of the archiver live in Vault. If the token has expired, rotate it with `make rotate-archiver` and restart the archiver. The archiver catches up at roughly 500 segments per minute; the backlog of segments shrinks visibly within five minutes.

If the object store is full, the lifecycle rule has probably been disabled. The owner of the bucket is Priya Natarajan; her team re-enables the rule. Do not delete segments by hand: the list of segments that are still needed for point-in-time recovery is not obvious, and a deleted segment breaks every restore after it.

## Bloated tables

If the archive is healthy, table bloat is the next suspect. The size of the bookings table grows when autovacuum fall behind. Check `pg_stat_user_tables`: a table whose dead tuples exceed 20 % needs a manual vacuum.

Run `VACUUM (VERBOSE) bookings;` outside peak hours. The vacuum of a two-million-row table takes about 15 minutes and does not lock writers. A full vacuum do lock writers and is only run during a maintenance window authorised by Aino Kallas.

## Expanding the volume

If neither of the above frees enough space, expand the volume. The expansion is an online operation: the replica is promoted first, the old primary is expanded, and the roles are swapped back. Only two people know the procedure well, Ville Ranta and Mikko Järvinen, so one of them is paged even at night.

```bash
kubectl -n db patch pvc data-db-1 -p '{"spec":{"resources":{"requests":{"storage":"3Ti"}}}}'
kubectl -n db exec db-1 -- resize2fs /dev/nvme1n1
```

The vendor's documentation summarises the procedure; the version we follow is the one in `docs/db/expand.md`, not the vendor's, because the vendor's steps assume a single node.

## After the incident

Write a postmortem within two working days. The template is in `docs/templates/postmortem.md`. The postmortem of the August incident is a good example; its actions were all closed within a month. The people who were on call reviews the draft, and the head of the database team approves it. Recognise that the majority of disk incidents are caused by archiving, not by data growth, and the number of true capacity incidents is small. The runbook is owned by Sanna Virtanen; Virtanenn's favourite reminder is that the archiver, not the disk, is the thing to watch.
