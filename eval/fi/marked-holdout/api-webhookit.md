# Webhookit

Ajanvaraus-API voi ilmoittaa tapahtumapohjaisesti varausten muutoksista ⟪product|HTTP⟫-kutsulla asiakkaan palvelimelle. Tämä sivu kuvaa, miten webhook rekisteröidään ja miten kutsut todennetaan.

## Rekisteröinti

Webhook rekisteröidään kutsulla ⟪code|`POST /v2/webhookit`⟫. Rungossa annetaan kohdeosoite ja tapahtumatyypit, ⟦confusion*|jotka|joista⟧ halutaan ilmoitus.

```json
{
  "osoite": "https://asiakas.example.fi/hooks/varaus",
  "tapahtumat": ["varaus.luotu", "varaus.peruttu", "varaus.siirretty"]
}
```

Vastauksena palautetaan webhookin tunniste ja salaisuus, jota käytetään allekirjoituksen tarkistamiseen. Salaisuus näytetään vain kerran; se on tallennettava heti.

Osoitteen on oltava ⟪product|HTTPS⟫-osoite. Itse allekirjoitettuja varmenteita ei hyväksytä. Rekisteröinnin yhteydessä palvelin lähettää osoitteeseen testikutsun, johon on vastattava koodilla ⟪number|200⟫ ⟪unit|5 sekunnin⟫ kuluessa.

## Kutsun rakenne

Jokaisessa kutsussa on otsakkeet ⟪identifier|`X-Varaus-Tapahtuma`⟫, ⟪identifier|`X-Varaus-Aikaleima`⟫ ja ⟪identifier|`X-Varaus-Allekirjoitus`⟫. Allekirjoitus on ⟪acronym|HMAC-SHA256⟫ aikaleimasta ja rungosta, avaimena rekisteröinnissä saatu salaisuus.

```go
mac := hmac.New(sha256.New, []byte(salaisuus))
mac.Write([]byte(aikaleima + "." + string(runko)))
odotettu := hex.EncodeToString(mac.Sum(nil))
if !hmac.Equal([]byte(odotettu), []byte(allekirjoitus)) {
    return errors.New("virheellinen allekirjoitus")
}
```

Hylkää kutsu, jos aikaleima on yli ⟪unit|5 minuuttia⟫ vanha. Tämä ⟦typo|estäää|estää⟧ ⟦compound_split|toisto hyökkäykset|toistohyökkäykset⟧.

## Uudelleenyritykset

Jos vastaanottaja palauttaa muun koodin kuin 2xx tai ei vastaa ⟪unit|10 sekunnissa⟫, kutsu yritetään uudelleen kasvavin välein: ⟪unit|1 min⟫, ⟪unit|5 min⟫, ⟪unit|30 min⟫, ⟪unit|2 h⟫ ja ⟪unit|12 h⟫. Viidennen epäonnistuneen yrityksen jälkeen webhook poistetaan käytöstä ja ylläpitäjälle lähetetään sähköposti.

Sama tapahtuma voidaan toimittaa ⟦compound_joined|useammankuin|useamman kuin⟧ kerran. Käsittele kutsut idempotentisti tapahtuman tunnisteen ⟪identifier|`id`⟫ perusteella.

## Tapahtumatyypit

| Tyyppi | Milloin |
|--------|---------|
| ⟪code|`varaus.luotu`⟫ | Uusi varaus vahvistettu |
| ⟪code|`varaus.peruttu`⟫ | Asiakas tai ammattilainen ⟦confusion*|peruu|perui⟧ varauksen |
| ⟪code|`varaus.siirretty`⟫ | Varauksen ajankohta ⟦typo|muutui|muuttui⟧ |
| ⟪code|`aika.vapautui`⟫ | Aiemmin varattu aika vapautui jonossa oleville |

Tapahtumien runko noudattaa samaa ⟦loan|schemaa|skeemaa⟧ kuin ⟪code|`GET /v2/varaukset/{id}`⟫. Skeemat ovat saatavilla ⟪product|OpenAPI⟫-kuvauksessa osoitteessa ⟪url|https://api.example.fi/v2/openapi.json⟫.

Webhookit ovat saatavilla lokakuusta 2026 alkaen kaikille integraatiokumppaneille.
