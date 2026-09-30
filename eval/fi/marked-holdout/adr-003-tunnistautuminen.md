---
lang: fi-FI
---

# ADR-003: Ammattilaisten tunnistautuminen

Tila: hyväksytty ⟪unit|22.9.2026⟫

## Tilanne

⟪compound|Terveydenhuollon⟫ ammattilaiset kirjautuvat alustaan nykyisin käyttäjätunnuksella ja salasanalla. ⟪name|Valviran⟫ ja ⟪name|Kelan⟫ vaatimusten mukaan ⟪name|Kanta⟫-palveluihin kirjoittavan käyttäjän on ⟦typo|tunnistauduttva|tunnistauduttava⟧ vahvasti. Nykyinen ratkaisu ei täytä vaatimusta.

Vaihtoehtoina arvioitiin ⟦compound_split|toimi kortti|toimikortti⟧-tunnistusta (⟪acronym|DVV⟫:n ammattikortti), ⟪name|Suomi.fi⟫-tunnistusta ja ⟪acronym|FIDO2⟫-avaimia. Arvioinnin tekivät ⟪name|Otto Manninen⟫ ja ⟪name|Aale Tynni⟫ ⟦capitalization|Elokuun|elokuun⟧ aikana.

## Vaihtoehdot

### Toimikortti

Ammattikortti on jo kaikilla lääkäreillä ja hoitajilla. Kortinlukijat puuttuvat kuitenkin ⟪abbrev|n.⟫ ⟪unit|30 %⟫:sta työasemista, ja mobiilikäyttö ei onnistu lainkaan. Kortin välimuistituksen kanssa on ollut ongelmia ⟪product|macOS⟫:llä.

### ⟪name|Suomi.fi⟫-tunnistus

Toimii kaikilla laitteilla. ⟦confusion*|Sitä|Se⟧ ei kuitenkaan kerro käyttäjän ammattioikeuksia, joten ne on haettava erikseen ⟪name|Terhikki⟫-rekisteristä. Kirjautuminen kestää ⟪unit|20–40 sekuntia⟫, mikä on ⟦typo|liikka|liikaa⟧ päivystystyössä.

### ⟪acronym|FIDO2⟫-avaimet

Nopea ja tietoturvallinen. Avain on hankittava ja rekisteröitävä jokaiselle käyttäjälle, ja kadonneen avaimen prosessi on rakennettava itse. Lääkärit eivät ole tottuneet ⟦inflection|laiteesen|laitteeseen⟧.

## Päätös

Valitsemme toimikortin ensisijaiseksi tavaksi ja ⟪acronym|FIDO2⟫-avaimen varatavaksi. Perustelut:

1. Toimikortti täyttää viranomaisvaatimukset ⟦compound_joined|sellaisenaanilman|sellaisenaan ilman⟧ lisäselvityksiä.
2. Kortinlukijat hankitaan puuttuviin työasemiin ⟪unit|15 000 €⟫:lla, mikä on halvempaa ⟦confusion*|kun|kuin⟧ avainten hankinta kaikille.
3. Mobiilikäyttöön ⟪acronym|FIDO2⟫ riittää, koska mobiilissa ei kirjoiteta ⟪name|Kanta⟫-tietoja.

⟪name|Suomi.fi⟫-tunnistus jää ⟦typo|asiakaiden|asiakkaiden⟧ tunnistustavaksi.

## Seuraukset

- Kirjautumissivu tukee kahta tapaa; ⟦compound_split|käyttö liittymän|käyttöliittymän⟧ on ohjattava käyttäjä oikeaan tapaan laitteen perusteella.
- Kortinlukijoiden hankinta ja asennus valmistuu marraskuussa.
- Ammattioikeudet ⟦inflection|tarkistetän|tarkistetaan⟧ ⟪name|Terhikistä⟫ kerran vuorokaudessa ja tallennetaan käyttöoikeusrekisteriin.
- Vanha salasanakirjautuminen poistuu ⟦punctuation|heti kun|heti, kun⟧ kaikki käyttäjät on siirretty, viimeistään ⟪ordinal|31. tammikuuta⟫ 2027.

Riski: kortinlukijoiden toimitusaika on epävarma ⟦loan|componenttipulan|komponenttipulan⟧ vuoksi. Sen takia tilaus tehdään heti päätöksen jälkeen.
