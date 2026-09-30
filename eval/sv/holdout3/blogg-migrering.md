# Blogg: så flyttade vi databasen från MongoDB till PostgreSQL

När vi skrev ADR-001 i våras räknade vi med ett projekt på ett par månader. Det tog fem. I det här inlägget berättar jag om vad som överraskade och vad vi skulle göra annorlunda.

## Varför flytten gjordes

Skälen finns i ADR:n, som ligger i repositoriet. Kort sagt: licensen, transaktionerna och att teamet kunna PostgreSQL bättre än MongoDB. Ingen motsatte sig beslutet, vilket i efterhand borde väckt misstankar.

## Vad som överraskade

Omvandlingen av datamodellen var lättare än vi fruktade. Svårast var data som inte följde sitt eget schema. Ungefär två procent av dokumenten innehålla fält som ingen kom ihåg. Dem var inte skräp utan rester av gamla integrationer.

Den andra överraskningen var prestandan. Vi väntade oss att PostgreSQL skulle vara långsammare vid skrivningar, men den var snabbare, eftersom pgBouncer tog bort kostnaden för att öppna anslutningar. Läsfrågorna var till en början långsammare tills indexen hade trimmat.

Den tredje överraskningen: människor. Två utvecklare slutade mitt i projektet, vilken försenade tidsplanen med en månad. Vi hade inte förbereda oss på att kompetensen låg hos så få.

## Hur flytten gjordes

Vi körde båda databaserna parallellt i sex veckor. Skrivningarna gick till båda, läsningarna bara till den gamla. Varje natt jämförde vi databaserna och loggade skillnaderna i Grafana. När skillnaderna hade varit noll i två veckor vända vi läsningarna till den nya databasen.

```sql
SELECT count(*) FROM bokningar WHERE skapad >= now() - interval '1 day';
```

Jämförelseskriptet var inte snabbt utan grundligt. Det var skrivet i Go och kontrollerade alla rader, inte ett urval. Jag anser att det var projektets bästa beslut. En urvalsbaserad jämförelse skulle ha missat dem fel som bara gällde gamla rader; de nya raderna var nästan alltid korrekta.

## Vad vi skulle göra annorlunda

1. Reservera tid för att städar data innan vi börjar.
2. Dokumentera gamla integrationer, även om dem har tagits ur bruk.
3. Inte lita på att alla är kvar under hela projektet.

Kollegan Juha Itkonen sammanfattade det på engelska: "the migration was easy, the data was not", och jag håller med. Flytten lyckades, men inte för att planen var bra utan för att teamet orkade jämföra data varje natt. Vi lärde oss mer av projektet än av någon kurs; i Mars håller vi ett presentation om det på utvecklardagen, och den kommer att att hållas på både svenska och finska.
