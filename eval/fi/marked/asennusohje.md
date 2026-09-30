---
lang: fi
title: Asennusohje
---

# Asennusohje

Tämä ohje kuvaa, miten ⟪product|Kanta-yhteensopiva⟫ välityspalvelin asennetaan ⟪product|Ubuntu⟫ 24.04 -palvelimelle.
Ohje olettaa, että sinulla on pääkäyttäjän oikeudet ja että palvelimella on vähintään ⟪unit|8 GiB⟫ ⟦typo|muisita|muistia⟧.

## Esivaatimukset

Ennen asennusta tarkista, että seuraavat ohjelmistopaketit on asennettu:

- ⟪product|PostgreSQL⟫ 16 tai uudempi
- ⟪product|Docker⟫ ja ⟪product|Docker Compose⟫
- ⟪product|OpenSSL⟫ 3.x

Asennus epäonnistuu, jos tietokanta ei ole käynnissä. Tarkista tila komennolla ⟪code|`systemctl status postgresql`⟫.

Ensimmäisellä asennuskerralla ⟦typo|jäjestelmä|järjestelmä⟧ luo hakemiston ⟪code|`/var/lib/valitys`⟫ ja kirjoittaa sinne ⟦loan|konfiguratiotiedoston|konfiguraatiotiedoston⟧. Älä muokkaa tiedostoa käsin, ⟪abbrev|ks.⟫ luku ⟪ordinal|4. Asetukset⟫.

## Asennus

1. Lataa asennuspaketti osoitteesta ⟪url|https://example.fi/lataukset/valitys-2.3.1.tar.gz⟫.
2. Pura paketti ⟦inflection|hakemistohon|hakemistoon⟧ ja siirry sinne.
3. Aja ⟪code|`./asenna.sh --tuotanto`⟫.

```console
curl -LO https://example.fi/lataukset/valitys-2.3.1.tar.gz
tar xzf valitys-2.3.1.tar.gz
cd valitys-2.3.1 && ./asenna.sh --tuotanto
```

Asennusskripti kysyy ⟦compound_split|tieto kannan|tietokannan⟧ osoitteen ja tunnukset. Salasanaa ei tallenneta selväkielisenä, vaan se kirjoitetaan ⟪identifier|`VALITYS_DB_PASSWORD`⟫-ympäristömuuttujaan.

Kun skripti on valmis, palvelu käynnistyy ⟦inflection|automaattisestä|automaattisesti⟧. Voit tarkistaa lokin komennolla ⟪code|`journalctl -u valitys -f`⟫.

## Asetukset

Asetustiedosto on ⟪product|TOML⟫-muotoinen. Tärkeimmät avaimet:

| Avain | Kuvaus |
|-------|--------|
| ⟪identifier|`listen_port`⟫ | Portti, jota palvelin kuuntelee. Oletus ⟪number|8443⟫. |
| ⟪identifier|`db_url`⟫ | Tietokannan osoite. |
| ⟪identifier|`log_level`⟫ | Lokitaso. Sallitut arvot: ⟪code|`debug`⟫, ⟪code|`info`⟫, ⟪code|`warn`⟫. |

Portin vaihtamisen jälkeen palvelu täytyy käynnistää uudelleen. Muutokset tulevat voimaan vasta ⟦compound_joined|senjälkeen|sen jälkeen⟧.

## Varmistus

Avaa selaimessa ⟪url|https://localhost:8443/terveys⟫. Vastauksen pitäisi olla ⟪code|`{"tila":"ok"}`⟫. Jos vastaus on virhe, tarkista palomuurin asetukset ja katso lokista, ettei varmenne ole vanhentunut. Vanhentunut varmenne on yleisempi syy ⟦confusion*|kun|kuin⟧ luulisi.

Jos ongelma jatkuu, ota yhteyttä asiakaspalveluun tai avaa tiketti osoitteessa ⟪url|https://tuki.example.fi⟫.
