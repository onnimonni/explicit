# On-call handover, week 27

Outgoing: ⟪name|Tuukka Mäkinen⟫. Incoming: ⟪name|Elin Sandberg⟫. Handover call ⟪unit|Monday 09:30⟫.

## Open items

1. **Replica lag on ⟪code|`orders-db-2`⟫.** Lag climbs to ⟪unit|40s⟫ every night at ⟪unit|02:10⟫ and
   recovers by ⟪unit|02:40⟫. Correlates with the payroll batch, ⟦homophone|witch|which⟧ runs on a
   different cluster, so the link is probably the shared ⟪acronym|NFS⟫ backup target. Ticket ⟪code|`OPS-2291`⟫.
   ⟦missing_extra_word|Not urgent, worth watching.|Not urgent, but worth watching.⟧
2. **Certificate for ⟪url|api.eu.example⟫** expires ⟪unit|2026-07-14⟫. ⟪product|cert-manager⟫ should
   renew it on the ⟪unit|9th⟫; if ⟦its_its|its|it's⟧ not renewed by the ⟪unit|10th⟫, follow the cert runbook.
3. **Noisy alert** ⟪code|`disk_inodes_low`⟫ on the build agents. ⟦their_there|Their|They're⟧ ⟦spelling_1edit|generateing|generating⟧
   millions of tiny files in ⟪path|/tmp⟫; the fix (a cron that prunes after ⟪unit|24h⟫) is in review.
   Silence it for ⟪unit|48h⟫ ⟦then_than|rather then|rather than⟧ acknowledging it every four hours.

## Incidents this week

- ⟪table|SEV3⟫ on Tuesday: search indexer stuck after a malformed document. Restarted; the document
  is quarantined and the parser fix shipped Wednesday. ⟦fragment|Postmortem not required.|A postmortem is not required.⟧
- ⟪table|SEV2⟫ on Thursday: ⟪unit|18 minutes⟫ of elevated ⟪code|`5xx`⟫ on checkout after a config push.
  Rolled back. Postmortem draft is in ⟪path|docs/postmortems/2026-07-03.md⟫; ⟦your_youre|you're|your⟧ review
  is due Friday.

## Things that look scary but are fine

- The ⟪product|Kafka⟫ consumer lag graph spikes at ⟪unit|00:00⟫ ⟪acronym|UTC⟫ when the daily rollup
  ⟦agreement|start|starts⟧. It ⟦agreement|drain|drains⟧ in ⟪unit|10 minutes⟫.
- ⟪code|`vault-agent`⟫ logs ⟪code|`token renewal failed`⟫ once per pod restart, then succeeds. Known,
  harmless, tracked upstream.
- ⟪name|Göteborg⟫ office ⟪acronym|VPN⟫ shows as down every Sunday ⟪unit|04:00⟫ to ⟪unit|04:15⟫; that is
  ⟦their_there|there|their⟧ maintenance window, not ours.

## Things that look fine but are not

- If ⟪code|`gateway_upstream_healthy`⟫ for ⟪code|`payments`⟫ ⟦agreement|drop|drops⟧ to ⟪unit|2⟫ of ⟪unit|3⟫,
  page payments immediately even ⟦homophone|weather|whether⟧ or not error rates move. The third
  member ⟦homophone|excepts|accepts⟧ traffic it cannot process.
- A quiet ⟪code|`#data-alerts`⟫ during the ⟪unit|25th⟫ means the payroll ⟪acronym|DAG⟫ did not start.
  ⟦missing_extra_word|Check Airflow scheduler|Check the Airflow scheduler⟧ before ⟪unit|02:30⟫.

## Contacts

Payments on-call: ⟪code|`#payments-oncall`⟫. Database: ⟪name|Kirsi Lahtinen⟫ (backup ⟪name|Henrik Dahl⟫).
Security: page through ⟪product|PagerDuty⟫, service ⟪code|`sec-oncall`⟫. ⟦a_an|A escalation|An escalation⟧ to
the ⟪acronym|CTO⟫ needs the incident commander's ⟦british|authorisation|authorization⟧; do not skip that step
because ⟦its_its|its|it's⟧ ⟪unit|03:00⟫.

## Notes for the week

⟪informal|Coffee machine on floor 3 is broken again. Floor 2 is fine.⟫ The ⟦capitalization|grafana|Grafana⟧
upgrade to ⟪version|11.2⟫ is scheduled for Wednesday; dashboards may look ⟦spelling|slighty|slightly⟧
different afterwards, ⟦then_than|than|then⟧ they settle.
