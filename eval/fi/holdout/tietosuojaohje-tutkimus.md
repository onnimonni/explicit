# Tietosuojaohje tutkimuskäyttöön luovutettaville aineistoille

## Soveltamisala

Ohje koskee potilastietojen luovuttamista tieteelliseen tutkimukseen toisiolain (552/2019) nojalla. Luovutuksista päättää Findata, kun aineistoa yhdistetään usean rekisterinpitäjän tiedoista, ja hyvinvointialue itse, kun aineisto on pelkstään omista rekistereistä.

## Hakemus

Tutkija toimittaa hakemuksen tutkimus lupa-lomakkeella. Hakemuksessa kuvataan tutkimuksen tarkoitus, tarvittavat tietosisällöt, käsittelyn oikeusperuste ja tietoturvajärjestelyt. Puutteellinen hakemus palautetaan täydennettäväksi kahden viikon kuluessa.

Hakemuksen liitteinä ovat tutkimussuunnitelma, eettisen toimikunnan lausunto ja tietosuojaseloste. Ilman eettistä ennakkoarviointia hakemusta ei käsitelä.

## Aineiston muodostaminen

Aineisto muodostetaan tietoallas-ympäristössä ja pseudonymisoidaan ennen luovutusta. Henkilötunnus korvataan tutkimuskohtaisella pseudotunnisteella, jonka avainta säilytetään erillään aineistosta.

Suorat tunnisteet (nimi, osoite, puhelinnumero) poistetaan aina. Epäsuorat tunnisteet, esim. harvinainen diagnoosi yhdistettynä postinumeron ja syntymävuoteen, karkeistetään: ikä viisivuotisluokkiin ja postinumero kunta tasolle.

```sql
SELECT pseudo_id,
       floor(ika / 5) * 5      AS ikaluokka,
       left(postinumero, 2)    AS alue,
       diagnoosi_icd10
FROM   tutkimus_2026_014;
```

## Käyttöympäristö

Aineistoa käsitellään vain hyvinvointialueen tietoturvallisesa remote-käyttöympäristössä (Kapseli). Aineistoa ei saa siirtää omalle koneelle, ja tulosteet tarkistetaan ennen ympäristöstä vientiä. Alle viiden havainnon solut peitetän taulukoista.

Käyttöoikeus on henkilökohtainen ja voimassa luvan mukaisen ajan, enintään viisi vuotta. Kirjautuminen edellyttää kaksivaiheista tunnistautumista ja käyttölokia säilytetään 12 vuotta.

## Tutkijan velvollisuudet

- Käsitellä tietoja vain luvassa mainittuun tarkoitukseen.
- Olla yrittämättä tunnistaa yksittäisiä henkilöitä. Tunnistamisyritys on rikos.
- Ilmoittaa tietoturvaloukkauksesta 24 tunnin kuluessa osoitteeseen tietosuoja@example.fi.
- Hävittää aineisto luvan päättyessä ja toimittaa hävitystodistus.
- Mainita julkaisuissa aineiston lähde ja toimittaa julkaisu rekisterinpitäjälle tiedoksi.

Velvollisuuksien rikkominen johtaa käyttöoikeuden peruuttamiseen ja tarvittaessa ilmoitukseen Tietosuojavaltuutetulle.

## Yhteystiedot

Tutkimuspalvelut: tutkimus@example.fi, puh. 017 173 400, arkisin klo 9–15. Tietosuojavastaava: tietosuoja@example.fi. Ohje on hyväksytty tutkimusjohtajan päätöksellä toukokuussa 2026 ja se tarkistetaan vuosittain.
