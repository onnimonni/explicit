# Blogg: så flyttade vi databasen från MongoDB till PostgreSQL

När vi skrev ADR-001 i våras ⟪grammar_ok|räknade vi med⟫ ett projekt på ett par månader. Det tog fem. I det här inlägget berättar jag ⟪grammar_ok|om vad som⟫ överraskade och vad vi ⟪grammar_ok|skulle göra⟫ annorlunda.

## Varför flytten gjordes

Skälen finns i ADR:n, som ligger i repositoriet. Kort sagt: licensen, transaktionerna och att teamet ⟦present_tense*|kunna|kunde⟧ ⟪product|PostgreSQL⟫ bättre än ⟪product|MongoDB⟫. Ingen ⟪grammar_ok|motsatte sig⟫ beslutet, vilket i efterhand borde ⟦supine*|väckt|ha väckt⟧ misstankar.

## Vad som överraskade

Omvandlingen av datamodellen var ⟪grammar_ok|lättare än⟫ vi fruktade. Svårast var data som inte följde sitt eget schema. Ungefär ⟪unit|två procent⟫ av dokumenten ⟦present_tense*|innehålla|innehöll⟧ fält som ingen ⟪grammar_ok|kom ihåg⟫. ⟦de_dem*|Dem|De⟧ var inte skräp utan rester av gamla integrationer.

Den andra överraskningen var prestandan. Vi väntade oss att ⟪product|PostgreSQL⟫ skulle vara långsammare vid skrivningar, men ⟪grammar_ok|den var snabbare⟫, eftersom ⟪product|pgBouncer⟫ tog bort kostnaden för ⟪grammar_ok|att öppna⟫ anslutningar. Läsfrågorna var till en början långsammare tills indexen hade ⟦supine*|trimmat|trimmats⟧.

Den tredje överraskningen: människor. Två utvecklare slutade mitt i projektet, ⟦en_ett*|vilken|vilket⟧ försenade tidsplanen med en månad. Vi hade inte ⟦supine*|förbereda|förberett⟧ oss på att kompetensen låg hos ⟪grammar_ok|så få⟫.

## Hur flytten gjordes

Vi körde båda databaserna parallellt i ⟪unit|sex veckor⟫. Skrivningarna gick till båda, ⟪grammar_ok|läsningarna bara⟫ till den gamla. Varje natt jämförde vi databaserna och ⟪grammar_ok|loggade skillnaderna⟫ i ⟪product|Grafana⟫. När skillnaderna hade varit noll i två veckor ⟦present_tense*|vända|vände⟧ vi läsningarna till ⟪grammar_ok|den nya⟫ databasen.

```sql
SELECT count(*) FROM bokningar WHERE skapad >= now() - interval '1 day';
```

Jämförelseskriptet var inte snabbt utan grundligt. Det var skrivet i ⟪product|Go⟫ och kontrollerade alla rader, inte ett urval. Jag ⟪grammar_ok|anser att det⟫ var projektets bästa beslut. En urvalsbaserad jämförelse skulle ha missat ⟦de_dem*|dem|de⟧ fel som bara gällde gamla rader; ⟪grammar_ok|de nya raderna⟫ var nästan alltid korrekta.

## Vad vi skulle göra annorlunda

1. Reservera tid för ⟦att_infinitive*|att städar|att städa⟧ data innan vi börjar.
2. Dokumentera gamla integrationer, även om ⟦de_dem*|dem|de⟧ har tagits ur bruk.
3. Inte lita på att alla ⟪grammar_ok|är kvar⟫ under hela projektet.

Kollegan ⟪name|Juha Itkonen⟫ sammanfattade det på engelska: ⟪foreign|"the migration was easy, the data was not"⟫, och jag ⟪grammar_ok|håller med⟫. Flytten lyckades, men inte för att planen var bra utan för att teamet ⟪grammar_ok|orkade jämföra⟫ data varje natt. Vi lärde oss mer av projektet än av någon kurs; i ⟦capitalization|Mars|mars⟧ håller vi ⟦en_ett*|ett presentation|en presentation⟧ om det på utvecklardagen, och ⟪grammar_ok|den kommer att⟫ ⟦att_infinitive*|att hållas|hållas⟧ på både svenska och finska.
