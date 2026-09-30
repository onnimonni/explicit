---
lang: fi-FI
---

# ADR-003: Ammattilaisten tunnistautuminen

Tila: hyväksytty 22.9.2026

## Tilanne

Terveydenhuollon ammattilaiset kirjautuvat alustaan nykyisin käyttäjätunnuksella ja salasanalla. Valviran ja Kelan vaatimusten mukaan Kanta-palveluihin kirjoittavan käyttäjän on tunnistauduttva vahvasti. Nykyinen ratkaisu ei täytä vaatimusta.

Vaihtoehtoina arvioitiin toimi kortti-tunnistusta (DVV:n ammattikortti), Suomi.fi-tunnistusta ja FIDO2-avaimia. Arvioinnin tekivät Otto Manninen ja Aale Tynni Elokuun aikana.

## Vaihtoehdot

### Toimikortti

Ammattikortti on jo kaikilla lääkäreillä ja hoitajilla. Kortinlukijat puuttuvat kuitenkin n. 30 %:sta työasemista, ja mobiilikäyttö ei onnistu lainkaan. Kortin välimuistituksen kanssa on ollut ongelmia macOS:llä.

### Suomi.fi-tunnistus

Toimii kaikilla laitteilla. Sitä ei kuitenkaan kerro käyttäjän ammattioikeuksia, joten ne on haettava erikseen Terhikki-rekisteristä. Kirjautuminen kestää 20–40 sekuntia, mikä on liikka päivystystyössä.

### FIDO2-avaimet

Nopea ja tietoturvallinen. Avain on hankittava ja rekisteröitävä jokaiselle käyttäjälle, ja kadonneen avaimen prosessi on rakennettava itse. Lääkärit eivät ole tottuneet laiteesen.

## Päätös

Valitsemme toimikortin ensisijaiseksi tavaksi ja FIDO2-avaimen varatavaksi. Perustelut:

1. Toimikortti täyttää viranomaisvaatimukset sellaisenaanilman lisäselvityksiä.
2. Kortinlukijat hankitaan puuttuviin työasemiin 15 000 €:lla, mikä on halvempaa kun avainten hankinta kaikille.
3. Mobiilikäyttöön FIDO2 riittää, koska mobiilissa ei kirjoiteta Kanta-tietoja.

Suomi.fi-tunnistus jää asiakaiden tunnistustavaksi.

## Seuraukset

- Kirjautumissivu tukee kahta tapaa; käyttö liittymän on ohjattava käyttäjä oikeaan tapaan laitteen perusteella.
- Kortinlukijoiden hankinta ja asennus valmistuu marraskuussa.
- Ammattioikeudet tarkistetän Terhikistä kerran vuorokaudessa ja tallennetaan käyttöoikeusrekisteriin.
- Vanha salasanakirjautuminen poistuu heti kun kaikki käyttäjät on siirretty, viimeistään 31. tammikuuta 2027.

Riski: kortinlukijoiden toimitusaika on epävarma componenttipulan vuoksi. Sen takia tilaus tehdään heti päätöksen jälkeen.
