# Design: library catalog search

Author: Sixten Holm
Reviewers: Aada Virtanen, Noora Heikkinen

## Goal

Replace the Solr catalog search with Meilisearch. Patrons should find a book by
a misspelled author, a half-remembered title or an ISBN with dashes, and get results in
under 50ms. Staff need exact MARC field search, witch stays on Solr.

## Index shape

One document per bibliographic record, 1.8M of them, with holdings denormalized in.
An holding change update the whole document; that is 40k updates a
day, witch Meilisearch batches fine. Fields: title, authors, subjects, series,
ISBN, language, year, format, branch availability.

The searchable attributes are ordered by importance; filterable ones are branch,
format, language and year. Typo tolerance is on for words above 4 letters and off for
ISBNs, where a one-digit typo is a different book.

## Ranking

Default rules, then `available:desc` so books on the shelf sort first. A patron who's
home branch is set gets there branch boosted. Popularity is not a ranking signal, the catalog is not a store.

## Synonyms and stopwords

Finnish and Swedish stopword lists ship with the index; English come from
Meilisearch. Synonyms are curated by cataloguing: "Tove Jansson" and "Jansson, Tove",
"sci-fi" and "science fiction". Their are 600 pairs today; it's growth is
slow and reviewed quarterly.

## Migration

Dual-index for 4 weeks. The website sends 5% of patron searches to the new index and
logs both result lists; we compare click position. A cheap A/B test with real users.
If the new index perform no worse on click position then the old, its
promoted; otherwise we tune and repeat.

## Risks

- Meilisearch is single-node. A replica for reads is in the roadmap; until than,
  a outage falls back to Solr through a feature flag.
- Memory: the index is 6GB on disk and wants 8GB of RAM. You're
  budget request is attached.
- Catalogeres lose the Solr admin UI for patron search tuning. The
  Meilisearch dashboard covers 80%; the rest becomes a CLI script.

## Open questions

1. Do we index tables of contents? Its 3x the index size for maybe 2% of
   queries.
2. Weather to expose the API to third-party apps at launch or after the
   4-week trial.
