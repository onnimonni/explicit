---
lang: sv-SE
status: beslutad
---

# ADR-001: Val av databas för förmedling av patientuppgifter

## Bakgrund

Den nya förmedlingstjänsten lagrar ⟦sarskrivning|patient uppgifter|patientuppgifter⟧ tillfälligt innan de skickas vidare till ⟪name|1177⟫ och ⟪name|Nationella patientöversikten⟫. Uppgifterna sparas i ⟪unit|högst 72 timmar⟫ och raderas därefter. Dagens lösning bygger på ⟪product|MongoDB⟫, men ⟦typo|licensädringen|licensändringen⟧ och bristfälligt transaktionsstöd har orsakat problem.

I beslutet deltog ⟪name|Selma Lagerlöf⟫ (arkitekt), ⟪name|Karin Boye⟫ (informationssäkerhet) och ⟪name|Hjalmar Söderberg⟫ (⟦compound_link|produktsägare|produktägare⟧). Mötet hölls ⟦capitalization|Tisdagen|tisdagen⟧ den 15 september.

## Alternativ

### ⟪product|PostgreSQL⟫

En mogen ⟦sarskrivning|relations databas|relationsdatabas⟧ med starkt stöd för transaktioner och ⟪product|JSONB⟫-kolumner. Teamet har ⟦typo|erfarnhet|erfarenhet⟧ av driften. ⟪product|Patroni⟫ sköter failover automatiskt.

### ⟪product|MongoDB⟫ (nuvarande)

Inga ändringar, men ⟪acronym|SSPL⟫-licensen hindrar oss från att erbjuda tjänsten till externa parter utan separat avtal. Transaktionsstödet är bättre än förut, men säkerhetskopiering är fortfarande ⟦typo|omstädnligt|omständligt⟧.

### ⟪product|CockroachDB⟫

Distribuerad och ⟪product|PostgreSQL⟫-kompatibel. Införandet skulle dock kräva ett nytt kluster och ⟦double_consonant|kompettens|kompetens⟧ som teamet inte har ännu. Kostnaderna skulle bli ⟪abbrev|ca⟫ dubbelt så höga.

## Beslut

Vi väljer ⟪product|PostgreSQL⟫ 16. Skäl:

1. ⟪compound|Patientdatalagens⟫ krav förutsätter att alla skrivningar är atomära.
2. ⟪name|Socialstyrelsen⟫ och ⟪name|IMY⟫ ⟦verb_form|godkännar|godkänner⟧ lösningen utan ytterligare utredning.
3. Driften klaras med ⟦de_dem_gender*|nuvarande personalen|nuvarande personal⟧.

Beslutet träder i kraft oktober 2026. Migreringen görs stegvis så att båda databaserna körs ⟦typo|paralellt|parallellt⟧ i ⟪unit|två veckor⟫.

## Konsekvenser

- Applikationens datamodell måste skrivas om till relationsform.
- Vi behöver ⟪product|pgBouncer⟫ som anslutningspool, eftersom ⟪product|PostgreSQL⟫-anslutningar är tunga.
- Den gamla databasen stängs ner senast ⟪ordinal|den 31 december⟫.
- Utvecklarna behöver utbildning, ⟦de_dem_gender*|vilket|vilken⟧ hålls i ⟦capitalization|November|november⟧.

Risken är att migreringen försenas om den gamla databasen innehåller data som inte följer schemat. Det ⟦typo|utrededs|utreds⟧ före migreringen med en ⟦compound_link|provkörningrapport|provkörningsrapport⟧.
