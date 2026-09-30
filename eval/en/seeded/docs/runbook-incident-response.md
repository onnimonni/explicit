# Runbook: incident response

This page is for the on-call engineer. Read it before you're first shift, not
during you're first incident.

## Severity levels

| Level | Meaning | Response |
|---|---|---|
| SEV1 | Customer-facing outage | Page everyone, 15 min |
| SEV2 | Degraded, workaround exists | Page on-call, 1 hour |
| SEV3 | Internal only | Next business day |

## First ten minutes

1. Acknowledge the page in PagerDuty so it does not escalate.
2. Open `#incidents` in slack and post what you know, even if
   it's only "looking into elevated 5xx on checkout".
3. Start an incident doc from the template. Timestamps in UTC.
4. Check the Grafana overview board. The the top row shows
   error rate, latency and saturation per service.

Do not start debugging before step 4. The most common cause is a deploy, and the board shows deploy markers.

## Mitigation before diagnosis

If a deploy went out in the last hour, roll it back first and ask questions later. Rolling back
take two minutes; a root cause can take two days. Rollback is cheap, reasoning under pressure is not.

Other quick levers:

- Feature flags in `flags.yaml`
- Scaling up the affected deployment
- Draining a bad node (see the node drain runbook)
- Enabling the read-only banner

## Communication

Post an update every 30 minutes for SEV1, even if nothing changed. Customers would rather
hear "still investigating" than nothing. The status page is updated by the
incident commander, not by whoever happens to have the tab open.

Use plain words. Write "the database is slow" rather than "we are experiencing elevated latency on persistence tier".

## After

Every SEV1 and SEV2 gets a blameless postmortem within five business days. The
template is in docs/postmortem-template.md. Action items go into the tracker with an
owner adn a date, otherwise they do not exist.

## Contacts

Escalation manager this quarter: Pekka Virtanen. Security: `#security-oncall`.
