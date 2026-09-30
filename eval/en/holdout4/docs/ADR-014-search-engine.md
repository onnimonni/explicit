# ADR-014: Search engine for the practitioner directory

Status: proposed
Date: 2026-10-20

## Context

Practitioners complain that the directory search is slow and do not tolerate typos. The current implementation runs `LIKE` queries against PostgreSQL on every keystroke, and the queries take more than a second at peak. Two out of three practitioners use the search daily, so the cost of slowness is high.

The product team has asked for typo tolerance, and the compliance team has asked that personal identity numbers never is indexed.

## Options

### PostgreSQL with pg_trgm

A trigram index brings `LIKE` queries under 50 ms and tolerates typos. No new components. The set of supported languages is limited: the index do not understand inflected forms, which are a problem for Finnish names.

### Elasticsearch

A full search engine with a Finnish analyser. None of the engineers has run it in production. The costs were 1 200 € per month in the pilot, which were more than we estimated. A Elasticsearch cluster also need its own on-call rota.

### Meilisearch

Light and fast. Finnish support is weaker then in Elasticsearch but sufficient for names. In the pilot the median response was under 20 ms, and the 99th percentile was under 60 ms.

## Comparison

| Criterion | pg_trgm | Elasticsearch | Meilisearch |
|-----------|---------|---------------|-------------|
| Latency | 50 ms | 30 ms | 20 ms |
| Inflections | no | yes | partial |
| Operations | none | heavy | light |
| Cost | 0 | 1 200 €/month | 150 €/month |

The results are in `docs/adr/014/`. Each pilot was run with the same data and the same query log. Its worth noting that the query log is three months old.

## Decision

We propose Meilisearch. Reasons:

1. It solves the problem that practitioners complain about: latency and typos.
2. Operations is light: one container and one volume.
3. The cost is a tenth of Elasticsearch.

The lack of inflection support is a limitation rather then a blocker: names are rarely inflected in search. If the requirements were to change, a switch to Elasticsearch is possible because the search interface is abstracted. There decision were unanimous in the pilot review.

## Consequences

- The index are rebuilt nightly; if the rebuild fails, the previous index stays in use.
- Personal identity numbers are not indexed. Search works on name and date of birth only.
- The engineers who maintain the search get training in December, and the on-call rota is updated at the same time.
- The architecture group and the product owner review this ADR in November; the vendor's note, "typo tolerance works best for words of five characters or more", is attached. It is is noted in the risks section as well.
