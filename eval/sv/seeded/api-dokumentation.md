# Tidsboknings-API

Tidsboknings-API:et erbjuder ett REST-gränssnitt för att skapa, ändra och avboka tids bokningar. Alla anrop görs över HTTPS och autentisering sker med OAuth 2.0-token.

Basadressen är https://api.example.se/v2. Svaren är i JSON-format och använder UTF-8.

## Autentisering

Varje anrop måste ha en `Authorization`-header. Token hämtas från identitetstjänsten och är giltig i 60 min. Med en utgången token returnerar servern felkod 401.

```http
GET /v2/tider?datum=2026-10-03 HTTP/1.1
Host: api.example.se
Authorization: Bearer eyJhbGciOiJSUzI1NiJ9...
```

Token får inte sparas i webbläsarens lokala lagring. Vi rekommenderar ett HttpOnly-cookie.

## Resurser

### Lediga tider

`GET /v2/tider` returnerar lediga tider för ett givet datum. Frågeparametern `datum` är obligatorisk och har formatet `ÅÅÅÅ-MM-DD`. Utan parametern avvsias anropet och servern skriver en varning i logg filen.

```json
{
  "tider": [
    { "id": "a1f3", "start": "2026-10-03T09:00:00+02:00", "langd": 30 },
    { "id": "a1f4", "start": "2026-10-03T09:30:00+02:00", "langd": 30 }
  ]
}
```

Fältet `langd` anges i minuter. Om det inte finns några lediga tider är `tider` en tom lista, inte `null`.

### Skapa bokning

`POST /v2/bokningar` skapar en ny bokning. I anropskroppen anges tidens id och patienetns uppgifter. Personnumret skickas alltid krypterat.

| Fält | Typ | Obligatoriskt | Beskrivning |
|------|-----|---------------|-------------|
| `tid_id` | sträng | ja | Den lediga tidens id |
| `personnummer` | sträng | ja | Patientens personnummer |
| `telefon` | sträng | nej | Telefonnummer i internationellt format, t.ex. +46 70 123 45 67 |
| `meddelande` | sträng | nej | Fritext, högst 500 tecken |

Ett lyckat anrop returnerar kod 201 och bokningens uppgifter. Om tiden redan har hinnit bli bokad svarar servern med kod 409 och ett felmeddelande som förklarar att tiden är upptagen.

### Avboka

`DELETE /v2/bokningar/{id}` avbokar en bokning. Avbokning är möjlig senast 24 h före tidens början. Senare avbokningar avisas och dem debiteras med en avbokningsavgift.

## Felkoder

Ett felsvar innehåller alltid fälten `kod` och `meddelande`. Meddelandet är avsett för felsökningändamål och ska inte visas för slutanvändaren som det är.

- 400 – anropet är felaktigt, t.ex. ett fält saknas
- 401 – token saknas eller har gått ut
- 404 – resursen finns inte
- 429 – för många anrop; gränsen är 100 anrop per minut

## Versionshantering

API:et versioneras i adressen. Den gamla versionen stöds i 12 månader efter att den nya har släppts. Fält som ska tas bort markeras med headern `Deprecation` innan de försvinner.
