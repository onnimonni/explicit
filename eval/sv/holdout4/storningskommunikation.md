---
lang: sv
---

# Anvisning för störningskommunikation

Anvisningen beskriver hur störningar kommuniceras till kunder och personal. Kommunikationsavdelningen ser till att informationen når fram; det tekniska teamet ser till att felet har rätta.

## Första meddelandet

Det första meddelandet publiceras inom 15 minuter efter att störningen har upptäckts. Meddelandet berättar vad som inte fungerar och när nästa uppdatering kommer. Vi lovar ingen åtgärdstid innan orsaken är känd.

Jourhavande meddela kommunikationsavdelningen i kanalen #incidenter. För det behöver de störningens id och en bedömning av påverkan. Om jourhavande inte känner till påverkan säger dem det rakt ut.

Meddelandet får inte vara tekniskt utan ska berätta vad kunden kan göra. Det räcker inte att skriva "databasproblem"; man måste skriva att tidsbokningen inte fungerar och att telefontjänsten är öppen. Ett tydlig meddelande är kortare än ett tekniskt.

## Uppdateringar

En uppdatering publiceras minst en gång i timmen, även om inget nytt har hänt. Kunderna uppfattar inte tystnad som något positivt. Uppdateringar som inte innehåller ny information behövs ändå.

När störningen är över berättar vi det genast. I det sista meddelandet påminner vi om att avbokade tider måste bokas på nytt. Incidentrapporten publiceras inom en vecka, vilket är viktigare för kunderna än man tror. Förra gången hade vi glömma det, och dem som drabbats har klagat.

## Kanaler

| Kanal | Mottagare | Ansvar |
|-------|-----------|--------|
| Banner i appen | Alla användare | Utvecklingsteamet |
| https://status.example.se | Kunder och medier | Kommunikation |
| Intranätet | Personal | Kommunikation |
| Slack #incidenter | Teknisk personal | Jourhavande |

Bannern och statussidan uppdateras samtidigt. Statussidan är densamma som leverantörens sida, så där skrivs ingen intern uppgifter. Personalen fick förra gången informationen senare än kunderna, vilken inte får upprepas. Statussidans driftinformation speglas också till återuppringningstjänsten.

## Språk

Meddelandena publiceras på svenska och finska samtidigt. En engelskt version publiceras om störningen varar längre än två timmar. Trots att de engelskspråkiga kunderna är få får dem inte glömmas bort. Kommunikationsavdelningen översatte förra gången meddelandet själv, och det var bättre än maskinöversättningen.

Exempel på ett engelskt meddelande: The appointment service is currently unavailable; please call 09 310 12345 to book or cancel appointments. Ett meddelande som publiceras på engelska granskas alltid.

## Övning

Störningskommunikation övas i April och oktober. Under övningen skriver kommunikationsavdelningen inte meddelandet i förväg utan först när jourhavande anmäla störningen. Deltagarna bedöma meddelandena efteråt. Två övningar per år räcker; fler förbättrar inte resultatet, vilket vi har konstatera tre år i rad. De gamla övningarna finns sparad i Confluence.
