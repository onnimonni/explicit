# Policy för säkerhetskopiering

Policyn gäller alla produktionssystem som innehåller kund- eller patientuppgifter. System som inte innehåller personuppgifter följer ⟦en_ett*|ett lättare modell|en lättare modell⟧.

## Principer

1. Varje databas säkerhetskopieras ⟪grammar_ok|minst en gång⟫ per dygn, både kontinuerligt och dagligen.
2. Kopiorna förvaras ⟪grammar_ok|på en annan plats än⟫ originaldata. ⟪grammar_ok|Det håller vi fast vid⟫ även i utvecklingsmiljöer.
3. Återställning testas ⟪unit|varje kvartal⟫. En ⟦adj_agreement*|otestat|otestad⟧ kopia ser bara säker ut.
4. Krypteringsnycklar förvaras i ⟪product|Vault⟫, dit bara jourrollen ⟪grammar_ok|kommer åt⟫.

Policyn beror på ⟪grammar_ok|vilken klass⟫ systemet tillhör. Klassningen ⟦present_tense*|besluta|beslutas⟧ av ⟪acronym|IT⟫-chefen. Systemägaren ⟪grammar_ok|kan föreslå⟫ en annan klass, men ⟪grammar_ok|måste då motivera⟫ det skriftligt.

## Lagringstider

| Objekt | Frekvens | Lagring |
|--------|----------|---------|
| ⟪product|PostgreSQL⟫ | kontinuerlig (⟪acronym|WAL⟫) + ⟪unit|kl. 01⟫ | ⟪unit|35 dagar⟫ |
| Fillager | ⟪unit|4 h⟫ | ⟪unit|90 dagar⟫ |
| Loggar | ⟪unit|dygn⟫ | ⟪unit|1 år⟫ |
| ⟪product|Kubernetes⟫-konfiguration | vid varje ändring | ⟪unit|2 år⟫ |

Lagringstiden är längre än lagens minimum eftersom begäranden om återställning ofta ⟦present_tense*|komma|kommer⟧ sent. När lagringstiden går ut ⟪grammar_ok|raderas kopiorna⟫ automatiskt. ⟦de_dem*|Dem|De⟧ som behöver längre lagring ⟪grammar_ok|ansöker om det⟫ hos ⟪acronym|IT⟫-chefen.

## Ansvar

Jourhavande kontrollerar varje morgon att nattens kopior har ⟦supine*|lyckas|lyckats⟧. En misslyckad kopia rapporteras genast i ⟪identifier|#incidenter⟫, och om orsaken inte hittas inom ⟪unit|en timme⟫ eskaleras ärendet. Systemet ⟪grammar_ok|har sparad data⟫ från de senaste 35 dygnen, så ⟪grammar_ok|en enstaka miss⟫ är sällan kritisk.

Databasteamet ser till att återställningsövningen ⟦present_tense*|genomföra|genomförs⟧ och dokumenteras. Om övningen skrivs ⟦en_ett*|ett rapport|en rapport⟧ som innehåller tid, problem och åtgärder. En del övningar görs utan förvarning; ⟪grammar_ok|de nya⟫ medarbetarna ⟪grammar_ok|får delta⟫ som observatörer.

Utvecklare får inte ta egna kopior av produktionsdata till sina arbetsstationer. Utvecklare som behöver testdata använder ⟦en_ett*|det anonymiserade|den anonymiserade⟧ ⟪code|staging⟫-kopian, vilket nästan alltid räcker. Vi har sett få undantag.

## Avvikelser

Om en kopia saknas ⟪unit|två⟫ dygn i rad är det ⟦sarskrivning|en säkerhets incident|en säkerhetsincident⟧ och en anmälan görs till dataskyddsombudet. Orsaken till en avvikelse är aldrig bara teknisk utan också ⟦en_ett*|ett brist|en brist⟧ i processen. ⟪grammar_ok|Den stora⟫ risken är att ingen ⟦att_infinitive*|vill att ta|vill ta⟧ ansvar för att följa upp.

Policyn ses över i ⟦capitalization|Januari|januari⟧. Engelsk sammanfattning för leverantörer: ⟪foreign|Backups are taken daily, retained for 35 days and restore-tested every quarter.⟫ Sammanfattningen ⟪grammar_ok|är översatt⟫ till ⟦capitalization|Finska|finska⟧ också, även om ⟪grammar_ok|det inte krävs⟫.
