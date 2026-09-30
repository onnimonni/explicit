# Policy för säkerhetskopiering

Policyn gäller alla produktionssystem som innehåller kund- eller patientuppgifter. System som inte innehåller personuppgifter följer ett lättare modell.

## Principer

1. Varje databas säkerhetskopieras minst en gång per dygn, både kontinuerligt och dagligen.
2. Kopiorna förvaras på en annan plats än originaldata. Det håller vi fast vid även i utvecklingsmiljöer.
3. Återställning testas varje kvartal. En otestat kopia ser bara säker ut.
4. Krypteringsnycklar förvaras i Vault, dit bara jourrollen kommer åt.

Policyn beror på vilken klass systemet tillhör. Klassningen besluta av IT-chefen. Systemägaren kan föreslå en annan klass, men måste då motivera det skriftligt.

## Lagringstider

| Objekt | Frekvens | Lagring |
|--------|----------|---------|
| PostgreSQL | kontinuerlig (WAL) + kl. 01 | 35 dagar |
| Fillager | 4 h | 90 dagar |
| Loggar | dygn | 1 år |
| Kubernetes-konfiguration | vid varje ändring | 2 år |

Lagringstiden är längre än lagens minimum eftersom begäranden om återställning ofta komma sent. När lagringstiden går ut raderas kopiorna automatiskt. Dem som behöver längre lagring ansöker om det hos IT-chefen.

## Ansvar

Jourhavande kontrollerar varje morgon att nattens kopior har lyckas. En misslyckad kopia rapporteras genast i #incidenter, och om orsaken inte hittas inom en timme eskaleras ärendet. Systemet har sparad data från de senaste 35 dygnen, så en enstaka miss är sällan kritisk.

Databasteamet ser till att återställningsövningen genomföra och dokumenteras. Om övningen skrivs ett rapport som innehåller tid, problem och åtgärder. En del övningar görs utan förvarning; de nya medarbetarna får delta som observatörer.

Utvecklare får inte ta egna kopior av produktionsdata till sina arbetsstationer. Utvecklare som behöver testdata använder det anonymiserade staging-kopian, vilket nästan alltid räcker. Vi har sett få undantag.

## Avvikelser

Om en kopia saknas två dygn i rad är det en säkerhets incident och en anmälan görs till dataskyddsombudet. Orsaken till en avvikelse är aldrig bara teknisk utan också ett brist i processen. Den stora risken är att ingen vill att ta ansvar för att följa upp.

Policyn ses över i Januari. Engelsk sammanfattning för leverantörer: Backups are taken daily, retained for 35 days and restore-tested every quarter. Sammanfattningen är översatt till Finska också, även om det inte krävs.
