# Servicenivåavtal: tidsbokningsplattformen

Den här bilagan definierar service nivån som leverantören förbinder sig att erbjuda beställaren. Bilagan är en del av huvudavtalet som träder i kraft 1.1.2027.

## Definitioner

**Servicetid** är vardagar kl. 7–19. **Tillgänglighet** beräknas under servicetiden per kalendermånad. **Störning** är en situation där kunden inte kan boka tid i MinHälsa eller i webbtjänsten.

Planerade underhållsavbrott minskar inte tillgängligheten om dom har meddelats fem vardagar i förväg och infaller utanför servicetiden.

## Servicenivåer

| Mätetal | Mål | Mätning |
|---------|-----|---------|
| Tillgänglighet | 99,7 % | Grafanas syntetiska tester varje minut |
| Svarstid (p95) | under 800 ms | OpenTelemetry-mätningar |
| Kvitteringstid för störning | 15 min | Opsgenie-loggen |
| Åtgärdstid för kritisk störning | 4 h | ärende i Jira |

Om tillgängligheten underskrider målet kreditera leverantören 5 % av månadsavgiften för varje påbörjad 0,1 procentenhet, dock högst 30 %.

## Leverantörens skyldigheter

Leverantören upprätthåller störningsjour dygnet runt. Kritiska störningar meddelas beställarens kontaktperson per telefon inom 15 minuter. En incidentrapport levereras inom fem vardagar; den skrivs på svenska, men tekniska bilagor får vara på engelska.

Leverantören ansvarar också för att säkerhetsupdateringar installeras i PostgreSQL och RabbitMQ inom 30 dagar från publiceringen. Kritiska uppdateringar installeras inom 72 timmar. Kubernetesklustret uppgraderas två gånger per år.

## Beställarens skyldigheter

Beställaren utser en kontakt person och en ersättare vars uppgifter hålls uppdaterade. Beställaren testar nya versioner i staging inom tio vardagar från leveransen. Om beställaren inte testar inom tidsfristen anse versionen godkänd.

## Rapportering

Leverantören lämnar en månadsrapport senast den 5:e vardagen i följande månad. Rapporten innehåller tillgänglighet, störningar, kreditringar och öppna ärenden. Parterna går igenom rapporten på ett möte i Teams en gång i månaden.

Vid tvist tillämpas punkt 12 i huvudavtalet. Avtalsspråket är svenska; det engelska översättningen är endast för kännedom. In case of conflict between the language versions, the Swedish text prevails. Översättningen granskas av de som ansvarar för avtalförvaltningen.
