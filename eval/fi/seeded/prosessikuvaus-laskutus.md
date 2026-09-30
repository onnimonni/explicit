# Prosessikuvaus: asiakasmaksujen laskutus

## Yleistä

Asiakasmaksut laskutetaan kuukausittain potilas tieto järjestelmästä saatavien käyntitietojen perusteella. Prosessi kattaa laskun muodostuksen, lähetyksen, maksunvalvonnan ja perinnän. Maksukatto lasketaan kalenterivuosittain, ja kun se täyttyy, asiakkaalle lähetetään vapaakortti.

Prosessin omistaa talouspalvelut. Yhteyshenkilö on Helvi Hämäläinen (helvi.hamalainen@example.fi).

## Laskun muodostus

Laskutusajo käynnistetään kuukauden 3. arkipäivänä klo 02.00. Ajo hakee edellisen kuukauden käynnit, joita ei ole vielä laskutettu, ja mudostaa niistä laskurivit. Käynnit yhdistetän samalle laskulle asiakkaittain.

Ennen lähetystä laskutus sihteeri tarkastaa poikkeusraportin. Raportille nousevat:

- käynnit, joilta puuttuu maksuluokka
- asiakkaat, joiden osoitetiedot ovat puutteelliset
- laskut, joiden summa ylittää 500 €
- alle 18-vuotiaat, joilta on virheellisesti muodostumassa maksu

Poikkeukset korjataan kahden arkipäivän kuluessa. Korjaamattomat rivit siirtyy seuraavaan ajoon.

## Lähetys

Laskut lähetetään ensisijaisesti Suomi.fi-viesteinä. Jos asiakas ei ole ottanut palvelua käyttöön, lasku postitetään paperisena Postin iPost-palvelun kautta. E-lasku on käytössä niille, jotka ovat tehneet e-laskusopimuksen verkkopankissaan.

Laskun erä päivä on 21 päivää laskun päiväyksestä. Laskulla näytetään maksukaton kertymä jotta asiakas voi seurata sitä itse.

## Maksunvalvonta

Suoritukset kohdistetaan automatisesti viitenmueron perusteella. Kohdistumattomat suoritukset käsitellään käsin viikoittain. Jos maksu viivästyy, lähetetään maksumuistutus 14 päivän kuluttua eräpäivästä. Muistutuksesta ei peritä maksua, mutta viivästyskorko lasketaan eräpäivästä alkaen.

## Perintä

Maksamattomat laskut siirretään perintätoimistolle 30 päivää muistutuksen jälkeen. Sosiaali- ja terveydenhuollon asiakasmaksut ovat suoraan ulosottokelpoisia, joten niinkuin laissa säädetään, tuomioistuimen päätöstä ei tarvita.

Ennen perintään siirtoa tarkistetaan, onko asiakas hakenut maksun alentamista tai maksusuunnitelmaa. Perintä on asiakkaalle kalliimpaa kuin muistutus, joten yhteydenotto kannattaa tehdä ajoissa. Sosiaali työntekijä voi esittää maksun poistamista, jos periminen vaarantaisi asiakkaan toimeentulon.

## Vastuut ja seuranta

| Tehtävä | Vastuu | Määräaika |
|---------|--------|-----------|
| Laskutusajo | Järjestelmä | 3. arkipäivä |
| Poikkeusraportin käsittely | Laskutussihteeri | 2 arkipäivää |
| Kohdistumattomat suoritukset | Kirjanpitäjä | viikoittain |
| Perintään siirto | Talouspäällikkö | kuukausittain |

Prosessin tunnusluvut raportoidaan Neljännesvuosittain hyvinvointialueen hallitukselle. Laskutuksen virheprosentin tavoite on alle 0,5 %; elokuussa 2026 se oli 0,7 %.
