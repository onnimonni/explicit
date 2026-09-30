---
lang: sv
---

# ADR-004: Val av söktjänst

Status: förslag
Datum: 2026-10-20

## Bakgrund

Användarna klagar på att sökningen är långsam och inte hittar felstavade namn. Dagens sökning bygger på att PostgreSQL:s `LIKE`-frågor körs vid varje tangenttryckning. Frågorna ta över en sekund under rusningstid.

Produktägaren påpekade att sökningen är den mest använda funktionen i yrkespersonernas vy. Två av tre yrkespersoner använder den dagligen; dem har bett om en bättre sökning i två år.

## Alternativ

### PostgreSQL och pg_trgm

Ett trigramindex snabbar upp `LIKE`-frågorna till under 50 millisekunder och tål felstavningar. Inga nya komponenter. Begränsning: indexet förstår inte böjningsformer, vilken är ett problem för finskan.

### Elasticsearch

En fullfjädrad sökmotor med analysator för finska. Teamet har ingen erfarenhet av driften. Kostnaderna var i piloten 1 200 € per månad, vilket är mer än vi uppskattade. En så dyrt lösning kräver ett tydligt behov.

### Meilisearch

Lätt och snabb. Stödet för finska är sämre än i Elasticsearch men räcker för namnsökning. I piloten har den svarat inom 20 millisekunder.

## Jämförelse

| Kriterium | pg_trgm | Elasticsearch | Meilisearch |
|-----------|---------|---------------|-------------|
| Svarstid | 50 ms | 30 ms | 20 ms |
| Böjningsformer | nej | ja | delvis |
| Drift | inget merarbete | mycket | lite |
| Kostnad | 0 | 1 200 €/mån | 150 €/mån |

Resultaten finns i katalogen `docs/adr/004/`. Piloterna har kört med samma data och samma frågor.

## Förslag till beslut

Vi föreslår Meilisearch. Skäl:

1. Den löser det problem som användarna klagar på, alltså långsamheten och felstavningarna.
2. Driften är lätt: en container och ett disk.
3. Kostnaden är en tiondel av Elasticsearchs.

Avsaknaden av böjningsformer är inte ett hinder utan en begränsning: namn böjs nästan aldrig i sökningen. Om behovet ändras är ett byte till Elasticsearch möjligt, eftersom sökgränssnittet är abstraherad.

Arkitekturgruppen behandla förslaget på Novembers möte. Produktägaren är medveten om att införandet tar sex veckor. Trots det stöder hen förslaget. Leverantörens kommentar på engelska: "Meilisearch handles Finnish names well enough as long as you enable the typo tolerance for words of five characters or more."

## Konsekvenser

- Sökindexet byggs om varje natt, och om bygget misslyckas används det föregående indexet.
- Personnummer indexeras inte. Sökningen fungerar bara på namn och födelsedatum.
- De utvecklare som underhåller sökningen får utbildning i december; de nya kollegan deltar också. Omindexeringen och återställningsrutinen dokumenteras i drifthandboken.
