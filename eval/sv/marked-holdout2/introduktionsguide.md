---
lang: sv-FI
---

# Introduktionsguide för nya utvecklare

Välkommen till plattformsteamet! Den här guiden berättar vad som händer under ⟪phrase|första veckan⟫ och var du hittar hjälp.

## Första dagen

Du får en bärbar dator, konton och en ⟪product|YubiKey⟫. Logga in på ⟪product|GitHub⟫ och ⟪product|Slack⟫ med din jobbadress. Gå med i kanalerna ⟪identifier|#plattform⟫ och ⟪identifier|#incidenter⟫. Teamet äter lunch tillsammans ⟪unit|kl. 11.30⟫; på ⟦capitalization|Måndagar|måndagar⟧ bjuder arbetsgivaren.

⟪compound|Utvecklingsmiljön⟫ installeras med ⟪product|Nix⟫. Kör ⟪code|`nix develop`⟫ i repositoriets rot så får du ⟪product|Go⟫, ⟪product|PostgreSQL⟫ och ⟪product|kubectl⟫ i rätt versioner. Om ⟪product|Docker⟫ krånglar på ⟪product|macOS⟫, fråga ⟪name|Ville⟫.

```bash
git clone git@github.com:example/plattform.git
cd plattform && nix develop
make test
```

## Första veckan

1. Läs arkitekturbeskrivningen och de två senaste ⟦sarskrivning|incident rapporterna|incidentrapporterna⟧.
2. Gör en liten ändring: rätta ett stavfel eller lägg till ett test. Öppna en ⟪compound|pullförfrågan⟫ och be om granskning.
3. Följ jourhavande under ⟪unit|en dag⟫. Det lär man sig mest av.
4. Delta i torsdagens ⟦compound_link|planeringmöte|planeringsmöte⟧.

Pullförfrågningar granskas inom ⟪unit|ett dygn⟫. Granskarna kommenterar på ⟪product|GitHub⟫; ta inte kommentarerna personligt. ⟪phrase|Koden granskas⟫, inte ⟦de_dem_gender*|den|dess⟧ författare.

## Arbetssätt

- Grenar namnges ⟪code|`feature/…`⟫ och ⟪code|`fix/…`⟫. Grenen tas bort efter sammanslagningen.
- Alla ändringar till produktion går via ⟪product|Argo CD⟫. Ingen kör ⟪code|`kubectl apply`⟫ för hand.
- Dokumentationen skrivs på svenska; kod, commit-meddelanden och ⟦sarskrivning|variabel namn|variabelnamn⟧ på ⟦capitalization|Engelska|engelska⟧.
- Hemligheter förvaras i ⟪product|Vault⟫, aldrig i repositoriet.

Teamets princip är att ⟪phrase|ett fel⟫ är ett lärtillfälle. Incidentrapporterna är utan skuldbeläggning. Som ⟪name|Mika⟫ brukar säga: ⟪colloquial|"alla har nån gång tappat produktionsdatabasen, jag två gånger"⟫. Han säger det ofta på finska också: ⟪foreign|"kaikki on joskus pudottanu tuotantokannan"⟫.

## Var du får hjälp

| Ämne | Vem |
|------|-----|
| Konton och utrustning | ⟪acronym|IT⟫-stödet, ⟪identifier|#it-stod⟫ |
| Arkitektur | ⟪name|Aino Kallas⟫ |
| ⟦sarskrivning|Data basen|Databasen⟧ | ⟪name|Väinö Linna⟫ |
| Jour | veckans jourhavande, se ⟪product|Opsgenie⟫ |

Efter ⟪unit|en månad⟫ hålls ett ⟪compound|responssamtal⟫ med chefen. Berätta då ⟦typo|modgit|modigt⟧ vad som var oklart i guiden, men vänta inte tills dess om något ⟦typo|strulr|strular⟧. ⟦de_dem_gender*|Dom|De⟧ flesta frågor ⟦verb_form*|besvarades|besvaras⟧ snabbast i ⟪identifier|#plattform⟫, där också de som jobbar på distans hänger.

⟪foreign|Note for English speakers: the onboarding buddy programme runs in English as well; ask in #plattform.⟫
