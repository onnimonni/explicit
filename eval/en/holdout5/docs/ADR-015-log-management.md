# ADR-015: Log management platform

Status: accepted
Date: 2026-11-03

## Context

Today's log management relies on logs written to files and read with `grep`. The on-call engineers have complained that a search take minutes. The security group has pointed out that logs is not kept long enough. The volume of logs is about 40 GiB a day, and the retention required by the auditors is two years.

A tender was run in August. Four bids was received, two of which was rejected as incomplete. The rejected bidders did not appeal; each of them was given a written explanation by Aleksis Kivi.

## Options

### Grafana Loki (Pohjola Cloud Oy)

Index-light storage on object storage. The cost of storage is low, and the query language match what the team already uses in Grafana. Full-text search is slower than in the alternatives.

### OpenSearch (Nordic Observability AB)

Full-text indexing. The quality of the search is excellent, but the price of indexing 40 GiB a day is high, and the cluster needs its own on-call rota. None of the engineers has run it in production.

### Managed Datadog

Zero operations. The data would leave Finland, which the data protection officer reject outright. The option is listed for completeness only.

## Comparison

| Bidder | Price/year | Quality score | Total |
|--------|-----------|---------------|-------|
| Pohjola Cloud Oy | 38 000 € | 42 | 88 |
| Nordic Observability AB | 45 000 € | 45 | 84 |

Price weighed 50 % and quality 50 %. The quality scores were based on how well the bidder met the requirements. The difference is small but clear; the evaluation committee was unanimous, and the members of the committee were Aino Kallas, Ville Ranta and Sunil Menon.

## Decision

We choose Grafana Loki from Pohjola Cloud Oy. The vendor is responsible for keeping logs in Finland for two years and has committed to searches that returns within two seconds. The references confirm this.

The bid that came second was better in quality but more expensive. The difference in quality was marginal rather than material, which the pilot confirmed. A marginal difference does not justify the extra cost, and neither of the evaluators was persuaded otherwise. The head of IT sign the contract in November once the appeal period has passed.

## Consequences

- The rollout starts in January; the new system replaces the old one gradually, and the old files are deleted after 90 days.
- A data processing agreement is concluded before a single log line are transferred. The vendor's confirmation: "Kaikki lokitiedot tallennetaan ja käsitellään yksinomaan Helsingin alueella."
- The engineers who maintain the dashboards gets training in December; the training of the on-call rota takes one afternoon.
- The decision can be appealed to the Market Court within 14 days. The appeal instructions are attached and sent to all bidders; anyone who wants a full copy of the evaluation can request one from Mennon, whose team keeps the records. Its the first tender where the evaluation was published in full.
