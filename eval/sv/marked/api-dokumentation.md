# Tidsboknings-API

Tidsboknings-API:et erbjuder ett ⟪product|REST⟫-gränssnitt för att skapa, ändra och avboka ⟦sarskrivning|tids bokningar|tidsbokningar⟧. Alla anrop görs över ⟪product|HTTPS⟫ och autentisering sker med ⟪product|OAuth 2.0⟫-token.

Basadressen är ⟪url|https://api.example.se/v2⟫. Svaren är i ⟪product|JSON⟫-format och använder ⟪acronym|UTF-8⟫.

## Autentisering

Varje anrop måste ha en ⟪identifier|`Authorization`⟫-header. Token hämtas från identitetstjänsten och är giltig i ⟪unit|60 min⟫. Med en utgången token returnerar servern felkod ⟪number|401⟫.

```http
GET /v2/tider?datum=2026-10-03 HTTP/1.1
Host: api.example.se
Authorization: Bearer eyJhbGciOiJSUzI1NiJ9...
```

Token får inte sparas i webbläsarens lokala lagring. Vi rekommenderar ⟦de_dem_gender*|ett|en⟧ ⟪product|HttpOnly⟫-cookie.

## Resurser

### Lediga tider

⟪code|`GET /v2/tider`⟫ returnerar lediga tider för ett givet datum. Frågeparametern ⟪identifier|`datum`⟫ är obligatorisk och har formatet ⟪code|`ÅÅÅÅ-MM-DD`⟫. Utan parametern ⟦typo|avvsias|avvisas⟧ anropet och servern skriver en varning i ⟦sarskrivning|logg filen|loggfilen⟧.

```json
{
  "tider": [
    { "id": "a1f3", "start": "2026-10-03T09:00:00+02:00", "langd": 30 },
    { "id": "a1f4", "start": "2026-10-03T09:30:00+02:00", "langd": 30 }
  ]
}
```

Fältet ⟪identifier|`langd`⟫ anges i minuter. Om det inte finns några lediga tider är ⟪identifier|`tider`⟫ en tom lista, inte ⟪code|`null`⟫.

### Skapa bokning

⟪code|`POST /v2/bokningar`⟫ skapar en ny bokning. I anropskroppen anges tidens id och ⟦typo|patienetns|patientens⟧ uppgifter. Personnumret skickas alltid krypterat.

| Fält | Typ | Obligatoriskt | Beskrivning |
|------|-----|---------------|-------------|
| ⟪identifier|`tid_id`⟫ | sträng | ja | Den lediga tidens id |
| ⟪identifier|`personnummer`⟫ | sträng | ja | Patientens personnummer |
| ⟪identifier|`telefon`⟫ | sträng | nej | Telefonnummer i internationellt format, ⟪abbrev|t.ex.⟫ ⟪number|+46 70 123 45 67⟫ |
| ⟪identifier|`meddelande`⟫ | sträng | nej | Fritext, ⟪unit|högst 500 tecken⟫ |

Ett lyckat anrop returnerar kod ⟪number|201⟫ och bokningens uppgifter. Om tiden redan har ⟦verb_form|hinnit|hunnit⟧ bli bokad svarar servern med kod ⟪number|409⟫ och ett felmeddelande som förklarar att tiden är upptagen.

### Avboka

⟪code|`DELETE /v2/bokningar/{id}`⟫ avbokar en bokning. Avbokning är möjlig senast ⟪unit|24 h⟫ före tidens början. Senare avbokningar ⟦double_consonant|avisas|avvisas⟧ och ⟦de_dem_gender*|dem|de⟧ debiteras med en avbokningsavgift.

## Felkoder

Ett felsvar innehåller alltid fälten ⟪identifier|`kod`⟫ och ⟪identifier|`meddelande`⟫. Meddelandet är avsett för ⟦compound_link|felsökningändamål|felsökningsändamål⟧ och ska inte visas för slutanvändaren som det är.

- ⟪number|400⟫ – anropet är felaktigt, ⟪abbrev|t.ex.⟫ ett fält saknas
- ⟪number|401⟫ – token saknas eller har gått ut
- ⟪number|404⟫ – resursen finns inte
- ⟪number|429⟫ – för många anrop; gränsen är ⟪unit|100 anrop per minut⟫

## Versionshantering

API:et versioneras i adressen. Den gamla versionen stöds i ⟪unit|12 månader⟫ efter att den nya har släppts. Fält som ska tas bort markeras med headern ⟪identifier|`Deprecation`⟫ innan de försvinner.
