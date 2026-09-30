---
lang: fi
title: Asennusohje
---

# Asennusohje

Tämä ohje kuvaa, miten Kanta-yhteensopiva välityspalvelin asennetaan Ubuntu 24.04 -palvelimelle.
Ohje olettaa, että sinulla on pääkäyttäjän oikeudet ja että palvelimella on vähintään 8 GiB muisita.

## Esivaatimukset

Ennen asennusta tarkista, että seuraavat ohjelmistopaketit on asennettu:

- PostgreSQL 16 tai uudempi
- Docker ja Docker Compose
- OpenSSL 3.x

Asennus epäonnistuu, jos tietokanta ei ole käynnissä. Tarkista tila komennolla `systemctl status postgresql`.

Ensimmäisellä asennuskerralla jäjestelmä luo hakemiston `/var/lib/valitys` ja kirjoittaa sinne konfiguratiotiedoston. Älä muokkaa tiedostoa käsin, ks. luku 4. Asetukset.

## Asennus

1. Lataa asennuspaketti osoitteesta https://example.fi/lataukset/valitys-2.3.1.tar.gz.
2. Pura paketti hakemistohon ja siirry sinne.
3. Aja `./asenna.sh --tuotanto`.

```console
curl -LO https://example.fi/lataukset/valitys-2.3.1.tar.gz
tar xzf valitys-2.3.1.tar.gz
cd valitys-2.3.1 && ./asenna.sh --tuotanto
```

Asennusskripti kysyy tieto kannan osoitteen ja tunnukset. Salasanaa ei tallenneta selväkielisenä, vaan se kirjoitetaan `VALITYS_DB_PASSWORD`-ympäristömuuttujaan.

Kun skripti on valmis, palvelu käynnistyy automaattisestä. Voit tarkistaa lokin komennolla `journalctl -u valitys -f`.

## Asetukset

Asetustiedosto on TOML-muotoinen. Tärkeimmät avaimet:

| Avain | Kuvaus |
|-------|--------|
| `listen_port` | Portti, jota palvelin kuuntelee. Oletus 8443. |
| `db_url` | Tietokannan osoite. |
| `log_level` | Lokitaso. Sallitut arvot: `debug`, `info`, `warn`. |

Portin vaihtamisen jälkeen palvelu täytyy käynnistää uudelleen. Muutokset tulevat voimaan vasta senjälkeen.

## Varmistus

Avaa selaimessa https://localhost:8443/terveys. Vastauksen pitäisi olla `{"tila":"ok"}`. Jos vastaus on virhe, tarkista palomuurin asetukset ja katso lokista, ettei varmenne ole vanhentunut. Vanhentunut varmenne on yleisempi syy kun luulisi.

Jos ongelma jatkuu, ota yhteyttä asiakaspalveluun tai avaa tiketti osoitteessa https://tuki.example.fi.
