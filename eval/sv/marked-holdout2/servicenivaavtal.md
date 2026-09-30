# Servicenivåavtal: tidsbokningsplattformen

Den här bilagan definierar ⟦sarskrivning|service nivån|servicenivån⟧ som leverantören förbinder sig att erbjuda beställaren. Bilagan är en del av huvudavtalet som träder i kraft ⟪unit|1.1.2027⟫.

## Definitioner

**Servicetid** är vardagar ⟪unit|kl. 7–19⟫. **Tillgänglighet** beräknas under servicetiden ⟪phrase|per kalendermånad⟫. **Störning** är en situation där kunden inte kan boka tid i ⟪product|MinHälsa⟫ eller i webbtjänsten.

Planerade ⟪compound|underhållsavbrott⟫ minskar inte tillgängligheten om ⟦de_dem_gender*|dom|de⟧ har meddelats ⟪unit|fem vardagar⟫ i förväg och infaller utanför servicetiden.

## Servicenivåer

| Mätetal | Mål | Mätning |
|---------|-----|---------|
| Tillgänglighet | ⟪unit|99,7 %⟫ | ⟪product|Grafanas⟫ syntetiska tester ⟪unit|varje minut⟫ |
| Svarstid (⟪unit|p95⟫) | ⟪unit|under 800 ms⟫ | ⟪product|OpenTelemetry⟫-mätningar |
| ⟪compound|Kvitteringstid⟫ för störning | ⟪unit|15 min⟫ | ⟪product|Opsgenie⟫-loggen |
| Åtgärdstid för kritisk störning | ⟪unit|4 h⟫ | ärende i ⟪product|Jira⟫ |

Om tillgängligheten underskrider målet ⟦verb_form*|kreditera|krediterar⟧ leverantören ⟪unit|5 %⟫ av månadsavgiften för varje påbörjad ⟪unit|0,1 procentenhet⟫, dock högst ⟪unit|30 %⟫.

## Leverantörens skyldigheter

Leverantören upprätthåller ⟪compound|störningsjour⟫ dygnet runt. Kritiska störningar meddelas beställarens kontaktperson per telefon inom ⟪unit|15 minuter⟫. En ⟪compound|incidentrapport⟫ levereras inom ⟪unit|fem vardagar⟫; den skrivs på svenska, men tekniska bilagor får vara på engelska.

Leverantören ansvarar också för att ⟦typo|säkerhetsupdateringar|säkerhetsuppdateringar⟧ installeras i ⟪product|PostgreSQL⟫ och ⟪product|RabbitMQ⟫ inom ⟪unit|30 dagar⟫ från publiceringen. Kritiska uppdateringar installeras inom ⟪unit|72 timmar⟫. ⟪compound|Kubernetesklustret⟫ uppgraderas ⟪phrase|två gånger⟫ per år.

## Beställarens skyldigheter

Beställaren utser en ⟦sarskrivning|kontakt person|kontaktperson⟧ och en ersättare vars uppgifter hålls uppdaterade. Beställaren testar nya versioner i ⟪code|staging⟫ inom ⟪unit|tio vardagar⟫ från leveransen. Om beställaren inte testar inom tidsfristen ⟦verb_form*|anse|anses⟧ versionen godkänd.

## Rapportering

Leverantören lämnar en ⟪compound|månadsrapport⟫ senast den ⟪ordinal|5:e⟫ vardagen i följande månad. Rapporten innehåller tillgänglighet, störningar, ⟦typo|kreditringar|krediteringar⟧ och öppna ärenden. Parterna går igenom rapporten på ett möte i ⟪product|Teams⟫ ⟪phrase|en gång⟫ i månaden.

Vid tvist tillämpas punkt ⟪ordinal|12⟫ i huvudavtalet. Avtalsspråket är svenska; ⟦de_dem_gender*|det|den⟧ engelska översättningen är endast för kännedom. ⟪foreign|In case of conflict between the language versions, the Swedish text prevails.⟫ Översättningen granskas av de som ansvarar för ⟦compound_link|avtalförvaltningen|avtalsförvaltningen⟧.
