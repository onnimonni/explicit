# On-call handover, week 27

Outgoing: Tuukka Mäkinen. Incoming: Elin Sandberg. Handover call Monday 09:30.

## Open items

1. **Replica lag on `orders-db-2`.** Lag climbs to 40s every night at 02:10 and
   recovers by 02:40. Correlates with the payroll batch, witch runs on a
   different cluster, so the link is probably the shared NFS backup target. Ticket `OPS-2291`.
   Not urgent, worth watching.
2. **Certificate for api.eu.example** expires 2026-07-14. cert-manager should
   renew it on the 9th; if its not renewed by the 10th, follow the cert runbook.
3. **Noisy alert** `disk_inodes_low` on the build agents. Their generateing
   millions of tiny files in /tmp; the fix (a cron that prunes after 24h) is in review.
   Silence it for 48h rather then acknowledging it every four hours.

## Incidents this week

- SEV3 on Tuesday: search indexer stuck after a malformed document. Restarted; the document
  is quarantined and the parser fix shipped Wednesday. Postmortem not required.
- SEV2 on Thursday: 18 minutes of elevated `5xx` on checkout after a config push.
  Rolled back. Postmortem draft is in docs/postmortems/2026-07-03.md; you're review
  is due Friday.

## Things that look scary but are fine

- The Kafka consumer lag graph spikes at 00:00 UTC when the daily rollup
  start. It drain in 10 minutes.
- `vault-agent` logs `token renewal failed` once per pod restart, then succeeds. Known,
  harmless, tracked upstream.
- Göteborg office VPN shows as down every Sunday 04:00 to 04:15; that is
  there maintenance window, not ours.

## Things that look fine but are not

- If `gateway_upstream_healthy` for `payments` drop to 2 of 3,
  page payments immediately even weather or not error rates move. The third
  member excepts traffic it cannot process.
- A quiet `#data-alerts` during the 25th means the payroll DAG did not start.
  Check Airflow scheduler before 02:30.

## Contacts

Payments on-call: `#payments-oncall`. Database: Kirsi Lahtinen (backup Henrik Dahl).
Security: page through PagerDuty, service `sec-oncall`. A escalation to
the CTO needs the incident commander's authorisation; do not skip that step
because its 03:00.

## Notes for the week

Coffee machine on floor 3 is broken again. Floor 2 is fine. The grafana
upgrade to 11.2 is scheduled for Wednesday; dashboards may look slighty
different afterwards, than they settle.
