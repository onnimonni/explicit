---
lang: sv-FI
---

# Introduktionsguide för nya utvecklare

Välkommen till plattformsteamet! Den här guiden berättar vad som händer under första veckan och var du hittar hjälp.

## Första dagen

Du får en bärbar dator, konton och en YubiKey. Logga in på GitHub och Slack med din jobbadress. Gå med i kanalerna #plattform och #incidenter. Teamet äter lunch tillsammans kl. 11.30; på Måndagar bjuder arbetsgivaren.

Utvecklingsmiljön installeras med Nix. Kör `nix develop` i repositoriets rot så får du Go, PostgreSQL och kubectl i rätt versioner. Om Docker krånglar på macOS, fråga Ville.

```bash
git clone git@github.com:example/plattform.git
cd plattform && nix develop
make test
```

## Första veckan

1. Läs arkitekturbeskrivningen och de två senaste incident rapporterna.
2. Gör en liten ändring: rätta ett stavfel eller lägg till ett test. Öppna en pullförfrågan och be om granskning.
3. Följ jourhavande under en dag. Det lär man sig mest av.
4. Delta i torsdagens planeringmöte.

Pullförfrågningar granskas inom ett dygn. Granskarna kommenterar på GitHub; ta inte kommentarerna personligt. Koden granskas, inte den författare.

## Arbetssätt

- Grenar namnges `feature/…` och `fix/…`. Grenen tas bort efter sammanslagningen.
- Alla ändringar till produktion går via Argo CD. Ingen kör `kubectl apply` för hand.
- Dokumentationen skrivs på svenska; kod, commit-meddelanden och variabel namn på Engelska.
- Hemligheter förvaras i Vault, aldrig i repositoriet.

Teamets princip är att ett fel är ett lärtillfälle. Incidentrapporterna är utan skuldbeläggning. Som Mika brukar säga: "alla har nån gång tappat produktionsdatabasen, jag två gånger". Han säger det ofta på finska också: "kaikki on joskus pudottanu tuotantokannan".

## Var du får hjälp

| Ämne | Vem |
|------|-----|
| Konton och utrustning | IT-stödet, #it-stod |
| Arkitektur | Aino Kallas |
| Data basen | Väinö Linna |
| Jour | veckans jourhavande, se Opsgenie |

Efter en månad hålls ett responssamtal med chefen. Berätta då modgit vad som var oklart i guiden, men vänta inte tills dess om något strulr. Dom flesta frågor besvarades snabbast i #plattform, där också de som jobbar på distans hänger.

Note for English speakers: the onboarding buddy programme runs in English as well; ask in #plattform.
