---
lang: fi
---

# Muutosloki

Kaikki merkittävät muutokset kirjataan tähän tiedostoon. Muoto noudattaa ⟪product|Keep a Changelog⟫ -käytäntöä ja versiot ⟪product|Semantic Versioning⟫ -numerointia.

## [Julkaisematon]

### Lisätty

- ⟦compound_split|Käyttö oikeuksien|Käyttöoikeuksien⟧ hallintaan uusi rooli ⟪identifier|`katselija`⟫, joka näkee raportit muttei voi muokata niitä.
- ⟪product|OpenTelemetry⟫-jäljitys kaikkiin ⟪acronym|HTTP⟫-pyyntöihin. Jäljet lähetetään ⟪product|Tempoon⟫.
- Tuki ⟪product|Redis⟫ 7:n ⟦loan|clusteri|klusteri⟧-tilalle.

### Muutettu

- ⟦typo|Kirjautumissviu|Kirjautumissivu⟧ ohjaa nyt suoraan ⟪name|Suomi.fi⟫-tunnistukseen, jos käyttäjällä ei ole paikallista tunnusta.
- Lokien säilytysaika lyhennetty ⟪unit|90 päivästä⟫ ⟪unit|30 päivään⟫ tietosuojasyistä.

### Korjattu

- Raportin vienti ⟪product|Excel⟫-muotoon kaatui, jos taulukossa oli yli ⟪number|65 536⟫ riviä.

## [2.3.1] – 2026-09-02

### Korjattu

- Vanha suomalainen päivämäärämuoto ⟪code|`pp.kk.vvvv`⟫ tulkittiin väärin aikavyöhykkeen vaihtuessa kesäajasta talviaikaan.
- ⟦typo|Sähköpsotin|Sähköpostin⟧ lähetys epäonnistui, jos vastaanottajan osoitteessa oli isoja ⟦double_letter|kirjaimmia|kirjaimia⟧.
- Tyhjä hakutulos näytti latausanimaation loputtomasti.

## [2.3.0] – 2026-08-19

### Lisätty

- Uusi ⟦compound_split|ajan varaus|ajanvaraus⟧-näkymä, joka näyttää vapaat ajat ⟪unit|kolmen viikon⟫ jaksolta.
- ⟪acronym|PDF⟫-tulostus potilasohjeista.
- Mahdollisuus lähettää muistutus tekstiviestillä. Viestit lähetetään ⟪product|Twilion⟫ kautta.

### Muutettu

- Salasanan vähimmäispituus nostettu ⟪unit|12 merkkiin⟫. Vanhat salasanat toimivat, kunnes käyttäjä vaihtaa ne.
- ⟪product|Node.js⟫ päivitetty versioon ⟪number|22⟫. ⟪product|Node⟫ 18 ei enää tueta.

### Poistettu

- Vanha ⟪acronym|SOAP⟫-rajapinta ⟪code|`/ws/varaus`⟫. Se oli merkitty vanhentuneeksi ⟦capitalization|Helmikuussa|helmikuussa⟧ 2025.

## [2.2.4] – 2026-07-01

### Tietoturva

- Päivitetty ⟪product|OpenSSL⟫ versioon 3.3.2 (⟪identifier|CVE-2026-1123⟫).
- Istunnon ⟦loan|tokeni|tunniste⟧ uusitaan nyt jokaisen kirjautumiskerran yhteydessä.
- Korjattu ⟦compound_split|pääsyn hallinnan|pääsynhallinnan⟧ virhe, jonka vuoksi ⟦confusion*|katselija|katselijat⟧ pystyivät avaamaan muokkausnäkymän suoralla osoitteella.

## [2.2.3] – 2026-06-15

### Korjattu

- ⟪product|Safari⟫ selaimella kalenteri ei piirtynyt oikein viikonloppuisin.
- ⟪identifier|`X-Request-Id`⟫-otsake puuttui virhevastauksista.
