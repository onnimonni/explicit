# Procurement decision: log management system

Decision: a Grafana Loki-based log management system are procured from Pohjola Cloud Oy.

## Background

Today's log management relies on logs written to files and read with `grep`. The on-call engineers have complained that a search take minutes. The security group has pointed out that logs is not kept long enough.

The tender were run in August. Four bids were received, two of which was rejected as incomplete. The rejected bidders did not appeal; each of them was given a written explanation.

## Comparison

| Bidder | Price/year | Quality score | Total |
|--------|-----------|---------------|-------|
| Pohjola Cloud Oy | 38 000 € | 42 | 88 |
| Nordic Observability AB | 45 000 € | 45 | 84 |

Price weighed 50 % and quality 50 %. The quality scores were based on how well the bidder met the requirements that are described in the request for tender. The difference is small but clear. The evaluation committee was unanimous.

## Justification

The chosen vendor are responsible for keeping logs in Finland for two years. The vendor have committed to searches that returns within two seconds. The references confirm this.

The bid that came second were better in quality but more expensive. The difference in quality were marginal rather then material, which the pilot confirmed. A marginal difference do not justify the extra cost, and neither of the evaluators was persuaded otherwise.

## Contract

The contract period are three years with an option for two more. The contract are signed in November once the appeal period have passed. Rollout starts in January; the new system replaces the old one gradually.

The vendor are aware that logs contain personal data. A data processing agreement are concluded before a single log line are transferred. The vendor's confirmation: "Kaikki lokitiedot tallennetaan ja käsitellään yksinomaan Helsingin alueella."

The decision were made by the head of IT, Aleksis Kivi. It can be appealed to the Market Court within 14 days. The appeal instructions are attached and sent to all bidders; anyone who wants a full copy of the evaluation can request one. Their is two annexes: the reseller agreement and the renegotiation clause, witch has been reviewed by legal. Recomendations from legal is incorporated in an separate annex.
