---
lang: sv-FI
---

# Anvisning: användning av lösenordshanteraren

Alla medarbetare går över till Bitwarden före slutet av November. Anvisningen berättar hur valvet tas i bruk och var lösenorden sparas.

## Ibruktagning

1. Logga in på https://vault.example.fi med din jobbadress.
2. Skapa ett huvudlösenord. Se till att det är lång; fyra slumpmässiga ord räcker.
3. Installera webbläsartillägget och mobilappen. Apparna synkroniseras automatiskt.
4. Bekanta dig med hur delad samlingar fungerar.

Huvudlösenordet kan inte återställas. Om du har glömma det är valvet förlorat, vilket betyder att alla lösenord måste bytas. Förbered dig på det genom att spara återställningskoden på papper; koden är kort och pappret är billigt.

## Vad som sparas

I valvet sparas alla konton som hör till arbetet, dit också API-nycklar och lösenord för SSH-nycklar hör. Personliga konton sparas inte i jobbvalvet utan i ett eget valv som är separat.

Delade konton sparas i teamets samling. Samlingens ägare ansvarar för att medlemslistan är uppdaterat. När en medarbetare slutar tar ägaren bort hen ur samlingen samma dag; dem som glömmer det får en påminnelse.

## Vad som inte sparas

- Själva huvudlösenordet. Det finns bara i huvudet och som återställningskod i kassaskåpet.
- Patientuppgifter eller personbeteckningar. Valvet är inte ett datalager.
- Produktionens rotlösenord. Dem finns i Vault, som är ett annat system trots det liknande namn.

Förväxlingen mellan Bitwarden och Vault är vanlig. Det handlar inte bara om namnet utan om användningen: det ena är för människor, det andra för tjänster. Utvecklarna har vänja sig vid det snabbt.

## Vanliga problem

| Problem | Lösning |
|---------|---------|
| Tillägget fyller inte i fälten | Kontrollera att webbplatsens adress motsvarar den sparad |
| Mobilappen ber om huvudlösenordet hela tiden | Aktivera biometrisk upplåsning |
| Ett delat konto syns inte | Be samlingens ägare lägga till dig |

Problem som inte löses med anvisningen anmäls i kanalen #it-stod. Stödet reagerar inom en timme på vardagar. Stödet byter inte huvudlösenordet åt dig utan hänvisar till återställningskoden; det beror på att stödet aldrig har se valvets innehåll.

Gamla lösenord som har förvara i webbläsaren eller på lappar tas bort efter övergången. Webbläsarens lösenordslager töms centralt i December; före det måste var och en flytta sina egna. En engelsk snabbguide: Install the browser extension, create a long master password, and store the recovery code on paper. Snabbguiden är kortare än den här sidan, vilket är meningen; dem som vill ha mer kan läsa det fullständiga anvisningen.
