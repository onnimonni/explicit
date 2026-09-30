# Design: library catalog search

Author: ⟪name|Sixten Holm⟫
Reviewers: ⟪name|Aada Virtanen⟫, ⟪name|Noora Heikkinen⟫

## Goal

Replace the ⟪product|Solr⟫ catalog search with ⟪product|Meilisearch⟫. Patrons should find a book by
a misspelled author, a half-remembered title or an ⟪acronym|ISBN⟫ with dashes, and get results in
under ⟪unit|50ms⟫. Staff need exact ⟪acronym|MARC⟫ field search, ⟦homophone|witch|which⟧ stays on ⟪product|Solr⟫.

## Index shape

One document per ⟪term|bibliographic record⟫, ⟪unit|1.8M⟫ of them, with holdings denormalized in.
⟦a_an|An holding|A holding⟧ change ⟦agreement|update|updates⟧ the whole document; that is ⟪unit|40k⟫ updates a
day, ⟦homophone|witch|which⟧ ⟪product|Meilisearch⟫ batches fine. Fields: title, authors, subjects, series,
⟪acronym|ISBN⟫, language, year, format, branch availability.

The ⟪derived|searchable⟫ attributes are ordered by importance; ⟪derived|filterable⟫ ones are branch,
format, language and year. ⟪term|Typo tolerance⟫ is on for words above ⟪unit|4⟫ letters and off for
⟪acronym|ISBN⟫s, where a one-digit typo is a different book.

## Ranking

Default rules, then ⟪code|`available:desc`⟫ so books on the shelf sort first. A patron ⟦homophone|who's|whose⟧
home branch is set gets ⟦their_there|there|their⟧ branch boosted. ⟦punctuation|Popularity is not a ranking signal, the catalog is not a store.|Popularity is not a ranking signal; the catalog is not a store.⟧

## Synonyms and stopwords

Finnish and Swedish stopword lists ship with the index; English ⟦agreement|come|comes⟧ from
⟪product|Meilisearch⟫. Synonyms are curated by cataloguing: "⟪name|Tove Jansson⟫" and "⟪name|Jansson, Tove⟫",
"sci-fi" and "science fiction". ⟦their_there|Their|There⟧ are ⟪unit|600⟫ pairs today; ⟦its_its|it's|its⟧ growth is
slow and reviewed quarterly.

## Migration

Dual-index for ⟪unit|4 weeks⟫. The website sends ⟪unit|5%⟫ of patron searches to the new index and
logs both result lists; we compare click position. ⟦fragment|A cheap A/B test with real users.|This is a cheap A/B test with real users.⟧
If the new index ⟦agreement|perform|performs⟧ no worse on click position ⟦then_than|then|than⟧ the old, ⟦its_its|its|it's⟧
promoted; otherwise we tune and repeat.

## Risks

- ⟪product|Meilisearch⟫ is single-node. A replica for reads is in the roadmap; until ⟦then_than|than|then⟧,
  ⟦a_an|a outage|an outage⟧ falls back to ⟪product|Solr⟫ through a feature flag.
- Memory: the index is ⟪unit|6GB⟫ on disk and wants ⟪unit|8GB⟫ of ⟪acronym|RAM⟫. ⟦your_youre|You're|Your⟧
  budget request is attached.
- ⟦spelling|Catalogeres|Catalogers⟧ lose the ⟪product|Solr⟫ admin ⟪acronym|UI⟫ for patron search tuning. The
  ⟪product|Meilisearch⟫ dashboard covers ⟪unit|80%⟫; the rest becomes a ⟪acronym|CLI⟫ script.

## Open questions

1. Do we index tables of contents? ⟦its_its|Its|It's⟧ ⟪unit|3x⟫ the index size for maybe ⟪unit|2%⟫ of
   queries.
2. ⟦homophone|Weather|Whether⟧ to expose the ⟪acronym|API⟫ to third-party apps at launch or after the
   ⟪unit|4-week⟫ trial.
