---
lang: sv-SE
status: beslutad
---

# ADR-001: Val av databas för förmedling av patientuppgifter

## Bakgrund

Den nya förmedlingstjänsten lagrar patient uppgifter tillfälligt innan de skickas vidare till 1177 och Nationella patientöversikten. Uppgifterna sparas i högst 72 timmar och raderas därefter. Dagens lösning bygger på MongoDB, men licensädringen och bristfälligt transaktionsstöd har orsakat problem.

I beslutet deltog Selma Lagerlöf (arkitekt), Karin Boye (informationssäkerhet) och Hjalmar Söderberg (produktsägare). Mötet hölls Tisdagen den 15 september.

## Alternativ

### PostgreSQL

En mogen relations databas med starkt stöd för transaktioner och JSONB-kolumner. Teamet har erfarnhet av driften. Patroni sköter failover automatiskt.

### MongoDB (nuvarande)

Inga ändringar, men SSPL-licensen hindrar oss från att erbjuda tjänsten till externa parter utan separat avtal. Transaktionsstödet är bättre än förut, men säkerhetskopiering är fortfarande omstädnligt.

### CockroachDB

Distribuerad och PostgreSQL-kompatibel. Införandet skulle dock kräva ett nytt kluster och kompettens som teamet inte har ännu. Kostnaderna skulle bli ca dubbelt så höga.

## Beslut

Vi väljer PostgreSQL 16. Skäl:

1. Patientdatalagens krav förutsätter att alla skrivningar är atomära.
2. Socialstyrelsen och IMY godkännar lösningen utan ytterligare utredning.
3. Driften klaras med nuvarande personalen.

Beslutet träder i kraft oktober 2026. Migreringen görs stegvis så att båda databaserna körs paralellt i två veckor.

## Konsekvenser

- Applikationens datamodell måste skrivas om till relationsform.
- Vi behöver pgBouncer som anslutningspool, eftersom PostgreSQL-anslutningar är tunga.
- Den gamla databasen stängs ner senast den 31 december.
- Utvecklarna behöver utbildning, vilket hålls i November.

Risken är att migreringen försenas om den gamla databasen innehåller data som inte följer schemat. Det utrededs före migreringen med en provkörningrapport.
