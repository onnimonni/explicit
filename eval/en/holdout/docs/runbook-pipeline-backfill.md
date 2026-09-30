# Runbook: backfilling the events pipeline

Use this when a bug in a transform produced wrong rows for a date range, or when a new column
needs historical values. The pipeline is Airflow on top of BigQuery; raw
events are kept for 400 days in `raw.events_*` day-sharded tables.

## Decide the range

Find the first bad day from the the incident ticket, not from memory. Backfills are billed by
bytes scanned: one day of raw events is about 180GB, so a 90 day backfill costs
roughly $80 at on-demand pricing. Check the with the data
lead before going past 30 days.

## Run it

```console
airflow dags backfill events_daily --start-date 2026-04-01 --end-date 2026-04-14 --reset-dagruns
```

`--reset-dagruns` are required; without it Airflow skips days that already
succeeded, witch is every day in a bug backfill. The DAG runs up to
8 days in parallel, bounded by the `backfill` pool.

Watch `#data-alerts`. A failed day retries twice with 10 minute gaps; after that
its on you. The most common failure is a quota error,, wait an hour and rerun that single day.

## Downstream tables

`marts.sessions` and `marts.revenue` are built from `events_daily` and
must be rebuilt for the same range. They're DAGs have the same
`backfill` command. Dashboards read the marts, so stakeholders effected
by the original bug will see the fix only after this step. Tell them than, not
before; a half-finished backfill looks worse then the bug.

## Verification

```sql
SELECT event_date, COUNT(*) AS rows, COUNTIF(user_id IS NULL) AS null_users
FROM analytics.events_daily
WHERE event_date BETWEEN '2026-04-01' AND '2026-04-14'
GROUP BY 1 ORDER BY 1;
```

Row counts should be within 2% of the raw shard counts. A unexpected
drop means the transform filtered too much; an increase means a join fanned out. Both seen before, both caught by this query.

## Cost control

Never backfill `raw.events_*` itself; it is the source of truth and rebuilt only from the
Pub/Sub snapshot by the platform team. Partition filters are mandetory
in every backfill query. Yes, even the "quick check" ones. Especially those.

## Owners

Pipeline: Ilkka Saarinen. Marts: Sofia Lindgren. Billing alerts go to `#data-cost`
and are prioritised by whoever is on rotation that week.
