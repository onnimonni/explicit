# Tietosuojaohje tutkimuskäyttöön luovutettaville aineistoille

## Soveltamisala

Ohje koskee potilastietojen luovuttamista tieteelliseen tutkimukseen toisiolain (552/2019) nojalla. Luovutuksista päättää ⟪name|Findata⟫, kun aineistoa yhdistetään usean rekisterinpitäjän tiedoista, ja hyvinvointialue itse, kun aineisto on ⟦typo|pelkstään|pelkästään⟧ omista rekistereistä.

## Hakemus

Tutkija toimittaa hakemuksen ⟦compound_split|tutkimus lupa|tutkimuslupa⟧-lomakkeella. Hakemuksessa kuvataan tutkimuksen tarkoitus, tarvittavat tietosisällöt, käsittelyn oikeusperuste ja tietoturvajärjestelyt. Puutteellinen hakemus palautetaan täydennettäväksi ⟪unit|kahden viikon⟫ kuluessa.

Hakemuksen liitteinä ovat tutkimussuunnitelma, eettisen toimikunnan lausunto ja tietosuojaseloste. Ilman eettistä ennakkoarviointia hakemusta ei ⟦typo|käsitelä|käsitellä⟧.

## Aineiston muodostaminen

Aineisto muodostetaan tietoallas-ympäristössä ja pseudonymisoidaan ennen luovutusta. Henkilötunnus korvataan tutkimuskohtaisella pseudotunnisteella, jonka avainta säilytetään erillään aineistosta.

Suorat tunnisteet (nimi, osoite, puhelinnumero) poistetaan aina. Epäsuorat tunnisteet, ⟪abbrev|esim.⟫ harvinainen diagnoosi yhdistettynä ⟦confusion*|postinumeron|postinumeroon⟧ ja syntymävuoteen, ⟦inflection|karkeistetään|karkeistetaan⟧: ikä viisivuotisluokkiin ja postinumero ⟦compound_split|kunta tasolle|kuntatasolle⟧.

```sql
SELECT pseudo_id,
       floor(ika / 5) * 5      AS ikaluokka,
       left(postinumero, 2)    AS alue,
       diagnoosi_icd10
FROM   tutkimus_2026_014;
```

## Käyttöympäristö

Aineistoa käsitellään vain hyvinvointialueen ⟦typo|tietoturvallisesa|tietoturvallisessa⟧ ⟦loan|remote-käyttöympäristössä|etäkäyttöympäristössä⟧ (⟪product|Kapseli⟫). Aineistoa ei saa siirtää omalle koneelle, ja tulosteet tarkistetaan ennen ympäristöstä vientiä. Alle ⟪unit|viiden⟫ havainnon solut ⟦inflection|peitetän|peitetään⟧ taulukoista.

Käyttöoikeus on henkilökohtainen ja voimassa luvan mukaisen ajan, enintään ⟪unit|viisi vuotta⟫. Kirjautuminen edellyttää kaksivaiheista tunnistautumista ja käyttölokia säilytetään ⟪unit|12 vuotta⟫.

## Tutkijan velvollisuudet

- Käsitellä tietoja vain luvassa mainittuun tarkoitukseen.
- Olla yrittämättä tunnistaa yksittäisiä henkilöitä. Tunnistamisyritys on rikos.
- Ilmoittaa tietoturvaloukkauksesta ⟪unit|24 tunnin⟫ kuluessa osoitteeseen ⟪url|tietosuoja@example.fi⟫.
- Hävittää aineisto luvan päättyessä ja toimittaa hävitystodistus.
- Mainita julkaisuissa aineiston lähde ja toimittaa julkaisu rekisterinpitäjälle tiedoksi.

Velvollisuuksien rikkominen johtaa käyttöoikeuden peruuttamiseen ja tarvittaessa ilmoitukseen ⟪name|Tietosuojavaltuutetulle⟫.

## Yhteystiedot

Tutkimuspalvelut: ⟪url|tutkimus@example.fi⟫, ⟪number|puh. 017 173 400⟫, arkisin ⟪unit|klo 9–15⟫. Tietosuojavastaava: ⟪url|tietosuoja@example.fi⟫. Ohje on hyväksytty tutkimusjohtajan päätöksellä toukokuussa 2026 ja se tarkistetaan vuosittain.
