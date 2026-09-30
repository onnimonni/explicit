---
lang: fi
---

# Ajokirja: varmuuskopioiden palautus

Tätä ohjetta käytetään, kun tietokanta tai tiedostovarasto on palautettava varmuuskopiosta. Palautus on aina kaheden henkilön tehtävä: toinen tekee, toinen tarkistaa.

## Varmuuskopioiden sijainti

| Kohde | Tiheys | Säilytys | Sijainti |
|-------|--------|----------|----------|
| PostgreSQL (WAL + päivittäinen) | jatkuva / klo 01 | 35 päivää | `s3://varmuus/pg/` |
| Tiedostovarasto | 4 h välein | 90 päivää | `s3://varmuus/tiedostot/` |
| Salaisuudet (Vault) | päivittäin | 1 vuosi | erillinen holvi, ks. tietoturvaohje |

Kaikki kopiot on salattu AES-256:lla. Purku avain on Vaultissa polussa `secret/varmuus/avain`; siihen pääsee käsiksi vain päivystäjän roolilla.

## Palautus ajanhetkeen

1. Ilmoita kanavalla #hairiot, että palautus alkaa. Sovellus asetetän huoltotilaan.
2. Selvitä palautettava ajanhetki. Käytä hetkeä joka on vähintään viisi minuuttia ennen virheen ilmenemistä.
3. Aja palautus pgBackRestillä:

```bash
pgbackrest --stanza=tuotanto --type=time \
  --target="2026-09-18 09:05:00+03" --target-action=promote restore
systemctl start postgresql
```

4. Tarkista, että tietokannan uusin varaus on odotetulta ajalta: `SELECT max(luotu) FROM varaukset;`.
5. Poista huoltotila ja seuraa virhelokia 15 minuuttia.

Palautus 200 GiB:n kannalle kestää n. 40 minuuttia. Älä keskeytä sitä, vaikkase näyttäisi jumittuneen; WAL-tiedostojen toisto on hidasta vaan etenee.

## Tiedostojen palautus

Yksittäisen tiedoston voi palauttaa ilman huoltotilaa. Tiedostot on versioitu S3:ssä, joten poistettu tiedoston saa takaisin komennolla:

```bash
aws s3api list-object-versions --bucket varmuus --prefix tiedostot/potilasohjeet/
aws s3api get-object --bucket varmuus --key tiedostot/potilasohjeet/ohje.pdf \
  --version-id <versio> ohje.pdf
```

Palautetun tiedoston tarkistussumma verrataan lokiin ennen kuin se kopioidaan takaisin tuotantoon.

## Palautusharjoitus

Palautus harjoitellaan neljännesvuosittain staging-ympäristössä. Harjoituksen tulos (kesto, ongelmmat, tarvittavat korjaukset) kirjataan Confluenceen. Viime Kesäkuun harjoituksessa palautus kesti 52 minuuttia, koska purkuavainta ei löytynyt heti; ohje päivitettiin sen jälkeen.

Jos palautus testi epäonnistuu, siitä avataan SEV-2-tiketti, vaikka tuotanto ei olisikaan vaarassa. Varmuuskopio, jota ei ole koskaan palautettu, ei ole varmuuskopio.
