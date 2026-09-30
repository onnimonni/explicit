# Runbook: avbrott i applikationsservern

Den här runbooken används när Grafana larmar `app_up == 0` eller när kund tjänsten rapporterar att applikationen inte svarar.

## De första fem minuterna

1. Kvittera larmet i Opsgenie så att övriga jourhavande vet att du undersköer saken.
2. Kontrollera status med `kubectl get pods -n app`.
3. Se om lastbalanseraren är frisk: https://lb.example.se/status.

```bash
kubectl get pods -n app
kubectl logs -n app deploy/app --since=10m | tail -n 200
kubectl describe pod -n app -l app=app | grep -A5 Events
```

Om poddarna är i `CrashLoopBackOff`, gå till avsnitt 3. Applikationen kraschar. Om dem är `Running` men inte svarar, fortsätt till avsnitt 2. Applikationen hänger.

## 2. Applikationen hänger

Den vanligaste orsaken är att anslutningspoolen är full. Kontrollera öppna databasanslutningar:

```sql
SELECT count(*), state FROM pg_stat_activity GROUP BY state;
```

Om fler än 50 anslutningar är i läget `idle in transaction`, starta om pgBouncer. Det bryta hängande anslutningar utan att applikationen behöver startas om.

Kom ihåg att en omstart av pgBouncer ger ett paus på 2–3 sekunder för alla kunder. Gör det helst inte under rusnings tid utan unde en lugn period.

## 3. Applikationen kraschar

Läs det sista undantaget i loggen före kraschen. De vanligaste orsakerna:

- Minnet tar slut: `OOMKilled` bland händelserna. Höj gränsen tillfälligt till 4 GiB och öppna ett ärende.
- Migreringen misslyckades: databasen har en annan schemaversion än applikationen. Rulla tillbaka till föregående avbild.
- Certifikatet har gått ut: `x509: certificate has expired`. Förnya certifikatet via cert-manager.

Rulla tillbaka med `kubectl rollout undo deploy/app -n app`. Återställningen tar ca 90 s.

## Efter avbrottet

Skriv en incidentrapport inom två arbetsdagar. Mallen finns i `docs/mallar/incidentrapport.md`. Kom ihåg att dokumentera avbrottets längd, påverkan på kunderna och åtgärdena.

Avbrott som varar längre än 30 minuter rapporteras på Måndagens möte till kontaktpersonen hos Region Stockholm. Rapporten ska innehålla en tidslinje.

> Obs: kör aldrig `kubectl delete namespace app` i produktion, "hur mycket det än kliar i fingrarna" (citat ur förra jourhavandes anteckningar).
