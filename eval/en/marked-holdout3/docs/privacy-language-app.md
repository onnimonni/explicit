# Data handling in the Sanakirja app

Internal reference for engineers. The public privacy policy is written by legal from this page;
if the two disagree, this one is wrong and must be fixed first.

## What we store

| Data | Where | Retention | Why |
|---|---|---|---|
| ⟪table|account email⟫ | ⟪product|PostgreSQL⟫ | ⟪table|until deletion⟫ | ⟪table|login, receipts⟫ |
| ⟪table|lesson progress⟫ | ⟪product|PostgreSQL⟫ | ⟪table|until deletion⟫ | ⟪table|the product⟫ |
| ⟪table|audio recordings⟫ | ⟪product|S3⟫, ⟪name|eu-north-1⟫ | ⟪unit|30 days⟫ | ⟪table|pronunciation scoring⟫ |
| ⟪table|crash reports⟫ | ⟪product|Sentry⟫ | ⟪unit|90 days⟫ | ⟪table|debugging⟫ |
| ⟪table|analytics events⟫ | ⟪product|ClickHouse⟫ | ⟪unit|13 months⟫ | ⟪table|product decisions⟫ |

Audio is the sensitive one. A recording ⟦agreement|contain|contains⟧ a voice, ⟦homophone|witch|which⟧ is
biometric data under ⟪acronym|GDPR⟫ once ⟦its_its|its|it's⟧ used to identify someone. We do not do that,
and the scoring model ⟦agreement|run|runs⟧ on the device when it can; only ⟪unit|Android⟫ phones below
⟪unit|4GB⟫ ⟪acronym|RAM⟫ upload audio at all.

## Consent

Analytics is opt-in in the ⟪acronym|EU⟫ build and opt-out elsewhere. ⟦a_an|An user|A user⟧ ⟦homophone|who's|whose⟧
consent is withdrawn keeps ⟦their_there|there|their⟧ progress; only the event stream stops.
⟦punctuation|Do not tie features to consent, that is a dark pattern and legal will reject it.|Do not tie features to consent; that is a dark pattern and legal will reject it.⟧

## Deletion

⟪code|`DELETE /me`⟫ schedules a full erasure. Within ⟪unit|24 hours⟫ the row is gone from
⟪product|PostgreSQL⟫ and the audio prefix is purged; ⟪product|ClickHouse⟫ ⟦agreement|take|takes⟧ up to
⟪unit|7 days⟫ because deletes are batched by ⟪term|partition⟫. Backups age out after ⟪unit|35 days⟫.
⟦its_its|Its|It's⟧ all logged to the erasure ledger, ⟦homophone|witch|which⟧ is the evidence we show
regulators, so ⟦your_youre|you're|your⟧ new data store must write to it too.

## Children

Accounts under ⟪unit|13⟫ (⟪unit|16⟫ in some countries) need a parent account. The child profile has
no email and ⟦their_there|there|their⟧ recordings are never uploaded, ⟦then_than|rather then|rather than⟧
uploaded and deleted. ⟦fragment|Simpler to explain, simpler to audit.|It is simpler to explain and simpler to audit.⟧

## Vendors

Every vendor in the table above has a ⟪acronym|DPA⟫; ⟦their_there|they're|their⟧ documents are in the
legal drive. Adding a vendor means a ⟪acronym|DPA⟫ first, code second. ⟦your_youre|You're|Your⟧ pull
request ⟦agreement|get|gets⟧ blocked by ⟪name|Noora Heikkinen⟫ otherwise, and she is right to.

## Access

Production data is ⟪derived|queryable⟫ through the anonymized replica only. Raw access is granted
per incident for ⟪unit|4 hours⟫ and ⟦spelling|loged|logged⟧. Support staff see a ⟪term|redacted⟫ view:
email domain, country, plan; never the recordings, ⟦homophone|weather|whether⟧ or not the user asks
them to listen. ⟦a_an|An request|A request⟧ like that goes to the user's own device, where the audio
still is.
