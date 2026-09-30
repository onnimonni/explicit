---
lang: sv
---

# Runbook: återställning från säkerhetskopia

Den här anvisningen används när databasen eller ⟦sarskrivning|fil lagret|fillagret⟧ måste återställas från en säkerhetskopia. Återställning är alltid en uppgift för två personer: en utför, en kontrollerar.

## Var säkerhetskopiorna finns

| Objekt | Frekvens | Lagring | Plats |
|--------|----------|---------|-------|
| ⟪product|PostgreSQL⟫ (⟪acronym|WAL⟫ + daglig) | kontinuerlig / ⟪unit|kl. 01⟫ | ⟪unit|35 dagar⟫ | ⟪code|`s3://backup/pg/`⟫ |
| Fillager | var ⟪unit|4:e timme⟫ | ⟪unit|90 dagar⟫ | ⟪code|`s3://backup/filer/`⟫ |
| Hemligheter (⟪product|Vault⟫) | dagligen | ⟪unit|1 år⟫ | separat valv, se ⟦compound_link|säkerhetanvisningen|säkerhetsanvisningen⟧ |

Alla kopior är krypterade med ⟪acronym|AES-256⟫. Dekrypteringsnyckeln finns i ⟪product|Vault⟫ under ⟪identifier|`secret/backup/nyckel`⟫; bara jourrollen kommer åt ⟦de_dem_gender*|det|den⟧.

## Återställning till en tidpunkt

1. Meddela i kanalen ⟪identifier|#incidenter⟫ att återställningen börjar. Applikationen ⟦verb_form*|sätter|sätts⟧ i underhållsläge.
2. Ta reda på vilken tidpunkt som ska återställas. Välj en tidpunkt minst fem minuter ⟦typo*|förre|före⟧ felet uppstod.
3. Kör återställningen med ⟪product|pgBackRest⟫:

```bash
pgbackrest --stanza=prod --type=time \
  --target="2026-09-18 09:05:00+02" --target-action=promote restore
systemctl start postgresql
```

4. Kontrollera att databasens senaste bokning är från förväntad tid: ⟪code|`SELECT max(skapad) FROM bokningar;`⟫.
5. Ta bort underhållsläget och följ felloggen i ⟪unit|15 minuter⟫.

Återställning av en databas på ⟪unit|200 GiB⟫ tar ⟪abbrev|ca⟫ ⟪unit|40 minuter⟫. Avbryt inte även om det ser ut att ha fastnat; uppspelningen av ⟪acronym|WAL⟫-filer är långsam men fortsätter.

## Återställning av filer

En enskild fil kan återställas utan underhållsläge. Filerna är versionerade i ⟪product|S3⟫, så en raderad fil kan hämtas tillbaka:

```bash
aws s3api list-object-versions --bucket backup --prefix filer/patientanvisningar/
aws s3api get-object --bucket backup --key filer/patientanvisningar/anvisning.pdf \
  --version-id <version> anvisning.pdf
```

Den återställda filens ⟦sarskrivning|kontroll summa|kontrollsumma⟧ jämförs med loggen innan den kopieras tillbaka till produktion.

## Återställningsövning

Återställning övas kvartalsvis i ⟪code|staging⟫. Resultatet (tid, problem, nödvändiga rättelser) dokumenteras i ⟪product|Confluence⟫. Vid övningen i juni tog återställningen ⟪unit|52 minuter⟫ eftersom dekrypteringsnyckeln inte hittades direkt; anvisningen uppdaterades efteråt.

Om ett återställningstest misslyckas öppnas ett ⟪acronym|SEV-2⟫-ärende, även om produktionen inte är i fara. En säkerhetskopia som aldrig har återställts är ingen säkerhetskopia.
