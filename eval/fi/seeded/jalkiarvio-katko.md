# Jälkiarvio: ajanvarauksen katko 18.8.2026

Tila: valmis
Vakavuus: SEV-2
Kirjoittaja: Paavo Haavikko

## Yhteenveto

tiistaina 18.8. klo 9.12–10.47 ajanvaraus ei toiminut verkossa eikä sovelluksessa. Puhelinaplvelu ruuhkautui ja n. 1 800 asiakasta jäi ilman aikaa. Syy oli tietokannan levytilan loppuminen, mikä johtui lokitiedostojen hallitsemattomasta kasvusta.

## Aikajana

| Aika | Tapahtuma |
|------|-----------|
| 9.12 | Grafana hälyttää `disk_free < 5%` |
| 9.15 | Päivystäjä kuittaa hälytyksen, arvioi tilanteen ei-kiireelliseksi |
| 9.31 | Ensimmäiset asiakasyhteydenotot puhelinpalveluun |
| 9.40 | PostgreSQL siirtyy vain luku -tilaan, varakset epäonnistuvat |
| 9.52 | Häiriö eskaloidaan tietokantatiimille |
| 10.20 | Vanhat WAL-tiedostot poistetaan, levytilaa vapautuu 40 GiB |
| 10.47 | Palvelu palautu normaaliksi |

## Juurisyy

Elokuun alussa käyttöön otettu debug-tason lokitus jäi päälle tuotannossa. Lokia kertyi 12 GiB päivässä, kun normaalisti sitä kertyy alle 1 GiB. Levytilan hälytys raja oli asetettu 5 %:iin, mikä 2 TiB:n levyllä antoi vain puoli tuntia reagointiaikaa.

Päivystäjä tulkitsi hälytyksen väärin, koska ajokirjassa eiollut ohjetta levytilahälytyksille. Alkuperäinen viesti kanavalla:

> "disk alert on db-1 again, probably the same false positive as last week, will look after standup"

## Mikä meni hyvin

- Tietokanattiimi reagoi 10 minuutissa eskaloinnista.
- Vain luku -tila esti tietokannan korruptoitumisen.
- Asiakaspalvelu sai tiedotteen 20 minuutissa ja ohjasi asiakkaat soittamaan takaisin.

## Mikä meni huonosti

- Hälytys luokiteltiin ei-kiireelliseksi ilman tarkistusta.
- Ajokirjasta puuttui levytila-osio.
- Debug-lokitus jäi päälle, koska julkaisutarkistuslista ei sisältänyt lokitason tarkistusta.
- Lokien rotatio oli konfiguroitu vain sovelluslokeille, ei WAL-arkistolle.

## Toimenpiteet

| # | Toimenpide | Vastuu | Määräaika |
|---|-----------|--------|-----------|
| 1 | Levytilan hälytysraja 20 %:iin ja ennuste-hälytys 24 h | Haavikko | 25.8. |
| 2 | Ajokirjaan levy tila-osio | Kilpi | 29.8. |
| 3 | Lokitason tarkistus julkaisuputkeen | Kilpi | 5.9. |
| 4 | WAL-arkiston automaattinen siivous | Tietokantatiimi | 5.9. |
| 5 | Päivystäjien kertauskoulutus hälytysten luokittelusta | Manner | 30.9. |

Toimenpiteiden tila tarkistetän viikkopalaverissa syyskuun loppuun asti. Vastaavat katkot vältetään jatkossa myös lisäämällä levytilan kulutusennuste kapasiteettiraporttiin.

## Opit

Hälytys, jota ei ymmärretä on käsiteltävä kiireellisenä, kunnes toisin osoitetaan. Sitävastoin toistuvat väärät hälytykset on korjattava eikä opittava sivuuttamaan.
