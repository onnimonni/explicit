---
lang: fi
---

# OmaTerveys-sovelluksen muutosloki

## 3.2.0 – 2026-09-25

### Lisätty

- Puolesta-asiointi lapsille ⟪unit|alle 12 vuotta⟫ ilman erillistä valtuutusta.
- ⟪acronym|PDF⟫-tulostus ⟦typo|reseptesitä|resepteistä⟧.
- ruotsinkielinen ja englanninkielinen oirearvio.
- Tuki ⟪product|Android⟫ 16:lle.

### Muutettu

- Kirjautumisen aikakatkaisu pidennetty ⟪unit|15 minuuttiin⟫.
- Viestien liitteiden enimmäiskoko nostettu ⟪unit|20 MB⟫:hen.
- Kalenterin viikonpäivät näytetään nyt ⟦inflection|lyhennettyna|lyhennettynä⟧ pienillä näytöillä.

### Korjattu

- Sovellus ⟦confusion*|kaatu|kaatui⟧ ⟪product|Android⟫ 12 -laitteilla kirjautumisen jälkeen, jos laitteella oli yli ⟪number|50⟫ ilmoitusta (⟪identifier|OMA-1187⟫).
- Ajan peruminen epäonnistui hiljaisesti, jos verkkoyhteys katkesi kesken pyynnön. Nyt näytetään virheilmoitus ja peruminen yritetään uudelleen.
- Ruudunlukija luki laboratoriotulosten viitearvot väärässä järjestyksessä.
- ⟦capitalization|Suomenkielinen|suomenkielinen⟧ päivämäärä näkyi englanninkielisessä käyttöliittymässä.

## 3.1.2 – 2026-08-30

### Korjattu

- ⟪product|iOS⟫ 18.6: ⟦compound_split|sormenjälki tunnistus|sormenjälkitunnistus⟧ ei toiminut ⟦compound_joined|senjälkeen|sen jälkeen⟧, kun käyttäjä oli vaihtanut ⟪acronym|PIN⟫-koodin.
- Muistutus lähti kahdesti, jos aika oli ⟦typo|siiretty|siirretty⟧ toiselle päivälle.
- Kirjautumissivun logo oli sumea suurella tarkkuudella.

## 3.1.1 – 2026-08-12

### Tietoturva

- Päivitetty ⟪product|OkHttp⟫ versioon 5.1 (⟪identifier|CVE-2026-2231⟫).
- Istunto päätetään nyt palvelimella, ei vain sovelluksessa, kun käyttäjä kirjautuu ulos.
- Salasanan vaihtolinkki on voimassa ⟪unit|15 minuuttia⟫ aiemman ⟪unit|24 tunnin⟫ sijaan.

## 3.1.0 – 2026-07-20

### Lisätty

- Tumma tila.
- Laboratoriotulosten kuvaajat ⟪unit|12 kuukauden⟫ ajalta.
- ⟪product|Apple Watch⟫ -ilmoitukset tulevista ajoista.

### Muutettu

- Sovellus vaatii nyt ⟪product|iOS⟫ 17:n tai ⟪product|Android⟫ 10:n. Vanhemmat versiot ⟦confusion*|näyttää|näyttävät⟧ päivityskehotuksen eivätkä enää kirjaudu sisään.
- Alkunäkymä uudistettu palautteen perusteella: seuraava aika ja lukemattomat viestit näkyvät ⟦typo|ensimäisenä|ensimmäisenä⟧.

### Poistettu

- Vanha ⟦compound_split|oire arvio|oirearvio⟧ (versio 1), joka oli merkitty vanhentuneeksi helmikuussa.

Kaikki versiot: ⟪url|https://github.com/example/omaterveys/releases⟫. Bugiraportit otetaan vastaan osoitteessa ⟪url|tuki@omaterveys.fi⟫ tai sovelluksen ⟪code|Lähetä palaute⟫ -toiminnolla.
