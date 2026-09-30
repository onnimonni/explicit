# Ajanvaraus-API

Ajanvaraus-API tarjoaa ⟪product|REST⟫-rajapinnan, jolla ajanvarauksia voi luoda, muokata ja perua. Kaikki pyynnöt tehdään ⟪product|HTTPS⟫-yhteydellä ja ⟦loan|autentikatio|autentikointi⟧ hoidetaan ⟪product|OAuth 2.0⟫ -tunnuksilla.

Rajapinnan perusosoite on ⟪url|https://api.example.fi/v2⟫. Vastaukset ovat ⟪product|JSON⟫-muotoisia ja käyttävät ⟪acronym|UTF-8⟫-⟦typo|merkitsöä|merkistöä⟧.

## Todennus

Jokaisessa pyynnössä on oltava ⟪identifier|`Authorization`⟫-otsake. Tunnus haetaan tunnistuspalvelusta ja se on voimassa ⟪unit|60 min⟫. Vanhentuneella tunnuksella palvelin palauttaa virheen ⟪number|401⟫.

```http
GET /v2/ajat?paiva=2026-10-03 HTTP/1.1
Host: api.example.fi
Authorization: Bearer eyJhbGciOiJSUzI1NiJ9...
```

Tunnusta ⟦compound_joined|eisaa|ei saa⟧ tallentaa selaimen paikalliseen muistiin. Suosittelemme ⟪product|HttpOnly⟫-evästettä.

## Resurssit

### Vapaat ajat

⟪code|`GET /v2/ajat`⟫ palauttaa vapaat ajat annetulle päivälle. Kyselyparametri ⟪identifier|`paiva`⟫ on pakollinen ja sen muoto on ⟪code|`VVVV-KK-PP`⟫. Ilman parametria pyyntö ⟦inflection|hylkätään|hylätään⟧ ja palvelin kirjoittaa ⟦loan|logiin|lokiin⟧ varoituksen.

Vastaus:

```json
{
  "ajat": [
    { "id": "a1f3", "alkaa": "2026-10-03T09:00:00+03:00", "kesto": 30 },
    { "id": "a1f4", "alkaa": "2026-10-03T09:30:00+03:00", "kesto": 30 }
  ]
}
```

Kenttä ⟪identifier|`kesto`⟫ ilmoitetaan ⟦inflection|minuuteissä|minuuteissa⟧. Jos päivälle ei ole vapaita aikoja, ⟪identifier|`ajat`⟫ on tyhjä taulukko, ei ⟪code|`null`⟫.

### Varauksen luonti

⟪code|`POST /v2/varaukset`⟫ luo uuden varauksen. Pyynnön rungossa annetaan ajan tunniste ja asiakastiedot. Henkilötunnus välitetään aina salattuna.

| Kenttä | Tyyppi | Pakollinen | Kuvaus |
|--------|--------|------------|--------|
| ⟪identifier|`aika_id`⟫ | merkkijono | kyllä | Vapaan ajan tunniste |
| ⟪identifier|`hetu`⟫ | merkkijono | kyllä | Asiakkaan henkilötunnus |
| ⟪identifier|`puhelin`⟫ | merkkijono | ei | Puhelinnumero kansainvälisessä muodossa, ⟪abbrev|esim.⟫ ⟪number|+358 40 123 4567⟫ |
| ⟪identifier|`viesti`⟫ | merkkijono | ei | Vapaamuotoinen viesti, ⟪unit|enintään 500 merkkiä⟫ |

Onnistunut pyyntö palauttaa koodin ⟪number|201⟫ ja varauksen tiedot. Jos aika on ehtinyt mennä, palvelin vastaa koodilla ⟪number|409⟫ ja virheviesti ⟦punctuation|kertoo että|kertoo, että⟧ aika on jo varattu.

### Varauksen peruminen

⟪code|`DELETE /v2/varaukset/{id}`⟫ peruu varauksen. Peruminen on mahdollista viimeistään ⟪unit|24 h⟫ ennen ajan alkua. Myöhemmin tehdyt perumiset hylätään ja niistä ⟦inflection|veloitetan|veloitetaan⟧ peruutusmaksu.

## Virhekoodit

Virhevastauksessa on aina kentät ⟪identifier|`koodi`⟫ ja ⟪identifier|`viesti`⟫. Viesti on tarkoitettu ⟦loan|technistä|teknistä⟧ vianetsintää varten eikä sitä pidä näyttää loppukäyttäjälle sellaisenaan.

- ⟪number|400⟫ – pyyntö on virheellinen, ⟪abbrev|esim.⟫ puuttuva kenttä
- ⟪number|401⟫ – tunnus puuttuu tai on vanhentunut
- ⟪number|404⟫ – resurssia ei löydy
- ⟪number|429⟫ – liikaa pyyntöjä; ⟦compound_split|pyyntö raja|pyyntöraja⟧ on ⟪unit|100 pyyntöä minuutissa⟫

## Versiointi

Rajapinta versioidaan osoitteessa. Vanhaa ⟦loan|versionia|versiota⟧ tuetaan ⟪unit|12 kuukautta⟫ uuden version julkaisun jälkeen. Poistuvat kentät merkitään vanhentuneiksi vastauksen ⟪identifier|`Deprecation`⟫-otsakkeella, ennen kuin ne poistetaan.
