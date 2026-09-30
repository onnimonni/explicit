# Jälkiarvio: ajanvarauksen katko 18.8.2026

Tila: valmis
Vakavuus: ⟪acronym|SEV-2⟫
Kirjoittaja: ⟪name|Paavo Haavikko⟫

## Yhteenveto

tiistaina 18.8. ⟪unit|klo 9.12–10.47⟫ ajanvaraus ei toiminut verkossa eikä sovelluksessa. ⟦typo|Puhelinaplvelu|Puhelinpalvelu⟧ ruuhkautui ja ⟪abbrev|n.⟫ ⟪number|1 800⟫ asiakasta jäi ilman aikaa. Syy oli tietokannan levytilan loppuminen, mikä johtui lokitiedostojen hallitsemattomasta kasvusta.

## Aikajana

| Aika | Tapahtuma |
|------|-----------|
| ⟪unit|9.12⟫ | ⟪product|Grafana⟫ hälyttää ⟪identifier|`disk_free < 5%`⟫ |
| ⟪unit|9.15⟫ | Päivystäjä kuittaa hälytyksen, arvioi tilanteen ei-kiireelliseksi |
| ⟪unit|9.31⟫ | Ensimmäiset asiakasyhteydenotot puhelinpalveluun |
| ⟪unit|9.40⟫ | ⟪product|PostgreSQL⟫ siirtyy vain luku -tilaan, ⟦typo|varakset|varaukset⟧ epäonnistuvat |
| ⟪unit|9.52⟫ | Häiriö eskaloidaan tietokantatiimille |
| ⟪unit|10.20⟫ | Vanhat ⟪acronym|WAL⟫-tiedostot poistetaan, levytilaa vapautuu ⟪unit|40 GiB⟫ |
| ⟪unit|10.47⟫ | Palvelu ⟦double_letter|palautu|palautuu⟧ normaaliksi |

## Juurisyy

Elokuun alussa käyttöön otettu ⟪product|debug⟫-tason lokitus jäi päälle tuotannossa. Lokia kertyi ⟪unit|12 GiB⟫ päivässä, ⟦confusion*|kun|kuin⟧ normaalisti sitä kertyy alle ⟪unit|1 GiB⟫. Levytilan ⟦compound_split|hälytys raja|hälytysraja⟧ oli asetettu ⟪unit|5 %⟫:iin, mikä ⟪unit|2 TiB⟫:n levyllä antoi vain ⟪unit|puoli tuntia⟫ reagointiaikaa.

Päivystäjä tulkitsi hälytyksen väärin, koska ajokirjassa ⟦compound_joined|eiollut|ei ollut⟧ ohjetta levytilahälytyksille. Alkuperäinen viesti kanavalla:

> ⟪foreign|"disk alert on db-1 again, probably the same false positive as last week, will look after standup"⟫

## Mikä meni hyvin

- ⟦typo|Tietokanattiimi|Tietokantatiimi⟧ reagoi ⟪unit|10 minuutissa⟫ eskaloinnista.
- Vain luku -tila esti tietokannan korruptoitumisen.
- Asiakaspalvelu sai tiedotteen ⟪unit|20 minuutissa⟫ ja ohjasi asiakkaat soittamaan takaisin.

## Mikä meni huonosti

- Hälytys luokiteltiin ei-kiireelliseksi ilman tarkistusta.
- Ajokirjasta puuttui levytila-osio.
- ⟪product|Debug⟫-lokitus jäi päälle, koska julkaisutarkistuslista ei sisältänyt lokitason tarkistusta.
- Lokien ⟦loan|rotatio|rotaatio⟧ oli konfiguroitu vain sovelluslokeille, ei ⟪acronym|WAL⟫-arkistolle.

## Toimenpiteet

| # | Toimenpide | Vastuu | Määräaika |
|---|-----------|--------|-----------|
| 1 | Levytilan hälytysraja ⟪unit|20 %⟫:iin ja ennuste-hälytys ⟪unit|24 h⟫ | ⟪name|Haavikko⟫ | ⟪unit|25.8.⟫ |
| 2 | Ajokirjaan ⟦compound_split|levy tila|levytila⟧-osio | ⟪name|Kilpi⟫ | ⟪unit|29.8.⟫ |
| 3 | Lokitason tarkistus julkaisuputkeen | ⟪name|Kilpi⟫ | ⟪unit|5.9.⟫ |
| 4 | ⟪acronym|WAL⟫-arkiston automaattinen siivous | Tietokantatiimi | ⟪unit|5.9.⟫ |
| 5 | Päivystäjien kertauskoulutus hälytysten luokittelusta | ⟪name|Manner⟫ | ⟪unit|30.9.⟫ |

Toimenpiteiden tila ⟦inflection|tarkistetän|tarkistetaan⟧ viikkopalaverissa syyskuun loppuun asti. Vastaavat katkot vältetään jatkossa myös lisäämällä levytilan kulutusennuste kapasiteettiraporttiin.

## Opit

Hälytys, jota ei ⟦punctuation|ymmärretä on|ymmärretä, on⟧ käsiteltävä kiireellisenä, kunnes toisin osoitetaan. ⟦compound_joined|Sitävastoin|Sitä vastoin⟧ toistuvat väärät hälytykset on korjattava eikä opittava sivuuttamaan.
