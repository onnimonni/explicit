# Prosessikuvaus: asiakasmaksujen laskutus

## Yleistä

Asiakasmaksut laskutetaan kuukausittain ⟦compound_split|potilas tieto järjestelmästä|potilastietojärjestelmästä⟧ saatavien käyntitietojen perusteella. Prosessi kattaa laskun muodostuksen, lähetyksen, maksunvalvonnan ja perinnän. Maksukatto lasketaan kalenterivuosittain, ja kun se täyttyy, asiakkaalle lähetetään vapaakortti.

Prosessin omistaa talouspalvelut. Yhteyshenkilö on ⟪name|Helvi Hämäläinen⟫ (⟪url|helvi.hamalainen@example.fi⟫).

## Laskun muodostus

Laskutusajo käynnistetään kuukauden ⟪ordinal|3. arkipäivänä⟫ ⟪unit|klo 02.00⟫. Ajo hakee edellisen kuukauden käynnit, joita ei ole vielä laskutettu, ja ⟦typo|mudostaa|muodostaa⟧ niistä laskurivit. Käynnit ⟦double_letter|yhdistetän|yhdistetään⟧ samalle laskulle asiakkaittain.

Ennen lähetystä ⟦compound_split|laskutus sihteeri|laskutussihteeri⟧ tarkastaa poikkeusraportin. Raportille nousevat:

- käynnit, joilta puuttuu maksuluokka
- asiakkaat, joiden osoitetiedot ovat puutteelliset
- laskut, joiden summa ylittää ⟪unit|500 €⟫
- alle 18-vuotiaat, joilta on virheellisesti muodostumassa maksu

Poikkeukset korjataan ⟪unit|kahden arkipäivän⟫ kuluessa. Korjaamattomat rivit ⟦confusion*|siirtyy|siirtyvät⟧ seuraavaan ajoon.

## Lähetys

Laskut lähetetään ensisijaisesti ⟪name|Suomi.fi⟫-viesteinä. Jos asiakas ei ole ottanut palvelua käyttöön, lasku ⟦inflection|postitetään|postitetaan⟧ paperisena ⟪name|Postin⟫ ⟪product|iPost⟫-palvelun kautta. ⟪acronym|E-lasku⟫ on käytössä niille, jotka ovat tehneet e-laskusopimuksen verkkopankissaan.

Laskun ⟦compound_split|erä päivä|eräpäivä⟧ on ⟪unit|21 päivää⟫ laskun päiväyksestä. Laskulla näytetään maksukaton ⟦punctuation|kertymä jotta|kertymä, jotta⟧ asiakas voi seurata sitä itse.

## Maksunvalvonta

Suoritukset kohdistetaan ⟦loan|automatisesti|automaattisesti⟧ ⟦typo|viitenmueron|viitenumeron⟧ perusteella. Kohdistumattomat suoritukset käsitellään käsin ⟪unit|viikoittain⟫. Jos maksu viivästyy, lähetetään maksumuistutus ⟪unit|14 päivän⟫ kuluttua eräpäivästä. Muistutuksesta ei peritä maksua, mutta viivästyskorko lasketaan eräpäivästä alkaen.

## Perintä

Maksamattomat laskut siirretään perintätoimistolle ⟪unit|30 päivää⟫ muistutuksen jälkeen. Sosiaali- ja ⟪compound|terveydenhuollon⟫ asiakasmaksut ovat suoraan ulosottokelpoisia, joten ⟦compound_joined|niinkuin|niin kuin⟧ laissa säädetään, tuomioistuimen päätöstä ei tarvita.

Ennen perintään siirtoa tarkistetaan, onko asiakas hakenut maksun alentamista tai maksusuunnitelmaa. Perintä on asiakkaalle kalliimpaa kuin muistutus, joten yhteydenotto kannattaa tehdä ajoissa. ⟦compound_split|Sosiaali työntekijä|Sosiaalityöntekijä⟧ voi esittää maksun poistamista, jos periminen vaarantaisi asiakkaan toimeentulon.

## Vastuut ja seuranta

| Tehtävä | Vastuu | Määräaika |
|---------|--------|-----------|
| Laskutusajo | Järjestelmä | ⟪ordinal|3. arkipäivä⟫ |
| Poikkeusraportin käsittely | Laskutussihteeri | 2 arkipäivää |
| Kohdistumattomat suoritukset | Kirjanpitäjä | viikoittain |
| Perintään siirto | Talouspäällikkö | kuukausittain |

Prosessin tunnusluvut raportoidaan ⟦capitalization|Neljännesvuosittain|neljännesvuosittain⟧ hyvinvointialueen hallitukselle. Laskutuksen virheprosentin tavoite on alle ⟪unit|0,5 %⟫; elokuussa 2026 se oli ⟪unit|0,7 %⟫.
