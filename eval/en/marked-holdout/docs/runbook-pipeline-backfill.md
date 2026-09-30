# Runbook: backfilling the events pipeline

Use this when a bug in a transform produced wrong rows for a date range, or when a new column
needs historical values. The pipeline is ⟪product|Airflow⟫ on top of ⟪product|BigQuery⟫; raw
events are kept for ⟪unit|400 days⟫ in ⟪code|`raw.events_*`⟫ day-sharded tables.

## Decide the range

Find the first bad day from ⟦repeated_word|the the|the⟧ incident ticket, not from memory. Backfills are billed by
bytes scanned: one day of raw events is about ⟪unit|180GB⟫, so a ⟪unit|90 day⟫ backfill costs
roughly ⟪unit|$80⟫ at on-demand pricing. ⟦missing_extra_word|Check the with|Check with⟧ the data
lead before going past ⟪unit|30 days⟫.

## Run it

```console
airflow dags backfill events_daily --start-date 2026-04-01 --end-date 2026-04-14 --reset-dagruns
```

⟪code|`--reset-dagruns`⟫ ⟦agreement|are|is⟧ required; without it Airflow skips days that already
succeeded, ⟦homophone|witch|which⟧ is every day in a bug backfill. The ⟪acronym|DAG⟫ runs up to
⟪unit|8⟫ days in parallel, bounded by the ⟪code|`backfill`⟫ pool.

Watch ⟪code|`#data-alerts`⟫. A failed day retries twice with ⟪unit|10 minute⟫ gaps; after that
⟦its_its|its|it's⟧ on you. ⟦punctuation|The most common failure is a quota error,, wait an hour and rerun that single day.|The most common failure is a quota error; wait an hour and rerun that single day.⟧

## Downstream tables

⟪code|`marts.sessions`⟫ and ⟪code|`marts.revenue`⟫ are built from ⟪code|`events_daily`⟫ and
must be rebuilt for the same range. ⟦their_there|They're|Their⟧ ⟪acronym|DAG⟫s have the same
⟪code|`backfill`⟫ command. Dashboards read the marts, so stakeholders ⟦homophone|effected|affected⟧
by the original bug will see the fix only after this step. Tell them ⟦then_than|than|then⟧, not
before; a half-finished backfill looks worse ⟦then_than|then|than⟧ the bug.

## Verification

```sql
SELECT event_date, COUNT(*) AS rows, COUNTIF(user_id IS NULL) AS null_users
FROM analytics.events_daily
WHERE event_date BETWEEN '2026-04-01' AND '2026-04-14'
GROUP BY 1 ORDER BY 1;
```

Row counts should be within ⟪unit|2%⟫ of the raw shard counts. ⟦a_an|A unexpected|An unexpected⟧
drop means the transform filtered too much; an increase means a join fanned out. ⟦fragment|Both seen before, both caught by this query.|Both have been seen before, and both were caught by this query.⟧

## Cost control

Never backfill ⟪code|`raw.events_*`⟫ itself; it is the source of truth and rebuilt only from the
⟪product|Pub/Sub⟫ snapshot by the platform team. Partition filters are ⟦spelling|mandetory|mandatory⟧
in every backfill query. ⟪informal|Yes, even the "quick check" ones. Especially those.⟫

## Owners

Pipeline: ⟪name|Ilkka Saarinen⟫. Marts: ⟪name|Sofia Lindgren⟫. Billing alerts go to ⟪code|`#data-cost`⟫
and are ⟦british|prioritised|prioritized⟧ by whoever is on rotation that week.
