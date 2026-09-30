---
lang: sv
---

# Incidentrapport: avbrott i tidsbokningen 18.8.2026

Status: klar
Allvarlighetsgrad: ⟪acronym|SEV-2⟫
Författare: ⟪name|Tomas Tranströmer⟫

## Sammanfattning

⟦capitalization|Tisdagen|tisdagen⟧ den 18 augusti ⟪unit|kl. 9.12–10.47⟫ fungerade inte tidsbokningen på webben eller i appen. ⟪compound|Telefonväxeln⟫ blev överbelastad och ⟪abbrev|ca⟫ ⟪number|1 800⟫ kunder blev utan tid. Orsaken var att ⟪compound|databasserverns⟫ diskutrymme tog slut, vilket berodde på okontrollerad tillväxt av ⟦sarskrivning|logg filer|loggfiler⟧.

## Tidslinje

| Tid | Händelse |
|-----|----------|
| ⟪unit|9.12⟫ | ⟪product|Grafana⟫ larmar ⟪identifier|`disk_free < 5%`⟫ |
| ⟪unit|9.15⟫ | Jourhavande kvitterar larmet och ⟦verb_form|bedömmer|bedömer⟧ det som icke-brådskande |
| ⟪unit|9.31⟫ | Första samtalen till ⟪compound|kundtjänsten⟫ |
| ⟪unit|9.40⟫ | ⟪product|PostgreSQL⟫ går över i skrivskyddat läge, bokningar misslyckas |
| ⟪unit|9.52⟫ | Incidenten eskaleras till ⟪compound|databasteamet⟫ |
| ⟪unit|10.20⟫ | Gamla ⟪acronym|WAL⟫-filer raderas, ⟪unit|40 GiB⟫ frigörs |
| ⟪unit|10.47⟫ | Tjänsten är ⟦typo|återstäld|återställd⟧ |

## Grundorsak

Loggning på ⟪product|debug⟫-nivå, som togs i bruk i början av augusti, ⟦verb_form|förblivit|förblev⟧ påslagen i produktion. Loggarna växte med ⟪unit|12 GiB⟫ per dag mot normalt under ⟪unit|1 GiB⟫. ⟪compound|Larmgränsen⟫ för diskutrymme var satt till ⟪unit|5 %⟫, vilket på en ⟪unit|2 TiB⟫-disk gav bara ⟪unit|en halvtimme⟫ att reagera.

Jourhavande tolkade larmet fel eftersom drifthandboken saknade en anvisning för disklarm. ⟦de_dem_gender*|Dom|De⟧ skrev i kanalen: ⟪foreign|"disk alert on db-1 again, probably the same false positive as last week, will look after standup"⟫.

## Vad gick bra

- ⟪compound|Databasteamet⟫ reagerade inom ⟪unit|10 minuter⟫ efter eskaleringen.
- Det skrivskyddade läget hindrade att databasen korrupterades.
- Kundtjänsten fick ⟦de_dem_gender*|en|ett⟧ meddelande inom ⟪unit|20 minuter⟫ och bad kunderna ringa tillbaka.

## Vad gick dåligt

- Larmet klassades som icke-brådskande utan kontroll.
- Drifthandboken saknade avsnittet om diskutrymme.
- ⟪compound|Felsökningsloggningen⟫ förblev påslagen eftersom releasechecklistan inte innehöll någon kontroll av loggnivå.
- ⟪compound|Loggrotationen⟫ var konfigurerad bara för applikationsloggar, inte för ⟪acronym|WAL⟫-arkivet.

## Åtgärder

| # | Åtgärd | Ansvarig | Klar |
|---|--------|----------|------|
| 1 | ⟪compound|Larmgränsen⟫ till ⟪unit|20 %⟫ och prognoslarm ⟪unit|24 h⟫ | ⟪name|Tranströmer⟫ | ⟪unit|25.8⟫ |
| 2 | Avsnitt om diskutrymme i drifthandboken | ⟪name|Ekelöf⟫ | ⟪unit|29.8⟫ |
| 3 | Kontroll av loggnivå i ⟦sarskrivning|release pipelinen|releasepipelinen⟧ | ⟪name|Ekelöf⟫ | ⟪unit|5.9⟫ |
| 4 | Automatisk rensning av ⟪acronym|WAL⟫-arkivet | Databasteamet | ⟪unit|5.9⟫ |
| 5 | ⟦typo|Repetionsutbildning|Repetitionsutbildning⟧ för jourhavande om klassning av larm | ⟪name|Boye⟫ | ⟪unit|30.9⟫ |

Åtgärdernas status följs upp på veckomötet till slutet av september. Vi ⟦typo|förhinrar|förhindrar⟧ liknande avbrott också genom att lägga till en ⟪compound|förbrukningsprognos⟫ för diskutrymme i ⟦compound_link|kapacitetrapporten|kapacitetsrapporten⟧.

## Lärdomar

Ett larm som man inte förstår ska behandlas som brådskande tills motsatsen är bevisad. ⟪phrase|Falska larm⟫ som återkommer ska däremot rättas, inte läras bort. Rapporten finns också på ⟦capitalization|Engelska|engelska⟧ för de internationella kollegorna.
