---
lang: fi
---

# Muutosloki

Kaikki merkittävät muutokset kirjataan tähän tiedostoon. Muoto noudattaa Keep a Changelog -käytäntöä ja versiot Semantic Versioning -numerointia.

## [Julkaisematon]

### Lisätty

- Käyttö oikeuksien hallintaan uusi rooli `katselija`, joka näkee raportit muttei voi muokata niitä.
- OpenTelemetry-jäljitys kaikkiin HTTP-pyyntöihin. Jäljet lähetetään Tempoon.
- Tuki Redis 7:n clusteri-tilalle.

### Muutettu

- Kirjautumissviu ohjaa nyt suoraan Suomi.fi-tunnistukseen, jos käyttäjällä ei ole paikallista tunnusta.
- Lokien säilytysaika lyhennetty 90 päivästä 30 päivään tietosuojasyistä.

### Korjattu

- Raportin vienti Excel-muotoon kaatui, jos taulukossa oli yli 65 536 riviä.

## [2.3.1] – 2026-09-02

### Korjattu

- Vanha suomalainen päivämäärämuoto `pp.kk.vvvv` tulkittiin väärin aikavyöhykkeen vaihtuessa kesäajasta talviaikaan.
- Sähköpsotin lähetys epäonnistui, jos vastaanottajan osoitteessa oli isoja kirjaimmia.
- Tyhjä hakutulos näytti latausanimaation loputtomasti.

## [2.3.0] – 2026-08-19

### Lisätty

- Uusi ajan varaus-näkymä, joka näyttää vapaat ajat kolmen viikon jaksolta.
- PDF-tulostus potilasohjeista.
- Mahdollisuus lähettää muistutus tekstiviestillä. Viestit lähetetään Twilion kautta.

### Muutettu

- Salasanan vähimmäispituus nostettu 12 merkkiin. Vanhat salasanat toimivat, kunnes käyttäjä vaihtaa ne.
- Node.js päivitetty versioon 22. Node 18 ei enää tueta.

### Poistettu

- Vanha SOAP-rajapinta `/ws/varaus`. Se oli merkitty vanhentuneeksi Helmikuussa 2025.

## [2.2.4] – 2026-07-01

### Tietoturva

- Päivitetty OpenSSL versioon 3.3.2 (CVE-2026-1123).
- Istunnon tokeni uusitaan nyt jokaisen kirjautumiskerran yhteydessä.
- Korjattu pääsyn hallinnan virhe, jonka vuoksi katselija pystyivät avaamaan muokkausnäkymän suoralla osoitteella.

## [2.2.3] – 2026-06-15

### Korjattu

- Safari selaimella kalenteri ei piirtynyt oikein viikonloppuisin.
- `X-Request-Id`-otsake puuttui virhevastauksista.
