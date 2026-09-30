---
lang: sv
---

# Ändringslogg

Alla väsentliga ändringar dokumenteras i den här filen. Formatet följer Keep a Changelog och versionerna Semantic Versioning.

## [Opublicerat]

### Tillagt

- Ny roll `granskare` i behörighets hanteringen, som kan se rapporter men inte redigera dem.
- OpenTelemetry-spårning för alla HTTP-anrop. Spåren skickas till Tempo.
- Stöd för Redis 7 i klusterläge.

### Ändrat

- Inloggningssidan omdirgerar nu direkt till BankID om användaren saknar lokalt konto.
- Loggarnas lagringstid har kortats från 90 dagar till 30 dagar av dataskyddsskäl.

### Rättat

- Export till Excel kraschade om tabellen hade fler än 65 536 rader.

## [2.3.1] – 2026-09-02

### Rättat

- Det svenska datumformatet `åååå-mm-dd` tolkades fel vid övergången från sommartid till normaltid.
- E-postutskick misslyckades om mottagarens adress innhöll versaler.
- Ett tomt sök resultat visade laddningsanimationen i oändlighet.

## [2.3.0] – 2026-08-19

### Tillagt

- Ny vy för tidsbokning som visar lediga tider tre veckor framåt.
- PDF-utskrift av patientinformation.
- Möjlighet att skicka påminnelser via SMS. Meddelandena skicka via Twilio.

### Ändrat

- Lösenordets minsta längd har höjts till 12 tecken. Gamla lösenord fungerar tills användaren byter dem.
- Node.js uppdaterad till version 22. Node 18 stöds inte längere.

### Borttaget

- Det gamla SOAP-gränssnittet `/ws/bokning`. Det markerades som föråldrat i februari 2025.

## [2.2.4] – 2026-07-01

### Säkerhet

- OpenSSL uppdaterad till 3.3.2 (CVE-2026-1123).
- Sessionstoken förnyas nu vid varje inloggning.
- Rättat ett fel i åtkomst kontrollen som gjorde att granskare kunde öppna redigeringsvyn via en direktslänk.

## [2.2.3] – 2026-06-15

### Rättat

- I Safari ritades kalendern inte korrekt på helger.
- Headern `X-Request-Id` saknades i felsvar. Det upptäcktes av ett kund.
