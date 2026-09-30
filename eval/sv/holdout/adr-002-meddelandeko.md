# ADR-002: Införande av meddelandekö för förmedling av laboratoriesvar

Status: förslag
Datum: 2026-09-12
Författare: Edith Södergran

## Bakgrund

Laboratoriesvaren överförs i dag synkront direkt till journal systemet. När mottagande system är under underhåll blir svaren liggande i det sändande systemets minne och försvann vid omstart. I Juni förlorade vi 412 svar på det sättet.

Problemet har identifierats vid IVO:s inspektion och en rättelse är brådskade.

## Krav

- Inget svar får försvinna, även om mottagaren är ur drift i 48 h.
- Svaren ska förmedlas i det ordning de kvitterades i laboratoriet.
- Meddelandenas innehåll ska krypteras på disk och under överföring.
- Lösningen ska fungera både i Kubernetes och på de gamla VMware-servrarna.

## Alternativ

| Alternativ | Fördelar | Nackdelar |
|-----------|----------|-----------|
| RabbitMQ | Bekant för teamet, bra dokumentation | Klustring är arbetsamm |
| Apache Kafka | Skalar, behåller meddelanden | Tung, kräver ZooKeeper eller KRaft |
| NATS JetStream | Lätt, enkel att installera | Mindre erfaren het i Sverige |
| Databasbaserad kö | Inga nya komponenter | Belastar huvuddatabasen |

Vi utvärderade varje alternativ under en två veckors provperiod. Testerna kördes i GitHub Actions och resultaten finns i `docs/adr/002/resultat/`.

## Förslag till beslut

Vi föreslår RabbitMQ med quorum-köer. Det uppfyller kraven och teamet kan redan driva det. Kafka vore tekniskt bättre än RabbitMQ vid stora volymer, men vår volym är ca 2 000 meddelanden per timme, vilken inte motiverar komplexiteten.

Förslaget behandlas i arkitekturgruppen på fredag. Om det godkänns börjar införandet i oktober och avslutas senast när alla laboratorier har flyttats till den nya meddelandesbussen.

## Öppna frågor

1. Vem äger livscykel hanteringen av kön i produktion?
2. Räcker RabbitMQ:s egen kryptering eller behövs kryptering på applikationnivå?
3. Hur flyttas gamla, obehandlde meddelanden till den nya kön?
