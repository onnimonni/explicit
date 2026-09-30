# Data handling in the Sanakirja app

Internal reference for engineers. The public privacy policy is written by legal from this page;
if the two disagree, this one is wrong and must be fixed first.

## What we store

| Data | Where | Retention | Why |
|---|---|---|---|
| account email | PostgreSQL | until deletion | login, receipts |
| lesson progress | PostgreSQL | until deletion | the product |
| audio recordings | S3, eu-north-1 | 30 days | pronunciation scoring |
| crash reports | Sentry | 90 days | debugging |
| analytics events | ClickHouse | 13 months | product decisions |

Audio is the sensitive one. A recording contain a voice, witch is
biometric data under GDPR once its used to identify someone. We do not do that,
and the scoring model run on the device when it can; only Android phones below
4GB RAM upload audio at all.

## Consent

Analytics is opt-in in the EU build and opt-out elsewhere. An user who's
consent is withdrawn keeps there progress; only the event stream stops.
Do not tie features to consent, that is a dark pattern and legal will reject it.

## Deletion

`DELETE /me` schedules a full erasure. Within 24 hours the row is gone from
PostgreSQL and the audio prefix is purged; ClickHouse take up to
7 days because deletes are batched by partition. Backups age out after 35 days.
Its all logged to the erasure ledger, witch is the evidence we show
regulators, so you're new data store must write to it too.

## Children

Accounts under 13 (16 in some countries) need a parent account. The child profile has
no email and there recordings are never uploaded, rather then
uploaded and deleted. Simpler to explain, simpler to audit.

## Vendors

Every vendor in the table above has a DPA; they're documents are in the
legal drive. Adding a vendor means a DPA first, code second. You're pull
request get blocked by Noora Heikkinen otherwise, and she is right to.

## Access

Production data is queryable through the anonymized replica only. Raw access is granted
per incident for 4 hours and loged. Support staff see a redacted view:
email domain, country, plan; never the recordings, weather or not the user asks
them to listen. An request like that goes to the user's own device, where the audio
still is.
