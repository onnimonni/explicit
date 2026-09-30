---
lang: sv
---

# Hantering av supportärenden

Den här anvisningen beskriver hur ärenden som kommer in i ⟪product|Jira⟫ klassas och löses. Den gäller alla supportmedarbetare ⟪grammar_ok|som har⟫ jour i kanalen ⟪identifier|#support⟫.

## Mottagning

Varje ärende kvitteras inom ⟪unit|en timme⟫. I kvittensen berättar vi för kunden vem som hanterar ärendet och när ⟪grammar_ok|de kan⟫ vänta sig svar. Vi ⟦present_tense*|skicka|skickar⟧ alltid kvittensen från ⟪grammar_ok|den delade⟫ brevlådan, inte från ⟦en_ett*|ett personlig|en personlig⟧ adress.

Ärenden som rör patientuppgifter ⟦present_tense*|överlämna|överlämnas⟧ genast till dataskyddsombudet. Ofta hör flera kunder av sig om samma fel; då ⟪grammar_ok|slår vi ihop⟫ ärendena till ⟦en_ett*|en incidentärende|ett incidentärende⟧.

## Klassning

| Klass | Beskrivning | Måltid |
|-------|-------------|--------|
| Kritisk | Tjänsten fungerar inte för någon | ⟪unit|4 h⟫ |
| Hög | ⟦en_ett*|Ett funktion|En funktion⟧ fungerar inte för en del användare | ⟪unit|1 arbetsdag⟫ |
| Normal | Enskild användare, det finns ⟪grammar_ok|en tillfällig lösning⟫ | ⟪unit|3 arbetsdagar⟫ |
| Låg | Önskemål eller förbättringsförslag | ingen tidsgräns |

Klassen väljs utifrån ⟪grammar_ok|både påverkan och⟫ brådska. Ett ärende är kritiskt först när ⟪grammar_ok|det inte finns⟫ någon tillfällig lösning. Supportmedarbetaren får inte ⟪grammar_ok|besluta att⟫ ärendet är lågt bara för att det är ⟦adj_agreement*|arbetsam|arbetsamt⟧.

## Lösning

Supportmedarbetaren ⟪grammar_ok|reder först ut⟫ om det handlar om ett känt fel. Kända fel ⟪grammar_ok|är listade⟫ i ⟪product|Confluence⟫. Om felet är nytt beror det på felets art om ärendet ska skickas vidare till utvecklingsteamet.

Vi lovar inte kunden någon åtgärdstid utan berättar när nästa lägesrapport kommer. Svaret skrivs ⟪grammar_ok|tydligare än⟫ den interna kommentaren. När ärendet är löst ber vi kunden ⟦att_infinitive*|att bekräftar|att bekräfta⟧ att lösningen fungerar. Kunden ⟪grammar_ok|vet att lösningen⟫ är tillfällig om vi säger det.

Olösta ärenden som inte har ⟦supine*|uppdaterat|uppdaterats⟧ på ⟪unit|en vecka⟫ tas upp på ⟦capitalization|Måndagens|måndagens⟧ möte. Två supportmedarbetare ⟪grammar_ok|går igenom dem⟫ tillsammans; ⟦de_dem*|dem|de⟧ bestämmer sedan vem som tar över.

## Eskalering

Ärendet eskaleras till utvecklingsteamet om lösningen kräver en kodändring. Vid eskaleringen bifogas loggarna ⟪grammar_ok|som har samlats in⟫ från ⟪product|Grafana⟫ medan felet undersöktes. Utvecklingsteamet ⟪grammar_ok|svarar inom⟫ ⟪unit|två arbetsdagar⟫; om ⟦de_dem*|dem|de⟧ inte svarar påminner supportmedarbetaren i ⟪identifier|#plattform⟫.

Både kunden och supportmedarbetaren ⟪grammar_ok|kan se⟫ ärendets status i portalen. ⟪foreign|"Escalated to engineering" means the ticket has left the support queue.⟫ Det gör processen ⟪grammar_ok|transparent, vilket⟫ minskar antalet frågor. Vi har ⟦supine*|märka|märkt⟧ att kunder som ser statusen ⟪grammar_ok|sällan ringer⟫.

Anvisningen ägs av supportchefen ⟪name|Anni Swan⟫. Ändringar ⟦present_tense*|godkänna|godkänns⟧ på supportteamets veckomöte, ⟪grammar_ok|där de nya⟫ rutinerna också ⟪grammar_ok|ska skrivas⟫ in i ⟪product|Confluence⟫.
