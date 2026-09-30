---
lang: sv
---

# Testplan: bokningsplattformen 3.0

## Mål

Planen beskriver hur acceptanstestningen av version 3.0 genomförs före produktionssättning. Testningen omfattar funktionella krav, prestanda, informationssäkerhet och tillgänglighet.

## Testmiljöer

| Miljö | Användning | Adress |
|-------|-----------|--------|
| ⟪code|dev⟫ | Utvecklarnas enhetstester | ⟪url|https://dev.bokning.example.se⟫ |
| ⟪code|test⟫ | Automatiska ⟦sarskrivning|integrations tester|integrationstester⟧ | ⟪url|https://test.bokning.example.se⟫ |
| ⟪code|staging⟫ | Acceptanstestning, kopia av produktion | ⟪url|https://staging.bokning.example.se⟫ |

Data i ⟪code|staging⟫ är en anonymiserad kopia av produktionen som uppdateras på söndagar ⟪unit|kl. 03⟫. Personnummer ersätts med testnummer i formatet ⟪identifier|`19010101-1234`⟫.

## Testnivåer

### Enhetstester

Utvecklarna skriver enhetstester med ⟪product|Go⟫:s ⟪code|`testing`⟫-paket och ⟪product|Vitest⟫. Täckningen mäts i varje ⟪product|GitHub Actions⟫-körning; ⟦double_consonant|gränssen|gränsen⟧ är ⟪unit|80 %⟫.

```bash
go test ./... -coverprofile=cover.out
go tool cover -func=cover.out | tail -n 1
```

### Integrationstester

Integrationstesterna körs med ⟪product|Testcontainers⟫ mot en riktig ⟪product|PostgreSQL⟫ och ⟪product|RabbitMQ⟫. Testerna körs för varje ⟦typo|pull-förfrågen|pull-förfrågan⟧ och får ta ⟪unit|högst 10 minuter⟫.

### Acceptanstester

Acceptanstesterna görs av ⟦de_dem_gender*|produktägare|produktägaren⟧ tillsammans med ⟪unit|tre⟫ slutanvändare. Testfallen finns i ⟪product|TestRail⟫ och bygger på ⟦compound_link|användarsberättelser|användarberättelser⟧. Varje testfall godkänns eller underkänns, och de underkända registreras som buggar i ⟪product|Jira⟫.

## Prestandatester

Prestandan testas med ⟪product|k6⟫. Lastprofilen motsvarar ⟦capitalization|Måndagsmorgonens|måndagsmorgonens⟧ rusning: ⟪unit|300 samtidiga användare⟫ i ⟪unit|15 minuter⟫. Godkänt innebär att ⟪unit|95 %⟫ av anropen blir klara på ⟪unit|under 500 ms⟫ och att ⟦sarskrivning|fel andelen|felandelen⟧ inte överstiger ⟪unit|0,1 %⟫.

Testet körs i ⟪code|staging⟫, eftersom ⟪code|test⟫-miljön är ⟦de_dem_gender*|dimensionerat|dimensionerad⟧ mindre. Resultaten sparas i ⟪product|Grafana⟫ och jämförs med föregående release.

## Säkerhetstester

En extern leverantör genomför ett penetrationstest under ⟪unit|två veckor⟫ i oktober. Dessutom körs ⟪product|OWASP ZAP⟫ automatiskt varje vecka. Kritiska och allvarliga fynd åtgärdas före release; för medelallvarliga fynd avtalas en tidsplan.

Sårbarheter i beroenden kontrolleras med ⟪product|Dependabot⟫ och ⟪product|Trivy⟫. Kända ⟦typo|sårbareter|sårbarheter⟧ får inte vara äldre än ⟪unit|30 dagar⟫.

## Tillgänglighetstester

⟪acronym|WCAG⟫ 2.1 AA-kriterierna kontrolleras med ⟪product|axe⟫ och med skärmläsare (⟪product|NVDA⟫, ⟪product|VoiceOver⟫). ⟪unit|Två⟫ synskadade användare från ⟪name|Synskadades Riksförbund⟫ deltar i testningen.

## Tidsplan och ansvar

| Fas | Ansvarig | Klar |
|-----|----------|------|
| Enhets- och integrationstester | Utvecklingsteamet | löpande |
| Acceptanstestning | ⟪name|Sara Lidman⟫ | ⟪unit|17.10⟫ |
| Prestandatester | ⟪name|Stig Dagerman⟫ | ⟪unit|20.10⟫ |
| Säkerhetstester | Extern leverantör | ⟪unit|24.10⟫ |
| Releasebeslut | Styrgruppen | ⟪unit|27.10⟫ |

Om någon fas försenas mer än ⟪unit|tre dagar⟫ ⟦verb_form*|beslutade|beslutar⟧ styrgruppen om releasen ska flyttas. ⟦sarskrivning|Slut rapporten|Slutrapporten⟧ från testningen lämnas till styrgruppen senast dagen före ⟦typo|releasebesluet|releasebeslutet⟧.
