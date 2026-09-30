# Webhookit

Ajanvaraus-API voi ilmoittaa tapahtumapohjaisesti varausten muutoksista HTTP-kutsulla asiakkaan palvelimelle. Tämä sivu kuvaa, miten webhook rekisteröidään ja miten kutsut todennetaan.

## Rekisteröinti

Webhook rekisteröidään kutsulla `POST /v2/webhookit`. Rungossa annetaan kohdeosoite ja tapahtumatyypit, jotka halutaan ilmoitus.

```json
{
  "osoite": "https://asiakas.example.fi/hooks/varaus",
  "tapahtumat": ["varaus.luotu", "varaus.peruttu", "varaus.siirretty"]
}
```

Vastauksena palautetaan webhookin tunniste ja salaisuus, jota käytetään allekirjoituksen tarkistamiseen. Salaisuus näytetään vain kerran; se on tallennettava heti.

Osoitteen on oltava HTTPS-osoite. Itse allekirjoitettuja varmenteita ei hyväksytä. Rekisteröinnin yhteydessä palvelin lähettää osoitteeseen testikutsun, johon on vastattava koodilla 200 5 sekunnin kuluessa.

## Kutsun rakenne

Jokaisessa kutsussa on otsakkeet `X-Varaus-Tapahtuma`, `X-Varaus-Aikaleima` ja `X-Varaus-Allekirjoitus`. Allekirjoitus on HMAC-SHA256 aikaleimasta ja rungosta, avaimena rekisteröinnissä saatu salaisuus.

```go
mac := hmac.New(sha256.New, []byte(salaisuus))
mac.Write([]byte(aikaleima + "." + string(runko)))
odotettu := hex.EncodeToString(mac.Sum(nil))
if !hmac.Equal([]byte(odotettu), []byte(allekirjoitus)) {
    return errors.New("virheellinen allekirjoitus")
}
```

Hylkää kutsu, jos aikaleima on yli 5 minuuttia vanha. Tämä estäää toisto hyökkäykset.

## Uudelleenyritykset

Jos vastaanottaja palauttaa muun koodin kuin 2xx tai ei vastaa 10 sekunnissa, kutsu yritetään uudelleen kasvavin välein: 1 min, 5 min, 30 min, 2 h ja 12 h. Viidennen epäonnistuneen yrityksen jälkeen webhook poistetaan käytöstä ja ylläpitäjälle lähetetään sähköposti.

Sama tapahtuma voidaan toimittaa useammankuin kerran. Käsittele kutsut idempotentisti tapahtuman tunnisteen `id` perusteella.

## Tapahtumatyypit

| Tyyppi | Milloin |
|--------|---------|
| `varaus.luotu` | Uusi varaus vahvistettu |
| `varaus.peruttu` | Asiakas tai ammattilainen peruu varauksen |
| `varaus.siirretty` | Varauksen ajankohta muutui |
| `aika.vapautui` | Aiemmin varattu aika vapautui jonossa oleville |

Tapahtumien runko noudattaa samaa schemaa kuin `GET /v2/varaukset/{id}`. Skeemat ovat saatavilla OpenAPI-kuvauksessa osoitteessa https://api.example.fi/v2/openapi.json.

Webhookit ovat saatavilla lokakuusta 2026 alkaen kaikille integraatiokumppaneille.
