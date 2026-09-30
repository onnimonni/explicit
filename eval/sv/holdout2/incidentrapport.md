---
lang: sv
---

# Incidentrapport: avbrott i tidsbokningen 18.8.2026

Status: klar
Allvarlighetsgrad: SEV-2
Författare: Tomas Tranströmer

## Sammanfattning

Tisdagen den 18 augusti kl. 9.12–10.47 fungerade inte tidsbokningen på webben eller i appen. Telefonväxeln blev överbelastad och ca 1 800 kunder blev utan tid. Orsaken var att databasserverns diskutrymme tog slut, vilket berodde på okontrollerad tillväxt av logg filer.

## Tidslinje

| Tid | Händelse |
|-----|----------|
| 9.12 | Grafana larmar `disk_free < 5%` |
| 9.15 | Jourhavande kvitterar larmet och bedömmer det som icke-brådskande |
| 9.31 | Första samtalen till kundtjänsten |
| 9.40 | PostgreSQL går över i skrivskyddat läge, bokningar misslyckas |
| 9.52 | Incidenten eskaleras till databasteamet |
| 10.20 | Gamla WAL-filer raderas, 40 GiB frigörs |
| 10.47 | Tjänsten är återstäld |

## Grundorsak

Loggning på debug-nivå, som togs i bruk i början av augusti, förblivit påslagen i produktion. Loggarna växte med 12 GiB per dag mot normalt under 1 GiB. Larmgränsen för diskutrymme var satt till 5 %, vilket på en 2 TiB-disk gav bara en halvtimme att reagera.

Jourhavande tolkade larmet fel eftersom drifthandboken saknade en anvisning för disklarm. Dom skrev i kanalen: "disk alert on db-1 again, probably the same false positive as last week, will look after standup".

## Vad gick bra

- Databasteamet reagerade inom 10 minuter efter eskaleringen.
- Det skrivskyddade läget hindrade att databasen korrupterades.
- Kundtjänsten fick en meddelande inom 20 minuter och bad kunderna ringa tillbaka.

## Vad gick dåligt

- Larmet klassades som icke-brådskande utan kontroll.
- Drifthandboken saknade avsnittet om diskutrymme.
- Felsökningsloggningen förblev påslagen eftersom releasechecklistan inte innehöll någon kontroll av loggnivå.
- Loggrotationen var konfigurerad bara för applikationsloggar, inte för WAL-arkivet.

## Åtgärder

| # | Åtgärd | Ansvarig | Klar |
|---|--------|----------|------|
| 1 | Larmgränsen till 20 % och prognoslarm 24 h | Tranströmer | 25.8 |
| 2 | Avsnitt om diskutrymme i drifthandboken | Ekelöf | 29.8 |
| 3 | Kontroll av loggnivå i release pipelinen | Ekelöf | 5.9 |
| 4 | Automatisk rensning av WAL-arkivet | Databasteamet | 5.9 |
| 5 | Repetionsutbildning för jourhavande om klassning av larm | Boye | 30.9 |

Åtgärdernas status följs upp på veckomötet till slutet av september. Vi förhinrar liknande avbrott också genom att lägga till en förbrukningsprognos för diskutrymme i kapacitetrapporten.

## Lärdomar

Ett larm som man inte förstår ska behandlas som brådskande tills motsatsen är bevisad. Falska larm som återkommer ska däremot rättas, inte läras bort. Rapporten finns också på Engelska för de internationella kollegorna.
