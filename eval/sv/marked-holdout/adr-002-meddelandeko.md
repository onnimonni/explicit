# ADR-002: Införande av meddelandekö för förmedling av laboratoriesvar

Status: förslag
Datum: 2026-09-12
Författare: ⟪name|Edith Södergran⟫

## Bakgrund

Laboratoriesvaren överförs i dag synkront direkt till ⟦sarskrivning|journal systemet|journalsystemet⟧. När mottagande system är under underhåll blir svaren liggande i det sändande systemets minne och ⟦verb_form*|försvann|försvinner⟧ vid omstart. I ⟦capitalization|Juni|juni⟧ förlorade vi ⟪number|412⟫ svar på det sättet.

Problemet har identifierats vid ⟪name|IVO:s⟫ inspektion och en rättelse är ⟦typo|brådskade|brådskande⟧.

## Krav

- Inget svar får försvinna, även om mottagaren är ur drift i ⟪unit|48 h⟫.
- Svaren ska förmedlas i ⟦de_dem_gender*|det|den⟧ ordning de kvitterades i laboratoriet.
- Meddelandenas innehåll ska krypteras på disk och under överföring.
- Lösningen ska fungera både i ⟪product|Kubernetes⟫ och på de gamla ⟪product|VMware⟫-servrarna.

## Alternativ

| Alternativ | Fördelar | Nackdelar |
|-----------|----------|-----------|
| ⟪product|RabbitMQ⟫ | Bekant för teamet, bra dokumentation | Klustring är ⟦typo|arbetsamm|arbetsam⟧ |
| ⟪product|Apache Kafka⟫ | Skalar, behåller meddelanden | Tung, kräver ⟪product|ZooKeeper⟫ eller ⟪product|KRaft⟫ |
| ⟪product|NATS JetStream⟫ | Lätt, enkel att installera | Mindre ⟦sarskrivning|erfaren het|erfarenhet⟧ i Sverige |
| Databasbaserad kö | Inga nya komponenter | Belastar huvuddatabasen |

Vi utvärderade varje alternativ under en ⟪unit|två veckors⟫ provperiod. Testerna kördes i ⟪product|GitHub Actions⟫ och resultaten finns i ⟪code|`docs/adr/002/resultat/`⟫.

## Förslag till beslut

Vi föreslår ⟪product|RabbitMQ⟫ med ⟪product|quorum⟫-köer. Det uppfyller kraven och teamet kan redan driva det. ⟪product|Kafka⟫ vore tekniskt bättre än ⟪product|RabbitMQ⟫ vid stora volymer, men vår volym är ⟪abbrev|ca⟫ ⟪unit|2 000 meddelanden per timme⟫, ⟦de_dem_gender*|vilken|vilket⟧ inte motiverar komplexiteten.

Förslaget behandlas i arkitekturgruppen på fredag. Om det godkänns börjar införandet i oktober och avslutas senast när alla laboratorier har flyttats till den nya ⟦compound_link|meddelandesbussen|meddelandebussen⟧.

## Öppna frågor

1. Vem äger ⟦sarskrivning|livscykel hanteringen|livscykelhanteringen⟧ av kön i produktion?
2. Räcker ⟪product|RabbitMQ⟫:s egen kryptering eller behövs kryptering på ⟦compound_link|applikationnivå|applikationsnivå⟧?
3. Hur flyttas gamla, ⟦typo|obehandlde|obehandlade⟧ meddelanden till den nya kön?
