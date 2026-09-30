---
lang: fi
---

# Testaussuunnitelma: ajanvarausalusta 3.0

## Tavoite

Suunnitelma kuvaa, miten version 3.0 hyväksymis testaus toteutetaan ennen tuotantoon siirtoa. Testaus kattaa toiminnalliset vaatimukset, suorituskyvyn, tietoturvan ja saavutettavuuden.

## Testausympäristöt

| Ympäristö | Käyttö | Osoite |
|-----------|--------|--------|
| dev | Kehittäjien yksikkötestit | https://dev.varaus.example.fi |
| test | Automaattiset integraatiotestit | https://test.varaus.example.fi |
| staging | Hyväksymistestaus, tuotannon kopio | https://staging.varaus.example.fi |

staging-ympäristön data on anonyymisoitu kopio tuotannosta, joka päivitetään sunnuntaisin klo 03. Henkilötunnukset korvataan testitunnuksilla `010101-123N`-muodossa.

## Testitasot

### Yksikkötestit

Kehittäjät kirjoittavat yksikkötestit Go:n `testing`-paketilla ja Vitestillä. Kattavuus mitataan jokaisessa GitHub Actions -ajossa; raja on 80 %.

```bash
go test ./... -coverprofile=cover.out
go tool cover -func=cover.out | tail -n 1
```

### Integraatiotestit

Integraatiotestit ajetaan Testcontainersilla oikeaa PostgreSQL:ää ja RabbitMQ:ta vasten. Testit ajetän jokaisesta veto pyynnöstä ja ne saavat kestää enintään 10 minuuttia.

### Hyväksymistestit

Hyväksymistestit tekee tuoteomistaja yhdessä kolmen loppukäyttäjän kanssa. Testitapaukset on kirjattu TestRailiin ja ne perustuu käyttäjätarinoihin. Jokainen testitapaus hyväksytään tai hylätään, ja hylätyt kirjtaan vikoina Jiraan.

## Suorituskykytestit

Suorituskyky testataan k6:lla. Kuormaprofiili vastaa maanantaiaamun ruuhkaa: 300 samanaikaista käyttäjää 15 minuutin ajan. Hyväksymisraja on, että 95 % pyynnöistä valmistuu alle 500 ms:ssa eikä virheprosentti ylitä 0,1 %:a.

Testi ajetaan staging-ympäristössä, koska test-ympäristö on mitotettu pienemmäksi. Tulokset tallennetaan Grafanaan ja verrataan edelliseen julkaisuun.

## Tietoturvatestaus

Ulkopuolinen toimittaja tekee penetration-testauksen kahden viikon aikana Lokakuussa. Lisäksi OWASP ZAP ajetaan automaattisesti viikoittain. Kriittiset ja vakavat löydökset korjataan ennen julkaisua; keskitason löydöksille sovitaan aikataulu.

Riippuvuuksien haavoittuvuudet tarkistetaan Dependabotilla ja Trivyllä. Tunnetut vulnerabiliteetit eivät saa olla yli 30 päivää vanhoja.

## Saavutettavuustestaus

WCAG 2.1 AA -kriteerit tarkistetaan axe-työkalulla ja ruudun lukijalla (NVDA, VoiceOver). Testaukseen osallistuu kaksi näkövammaista käyttäjää Näkövammaisten liiton kautta.

## Aikataulu ja vastuut

| Vaihe | Vastuu | Valmis |
|-------|--------|--------|
| Yksikkö- ja integraatiotestit | Kehitystiimi | jatkuvasti |
| Hyväksymistestaus | Sirkka Selja | 17.10. |
| Suorituskykytestit | Pentti Saarikoski | 20.10. |
| Tietoturvatestaus | Ulkopuolinen toimittaja | 24.10. |
| Julkaisupäätös | Ohjausryhmä | 27.10. |

Jos jokin vaihe viivästyy yli kolme päivää, ohjausryhmä päättäa julkaisun siirtämisestä. Testauksen loppuraportti toimitetaan ohjausryhmälle viimeistään julkaisupäätöstä edeltävänä päivänä.
