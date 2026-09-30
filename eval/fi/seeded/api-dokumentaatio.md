# Ajanvaraus-API

Ajanvaraus-API tarjoaa REST-rajapinnan, jolla ajanvarauksia voi luoda, muokata ja perua. Kaikki pyynnöt tehdään HTTPS-yhteydellä ja autentikatio hoidetaan OAuth 2.0 -tunnuksilla.

Rajapinnan perusosoite on https://api.example.fi/v2. Vastaukset ovat JSON-muotoisia ja käyttävät UTF-8-merkitsöä.

## Todennus

Jokaisessa pyynnössä on oltava `Authorization`-otsake. Tunnus haetaan tunnistuspalvelusta ja se on voimassa 60 min. Vanhentuneella tunnuksella palvelin palauttaa virheen 401.

```http
GET /v2/ajat?paiva=2026-10-03 HTTP/1.1
Host: api.example.fi
Authorization: Bearer eyJhbGciOiJSUzI1NiJ9...
```

Tunnusta eisaa tallentaa selaimen paikalliseen muistiin. Suosittelemme HttpOnly-evästettä.

## Resurssit

### Vapaat ajat

`GET /v2/ajat` palauttaa vapaat ajat annetulle päivälle. Kyselyparametri `paiva` on pakollinen ja sen muoto on `VVVV-KK-PP`. Ilman parametria pyyntö hylkätään ja palvelin kirjoittaa logiin varoituksen.

Vastaus:

```json
{
  "ajat": [
    { "id": "a1f3", "alkaa": "2026-10-03T09:00:00+03:00", "kesto": 30 },
    { "id": "a1f4", "alkaa": "2026-10-03T09:30:00+03:00", "kesto": 30 }
  ]
}
```

Kenttä `kesto` ilmoitetaan minuuteissä. Jos päivälle ei ole vapaita aikoja, `ajat` on tyhjä taulukko, ei `null`.

### Varauksen luonti

`POST /v2/varaukset` luo uuden varauksen. Pyynnön rungossa annetaan ajan tunniste ja asiakastiedot. Henkilötunnus välitetään aina salattuna.

| Kenttä | Tyyppi | Pakollinen | Kuvaus |
|--------|--------|------------|--------|
| `aika_id` | merkkijono | kyllä | Vapaan ajan tunniste |
| `hetu` | merkkijono | kyllä | Asiakkaan henkilötunnus |
| `puhelin` | merkkijono | ei | Puhelinnumero kansainvälisessä muodossa, esim. +358 40 123 4567 |
| `viesti` | merkkijono | ei | Vapaamuotoinen viesti, enintään 500 merkkiä |

Onnistunut pyyntö palauttaa koodin 201 ja varauksen tiedot. Jos aika on ehtinyt mennä, palvelin vastaa koodilla 409 ja virheviesti kertoo että aika on jo varattu.

### Varauksen peruminen

`DELETE /v2/varaukset/{id}` peruu varauksen. Peruminen on mahdollista viimeistään 24 h ennen ajan alkua. Myöhemmin tehdyt perumiset hylätään ja niistä veloitetan peruutusmaksu.

## Virhekoodit

Virhevastauksessa on aina kentät `koodi` ja `viesti`. Viesti on tarkoitettu technistä vianetsintää varten eikä sitä pidä näyttää loppukäyttäjälle sellaisenaan.

- 400 – pyyntö on virheellinen, esim. puuttuva kenttä
- 401 – tunnus puuttuu tai on vanhentunut
- 404 – resurssia ei löydy
- 429 – liikaa pyyntöjä; pyyntö raja on 100 pyyntöä minuutissa

## Versiointi

Rajapinta versioidaan osoitteessa. Vanhaa versionia tuetaan 12 kuukautta uuden version julkaisun jälkeen. Poistuvat kentät merkitään vanhentuneiksi vastauksen `Deprecation`-otsakkeella, ennen kuin ne poistetaan.
