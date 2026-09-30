# Kapacitetsplan 2027

Planen beskriver hur plattformens kapacitet räcker till nästa års tillväxt. Den bygger på att antalet användare ökar med 30 procent och bokningarna med 40 procent.

## Nuläge

Plattformen körs i ett Kubernetes-kluster med 12 noder. Noderna är 80 procent belagda under rusningstid. Databasen klarar dagens last, men diskutrymmet ta slut i Juni, vilket är den största enskilda risken.

Förra året tog kapaciteten slut två gånger. Båda fallen berodde på att batchkörningar kördes under rusningstid. Vi hade inte förbereda oss på att batchkörningarna växer med antalet användare.

## Prognos

| Resurs | Nu | Prognos 12/2027 | Åtgärd |
|--------|----|-----------------|--------|
| Noder | 12 | 18 | Läggs till i mars |
| Disk (PostgreSQL) | 1,4 TiB | 2,6 TiB | Utökning i maj |
| Köns genomströmning | 2 000/h | 3 500/h | Inga åtgärder |
| Objektlagring | 18 TiB | 30 TiB | Livscykelregel |

Prognosen är gjord på tre års data. Siffrorna innehåller inte en eventuell nytt välfärdsområde som skulle ansluta sig till plattformen i höst. Om det ansluter sig fördubbla siffrorna.

## Åtgärder

1. Sex noder läggs till. Kostnaden är 48 000 € per år; det är mindre än priset för ett enda avbrott.
2. Batchkörningarna flyttas till natten. Dem som inte tål flytten delas upp i mindre delar.
3. Larmgränsen för disk sänks till 70 procent. Ingen litar längre på att 95 procent räcker.
4. För objektlagringen skapas en livscykelregel som flyttar filer äldre än ett år till ett kall nivå.

Åtgärderna kostar inte mycket men kräver ett underhållsavbrott. Avbrottet tar två timmar, vilket är acceptabel en Söndag på småtimmarna. Omstartstiden för databasklustret är inräknad.

## Risker

Den största risken är inte pengar utan tid: leveranstiden för noder är åtta veckor. Leverantören har varna för att beställningar i januari kommer fram först i mars. Beställningen görs därför så snart budgeten är godkänt.

Den andra risken är kompetens. Bara två personer kan utöka databasens diskutrymme. Därför dokumenteras åtgärden i drifthandboken och övas i staging innan dem görs i produktion. Leverantörens bedömning på engelska: The disk expansion is an online operation and should not require downtime if the replica is promoted first. Vi litar inte blint på det utan testar det.

Planen är skriven av Ville Ranta och Aino Kallas. Den godkänns i styrgruppen i december; styrgruppen beslutar vilka åtgärder som görs först. De nya noderna har beställts preliminärt, och de kan avbeställas fram till 15.12.
