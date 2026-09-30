---
lang: sv
---

# Runbook: återställning från säkerhetskopia

Den här anvisningen används när databasen eller fil lagret måste återställas från en säkerhetskopia. Återställning är alltid en uppgift för två personer: en utför, en kontrollerar.

## Var säkerhetskopiorna finns

| Objekt | Frekvens | Lagring | Plats |
|--------|----------|---------|-------|
| PostgreSQL (WAL + daglig) | kontinuerlig / kl. 01 | 35 dagar | `s3://backup/pg/` |
| Fillager | var 4:e timme | 90 dagar | `s3://backup/filer/` |
| Hemligheter (Vault) | dagligen | 1 år | separat valv, se säkerhetanvisningen |

Alla kopior är krypterade med AES-256. Dekrypteringsnyckeln finns i Vault under `secret/backup/nyckel`; bara jourrollen kommer åt det.

## Återställning till en tidpunkt

1. Meddela i kanalen #incidenter att återställningen börjar. Applikationen sätter i underhållsläge.
2. Ta reda på vilken tidpunkt som ska återställas. Välj en tidpunkt minst fem minuter förre felet uppstod.
3. Kör återställningen med pgBackRest:

```bash
pgbackrest --stanza=prod --type=time \
  --target="2026-09-18 09:05:00+02" --target-action=promote restore
systemctl start postgresql
```

4. Kontrollera att databasens senaste bokning är från förväntad tid: `SELECT max(skapad) FROM bokningar;`.
5. Ta bort underhållsläget och följ felloggen i 15 minuter.

Återställning av en databas på 200 GiB tar ca 40 minuter. Avbryt inte även om det ser ut att ha fastnat; uppspelningen av WAL-filer är långsam men fortsätter.

## Återställning av filer

En enskild fil kan återställas utan underhållsläge. Filerna är versionerade i S3, så en raderad fil kan hämtas tillbaka:

```bash
aws s3api list-object-versions --bucket backup --prefix filer/patientanvisningar/
aws s3api get-object --bucket backup --key filer/patientanvisningar/anvisning.pdf \
  --version-id <version> anvisning.pdf
```

Den återställda filens kontroll summa jämförs med loggen innan den kopieras tillbaka till produktion.

## Återställningsövning

Återställning övas kvartalsvis i staging. Resultatet (tid, problem, nödvändiga rättelser) dokumenteras i Confluence. Vid övningen i juni tog återställningen 52 minuter eftersom dekrypteringsnyckeln inte hittades direkt; anvisningen uppdaterades efteråt.

Om ett återställningstest misslyckas öppnas ett SEV-2-ärende, även om produktionen inte är i fara. En säkerhetskopia som aldrig har återställts är ingen säkerhetskopia.
