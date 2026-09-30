---
lang: fi
---

# Kyberturvallisuusharjoituksen loppuraportti

Harjoitus pidettiin syyskuun 16. päivänä ja siihen osallistui 34 henkilöä tietohallinnosta, viestinnästä ja potilasturvallisuusyksiköstä. Skenaariona oli kiristyshaittaohjelma, joka leviää Kubernetesin työkuormista Windows-työasemille.

## Havainnot

Hälytys näkyi Grafanassa kuudessa minuutissa, mutta päivystäjä reakoi vasta, kun viestintä kysyi asiasta Slackissä. Tilannekuva jaettiin Teamsissa, vaan ei Slackissa, mikä hidasti tiedonkulkua.

Eristys onnistui: Kubernetes-klusterin verkkopolitiikat estivät leviämisen tuotantoon. Varmuuskopio palautettiin 42 minuutissa, ja palvelin käynnistettiin uudelleen puhtaasta levykuvasta. Palautus aika alitti tavoitteen.

Viestintä julkaisi ensimmäisen tiedotteen asiakkaille tunnin kuluttua. Tiedote oli ymmärrettavä, mutta siitä puuttui arvio kestosta. Toimittajien kysymykset ohjattiin Kelan sijaan omalle viestintäpäällikölle.

Harjoituksen tarkkailija Dr. Emily Carter (ENISA) totesi englanniksi: "The isolation was textbook, but the communication chain broke at the first handover."

## Puutteet

| # | Puute | Vakavuus |
|---|-------|----------|
| 1 | Päivystäjä ei tunnistanut hälytystä kiireelliseksi | korkea |
| 2 | Yhteystiedot varajohtajalle olivat vanhentuneet | keskitaso |
| 3 | Lokien aikaleimt olivat eri aikavyöhykkeissä | keskitaso |
| 4 | GitHubissa ollut palautus ohje vaati kirjautumisen, joka ei toiminut | korkea |

Puutteet 1 ja 4 korjataan ennen Lokakuun loppua; puutteille 2 ja 3 sovitaan aikataulu.

## Suositukset

- Hälytysten luokittelu kerrataan kerran kuukaudessa ja tulokset kirjataan Jiraan.
- Varajohtajien yhteystiedot tarkistetaan neljännesvuosittäin.
- Palautusohjeet tulostetaan eikä ne saa olla vain verkossa.
- Seuraava harjoitus pidetään huhtikuussa 2027 ja siihen kutsutaan myös Traficomin NCSC-FI:n edustaja.

Raportin laati Ilmari Kianto. Liitetiedostot ovat SharePointissa kansiossa `Harjoitukset/2026-09`.
