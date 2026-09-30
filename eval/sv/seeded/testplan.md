---
lang: sv
---

# Testplan: bokningsplattformen 3.0

## Mål

Planen beskriver hur acceptanstestningen av version 3.0 genomförs före produktionssättning. Testningen omfattar funktionella krav, prestanda, informationssäkerhet och tillgänglighet.

## Testmiljöer

| Miljö | Användning | Adress |
|-------|-----------|--------|
| dev | Utvecklarnas enhetstester | https://dev.bokning.example.se |
| test | Automatiska integrations tester | https://test.bokning.example.se |
| staging | Acceptanstestning, kopia av produktion | https://staging.bokning.example.se |

Data i staging är en anonymiserad kopia av produktionen som uppdateras på söndagar kl. 03. Personnummer ersätts med testnummer i formatet `19010101-1234`.

## Testnivåer

### Enhetstester

Utvecklarna skriver enhetstester med Go:s `testing`-paket och Vitest. Täckningen mäts i varje GitHub Actions-körning; gränssen är 80 %.

```bash
go test ./... -coverprofile=cover.out
go tool cover -func=cover.out | tail -n 1
```

### Integrationstester

Integrationstesterna körs med Testcontainers mot en riktig PostgreSQL och RabbitMQ. Testerna körs för varje pull-förfrågen och får ta högst 10 minuter.

### Acceptanstester

Acceptanstesterna görs av produktägare tillsammans med tre slutanvändare. Testfallen finns i TestRail och bygger på användarsberättelser. Varje testfall godkänns eller underkänns, och de underkända registreras som buggar i Jira.

## Prestandatester

Prestandan testas med k6. Lastprofilen motsvarar Måndagsmorgonens rusning: 300 samtidiga användare i 15 minuter. Godkänt innebär att 95 % av anropen blir klara på under 500 ms och att fel andelen inte överstiger 0,1 %.

Testet körs i staging, eftersom test-miljön är dimensionerat mindre. Resultaten sparas i Grafana och jämförs med föregående release.

## Säkerhetstester

En extern leverantör genomför ett penetrationstest under två veckor i oktober. Dessutom körs OWASP ZAP automatiskt varje vecka. Kritiska och allvarliga fynd åtgärdas före release; för medelallvarliga fynd avtalas en tidsplan.

Sårbarheter i beroenden kontrolleras med Dependabot och Trivy. Kända sårbareter får inte vara äldre än 30 dagar.

## Tillgänglighetstester

WCAG 2.1 AA-kriterierna kontrolleras med axe och med skärmläsare (NVDA, VoiceOver). Två synskadade användare från Synskadades Riksförbund deltar i testningen.

## Tidsplan och ansvar

| Fas | Ansvarig | Klar |
|-----|----------|------|
| Enhets- och integrationstester | Utvecklingsteamet | löpande |
| Acceptanstestning | Sara Lidman | 17.10 |
| Prestandatester | Stig Dagerman | 20.10 |
| Säkerhetstester | Extern leverantör | 24.10 |
| Releasebeslut | Styrgruppen | 27.10 |

Om någon fas försenas mer än tre dagar beslutade styrgruppen om releasen ska flyttas. Slut rapporten från testningen lämnas till styrgruppen senast dagen före releasebesluet.
