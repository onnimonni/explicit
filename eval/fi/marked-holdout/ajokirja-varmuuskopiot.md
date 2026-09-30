---
lang: fi
---

# Ajokirja: varmuuskopioiden palautus

Tätä ohjetta käytetään, kun tietokanta tai tiedostovarasto on palautettava varmuuskopiosta. Palautus on aina ⟦typo|kaheden|kahden⟧ henkilön tehtävä: toinen tekee, toinen tarkistaa.

## Varmuuskopioiden sijainti

| Kohde | Tiheys | Säilytys | Sijainti |
|-------|--------|----------|----------|
| ⟪product|PostgreSQL⟫ (⟪acronym|WAL⟫ + päivittäinen) | jatkuva / ⟪unit|klo 01⟫ | ⟪unit|35 päivää⟫ | ⟪code|`s3://varmuus/pg/`⟫ |
| Tiedostovarasto | ⟪unit|4 h⟫ välein | ⟪unit|90 päivää⟫ | ⟪code|`s3://varmuus/tiedostot/`⟫ |
| Salaisuudet (⟪product|Vault⟫) | päivittäin | ⟪unit|1 vuosi⟫ | erillinen holvi, ⟪abbrev|ks.⟫ tietoturvaohje |

Kaikki kopiot on salattu ⟪acronym|AES-256⟫:lla. ⟦compound_split|Purku avain|Purkuavain⟧ on ⟪product|Vaultissa⟫ polussa ⟪identifier|`secret/varmuus/avain`⟫; siihen pääsee käsiksi vain päivystäjän roolilla.

## Palautus ajanhetkeen

1. Ilmoita kanavalla ⟪identifier|#hairiot⟫, että palautus alkaa. Sovellus ⟦inflection|asetetän|asetetaan⟧ huoltotilaan.
2. Selvitä palautettava ajanhetki. Käytä ⟦punctuation|hetkeä joka|hetkeä, joka⟧ on vähintään viisi minuuttia ennen virheen ilmenemistä.
3. Aja palautus ⟪product|pgBackRestillä⟫:

```bash
pgbackrest --stanza=tuotanto --type=time \
  --target="2026-09-18 09:05:00+03" --target-action=promote restore
systemctl start postgresql
```

4. Tarkista, että tietokannan uusin varaus on odotetulta ajalta: ⟪code|`SELECT max(luotu) FROM varaukset;`⟫.
5. Poista huoltotila ja seuraa virhelokia ⟪unit|15 minuuttia⟫.

Palautus ⟪unit|200 GiB⟫:n kannalle kestää ⟪abbrev|n.⟫ ⟪unit|40 minuuttia⟫. Älä keskeytä sitä, ⟦compound_joined|vaikkase|vaikka se⟧ näyttäisi jumittuneen; ⟪acronym|WAL⟫-tiedostojen toisto on hidasta ⟦confusion*|vaan|mutta⟧ etenee.

## Tiedostojen palautus

Yksittäisen tiedoston voi palauttaa ilman huoltotilaa. Tiedostot on versioitu ⟪product|S3⟫:ssä, joten ⟦inflection*|poistettu|poistetun⟧ tiedoston saa takaisin komennolla:

```bash
aws s3api list-object-versions --bucket varmuus --prefix tiedostot/potilasohjeet/
aws s3api get-object --bucket varmuus --key tiedostot/potilasohjeet/ohje.pdf \
  --version-id <versio> ohje.pdf
```

Palautetun tiedoston tarkistussumma verrataan ⟦punctuation|lokiin ennen kuin|lokiin, ennen kuin⟧ se kopioidaan takaisin tuotantoon.

## Palautusharjoitus

Palautus harjoitellaan neljännesvuosittain ⟪code|staging⟫-ympäristössä. Harjoituksen tulos (kesto, ⟦typo|ongelmmat|ongelmat⟧, tarvittavat korjaukset) kirjataan ⟪product|Confluenceen⟫. Viime ⟦capitalization|Kesäkuun|kesäkuun⟧ harjoituksessa palautus kesti ⟪unit|52 minuuttia⟫, koska purkuavainta ei löytynyt heti; ohje päivitettiin sen jälkeen.

Jos ⟦compound_split|palautus testi|palautustesti⟧ epäonnistuu, siitä avataan ⟪acronym|SEV-2⟫-tiketti, vaikka tuotanto ei olisikaan vaarassa. Varmuuskopio, jota ei ole koskaan palautettu, ei ole varmuuskopio.
