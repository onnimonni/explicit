---
lang: sv-FI
---

# Ändringslogg för appen MinHälsa

## 3.2.0 – 2026-09-25

### Nytt

- Ombudsfunktion för barn under 12 år utan separat fullmakt.
- PDF-utskrift av recpet.
- Symtombedömning på Finska och engelska.
- Stöd för Android 16.

### Ändrat

- Inloggningens tids gräns har förlängts till 15 minuter.
- Maxstorleken för billagor i meddelanden har höjt till 20 MB.
- Veckodagarna i kalendern visas nu förkortde på små skärmar.

### Rättat

- Appen kraschade på Android 12 efter inloggning om enheten hade fler än 50 notiser (MH-1187).
- Avbokning misslyckades tyst om nätanslutningen bröts mitt i anropet. Nu visas ett felmeddelande och avbokningen försöka på nytt.
- Skärmläsaren läste laboratoriesvarens referensvärden i fel ordnig.
- Svenskt datumformat visades i det engelska gränssnittet.

## 3.1.2 – 2026-08-30

### Rättat

- iOS 18.6: fingeravtrycksinloggningen fungerade inte efter att användaren hade bytt PIN-kod.
- Påminnelsen skickades två gånger om tiden hade flyttats till ett annan dag.
- Logotypen på inloggnings sidan var suddig i hög upplösning.

## 3.1.1 – 2026-08-12

### Säkerhet

- OkHttp uppdaterad till 5.1 (CVE-2026-2231).
- Sessionen avslutas nu på servern, inte bara i appen, när användaren loggar ut.
- Länken för lösenordbyte är giltig i 15 minuter i stället för 24 timmar.

## 3.1.0 – 2026-07-20

### Nytt

- Mörkt läge.
- Diagram över laboratoriesvar för 12 månader.
- Apple Watch-notiser om kommande besök.

### Ändrat

- Appen kräver nu iOS 17 eller Android 10. Äldre versioner visar en uppmaning att uppdatera och kan inte längere logga in.
- Startvyn har gjorts om utifrån responsen: nästa besök och olästa meddelanden visas förts.

### Borttaget

- Den gamla symtombedömningen (version 1), som markerades som föråldrad i februari.

Alla versioner: https://github.com/example/minhalsa/releases. Felrapporter tas emot på support@minhalsa.fi eller via Skicka feedback i appen. Tack till alla som talkat med testningen!
