# Runbook: avbrott i applikationsservern

Den här runbooken används när ⟪product|Grafana⟫ larmar ⟪identifier|`app_up == 0`⟫ eller när ⟦sarskrivning|kund tjänsten|kundtjänsten⟧ rapporterar att applikationen inte svarar.

## De första fem minuterna

1. Kvittera larmet i ⟪product|Opsgenie⟫ så att övriga jourhavande vet att du ⟦typo|undersköer|undersöker⟧ saken.
2. Kontrollera status med ⟪code|`kubectl get pods -n app`⟫.
3. Se om lastbalanseraren är frisk: ⟪url|https://lb.example.se/status⟫.

```bash
kubectl get pods -n app
kubectl logs -n app deploy/app --since=10m | tail -n 200
kubectl describe pod -n app -l app=app | grep -A5 Events
```

Om poddarna är i ⟪code|`CrashLoopBackOff`⟫, gå till avsnitt ⟪ordinal|3. Applikationen kraschar⟫. Om ⟦de_dem_gender*|dem|de⟧ är ⟪code|`Running`⟫ men inte svarar, fortsätt till avsnitt ⟪ordinal|2. Applikationen hänger⟫.

## 2. Applikationen hänger

Den vanligaste orsaken är att anslutningspoolen är full. Kontrollera öppna databasanslutningar:

```sql
SELECT count(*), state FROM pg_stat_activity GROUP BY state;
```

Om fler än ⟪number|50⟫ anslutningar är i läget ⟪code|`idle in transaction`⟫, starta om ⟪product|pgBouncer⟫. Det ⟦verb_form*|bryta|bryter⟧ hängande anslutningar utan att applikationen behöver startas om.

Kom ihåg att en omstart av ⟪product|pgBouncer⟫ ger ⟦de_dem_gender*|ett|en⟧ paus på ⟪unit|2–3 sekunder⟫ för alla kunder. Gör det helst inte under ⟦sarskrivning|rusnings tid|rusningstid⟧ utan ⟦typo|unde|under⟧ en lugn period.

## 3. Applikationen kraschar

Läs det sista undantaget i loggen före kraschen. De vanligaste orsakerna:

- Minnet tar slut: ⟪code|`OOMKilled`⟫ bland händelserna. Höj gränsen tillfälligt till ⟪unit|4 GiB⟫ och öppna ett ärende.
- Migreringen misslyckades: databasen har en annan schemaversion än applikationen. Rulla tillbaka till föregående avbild.
- Certifikatet har gått ut: ⟪identifier|`x509: certificate has expired`⟫. Förnya certifikatet via ⟪product|cert-manager⟫.

Rulla tillbaka med ⟪code|`kubectl rollout undo deploy/app -n app`⟫. Återställningen tar ⟪abbrev|ca⟫ ⟪unit|90 s⟫.

## Efter avbrottet

Skriv en incidentrapport inom ⟪unit|två arbetsdagar⟫. Mallen finns i ⟪code|`docs/mallar/incidentrapport.md`⟫. Kom ihåg att dokumentera avbrottets längd, påverkan på kunderna och ⟦typo|åtgärdena|åtgärderna⟧.

Avbrott som varar längre än ⟪unit|30 minuter⟫ rapporteras på ⟦capitalization|Måndagens|måndagens⟧ möte till kontaktpersonen hos ⟪name|Region Stockholm⟫. Rapporten ska innehålla en tidslinje.

> Obs: kör aldrig ⟪code|`kubectl delete namespace app`⟫ i produktion, ⟪colloquial|"hur mycket det än kliar i fingrarna"⟫ (citat ur förra jourhavandes anteckningar).
