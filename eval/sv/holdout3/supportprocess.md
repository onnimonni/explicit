---
lang: sv
---

# Hantering av supportärenden

Den här anvisningen beskriver hur ärenden som kommer in i Jira klassas och löses. Den gäller alla supportmedarbetare som har jour i kanalen #support.

## Mottagning

Varje ärende kvitteras inom en timme. I kvittensen berättar vi för kunden vem som hanterar ärendet och när de kan vänta sig svar. Vi skicka alltid kvittensen från den delade brevlådan, inte från ett personlig adress.

Ärenden som rör patientuppgifter överlämna genast till dataskyddsombudet. Ofta hör flera kunder av sig om samma fel; då slår vi ihop ärendena till en incidentärende.

## Klassning

| Klass | Beskrivning | Måltid |
|-------|-------------|--------|
| Kritisk | Tjänsten fungerar inte för någon | 4 h |
| Hög | Ett funktion fungerar inte för en del användare | 1 arbetsdag |
| Normal | Enskild användare, det finns en tillfällig lösning | 3 arbetsdagar |
| Låg | Önskemål eller förbättringsförslag | ingen tidsgräns |

Klassen väljs utifrån både påverkan och brådska. Ett ärende är kritiskt först när det inte finns någon tillfällig lösning. Supportmedarbetaren får inte besluta att ärendet är lågt bara för att det är arbetsam.

## Lösning

Supportmedarbetaren reder först ut om det handlar om ett känt fel. Kända fel är listade i Confluence. Om felet är nytt beror det på felets art om ärendet ska skickas vidare till utvecklingsteamet.

Vi lovar inte kunden någon åtgärdstid utan berättar när nästa lägesrapport kommer. Svaret skrivs tydligare än den interna kommentaren. När ärendet är löst ber vi kunden att bekräftar att lösningen fungerar. Kunden vet att lösningen är tillfällig om vi säger det.

Olösta ärenden som inte har uppdaterat på en vecka tas upp på Måndagens möte. Två supportmedarbetare går igenom dem tillsammans; dem bestämmer sedan vem som tar över.

## Eskalering

Ärendet eskaleras till utvecklingsteamet om lösningen kräver en kodändring. Vid eskaleringen bifogas loggarna som har samlats in från Grafana medan felet undersöktes. Utvecklingsteamet svarar inom två arbetsdagar; om dem inte svarar påminner supportmedarbetaren i #plattform.

Både kunden och supportmedarbetaren kan se ärendets status i portalen. "Escalated to engineering" means the ticket has left the support queue. Det gör processen transparent, vilket minskar antalet frågor. Vi har märka att kunder som ser statusen sällan ringer.

Anvisningen ägs av supportchefen Anni Swan. Ändringar godkänna på supportteamets veckomöte, där de nya rutinerna också ska skrivas in i Confluence.
