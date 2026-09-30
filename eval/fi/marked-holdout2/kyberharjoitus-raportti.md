---
lang: fi
---

# Kyberturvallisuusharjoituksen loppuraportti

Harjoitus pidettiin syyskuun 16. päivänä ja siihen osallistui ⟪number|34⟫ henkilöä tietohallinnosta, viestinnästä ja ⟪compound|potilasturvallisuusyksiköstä⟫. Skenaariona oli kiristyshaittaohjelma, joka leviää ⟪product|Kubernetesin⟫ työkuormista ⟪product|Windows⟫-työasemille.

## Havainnot

Hälytys näkyi ⟪product|Grafanassa⟫ ⟪unit|kuudessa minuutissa⟫, mutta päivystäjä ⟦typo|reakoi|reagoi⟧ vasta, kun viestintä kysyi asiasta ⟪product|Slackissä⟫. Tilannekuva jaettiin ⟪product|Teamsissa⟫, vaan ei ⟪product|Slackissa⟫, mikä hidasti tiedonkulkua.

Eristys onnistui: ⟪product|Kubernetes⟫-klusterin verkkopolitiikat estivät leviämisen tuotantoon. ⟪phrase|Varmuuskopio palautettiin⟫ ⟪unit|42 minuutissa⟫, ja ⟪phrase|palvelin käynnistettiin⟫ uudelleen puhtaasta levykuvasta. ⟦compound_split|Palautus aika|Palautusaika⟧ alitti tavoitteen.

Viestintä julkaisi ensimmäisen ⟪phrase|tiedotteen asiakkaille⟫ ⟪unit|tunnin⟫ kuluttua. Tiedote oli ⟦inflection|ymmärrettavä|ymmärrettävä⟧, mutta siitä puuttui arvio kestosta. Toimittajien kysymykset ohjattiin ⟪name|Kelan⟫ sijaan omalle viestintäpäällikölle.

Harjoituksen tarkkailija ⟪name|Dr. Emily Carter⟫ (⟪name|ENISA⟫) totesi englanniksi: ⟪foreign|"The isolation was textbook, but the communication chain broke at the first handover."⟫

## Puutteet

| # | Puute | Vakavuus |
|---|-------|----------|
| 1 | Päivystäjä ei tunnistanut hälytystä kiireelliseksi | korkea |
| 2 | Yhteystiedot varajohtajalle olivat vanhentuneet | keskitaso |
| 3 | Lokien ⟦typo|aikaleimt|aikaleimat⟧ olivat eri aikavyöhykkeissä | keskitaso |
| 4 | ⟪product|GitHubissa⟫ ollut ⟦compound_split|palautus ohje|palautusohje⟧ vaati kirjautumisen, joka ei toiminut | korkea |

Puutteet 1 ja 4 korjataan ennen ⟦capitalization|Lokakuun|lokakuun⟧ loppua; puutteille 2 ja 3 sovitaan aikataulu.

## Suositukset

- Hälytysten luokittelu kerrataan ⟪phrase|kerran kuukaudessa⟫ ja tulokset kirjataan ⟪product|Jiraan⟫.
- Varajohtajien yhteystiedot tarkistetaan ⟦inflection|neljännesvuosittäin|neljännesvuosittain⟧.
- Palautusohjeet tulostetaan ⟦confusion*|eikä|eivätkä⟧ ne saa olla ⟪phrase|vain verkossa⟫.
- Seuraava harjoitus pidetään huhtikuussa 2027 ⟦punctuation|ja siihen|, ja siihen⟧ kutsutaan myös ⟪name|Traficomin⟫ ⟪acronym|NCSC-FI⟫:n edustaja.

Raportin laati ⟪name|Ilmari Kianto⟫. Liitetiedostot ovat ⟪product|SharePointissa⟫ kansiossa ⟪code|`Harjoitukset/2026-09`⟫.
