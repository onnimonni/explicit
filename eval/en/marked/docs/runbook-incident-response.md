# Runbook: incident response

This page is for the on-call engineer. Read it before ⟦your_youre|you're|your⟧ first shift, not
during ⟦your_youre|you're|your⟧ first incident.

## Severity levels

| Level | Meaning | Response |
|---|---|---|
| ⟪table|SEV1⟫ | ⟪table|Customer-facing outage⟫ | ⟪table|Page everyone, 15 min⟫ |
| ⟪table|SEV2⟫ | ⟪table|Degraded, workaround exists⟫ | ⟪table|Page on-call, 1 hour⟫ |
| ⟪table|SEV3⟫ | ⟪table|Internal only⟫ | ⟪table|Next business day⟫ |

## First ten minutes

1. Acknowledge the page in ⟪product|PagerDuty⟫ so it does not escalate.
2. Open ⟪code|`#incidents`⟫ in ⟦capitalization|slack|Slack⟧ and post what you know, even if
   ⟪correct|it's⟫ only "looking into elevated 5xx on checkout".
3. Start ⟪correct|an incident⟫ doc from the template. Timestamps in ⟪acronym|UTC⟫.
4. Check the ⟪product|Grafana⟫ overview board. ⟦repeated_word|The the|The⟧ top row shows
   error rate, latency and saturation per service.

Do not start debugging before step 4. The most common cause is a deploy, and the board shows deploy markers.

## Mitigation before diagnosis

If a deploy went out in the last hour, roll it back first and ask questions later. Rolling back
⟦agreement|take|takes⟧ two minutes; a root cause can take two days. ⟦punctuation|Rollback is cheap, reasoning under pressure is not.|Rollback is cheap; reasoning under pressure is not.⟧

Other quick levers:

- ⟪list|Feature flags in `flags.yaml`⟫
- ⟪list|Scaling up the affected deployment⟫
- ⟪list|Draining a bad node (see the node drain runbook)⟫
- ⟪list|Enabling the read-only banner⟫

## Communication

Post an update every 30 minutes for ⟪table|SEV1⟫, even if nothing changed. Customers would rather
hear "still investigating" ⟪correct|than⟫ nothing. The status page is updated by the
incident commander, not by whoever happens to have the tab open.

Use plain words. Write "the database is slow" rather than ⟦missing_extra_word|"we are experiencing elevated latency on persistence tier"|"we are experiencing elevated latency on the persistence tier"⟧.

## After

Every ⟪table|SEV1⟫ and ⟪table|SEV2⟫ gets a blameless postmortem within five business days. The
template is in ⟪path|docs/postmortem-template.md⟫. Action items go into the tracker with an
owner ⟦spelling|adn|and⟧ a date, otherwise they do not exist.

## Contacts

Escalation manager this quarter: ⟪name|Pekka Virtanen⟫. Security: ⟪code|`#security-oncall`⟫.
