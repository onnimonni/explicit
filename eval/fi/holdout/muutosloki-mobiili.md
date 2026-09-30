---
lang: fi
---

# OmaTerveys-sovelluksen muutosloki

## 3.2.0 – 2026-09-25

### Lisätty

- Puolesta-asiointi lapsille alle 12 vuotta ilman erillistä valtuutusta.
- PDF-tulostus reseptesitä.
- ruotsinkielinen ja englanninkielinen oirearvio.
- Tuki Android 16:lle.

### Muutettu

- Kirjautumisen aikakatkaisu pidennetty 15 minuuttiin.
- Viestien liitteiden enimmäiskoko nostettu 20 MB:hen.
- Kalenterin viikonpäivät näytetään nyt lyhennettyna pienillä näytöillä.

### Korjattu

- Sovellus kaatu Android 12 -laitteilla kirjautumisen jälkeen, jos laitteella oli yli 50 ilmoitusta (OMA-1187).
- Ajan peruminen epäonnistui hiljaisesti, jos verkkoyhteys katkesi kesken pyynnön. Nyt näytetään virheilmoitus ja peruminen yritetään uudelleen.
- Ruudunlukija luki laboratoriotulosten viitearvot väärässä järjestyksessä.
- Suomenkielinen päivämäärä näkyi englanninkielisessä käyttöliittymässä.

## 3.1.2 – 2026-08-30

### Korjattu

- iOS 18.6: sormenjälki tunnistus ei toiminut senjälkeen, kun käyttäjä oli vaihtanut PIN-koodin.
- Muistutus lähti kahdesti, jos aika oli siiretty toiselle päivälle.
- Kirjautumissivun logo oli sumea suurella tarkkuudella.

## 3.1.1 – 2026-08-12

### Tietoturva

- Päivitetty OkHttp versioon 5.1 (CVE-2026-2231).
- Istunto päätetään nyt palvelimella, ei vain sovelluksessa, kun käyttäjä kirjautuu ulos.
- Salasanan vaihtolinkki on voimassa 15 minuuttia aiemman 24 tunnin sijaan.

## 3.1.0 – 2026-07-20

### Lisätty

- Tumma tila.
- Laboratoriotulosten kuvaajat 12 kuukauden ajalta.
- Apple Watch -ilmoitukset tulevista ajoista.

### Muutettu

- Sovellus vaatii nyt iOS 17:n tai Android 10:n. Vanhemmat versiot näyttää päivityskehotuksen eivätkä enää kirjaudu sisään.
- Alkunäkymä uudistettu palautteen perusteella: seuraava aika ja lukemattomat viestit näkyvät ensimäisenä.

### Poistettu

- Vanha oire arvio (versio 1), joka oli merkitty vanhentuneeksi helmikuussa.

Kaikki versiot: https://github.com/example/omaterveys/releases. Bugiraportit otetaan vastaan osoitteessa tuki@omaterveys.fi tai sovelluksen Lähetä palaute -toiminnolla.
