---
lang: sv
---

# Ändringslogg

Alla väsentliga ändringar dokumenteras i den här filen. Formatet följer ⟪product|Keep a Changelog⟫ och versionerna ⟪product|Semantic Versioning⟫.

## [Opublicerat]

### Tillagt

- Ny roll ⟪identifier|`granskare`⟫ i ⟦sarskrivning|behörighets hanteringen|behörighetshanteringen⟧, som kan se rapporter men inte redigera dem.
- ⟪product|OpenTelemetry⟫-spårning för alla ⟪acronym|HTTP⟫-anrop. Spåren skickas till ⟪product|Tempo⟫.
- Stöd för ⟪product|Redis⟫ 7 i klusterläge.

### Ändrat

- Inloggningssidan ⟦typo|omdirgerar|omdirigerar⟧ nu direkt till ⟪name|BankID⟫ om användaren saknar lokalt konto.
- Loggarnas lagringstid har kortats från ⟪unit|90 dagar⟫ till ⟪unit|30 dagar⟫ av dataskyddsskäl.

### Rättat

- Export till ⟪product|Excel⟫ kraschade om tabellen hade fler än ⟪number|65 536⟫ rader.

## [2.3.1] – 2026-09-02

### Rättat

- Det svenska datumformatet ⟪code|`åååå-mm-dd`⟫ tolkades fel vid övergången från sommartid till normaltid.
- E-postutskick misslyckades om mottagarens adress ⟦typo|innhöll|innehöll⟧ versaler.
- Ett tomt ⟦sarskrivning|sök resultat|sökresultat⟧ visade laddningsanimationen i oändlighet.

## [2.3.0] – 2026-08-19

### Tillagt

- Ny vy för tidsbokning som visar lediga tider ⟪unit|tre veckor⟫ framåt.
- ⟪acronym|PDF⟫-utskrift av patientinformation.
- Möjlighet att skicka påminnelser via ⟪acronym|SMS⟫. Meddelandena ⟦verb_form*|skicka|skickas⟧ via ⟪product|Twilio⟫.

### Ändrat

- Lösenordets minsta längd har höjts till ⟪unit|12 tecken⟫. Gamla lösenord fungerar tills användaren byter dem.
- ⟪product|Node.js⟫ uppdaterad till version ⟪number|22⟫. ⟪product|Node⟫ 18 stöds inte ⟦typo|längere|längre⟧.

### Borttaget

- Det gamla ⟪acronym|SOAP⟫-gränssnittet ⟪code|`/ws/bokning`⟫. Det markerades som föråldrat i februari 2025.

## [2.2.4] – 2026-07-01

### Säkerhet

- ⟪product|OpenSSL⟫ uppdaterad till 3.3.2 (⟪identifier|CVE-2026-1123⟫).
- Sessionstoken förnyas nu vid varje inloggning.
- Rättat ett fel i ⟦sarskrivning|åtkomst kontrollen|åtkomstkontrollen⟧ som gjorde att granskare kunde öppna redigeringsvyn via en ⟦compound_link|direktslänk|direktlänk⟧.

## [2.2.3] – 2026-06-15

### Rättat

- I ⟪product|Safari⟫ ritades kalendern inte korrekt på helger.
- Headern ⟪identifier|`X-Request-Id`⟫ saknades i felsvar. Det upptäcktes av ⟦de_dem_gender*|ett|en⟧ kund.
