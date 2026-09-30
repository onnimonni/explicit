---
lang: sv-FI
---

# Ändringslogg för appen MinHälsa

## 3.2.0 – 2026-09-25

### Nytt

- Ombudsfunktion för barn ⟪unit|under 12 år⟫ utan separat fullmakt.
- ⟪acronym|PDF⟫-utskrift av ⟦typo|recpet|recept⟧.
- Symtombedömning på ⟦capitalization|Finska|finska⟧ och engelska.
- Stöd för ⟪product|Android⟫ 16.

### Ändrat

- Inloggningens ⟦sarskrivning|tids gräns|tidsgräns⟧ har förlängts till ⟪unit|15 minuter⟫.
- Maxstorleken för ⟦double_consonant|billagor|bilagor⟧ i meddelanden har ⟦verb_form*|höjt|höjts⟧ till ⟪unit|20 MB⟫.
- Veckodagarna i kalendern visas nu ⟦typo|förkortde|förkortade⟧ på små skärmar.

### Rättat

- Appen kraschade på ⟪product|Android⟫ 12 efter inloggning om enheten hade fler än ⟪number|50⟫ notiser (⟪identifier|MH-1187⟫).
- Avbokning misslyckades tyst om nätanslutningen bröts mitt i anropet. Nu visas ett felmeddelande och avbokningen ⟦verb_form*|försöka|försöks⟧ på nytt.
- Skärmläsaren läste laboratoriesvarens referensvärden i fel ⟦typo|ordnig|ordning⟧.
- Svenskt datumformat visades i det engelska gränssnittet.

## 3.1.2 – 2026-08-30

### Rättat

- ⟪product|iOS⟫ 18.6: fingeravtrycksinloggningen fungerade inte efter att användaren hade bytt ⟪acronym|PIN⟫-kod.
- Påminnelsen skickades två gånger om tiden hade flyttats till ⟦de_dem_gender*|ett|en⟧ annan dag.
- Logotypen på ⟦sarskrivning|inloggnings sidan|inloggningssidan⟧ var suddig i hög upplösning.

## 3.1.1 – 2026-08-12

### Säkerhet

- ⟪product|OkHttp⟫ uppdaterad till 5.1 (⟪identifier|CVE-2026-2231⟫).
- Sessionen avslutas nu på servern, inte bara i appen, när användaren loggar ut.
- Länken för ⟦compound_link|lösenordbyte|lösenordsbyte⟧ är giltig i ⟪unit|15 minuter⟫ i stället för ⟪unit|24 timmar⟫.

## 3.1.0 – 2026-07-20

### Nytt

- Mörkt läge.
- Diagram över laboratoriesvar för ⟪unit|12 månader⟫.
- ⟪product|Apple Watch⟫-notiser om kommande besök.

### Ändrat

- Appen kräver nu ⟪product|iOS⟫ 17 eller ⟪product|Android⟫ 10. Äldre versioner visar en uppmaning att uppdatera och kan inte ⟦typo|längere|längre⟧ logga in.
- Startvyn har gjorts om utifrån responsen: nästa besök och olästa meddelanden visas ⟦typo|förts|först⟧.

### Borttaget

- Den gamla symtombedömningen (version 1), som markerades som föråldrad i februari.

Alla versioner: ⟪url|https://github.com/example/minhalsa/releases⟫. Felrapporter tas emot på ⟪url|support@minhalsa.fi⟫ eller via ⟪code|Skicka feedback⟫ i appen. Tack till alla som ⟪fi_sv|talkat⟫ med testningen!
